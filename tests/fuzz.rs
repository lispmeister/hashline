/// Property-based fuzz tests for hashline core operations.
///
/// These use `proptest` to generate random inputs and verify invariants
/// that must always hold, regardless of input. Goals:
///   - No panics in any public function given arbitrary input
///   - Output format invariants (hash is always 2 hex chars, etc.)
///   - Round-trip properties (read then apply no-op edits = no change)
use hashline::*;
use proptest::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════
// compute_line_hash — no panics, always 2 hex chars
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #[test]
    fn fuzz_hash_always_2_hex_chars(line in ".*") {
        let hash = compute_line_hash(1, &line);
        prop_assert_eq!(hash.len(), 2);
        prop_assert!(hash.chars().all(|c| c.is_ascii_hexdigit()),
            "hash {:?} contains non-hex chars", hash);
    }

    #[test]
    fn fuzz_hash_whitespace_invariant(line in ".*") {
        // Stripping whitespace should not change the hash
        let stripped: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        prop_assert_eq!(
            compute_line_hash(1, &line),
            compute_line_hash(1, &stripped)
        );
    }

    #[test]
    fn fuzz_hash_line_index_ignored(line in ".*", idx in 1usize..100000) {
        // Line index is accepted but must not affect output
        prop_assert_eq!(
            compute_line_hash(idx, &line),
            compute_line_hash(1, &line)
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// parse_line_ref — no panics, consistent error/success
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #[test]
    fn fuzz_parse_line_ref_no_panic(s in ".*") {
        // Must not panic — either Ok or Err
        let _ = parse_line_ref(&s);
    }

    #[test]
    fn fuzz_parse_valid_ref_roundtrips(
        line in 1usize..100000,
        content in ".*"
    ) {
        // A properly formatted ref must always parse successfully
        let hash = compute_line_hash(line, &content);
        let ref_str = format!("{}:{}", line, hash);
        let parsed = parse_line_ref(&ref_str);
        prop_assert!(parsed.is_ok(), "failed to parse {:?}: {:?}", ref_str, parsed);
        let r = parsed.unwrap();
        prop_assert_eq!(r.line, line);
        prop_assert_eq!(&r.hash, &hash);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// format_hashlines — no panics, output is parseable
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #[test]
    fn fuzz_format_no_panic(content in ".*", start in 1usize..100000) {
        let _ = format_hashlines(&content, start);
    }

    #[test]
    fn fuzz_format_line_count_matches(
        lines in prop::collection::vec(".*", 1..20),
        start in 1usize..1000
    ) {
        let content = lines.join("\n");
        let formatted = format_hashlines(&content, start);
        let out_lines: Vec<&str> = formatted.split('\n').collect();
        prop_assert_eq!(out_lines.len(), lines.len());
    }

    #[test]
    fn fuzz_format_line_numbers_are_sequential(
        lines in prop::collection::vec("[^\n]*", 1..20),
        start in 1usize..1000
    ) {
        let content = lines.join("\n");
        let formatted = format_hashlines(&content, start);
        for (i, out) in formatted.split('\n').enumerate() {
            let expected_num = start + i;
            prop_assert!(
                out.starts_with(&format!("{}:", expected_num)),
                "line {} should start with {}:, got {:?}", i, expected_num, out
            );
        }
    }

    #[test]
    fn fuzz_format_hashes_verify(
        lines in prop::collection::vec("[^\n]*", 1..20),
        start in 1usize..1000
    ) {
        // Every output line's hash must match compute_line_hash of its content
        let content = lines.join("\n");
        let formatted = format_hashlines(&content, start);
        for (i, out) in formatted.split('\n').enumerate() {
            let pipe = out.find('|').expect("no pipe separator");
            let prefix = &out[..pipe];
            let content_part = &out[pipe + 1..];
            let colon = prefix.find(':').expect("no colon");
            let num: usize = prefix[..colon].parse().expect("non-numeric line num");
            let hash = &prefix[colon + 1..];
            prop_assert_eq!(
                compute_line_hash(num, content_part), hash,
                "hash mismatch on line {}", i
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// apply_hashline_edits — no panics on arbitrary input
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    #[test]
    fn fuzz_apply_empty_edits_is_noop(content in "[^\x00]*") {
        // Empty edit list must never panic and must return content unchanged
        let result = apply_hashline_edits(&content, &[]);
        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap().content, content);
    }

    #[test]
    fn fuzz_apply_set_line_no_panic(
        lines in prop::collection::vec("[^\n\x00]*", 1..20),
        target_line in 1usize..20,
        new_text in "[^\n\x00]*"
    ) {
        let content = lines.join("\n");
        // Use a deliberately wrong hash — engine should return Err, not panic
        let anchor = format!("{}:zz", target_line);
        let edits = vec![HashlineEdit::SetLine {
            set_line: hashline::edit::SetLineOp {
                anchor,
                new_text,
            },
        }];
        // Must not panic regardless of whether it succeeds or fails
        let _ = apply_hashline_edits(&content, &edits);
    }

    #[test]
    fn fuzz_apply_correct_anchor_succeeds(
        lines in prop::collection::vec("[^\n\x00]*", 1..10),
        target_idx in 0usize..10,
        new_text in "[^\n\x00]*"
    ) {
        let content = lines.join("\n");
        let file_lines: Vec<&str> = content.split('\n').collect();
        let n = file_lines.len();
        let idx = target_idx % n;
        let line_num = idx + 1;
        let anchor = format!("{}:{}", line_num, compute_line_hash(line_num, file_lines[idx]));
        let edits = vec![HashlineEdit::SetLine {
            set_line: hashline::edit::SetLineOp {
                anchor,
                new_text: new_text.clone(),
            },
        }];
        let result = apply_hashline_edits(&content, &edits);
        // With a valid anchor the edit must succeed (unless new_text triggers a heuristic no-op check)
        prop_assert!(result.is_ok(), "edit failed: {:?}", result);
    }
}

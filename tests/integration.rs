use hashline::{apply_replace_edits, *};
use std::process::Command;

fn hashline_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hashline"))
}

fn make_ref(line_num: usize, content: &str) -> String {
    format!("{}:{}", line_num, compute_line_hash(line_num, content))
}

// ═══════════════════════════════════════════════════════════════════════════
// Hash compatibility — vectors generated from Bun.hash.xxHash32(normalized, 0) % 256
// Verifies our Rust implementation is byte-for-byte compatible with the TS reference.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn hash_compat_bun_vectors() {
    // Generated via: Bun.hash.xxHash32(line.replace(/\s/g, ""), 0) % 256
    let vectors: &[(&str, &str)] = &[
        ("", "05"),
        ("hello", "f9"),
        ("world", "b3"),
        ("function hello() {", "42"),
        ("  return 42;", "90"),
        ("}", "18"),
        ("    const x = 1;", "7d"),
        ("use std::io;", "a4"),
        // whitespace normalization: these two must produce the same hash
        ("  hello  world  ", "02"),
        ("helloworld", "02"),
    ];
    for (line, expected) in vectors {
        let got = compute_line_hash(1, line);
        assert_eq!(
            &got, expected,
            "hash mismatch for {:?}: got {} expected {}",
            line, got, expected
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// computeLineHash
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn hash_returns_2_char_hex() {
    let hash = compute_line_hash(1, "hello");
    assert_eq!(hash.len(), 2);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn hash_same_content_same_hash() {
    assert_eq!(compute_line_hash(1, "hello"), compute_line_hash(1, "hello"));
}

#[test]
fn hash_different_content_different_hash() {
    assert_ne!(compute_line_hash(1, "hello"), compute_line_hash(1, "world"));
}

#[test]
fn hash_empty_line() {
    let hash = compute_line_hash(1, "");
    assert_eq!(hash.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// formatHashLines
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn format_single_line() {
    let result = format_hashlines("hello", 1);
    let hash = compute_line_hash(1, "hello");
    assert_eq!(result, format!("1:{}|hello", hash));
}

#[test]
fn format_multiple_lines() {
    let result = format_hashlines("foo\nbar\nbaz", 1);
    let lines: Vec<&str> = result.split('\n').collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("1:"));
    assert!(lines[1].starts_with("2:"));
    assert!(lines[2].starts_with("3:"));
}

#[test]
fn format_custom_start_line() {
    let result = format_hashlines("foo\nbar", 10);
    let lines: Vec<&str> = result.split('\n').collect();
    assert!(lines[0].starts_with("10:"));
    assert!(lines[1].starts_with("11:"));
}

#[test]
fn format_round_trip() {
    let content = "function hello() {\n  return 42;\n}";
    let formatted = format_hashlines(content, 1);
    for line in formatted.split('\n') {
        let pipe = line.find('|').unwrap();
        let prefix = &line[..pipe];
        let content_part = &line[pipe + 1..];
        let colon = prefix.find(':').unwrap();
        let num: usize = prefix[..colon].parse().unwrap();
        let hash = &prefix[colon + 1..];
        assert_eq!(compute_line_hash(num, content_part), hash);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// parseLineRef
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn parse_valid_ref() {
    let r = parse_line_ref("5:abcd").unwrap();
    assert_eq!(
        r,
        LineRef {
            line: 5,
            hash: "abcd".into()
        }
    );
}

#[test]
fn parse_single_digit_hash() {
    let r = parse_line_ref("1:a").unwrap();
    assert_eq!(
        r,
        LineRef {
            line: 1,
            hash: "a".into()
        }
    );
}

#[test]
fn parse_long_hash() {
    let r = parse_line_ref("100:abcdef0123456789").unwrap();
    assert_eq!(
        r,
        LineRef {
            line: 100,
            hash: "abcdef0123456789".into()
        }
    );
}

#[test]
fn parse_rejects_missing_colon() {
    assert!(parse_line_ref("5abcd").is_err());
}

#[test]
fn parse_rejects_non_numeric_line() {
    assert!(parse_line_ref("abc:1234").is_err());
}

#[test]
fn parse_rejects_non_alphanumeric_hash() {
    assert!(parse_line_ref("5:$$$$").is_err());
}

#[test]
fn parse_rejects_line_0() {
    let err = parse_line_ref("0:abcd").unwrap_err();
    assert!(err.contains(">= 1"));
}

#[test]
fn parse_rejects_empty() {
    assert!(parse_line_ref("").is_err());
}

#[test]
fn parse_rejects_empty_hash() {
    assert!(parse_line_ref("5:").is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// applyHashlineEdits — replace
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn edit_replace_single_line() {
    let content = "aaa\nbbb\nccc";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(2, "bbb"),
            new_text: "BBB".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nBBB\nccc");
    assert_eq!(result.first_changed_line, Some(2));
}

#[test]
fn edit_range_replace_shrink() {
    let content = "aaa\nbbb\nccc\nddd";
    let edits = vec![HashlineEdit::ReplaceLines {
        replace_lines: hashline::edit::ReplaceLinesOp {
            start_anchor: make_ref(2, "bbb"),
            end_anchor: Some(make_ref(3, "ccc")),
            new_text: Some("ONE".into()),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nONE\nddd");
}

#[test]
fn edit_range_replace_same_count() {
    let content = "aaa\nbbb\nccc\nddd";
    let edits = vec![HashlineEdit::ReplaceLines {
        replace_lines: hashline::edit::ReplaceLinesOp {
            start_anchor: make_ref(2, "bbb"),
            end_anchor: Some(make_ref(3, "ccc")),
            new_text: Some("XXX\nYYY".into()),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nXXX\nYYY\nddd");
    assert_eq!(result.first_changed_line, Some(2));
}

#[test]
fn edit_replace_first_line() {
    let content = "first\nsecond\nthird";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(1, "first"),
            new_text: "FIRST".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "FIRST\nsecond\nthird");
    assert_eq!(result.first_changed_line, Some(1));
}

#[test]
fn edit_replace_last_line() {
    let content = "first\nsecond\nthird";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(3, "third"),
            new_text: "THIRD".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "first\nsecond\nTHIRD");
}

// ═══════════════════════════════════════════════════════════════════════════
// applyHashlineEdits — delete
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn edit_delete_single_line() {
    let content = "aaa\nbbb\nccc";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(2, "bbb"),
            new_text: "".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nccc");
    assert_eq!(result.first_changed_line, Some(2));
}

#[test]
fn edit_delete_range() {
    let content = "aaa\nbbb\nccc\nddd";
    let edits = vec![HashlineEdit::ReplaceLines {
        replace_lines: hashline::edit::ReplaceLinesOp {
            start_anchor: make_ref(2, "bbb"),
            end_anchor: Some(make_ref(3, "ccc")),
            new_text: Some("".into()),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nddd");
}

#[test]
fn edit_delete_first_line() {
    let content = "aaa\nbbb\nccc";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(1, "aaa"),
            new_text: "".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "bbb\nccc");
}

#[test]
fn edit_delete_last_line() {
    let content = "aaa\nbbb\nccc";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(3, "ccc"),
            new_text: "".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nbbb");
}

// ═══════════════════════════════════════════════════════════════════════════
// applyHashlineEdits — insert
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn edit_insert_after_line() {
    let content = "aaa\nbbb\nccc";
    let edits = vec![HashlineEdit::InsertAfter {
        insert_after: hashline::edit::InsertAfterOp {
            anchor: make_ref(1, "aaa"),
            text: Some("NEW".into()),
            content: None,
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nNEW\nbbb\nccc");
    assert_eq!(result.first_changed_line, Some(2));
}

#[test]
fn edit_insert_multiple_lines() {
    let content = "aaa\nbbb";
    let edits = vec![HashlineEdit::InsertAfter {
        insert_after: hashline::edit::InsertAfterOp {
            anchor: make_ref(1, "aaa"),
            text: Some("x\ny\nz".into()),
            content: None,
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nx\ny\nz\nbbb");
}

#[test]
fn edit_insert_after_last_line() {
    let content = "aaa\nbbb";
    let edits = vec![HashlineEdit::InsertAfter {
        insert_after: hashline::edit::InsertAfterOp {
            anchor: make_ref(2, "bbb"),
            text: Some("NEW".into()),
            content: None,
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nbbb\nNEW");
}

#[test]
fn edit_insert_empty_dst_throws() {
    let content = "aaa\nbbb";
    let edits = vec![HashlineEdit::InsertAfter {
        insert_after: hashline::edit::InsertAfterOp {
            anchor: make_ref(1, "aaa"),
            text: Some("".into()),
            content: None,
        },
    }];
    assert!(apply_hashline_edits(content, &edits).is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// applyHashlineEdits — heuristics
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn heuristic_strips_insert_anchor_echo() {
    let content = "aaa\nbbb\nccc";
    let edits = vec![HashlineEdit::InsertAfter {
        insert_after: hashline::edit::InsertAfterOp {
            anchor: make_ref(2, "bbb"),
            text: Some("bbb\nNEW".into()),
            content: None,
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nbbb\nNEW\nccc");
}

#[test]
fn heuristic_strips_range_boundary_echo() {
    let content = [
        "import { foo } from 'x';",
        "if (cond) {",
        "  doA();",
        "} else {",
        "  doB();",
        "}",
        "after();",
    ]
    .join("\n");

    let edits = vec![HashlineEdit::ReplaceLines {
        replace_lines: hashline::edit::ReplaceLinesOp {
            start_anchor: make_ref(2, "if (cond) {"),
            end_anchor: Some(make_ref(6, "}")),
            new_text: Some(
                [
                    "if (cond) {",
                    "  doA();",
                    "} else {",
                    "  doB();",
                    "}",
                    "after();",
                ]
                .join("\n"),
            ),
        },
    }];

    let result = apply_hashline_edits(&content, &edits).unwrap();
    assert_eq!(result.content.split('\n').count(), 7);
    assert_eq!(result.content, content);
}

#[test]
fn heuristic_restores_wrapped_line() {
    let long_line =
        "const options = veryLongIdentifier + anotherLongIdentifier + thirdLongIdentifier + fourthLongIdentifier;";
    let content = format!("before();\n{}\nafter();", long_line);
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(2, long_line),
            new_text: [
                "const",
                "options",
                "=",
                "veryLongIdentifier",
                "+",
                "anotherLongIdentifier",
                "+",
                "thirdLongIdentifier",
                "+",
                "fourthLongIdentifier;",
            ]
            .join("\n"),
        },
    }];
    let result = apply_hashline_edits(&content, &edits).unwrap();
    assert_eq!(result.content, content);
}

#[test]
fn heuristic_merge_absorbed_next_line() {
    let content =
        "    typeof HOOK === 'undefined' &&\n    typeof HOOK.checkDCE !== 'function'\ntail();";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(1, "    typeof HOOK === 'undefined' &&"),
            new_text: "typeof HOOK === 'undefined' || typeof HOOK.checkDCE !== 'function'".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(
        result.content,
        "    typeof HOOK === 'undefined' || typeof HOOK.checkDCE !== 'function'\ntail();"
    );
}

#[test]
fn heuristic_merge_absorbed_prev_line() {
    let content = "  const nativeStyleResolver: ResolveNativeStyle | void =\n    resolveRNStyle || hook.resolveRNStyle;\n  after();";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(2, "    resolveRNStyle || hook.resolveRNStyle;"),
            new_text: "const nativeStyleResolver: ResolveNativeStyle | void = resolveRNStyle ?? hook.resolveRNStyle;".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(
        result.content,
        "  const nativeStyleResolver: ResolveNativeStyle | void = resolveRNStyle ?? hook.resolveRNStyle;\n  after();"
    );
}

#[test]
fn heuristic_polluted_anchor() {
    let content = "aaa\nbbb\nccc";
    let src_hash = compute_line_hash(2, "bbb");
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: format!("2:{}export function foo(a, b) {{}}", src_hash),
            new_text: "BBB".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nBBB\nccc");
}

#[test]
fn heuristic_same_line_range_as_single() {
    let content = "aaa\nbbb\nccc";
    let good = make_ref(2, "bbb");
    let edits = vec![HashlineEdit::ReplaceLines {
        replace_lines: hashline::edit::ReplaceLinesOp {
            start_anchor: good.clone(),
            end_anchor: Some(good),
            new_text: Some("BBB".into()),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nBBB\nccc");
}

#[test]
fn heuristic_normalize_confusable_hyphens_on_noop() {
    // en-dash \u{2013}
    let content = "aaa\ndevtools\u{2013}unsupported-bridge-protocol\nccc";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(2, "devtools\u{2013}unsupported-bridge-protocol"),
            new_text: "devtools\u{2013}unsupported-bridge-protocol".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(
        result.content,
        "aaa\ndevtools-unsupported-bridge-protocol\nccc"
    );
}

#[test]
fn heuristic_strips_hashline_prefixes_from_new_text() {
    // Model echoes hashline-formatted lines as the replacement content
    let content = "aaa\nbbb\nccc";
    let hash_bbb = compute_line_hash(2, "bbb");
    let new_text = format!("2:{}|BBB", hash_bbb);
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(2, "bbb"),
            new_text,
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nBBB\nccc");
}

#[test]
fn heuristic_strips_diff_plus_prefix_from_new_text() {
    // Model outputs unified-diff style lines as replacement
    let content = "aaa\nbbb\nccc";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(2, "bbb"),
            new_text: "+BBB".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nBBB\nccc");
}

#[test]
fn heuristic_restores_indent_for_paired_replacement() {
    // Same line count, model drops leading indent on one line
    let content = "    foo();\n    bar();\n    baz();";
    let edits = vec![HashlineEdit::ReplaceLines {
        replace_lines: hashline::edit::ReplaceLinesOp {
            start_anchor: make_ref(1, "    foo();"),
            end_anchor: Some(make_ref(2, "    bar();")),
            new_text: Some("foo();\nbar();".into()), // indent stripped by model
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "    foo();\n    bar();\n    baz();");
}

// ═══════════════════════════════════════════════════════════════════════════
// applyHashlineEdits — multiple edits
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn edit_two_non_overlapping_replaces() {
    let content = "aaa\nbbb\nccc\nddd\neee";
    let edits = vec![
        HashlineEdit::SetLine {
            set_line: hashline::edit::SetLineOp {
                anchor: make_ref(2, "bbb"),
                new_text: "BBB".into(),
            },
        },
        HashlineEdit::SetLine {
            set_line: hashline::edit::SetLineOp {
                anchor: make_ref(4, "ddd"),
                new_text: "DDD".into(),
            },
        },
    ];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nBBB\nccc\nDDD\neee");
    assert_eq!(result.first_changed_line, Some(2));
}

#[test]
fn edit_replace_plus_delete() {
    let content = "aaa\nbbb\nccc\nddd";
    let edits = vec![
        HashlineEdit::SetLine {
            set_line: hashline::edit::SetLineOp {
                anchor: make_ref(2, "bbb"),
                new_text: "BBB".into(),
            },
        },
        HashlineEdit::SetLine {
            set_line: hashline::edit::SetLineOp {
                anchor: make_ref(4, "ddd"),
                new_text: "".into(),
            },
        },
    ];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nBBB\nccc");
}

#[test]
fn edit_replace_plus_insert() {
    let content = "aaa\nbbb\nccc";
    let edits = vec![
        HashlineEdit::SetLine {
            set_line: hashline::edit::SetLineOp {
                anchor: make_ref(3, "ccc"),
                new_text: "CCC".into(),
            },
        },
        HashlineEdit::InsertAfter {
            insert_after: hashline::edit::InsertAfterOp {
                anchor: make_ref(1, "aaa"),
                text: Some("INSERTED".into()),
                content: None,
            },
        },
    ];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nINSERTED\nbbb\nCCC");
}

#[test]
fn edit_empty_edits_noop() {
    let content = "aaa\nbbb";
    let result = apply_hashline_edits(content, &[]).unwrap();
    assert_eq!(result.content, content);
    assert_eq!(result.first_changed_line, None);
}

// ═══════════════════════════════════════════════════════════════════════════
// applyHashlineEdits — errors
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn error_stale_hash() {
    let content = "aaa\nbbb\nccc";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: "2:zz".into(),
            new_text: "BBB".into(),
        },
    }];
    let err = apply_hashline_edits(content, &edits).unwrap_err();
    assert!(err.downcast_ref::<HashlineMismatchError>().is_some());
}

#[test]
fn error_stale_hash_shows_markers() {
    let content = "aaa\nbbb\nccc\nddd\neee";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: "2:zz".into(),
            new_text: "BBB".into(),
        },
    }];
    let err = apply_hashline_edits(content, &edits).unwrap_err();
    let mismatch = err.downcast_ref::<HashlineMismatchError>().unwrap();
    let msg = mismatch.format_message();
    assert!(msg.contains(">>>"));
    let correct_hash = compute_line_hash(2, "bbb");
    assert!(msg.contains(&format!("2:{}|bbb", correct_hash)));
}

#[test]
fn error_collects_all_mismatches() {
    let content = "aaa\nbbb\nccc\nddd\neee";
    let edits = vec![
        HashlineEdit::SetLine {
            set_line: hashline::edit::SetLineOp {
                anchor: "2:zz".into(),
                new_text: "BBB".into(),
            },
        },
        HashlineEdit::SetLine {
            set_line: hashline::edit::SetLineOp {
                anchor: "4:zz".into(),
                new_text: "DDD".into(),
            },
        },
    ];
    let err = apply_hashline_edits(content, &edits).unwrap_err();
    let mismatch = err.downcast_ref::<HashlineMismatchError>().unwrap();
    assert_eq!(mismatch.mismatches.len(), 2);
    let msg = mismatch.format_message();
    let marker_lines: Vec<&str> = msg.split('\n').filter(|l| l.starts_with(">>>")).collect();
    assert_eq!(marker_lines.len(), 2);
}

#[test]
fn error_relocates_unique_hash() {
    let content = "aaa\nbbb\nccc";
    // ccc's hash at line 3, but ref says line 2
    let stale = format!("2:{}", compute_line_hash(3, "ccc"));
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: stale,
            new_text: "CCC".into(),
        },
    }];
    let result = apply_hashline_edits(content, &edits).unwrap();
    assert_eq!(result.content, "aaa\nbbb\nCCC");
}

#[test]
fn error_no_relocate_duplicate_hash() {
    let content = "dup\nmid\ndup";
    let stale = format!("2:{}", compute_line_hash(1, "dup"));
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: stale,
            new_text: "DUP".into(),
        },
    }];
    assert!(apply_hashline_edits(content, &edits).is_err());
}

#[test]
fn error_out_of_range_line() {
    let content = "aaa\nbbb";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: "10:aa".into(),
            new_text: "X".into(),
        },
    }];
    let err = apply_hashline_edits(content, &edits).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn error_range_start_gt_end() {
    let content = "aaa\nbbb\nccc\nddd\neee";
    let edits = vec![HashlineEdit::ReplaceLines {
        replace_lines: hashline::edit::ReplaceLinesOp {
            start_anchor: make_ref(5, "eee"),
            end_anchor: Some(make_ref(2, "bbb")),
            new_text: Some("X".into()),
        },
    }];
    assert!(apply_hashline_edits(content, &edits).is_err());
}

#[test]
fn error_insert_empty_dst() {
    let content = "aaa\nbbb";
    let edits = vec![HashlineEdit::InsertAfter {
        insert_after: hashline::edit::InsertAfterOp {
            anchor: make_ref(1, "aaa"),
            text: Some("".into()),
            content: None,
        },
    }];
    assert!(apply_hashline_edits(content, &edits).is_err());
}

#[test]
fn error_reject_replace_edit() {
    let content = "aaa\nbbb";
    let edits = vec![HashlineEdit::Replace {
        replace: hashline::edit::ReplaceOp {
            old_text: "aaa".into(),
            new_text: "AAA".into(),
        },
    }];
    let err = apply_hashline_edits(content, &edits).unwrap_err();
    assert!(err
        .to_string()
        .contains("replace edits are applied separately"));
}

// ═══════════════════════════════════════════════════════════════════════════
// apply_replace_edits
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn replace_basic_substitution() {
    let content = "hello world\ngoodbye world";
    let edits = vec![HashlineEdit::Replace {
        replace: hashline::edit::ReplaceOp {
            old_text: "hello world".into(),
            new_text: "hi world".into(),
        },
    }];
    let result = apply_replace_edits(content, &edits).unwrap();
    assert_eq!(result.content, "hi world\ngoodbye world");
    assert_eq!(result.replacements, 1);
}

#[test]
fn replace_multiline_old_text() {
    let content = "fn foo() {\n    let x = 1;\n}\n";
    let edits = vec![HashlineEdit::Replace {
        replace: hashline::edit::ReplaceOp {
            old_text: "let x = 1;".into(),
            new_text: "let x = 42;".into(),
        },
    }];
    let result = apply_replace_edits(content, &edits).unwrap();
    assert_eq!(result.content, "fn foo() {\n    let x = 42;\n}\n");
}

#[test]
fn replace_errors_on_not_found() {
    let content = "aaa\nbbb\nccc";
    let edits = vec![HashlineEdit::Replace {
        replace: hashline::edit::ReplaceOp {
            old_text: "zzz".into(),
            new_text: "ZZZ".into(),
        },
    }];
    let err = apply_replace_edits(content, &edits).unwrap_err();
    assert!(err.to_string().contains("not found"), "err: {}", err);
}

#[test]
fn replace_errors_on_ambiguous_match() {
    let content = "foo\nfoo\nbar";
    let edits = vec![HashlineEdit::Replace {
        replace: hashline::edit::ReplaceOp {
            old_text: "foo".into(),
            new_text: "FOO".into(),
        },
    }];
    let err = apply_replace_edits(content, &edits).unwrap_err();
    assert!(err.to_string().contains("matches 2"), "err: {}", err);
}

#[test]
fn replace_errors_on_empty_old_text() {
    let content = "aaa";
    let edits = vec![HashlineEdit::Replace {
        replace: hashline::edit::ReplaceOp {
            old_text: "".into(),
            new_text: "x".into(),
        },
    }];
    assert!(apply_replace_edits(content, &edits).is_err());
}

#[test]
fn replace_skips_anchor_edits() {
    // apply_replace_edits should silently ignore non-replace edits
    let content = "aaa\nbbb";
    let edits = vec![HashlineEdit::SetLine {
        set_line: hashline::edit::SetLineOp {
            anchor: make_ref(1, "aaa"),
            new_text: "AAA".into(),
        },
    }];
    let result = apply_replace_edits(content, &edits).unwrap();
    assert_eq!(result.content, content); // unchanged
    assert_eq!(result.replacements, 0);
}

#[test]
fn replace_multiple_ops_sequential() {
    let content = "alpha beta gamma";
    let edits = vec![
        HashlineEdit::Replace {
            replace: hashline::edit::ReplaceOp {
                old_text: "alpha".into(),
                new_text: "ALPHA".into(),
            },
        },
        HashlineEdit::Replace {
            replace: hashline::edit::ReplaceOp {
                old_text: "gamma".into(),
                new_text: "GAMMA".into(),
            },
        },
    ];
    let result = apply_replace_edits(content, &edits).unwrap();
    assert_eq!(result.content, "ALPHA beta GAMMA");
    assert_eq!(result.replacements, 2);
}

#[test]
fn replace_json_roundtrip() {
    let json = r#"{"replace":{"old_text":"foo","new_text":"bar"}}"#;
    let edit: HashlineEdit = serde_json::from_str(json).unwrap();
    match &edit {
        HashlineEdit::Replace { replace } => {
            assert_eq!(replace.old_text, "foo");
            assert_eq!(replace.new_text, "bar");
        }
        _ => panic!("expected Replace"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// JSON deserialization
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn json_deserialize_set_line() {
    let json = r#"{"set_line":{"anchor":"2:ab","new_text":"hello"}}"#;
    let edit: HashlineEdit = serde_json::from_str(json).unwrap();
    match &edit {
        HashlineEdit::SetLine { set_line } => {
            assert_eq!(set_line.anchor, "2:ab");
            assert_eq!(set_line.new_text, "hello");
        }
        _ => panic!("expected SetLine"),
    }
}

#[test]
fn json_deserialize_params() {
    let json =
        r#"{"path":"/tmp/test.rs","edits":[{"set_line":{"anchor":"1:ab","new_text":"hi"}}]}"#;
    let params: hashline::HashlineParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.path, "/tmp/test.rs");
    assert_eq!(params.edits.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// CLI — argument validation
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn cli_rejects_start_line_zero() {
    let output = hashline_bin()
        .args(["read", "--start-line", "0", "src/cli.rs"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value '0'"), "stderr: {}", stderr);
}

#[test]
fn cli_rejects_negative_start_line() {
    let output = hashline_bin()
        .args(["read", "--start-line", "-1", "src/cli.rs"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn cli_rejects_lines_zero() {
    let output = hashline_bin()
        .args(["read", "--lines", "0", "src/cli.rs"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value '0'"), "stderr: {}", stderr);
}

#[test]
fn cli_rejects_negative_lines() {
    let output = hashline_bin()
        .args(["read", "--lines", "-1", "src/cli.rs"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn cli_start_line_and_lines_work_together() {
    let output = hashline_bin()
        .args(["read", "--start-line", "2", "--lines", "2", "src/cli.rs"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("2:"));
    assert!(lines[1].starts_with("3:"));
}

#[test]
fn cli_start_line_beyond_file_produces_no_output() {
    let output = hashline_bin()
        .args(["read", "--start-line", "99999", "src/cli.rs"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn cli_rejects_start_line_above_u32_max() {
    let above = (u32::MAX as u64 + 1).to_string();
    let output = hashline_bin()
        .args(["read", "--start-line", &above, "src/cli.rs"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value"), "stderr: {}", stderr);
}

#[test]
fn cli_rejects_lines_above_u32_max() {
    let above = (u32::MAX as u64 + 1).to_string();
    let output = hashline_bin()
        .args(["read", "--lines", &above, "src/cli.rs"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid value"), "stderr: {}", stderr);
}

#[test]
fn cli_accepts_u32_max_start_line() {
    let max = u32::MAX.to_string();
    let output = hashline_bin()
        .args(["read", "--start-line", &max, "src/cli.rs"])
        .output()
        .unwrap();
    assert!(output.status.success());
    // File is much smaller, so no output — but the value is accepted
    assert!(output.stdout.is_empty());
}

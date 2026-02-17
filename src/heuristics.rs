use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Pattern matching hashline display format: `LINE:HASH|CONTENT`
static HASHLINE_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:>>>|>>)?\s*\d+:[0-9a-zA-Z]{1,16}\|").unwrap());

/// Check if a line starts with a unified-diff `+` prefix (but not `++`).
fn has_diff_plus_prefix(s: &str) -> bool {
    s.starts_with('+') && !s.starts_with("++")
}

/// Strip the leading `+` from a diff line.
fn strip_diff_plus(s: &str) -> String {
    if has_diff_plus_prefix(s) {
        s[1..].to_string()
    } else {
        s.to_string()
    }
}

/// Unicode confusable hyphens
static CONFUSABLE_HYPHENS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("[\u{2010}\u{2011}\u{2012}\u{2013}\u{2014}\u{2212}\u{FE63}\u{FF0D}]").unwrap()
});

/// Check if a string contains confusable hyphens.
pub fn has_confusable_hyphens(s: &str) -> bool {
    CONFUSABLE_HYPHENS_RE.is_match(s)
}

/// Replace confusable Unicode hyphens with ASCII hyphen.
pub fn normalize_confusable_hyphens(s: &str) -> String {
    CONFUSABLE_HYPHENS_RE.replace_all(s, "-").to_string()
}

pub fn normalize_confusable_hyphens_in_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| normalize_confusable_hyphens(l))
        .collect()
}

fn strip_all_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn leading_whitespace(s: &str) -> &str {
    let end = s.len() - s.trim_start().len();
    &s[..end]
}

fn restore_leading_indent(template_line: &str, line: &str) -> String {
    if line.is_empty() {
        return line.to_string();
    }
    let template_indent = leading_whitespace(template_line);
    if template_indent.is_empty() {
        return line.to_string();
    }
    let indent = leading_whitespace(line);
    if !indent.is_empty() {
        return line.to_string();
    }
    format!("{}{}", template_indent, line)
}

fn equals_ignoring_whitespace(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    strip_all_whitespace(a) == strip_all_whitespace(b)
}

static TRAILING_CONTINUATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:&&|\|\||\?\?|\?|:|=|,|\+|-|\*|/|\.|\()\s*$").unwrap());

fn strip_trailing_continuation_tokens(s: &str) -> String {
    TRAILING_CONTINUATION_RE.replace(s, "").to_string()
}

fn strip_merge_operator_chars(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '|' | '&' | '?'))
        .collect()
}

/// Strip hashline display prefixes and diff `+` markers from replacement lines.
pub fn strip_new_line_prefixes(lines: &[String]) -> Vec<String> {
    let mut hash_prefix_count = 0;
    let mut diff_plus_count = 0;
    let mut non_empty = 0;

    for l in lines {
        if l.is_empty() {
            continue;
        }
        non_empty += 1;
        if HASHLINE_PREFIX_RE.is_match(l) {
            hash_prefix_count += 1;
        }
        if has_diff_plus_prefix(l) {
            diff_plus_count += 1;
        }
    }

    if non_empty == 0 {
        return lines.to_vec();
    }

    let strip_hash = hash_prefix_count > 0 && hash_prefix_count * 2 >= non_empty;
    let strip_plus = !strip_hash && diff_plus_count > 0 && diff_plus_count * 2 >= non_empty;

    if !strip_hash && !strip_plus {
        return lines.to_vec();
    }

    lines
        .iter()
        .map(|l| {
            if strip_hash {
                HASHLINE_PREFIX_RE.replace(l, "").to_string()
            } else if strip_plus {
                strip_diff_plus(l)
            } else {
                l.clone()
            }
        })
        .collect()
}

/// Restore indentation for paired old/new replacement lines.
pub fn restore_indent_for_paired_replacement(
    old_lines: &[String],
    new_lines: &[String],
) -> Vec<String> {
    if old_lines.len() != new_lines.len() {
        return new_lines.to_vec();
    }
    let mut changed = false;
    let mut out = Vec::with_capacity(new_lines.len());
    for (old, new) in old_lines.iter().zip(new_lines.iter()) {
        let restored = restore_leading_indent(old, new);
        if restored != *new {
            changed = true;
        }
        out.push(restored);
    }
    if changed {
        out
    } else {
        new_lines.to_vec()
    }
}

/// Undo pure formatting rewrites where the model reflows a single logical line
/// into multiple lines (or similar), but the token stream is identical.
pub fn restore_old_wrapped_lines(old_lines: &[String], new_lines: &[String]) -> Vec<String> {
    if old_lines.is_empty() || new_lines.len() < 2 {
        return new_lines.to_vec();
    }

    let mut canon_to_old: HashMap<String, (String, usize)> = HashMap::new();
    for line in old_lines {
        let canon = strip_all_whitespace(line);
        let entry = canon_to_old
            .entry(canon)
            .or_insert_with(|| (line.clone(), 0));
        entry.1 += 1;
    }

    struct Candidate {
        start: usize,
        len: usize,
        replacement: String,
        canon: String,
    }

    let mut candidates = Vec::new();
    for start in 0..new_lines.len() {
        for len in 2..=10.min(new_lines.len() - start) {
            let joined: String = new_lines[start..start + len].concat();
            let canon_span = strip_all_whitespace(&joined);
            if let Some((old_line, count)) = canon_to_old.get(&canon_span) {
                if *count == 1 && canon_span.len() >= 6 {
                    candidates.push(Candidate {
                        start,
                        len,
                        replacement: old_line.clone(),
                        canon: canon_span,
                    });
                }
            }
        }
    }

    if candidates.is_empty() {
        return new_lines.to_vec();
    }

    // Keep only spans whose canonical match is unique in the new output
    let mut canon_counts: HashMap<String, usize> = HashMap::new();
    for c in &candidates {
        *canon_counts.entry(c.canon.clone()).or_insert(0) += 1;
    }
    let unique_candidates: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| canon_counts.get(&c.canon).copied().unwrap_or(0) == 1)
        .collect();

    if unique_candidates.is_empty() {
        return new_lines.to_vec();
    }

    // Sort by start descending for back-to-front application
    let mut sorted: Vec<&Candidate> = unique_candidates;
    sorted.sort_by(|a, b| b.start.cmp(&a.start));

    let mut out: Vec<String> = new_lines.to_vec();
    for c in sorted {
        out.splice(
            c.start..c.start + c.len,
            std::iter::once(c.replacement.clone()),
        );
    }
    out
}

/// Strip echoed anchor line from insert-after content.
pub fn strip_insert_anchor_echo_after(anchor_line: &str, dst_lines: &[String]) -> Vec<String> {
    if dst_lines.len() <= 1 {
        return dst_lines.to_vec();
    }
    if equals_ignoring_whitespace(&dst_lines[0], anchor_line) {
        dst_lines[1..].to_vec()
    } else {
        dst_lines.to_vec()
    }
}

/// Strip echoed boundary lines from range replacement content.
pub fn strip_range_boundary_echo(
    file_lines: &[String],
    start_line: usize,
    end_line: usize,
    dst_lines: &[String],
) -> Vec<String> {
    let count = end_line - start_line + 1;
    if dst_lines.len() <= 1 || dst_lines.len() <= count {
        return dst_lines.to_vec();
    }

    let mut out = dst_lines.to_vec();

    // Check if first dst line echoes line before the range
    if start_line >= 2 {
        let before_idx = start_line - 2;
        if equals_ignoring_whitespace(&out[0], &file_lines[before_idx]) {
            out = out[1..].to_vec();
        }
    }

    // Check if last dst line echoes line after the range
    let after_idx = end_line; // 0-indexed = end_line (since end_line is 1-indexed)
    if after_idx < file_lines.len()
        && !out.is_empty()
        && equals_ignoring_whitespace(out.last().unwrap(), &file_lines[after_idx])
    {
        out.pop();
    }

    out
}

/// Detect when model merges a single-line edit with adjacent continuation lines.
///
/// Returns the expanded splice if a merge is detected.
pub fn maybe_expand_single_line_merge(
    line: usize,
    dst: &[String],
    file_lines: &[String],
    explicitly_touched_lines: &std::collections::HashSet<usize>,
) -> Option<(usize, usize, Vec<String>)> {
    if dst.len() != 1 {
        return None;
    }
    if line < 1 || line > file_lines.len() {
        return None;
    }

    let new_line = &dst[0];
    let new_canon = strip_all_whitespace(new_line);
    let new_canon_for_merge_ops = strip_merge_operator_chars(&new_canon);
    if new_canon.is_empty() {
        return None;
    }

    let orig = &file_lines[line - 1];
    let orig_canon = strip_all_whitespace(orig);
    let orig_canon_for_match = strip_trailing_continuation_tokens(&orig_canon);
    let orig_canon_for_merge_ops = strip_merge_operator_chars(&orig_canon);
    let orig_looks_like_continuation = orig_canon_for_match.len() < orig_canon.len();
    if orig_canon.is_empty() {
        return None;
    }

    let next_idx = line; // 0-indexed next line
    let prev_idx = if line >= 2 { Some(line - 2) } else { None };

    // Case A: dst absorbed the next continuation line
    if orig_looks_like_continuation
        && next_idx < file_lines.len()
        && !explicitly_touched_lines.contains(&(line + 1))
    {
        let next = &file_lines[next_idx];
        let next_canon = strip_all_whitespace(next);
        if let (Some(a), Some(b)) = (
            new_canon.find(&*orig_canon_for_match),
            new_canon.find(&*next_canon),
        ) {
            if a < b && new_canon.len() <= orig_canon.len() + next_canon.len() + 32 {
                return Some((line, 2, vec![new_line.clone()]));
            }
        }
    }

    // Case B: dst absorbed the previous declaration/continuation line
    if let Some(prev_idx) = prev_idx {
        if !explicitly_touched_lines.contains(&(line - 1)) {
            let prev = &file_lines[prev_idx];
            let prev_canon = strip_all_whitespace(prev);
            let prev_canon_for_match = strip_trailing_continuation_tokens(&prev_canon);
            let prev_looks_like_continuation = prev_canon_for_match.len() < prev_canon.len();
            if !prev_looks_like_continuation {
                return None;
            }
            let a =
                new_canon_for_merge_ops.find(&strip_merge_operator_chars(&prev_canon_for_match));
            let b = new_canon_for_merge_ops.find(&orig_canon_for_merge_ops);
            if let (Some(a), Some(b)) = (a, b) {
                if a < b && new_canon.len() <= prev_canon.len() + orig_canon.len() + 32 {
                    return Some((line - 1, 2, vec![new_line.clone()]));
                }
            }
        }
    }

    None
}

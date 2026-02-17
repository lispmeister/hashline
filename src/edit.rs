use crate::error::{HashMismatch, HashlineMismatchError};
use crate::hash::compute_line_hash;
use crate::heuristics;
use crate::parse::parse_line_ref;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

/// Edit operations matching the TypeScript schema.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum HashlineEdit {
    SetLine {
        set_line: SetLineOp,
    },
    ReplaceLines {
        replace_lines: ReplaceLinesOp,
    },
    InsertAfter {
        insert_after: InsertAfterOp,
    },
    Replace {
        replace: ReplaceOp,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetLineOp {
    pub anchor: String,
    pub new_text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplaceLinesOp {
    pub start_anchor: String,
    pub end_anchor: Option<String>,
    pub new_text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InsertAfterOp {
    pub anchor: String,
    pub text: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplaceOp {
    pub old_text: String,
    pub new_text: String,
}

/// JSON input format for the CLI.
#[derive(Debug, Clone, Deserialize)]
pub struct HashlineParams {
    pub path: String,
    pub edits: Vec<HashlineEdit>,
}

/// Result of applying edits.
#[derive(Debug)]
pub struct ApplyResult {
    pub content: String,
    pub first_changed_line: Option<usize>,
    pub warnings: Vec<String>,
    pub noop_edits: Vec<NoopEdit>,
}

#[derive(Debug)]
pub struct NoopEdit {
    pub edit_index: usize,
    pub loc: String,
    pub current_content: String,
}

// Internal parsed refs
enum ParsedRefs {
    Single { line: usize, hash: String },
    Range { start_line: usize, start_hash: String, end_line: usize, end_hash: String },
    InsertAfter { line: usize, hash: String },
}

struct ParsedEdit {
    spec: ParsedRefs,
    dst_lines: Vec<String>,
}

fn parse_hashline_edit(edit: &HashlineEdit) -> Result<(ParsedRefs, String), String> {
    match edit {
        HashlineEdit::SetLine { set_line } => {
            let r = parse_line_ref(&set_line.anchor)?;
            Ok((ParsedRefs::Single { line: r.line, hash: r.hash }, set_line.new_text.clone()))
        }
        HashlineEdit::ReplaceLines { replace_lines } => {
            let start = parse_line_ref(&replace_lines.start_anchor)?;
            let new_text = replace_lines.new_text.clone().unwrap_or_default();
            match &replace_lines.end_anchor {
                None => Ok((ParsedRefs::Single { line: start.line, hash: start.hash }, new_text)),
                Some(end_str) => {
                    let end = parse_line_ref(end_str)?;
                    if start.line == end.line {
                        Ok((ParsedRefs::Single { line: start.line, hash: start.hash }, new_text))
                    } else {
                        Ok((
                            ParsedRefs::Range {
                                start_line: start.line,
                                start_hash: start.hash,
                                end_line: end.line,
                                end_hash: end.hash,
                            },
                            new_text,
                        ))
                    }
                }
            }
        }
        HashlineEdit::InsertAfter { insert_after } => {
            let r = parse_line_ref(&insert_after.anchor)?;
            let text = insert_after
                .text
                .clone()
                .or_else(|| insert_after.content.clone())
                .unwrap_or_default();
            Ok((ParsedRefs::InsertAfter { line: r.line, hash: r.hash }, text))
        }
        HashlineEdit::Replace { .. } => {
            Err("replace edits are applied separately; do not pass them to applyHashlineEdits".into())
        }
    }
}

fn split_dst_lines(dst: &str) -> Vec<String> {
    if dst.is_empty() {
        vec![]
    } else {
        dst.split('\n').map(|s| s.to_string()).collect()
    }
}

/// Apply an array of hashline edits to file content.
pub fn apply_hashline_edits(
    content: &str,
    edits: &[HashlineEdit],
) -> Result<ApplyResult, Box<dyn std::error::Error>> {
    if edits.is_empty() {
        return Ok(ApplyResult {
            content: content.to_string(),
            first_changed_line: None,
            warnings: vec![],
            noop_edits: vec![],
        });
    }

    let file_lines: Vec<String> = content.split('\n').map(|s| s.to_string()).collect();
    let original_file_lines = file_lines.clone();
    let mut file_lines = file_lines;
    let mut first_changed_line: Option<usize> = None;
    let mut noop_edits: Vec<NoopEdit> = Vec::new();

    // Parse all edits up front
    let mut parsed: Vec<(usize, ParsedEdit)> = Vec::new();
    for (i, edit) in edits.iter().enumerate() {
        let (spec, dst) = parse_hashline_edit(edit)?;
        let dst_lines = heuristics::strip_new_line_prefixes(&split_dst_lines(&dst));
        parsed.push((i, ParsedEdit { spec, dst_lines }));
    }

    // Collect explicitly touched lines
    let collect_touched = |parsed: &[(usize, ParsedEdit)]| -> HashSet<usize> {
        let mut touched = HashSet::new();
        for (_, p) in parsed {
            match &p.spec {
                ParsedRefs::Single { line, .. } => { touched.insert(*line); }
                ParsedRefs::Range { start_line, end_line, .. } => {
                    for ln in *start_line..=*end_line {
                        touched.insert(ln);
                    }
                }
                ParsedRefs::InsertAfter { line, .. } => { touched.insert(*line); }
            }
        }
        touched
    };

    let mut _explicitly_touched = collect_touched(&parsed);

    // Build unique hash map for relocation
    let mut unique_line_by_hash: HashMap<String, usize> = HashMap::new();
    let mut seen_duplicate_hashes: HashSet<String> = HashSet::new();
    for (i, line) in file_lines.iter().enumerate() {
        let line_no = i + 1;
        let hash = compute_line_hash(line_no, line);
        if seen_duplicate_hashes.contains(&hash) {
            continue;
        }
        if unique_line_by_hash.contains_key(&hash) {
            unique_line_by_hash.remove(&hash);
            seen_duplicate_hashes.insert(hash);
            continue;
        }
        unique_line_by_hash.insert(hash, line_no);
    }

    // Pre-validate all hashes
    let mut mismatches: Vec<HashMismatch> = Vec::new();

    let validate_or_relocate = |line: &mut usize, hash: &str, file_lines: &[String], unique_line_by_hash: &HashMap<String, usize>, mismatches: &mut Vec<HashMismatch>| -> bool {
        if *line < 1 || *line > file_lines.len() {
            return false; // will be caught as out-of-range error
        }
        let expected = hash.to_lowercase();
        let actual = compute_line_hash(*line, &file_lines[*line - 1]);
        if actual == expected {
            return true;
        }
        if let Some(&relocated) = unique_line_by_hash.get(&expected) {
            *line = relocated;
            return true;
        }
        mismatches.push(HashMismatch {
            line: *line,
            expected: hash.to_string(),
            actual,
        });
        false
    };

    for (_, p) in parsed.iter_mut() {
        match &mut p.spec {
            ParsedRefs::Single { line, hash } => {
                if *line < 1 || *line > file_lines.len() {
                    return Err(format!(
                        "Line {} does not exist (file has {} lines)",
                        line,
                        file_lines.len()
                    ).into());
                }
                validate_or_relocate(line, hash, &file_lines, &unique_line_by_hash, &mut mismatches);
            }
            ParsedRefs::InsertAfter { line, hash } => {
                if *line < 1 || *line > file_lines.len() {
                    return Err(format!(
                        "Line {} does not exist (file has {} lines)",
                        line,
                        file_lines.len()
                    ).into());
                }
                if p.dst_lines.is_empty() {
                    return Err("Insert-after edit requires non-empty dst".into());
                }
                validate_or_relocate(line, hash, &file_lines, &unique_line_by_hash, &mut mismatches);
            }
            ParsedRefs::Range { start_line, start_hash, end_line, end_hash } => {
                if *start_line < 1 || *start_line > file_lines.len() {
                    return Err(format!(
                        "Line {} does not exist (file has {} lines)",
                        start_line,
                        file_lines.len()
                    ).into());
                }
                if *end_line < 1 || *end_line > file_lines.len() {
                    return Err(format!(
                        "Line {} does not exist (file has {} lines)",
                        end_line,
                        file_lines.len()
                    ).into());
                }
                if *start_line > *end_line {
                    return Err(format!(
                        "Range start line {} must be <= end line {}",
                        start_line, end_line
                    ).into());
                }

                let original_start = *start_line;
                let original_end = *end_line;
                let original_count = original_end - original_start + 1;

                let start_ok = validate_or_relocate(start_line, start_hash, &file_lines, &unique_line_by_hash, &mut mismatches);
                let end_ok = validate_or_relocate(end_line, end_hash, &file_lines, &unique_line_by_hash, &mut mismatches);

                if start_ok && end_ok {
                    let relocated_count = *end_line - *start_line + 1;
                    let changed_by_relocation = *start_line != original_start || *end_line != original_end;
                    let invalid_range = *start_line > *end_line;
                    let scope_changed = relocated_count != original_count;

                    if changed_by_relocation && (invalid_range || scope_changed) {
                        *start_line = original_start;
                        *end_line = original_end;
                        // Remove any mismatches we didn't add and add new ones
                        mismatches.push(HashMismatch {
                            line: original_start,
                            expected: start_hash.clone(),
                            actual: compute_line_hash(original_start, &file_lines[original_start - 1]),
                        });
                        mismatches.push(HashMismatch {
                            line: original_end,
                            expected: end_hash.clone(),
                            actual: compute_line_hash(original_end, &file_lines[original_end - 1]),
                        });
                    }
                }
            }
        }
    }

    if !mismatches.is_empty() {
        return Err(Box::new(HashlineMismatchError::new(
            mismatches,
            file_lines,
        )));
    }

    // Recompute touched lines after relocation
    let explicitly_touched_lines = collect_touched(&parsed);

    // Deduplicate identical edits
    let mut seen_edit_keys: HashMap<String, usize> = HashMap::new();
    let mut dedup_indices: HashSet<usize> = HashSet::new();
    for (i, (_, p)) in parsed.iter().enumerate() {
        let line_key = match &p.spec {
            ParsedRefs::Single { line, .. } => format!("s:{}", line),
            ParsedRefs::Range { start_line, end_line, .. } => format!("r:{}:{}", start_line, end_line),
            ParsedRefs::InsertAfter { line, .. } => format!("i:{}", line),
        };
        let dst_key = format!("{}|{}", line_key, p.dst_lines.join("\n"));
        if seen_edit_keys.contains_key(&dst_key) {
            dedup_indices.insert(i);
        } else {
            seen_edit_keys.insert(dst_key, i);
        }
    }
    if !dedup_indices.is_empty() {
        let mut i = parsed.len();
        while i > 0 {
            i -= 1;
            if dedup_indices.contains(&i) {
                parsed.remove(i);
            }
        }
    }

    // Sort bottom-up (descending line number)
    parsed.sort_by(|a, b| {
        let sort_line_a = match &a.1.spec {
            ParsedRefs::Single { line, .. } => *line,
            ParsedRefs::Range { end_line, .. } => *end_line,
            ParsedRefs::InsertAfter { line, .. } => *line,
        };
        let sort_line_b = match &b.1.spec {
            ParsedRefs::Single { line, .. } => *line,
            ParsedRefs::Range { end_line, .. } => *end_line,
            ParsedRefs::InsertAfter { line, .. } => *line,
        };
        let prec_a = match &a.1.spec {
            ParsedRefs::InsertAfter { .. } => 1,
            _ => 0,
        };
        let prec_b = match &b.1.spec {
            ParsedRefs::InsertAfter { .. } => 1,
            _ => 0,
        };
        sort_line_b.cmp(&sort_line_a)
            .then(prec_a.cmp(&prec_b))
            .then(a.0.cmp(&b.0))
    });

    // Apply edits bottom-up
    for (idx, edit) in &parsed {
        match &edit.spec {
            ParsedRefs::Single { line, hash } => {
                let line = *line;
                // Try merge expansion
                if let Some((start, delete_count, new_lines)) =
                    heuristics::maybe_expand_single_line_merge(
                        line,
                        &edit.dst_lines,
                        &file_lines,
                        &explicitly_touched_lines,
                    )
                {
                    let orig_lines: Vec<String> = original_file_lines[start - 1..start - 1 + delete_count].to_vec();
                    let mut next_lines = heuristics::restore_indent_for_paired_replacement(
                        &[orig_lines.first().cloned().unwrap_or_default()],
                        &new_lines,
                    );
                    if orig_lines.join("\n") == next_lines.join("\n")
                        && orig_lines.iter().any(|l| heuristics::has_confusable_hyphens(l))
                    {
                        next_lines = heuristics::normalize_confusable_hyphens_in_lines(&next_lines);
                    }
                    if orig_lines.join("\n") == next_lines.join("\n") {
                        noop_edits.push(NoopEdit {
                            edit_index: *idx,
                            loc: format!("{}:{}", line, hash),
                            current_content: orig_lines.join("\n"),
                        });
                        continue;
                    }
                    file_lines.splice(start - 1..start - 1 + delete_count, next_lines);
                    track_first_changed(&mut first_changed_line, start);
                    continue;
                }

                let orig_lines: Vec<String> = original_file_lines[line - 1..line].to_vec();
                let stripped = heuristics::strip_range_boundary_echo(
                    &original_file_lines, line, line, &edit.dst_lines,
                );
                let stripped = heuristics::restore_old_wrapped_lines(&orig_lines, &stripped);
                let mut new_lines = heuristics::restore_indent_for_paired_replacement(&orig_lines, &stripped);
                if orig_lines.join("\n") == new_lines.join("\n")
                    && orig_lines.iter().any(|l| heuristics::has_confusable_hyphens(l))
                {
                    new_lines = heuristics::normalize_confusable_hyphens_in_lines(&new_lines);
                }
                if orig_lines.join("\n") == new_lines.join("\n") {
                    noop_edits.push(NoopEdit {
                        edit_index: *idx,
                        loc: format!("{}:{}", line, hash),
                        current_content: orig_lines.join("\n"),
                    });
                    continue;
                }
                file_lines.splice(line - 1..line, new_lines);
                track_first_changed(&mut first_changed_line, line);
            }
            ParsedRefs::Range { start_line, start_hash, end_line, .. } => {
                let start = *start_line;
                let end = *end_line;
                let count = end - start + 1;
                let orig_lines: Vec<String> = original_file_lines[start - 1..start - 1 + count].to_vec();
                let stripped = heuristics::strip_range_boundary_echo(
                    &original_file_lines, start, end, &edit.dst_lines,
                );
                let stripped = heuristics::restore_old_wrapped_lines(&orig_lines, &stripped);
                let mut new_lines = heuristics::restore_indent_for_paired_replacement(&orig_lines, &stripped);
                if orig_lines.join("\n") == new_lines.join("\n")
                    && orig_lines.iter().any(|l| heuristics::has_confusable_hyphens(l))
                {
                    new_lines = heuristics::normalize_confusable_hyphens_in_lines(&new_lines);
                }
                if orig_lines.join("\n") == new_lines.join("\n") {
                    noop_edits.push(NoopEdit {
                        edit_index: *idx,
                        loc: format!("{}:{}", start, start_hash),
                        current_content: orig_lines.join("\n"),
                    });
                    continue;
                }
                file_lines.splice(start - 1..start - 1 + count, new_lines);
                track_first_changed(&mut first_changed_line, start);
            }
            ParsedRefs::InsertAfter { line, hash } => {
                let line = *line;
                let anchor_line = &original_file_lines[line - 1];
                let inserted = heuristics::strip_insert_anchor_echo_after(anchor_line, &edit.dst_lines);
                if inserted.is_empty() {
                    noop_edits.push(NoopEdit {
                        edit_index: *idx,
                        loc: format!("{}:{}", line, hash),
                        current_content: original_file_lines[line - 1].clone(),
                    });
                    continue;
                }
                file_lines.splice(line..line, inserted);
                track_first_changed(&mut first_changed_line, line + 1);
            }
        }
    }

    // Warnings
    let mut warnings = Vec::new();
    let mut diff_line_count = (file_lines.len() as isize - original_file_lines.len() as isize).unsigned_abs();
    for i in 0..std::cmp::min(file_lines.len(), original_file_lines.len()) {
        if file_lines[i] != original_file_lines[i] {
            diff_line_count += 1;
        }
    }
    if diff_line_count > edits.len() * 4 {
        warnings.push(format!(
            "Edit changed {} lines across {} operations — verify no unintended reformatting.",
            diff_line_count,
            edits.len()
        ));
    }

    Ok(ApplyResult {
        content: file_lines.join("\n"),
        first_changed_line,
        warnings,
        noop_edits,
    })
}

fn track_first_changed(first: &mut Option<usize>, line: usize) {
    if first.is_none() || line < first.unwrap() {
        *first = Some(line);
    }
}

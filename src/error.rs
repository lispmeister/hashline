use crate::hash::compute_line_hash;
use std::fmt;

/// A single hash mismatch found during validation.
#[derive(Debug, Clone)]
pub struct HashMismatch {
    pub line: usize,
    pub expected: String,
    pub actual: String,
}

/// Number of context lines shown above/below each mismatched line.
const MISMATCH_CONTEXT: usize = 2;

/// Error when one or more hashline references have stale hashes.
#[derive(Debug, Clone)]
pub struct HashlineMismatchError {
    pub mismatches: Vec<HashMismatch>,
    pub file_lines: Vec<String>,
}

impl HashlineMismatchError {
    pub fn new(mismatches: Vec<HashMismatch>, file_lines: Vec<String>) -> Self {
        Self {
            mismatches,
            file_lines,
        }
    }

    pub fn format_message(&self) -> String {
        let mut mismatch_set = std::collections::HashMap::new();
        for m in &self.mismatches {
            mismatch_set.insert(m.line, m);
        }

        let mut display_lines = std::collections::BTreeSet::new();
        for m in &self.mismatches {
            let lo = if m.line > MISMATCH_CONTEXT {
                m.line - MISMATCH_CONTEXT
            } else {
                1
            };
            let hi = std::cmp::min(self.file_lines.len(), m.line + MISMATCH_CONTEXT);
            for i in lo..=hi {
                display_lines.insert(i);
            }
        }

        let mut lines = Vec::new();
        let count = self.mismatches.len();
        lines.push(format!(
            "{} line{} changed since last read. Use the updated LINE:HASH references shown below (>>> marks changed lines).",
            count,
            if count > 1 { "s have" } else { " has" }
        ));
        lines.push(String::new());

        let sorted: Vec<usize> = display_lines.into_iter().collect();
        let mut prev_line: Option<usize> = None;

        for &line_num in &sorted {
            if let Some(prev) = prev_line {
                if line_num > prev + 1 {
                    lines.push("    ...".to_string());
                }
            }
            prev_line = Some(line_num);

            let content = &self.file_lines[line_num - 1];
            let hash = compute_line_hash(line_num, content);
            let prefix = format!("{}:{}", line_num, hash);

            if mismatch_set.contains_key(&line_num) {
                lines.push(format!(">>> {}|{}", prefix, content));
            } else {
                lines.push(format!("    {}|{}", prefix, content));
            }
        }

        lines.join("\n")
    }

    /// Build a map from old "LINE:HASH" → new "LINE:HASH" for each mismatch.
    pub fn remaps(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        for m in &self.mismatches {
            let actual = compute_line_hash(m.line, &self.file_lines[m.line - 1]);
            map.insert(
                format!("{}:{}", m.line, m.expected),
                format!("{}:{}", m.line, actual),
            );
        }
        map
    }
}

impl fmt::Display for HashlineMismatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_message())
    }
}

impl std::error::Error for HashlineMismatchError {}

use crate::hash::compute_line_hash;

/// Format file content with hashline prefixes for display.
///
/// Each line becomes `LINENUM:HASH|CONTENT` where LINENUM is 1-indexed.
pub fn format_hashlines(content: &str, start_line: usize) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let num = start_line + i;
            let hash = compute_line_hash(num, line);
            format!("{}:{}|{}", num, hash, line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::compute_line_hash;

    #[test]
    fn formats_single_line() {
        let result = format_hashlines("hello", 1);
        let hash = compute_line_hash(1, "hello");
        assert_eq!(result, format!("1:{}|hello", hash));
    }

    #[test]
    fn formats_multiple_lines() {
        let result = format_hashlines("foo\nbar\nbaz", 1);
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("1:"));
        assert!(lines[1].starts_with("2:"));
        assert!(lines[2].starts_with("3:"));
    }

    #[test]
    fn respects_custom_start_line() {
        let result = format_hashlines("foo\nbar", 10);
        let lines: Vec<&str> = result.split('\n').collect();
        assert!(lines[0].starts_with("10:"));
        assert!(lines[1].starts_with("11:"));
    }

    #[test]
    fn handles_empty_lines() {
        let result = format_hashlines("foo\n\nbar", 1);
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines.len(), 3);
        // Empty line should be like "2:XX|"
        assert!(lines[1].contains('|'));
        assert!(lines[1].ends_with('|'));
    }

    #[test]
    fn round_trips_with_compute_line_hash() {
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
}

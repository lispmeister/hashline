use regex::Regex;
use std::sync::LazyLock;

/// A parsed line reference: 1-indexed line number + hash string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRef {
    pub line: usize,
    pub hash: String,
}

static STRICT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+):([0-9a-zA-Z]{1,16})$").unwrap());
static PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+):([0-9a-zA-Z]{2})").unwrap());

/// Parse a line reference string like `"5:ab"` into structured form.
///
/// Handles display-format suffixes (`5:ab|content`), legacy format (`5:ab  content`),
/// and `>>>` prefixes from error output.
pub fn parse_line_ref(ref_str: &str) -> Result<LineRef, String> {
    // Strip display-format suffix, legacy suffix, leading >>> markers
    let cleaned = ref_str.split('|').next().unwrap_or(ref_str);
    // Strip legacy "  content" suffix
    let cleaned = if let Some(pos) = cleaned.find("  ") {
        &cleaned[..pos]
    } else {
        cleaned
    };
    // Strip leading >>> markers
    let cleaned = cleaned.trim_start_matches('>').trim();
    // Normalize whitespace around colon
    let normalized = COLON_WS_RE.replace(cleaned, ":").to_string();

    // Try strict match first
    if let Some(caps) = STRICT_RE.captures(&normalized) {
        let line: usize = caps[1].parse().unwrap();
        if line < 1 {
            return Err(format!(
                "Line number must be >= 1, got {} in {:?}.",
                line, ref_str
            ));
        }
        return Ok(LineRef {
            line,
            hash: caps[2].to_string(),
        });
    }

    // Then try prefix match (HASH_LEN=2 chars)
    if let Some(caps) = PREFIX_RE.captures(&normalized) {
        let line: usize = caps[1].parse().unwrap();
        if line < 1 {
            return Err(format!(
                "Line number must be >= 1, got {} in {:?}.",
                line, ref_str
            ));
        }
        return Ok(LineRef {
            line,
            hash: caps[2].to_string(),
        });
    }

    Err(format!(
        "Invalid line reference {:?}. Expected format \"LINE:HASH\" (e.g. \"5:aa\").",
        ref_str
    ))
}

static COLON_WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*:\s*").unwrap());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_reference() {
        assert_eq!(
            parse_line_ref("5:abcd").unwrap(),
            LineRef {
                line: 5,
                hash: "abcd".into()
            }
        );
    }

    #[test]
    fn parses_single_digit_hash() {
        assert_eq!(
            parse_line_ref("1:a").unwrap(),
            LineRef {
                line: 1,
                hash: "a".into()
            }
        );
    }

    #[test]
    fn parses_long_hash() {
        assert_eq!(
            parse_line_ref("100:abcdef0123456789").unwrap(),
            LineRef {
                line: 100,
                hash: "abcdef0123456789".into()
            }
        );
    }

    #[test]
    fn strips_display_suffix() {
        assert_eq!(
            parse_line_ref("5:ab|some content").unwrap(),
            LineRef {
                line: 5,
                hash: "ab".into()
            }
        );
    }

    #[test]
    fn strips_legacy_suffix() {
        assert_eq!(
            parse_line_ref("5:ab  some content").unwrap(),
            LineRef {
                line: 5,
                hash: "ab".into()
            }
        );
    }

    #[test]
    fn strips_arrow_prefix() {
        assert_eq!(
            parse_line_ref(">>> 5:ab").unwrap(),
            LineRef {
                line: 5,
                hash: "ab".into()
            }
        );
    }

    #[test]
    fn rejects_missing_colon() {
        assert!(parse_line_ref("5abcd").is_err());
    }

    #[test]
    fn rejects_non_numeric_line() {
        assert!(parse_line_ref("abc:1234").is_err());
    }

    #[test]
    fn rejects_non_alphanumeric_hash() {
        assert!(parse_line_ref("5:$$$$").is_err());
    }

    #[test]
    fn rejects_line_number_0() {
        let err = parse_line_ref("0:abcd").unwrap_err();
        assert!(err.contains(">= 1"));
    }

    #[test]
    fn rejects_empty_string() {
        assert!(parse_line_ref("").is_err());
    }

    #[test]
    fn rejects_empty_hash() {
        assert!(parse_line_ref("5:").is_err());
    }

    #[test]
    fn parses_polluted_trailing_content() {
        // "2:ABexport function foo(a, b) {}" → prefix match grabs "2:AB"
        let result = parse_line_ref("2:abexport function foo(a, b) {}").unwrap();
        assert_eq!(result.line, 2);
        assert_eq!(result.hash, "ab");
    }
}

use xxhash_rust::xxh32::xxh32;

const HASH_LEN: u32 = 2;
const RADIX: u32 = 16;
const HASH_MOD: u32 = RADIX.pow(HASH_LEN);

/// Compute a short hex hash of a single line.
///
/// Normalizes whitespace (strips all `\s` chars), computes xxHash32 with seed 0,
/// then returns `hash % 256` as a 2-char lowercase hex string.
/// The `_idx` parameter is accepted for compatibility but unused.
pub fn compute_line_hash(_idx: usize, line: &str) -> String {
    let mut line = line;
    // Strip trailing \r
    if line.ends_with('\r') {
        line = &line[..line.len() - 1];
    }
    // Strip all whitespace
    let normalized: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    let h = xxh32(normalized.as_bytes(), 0) % HASH_MOD;
    format!("{:02x}", h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_2_char_hex_hash() {
        let hash = compute_line_hash(1, "hello");
        assert_eq!(hash.len(), 2);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn same_content_same_hash() {
        assert_eq!(compute_line_hash(1, "hello"), compute_line_hash(1, "hello"));
    }

    #[test]
    fn different_content_different_hash() {
        assert_ne!(compute_line_hash(1, "hello"), compute_line_hash(1, "world"));
    }

    #[test]
    fn empty_line_produces_valid_hash() {
        let hash = compute_line_hash(1, "");
        assert_eq!(hash.len(), 2);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn whitespace_insensitive() {
        assert_eq!(
            compute_line_hash(1, "  hello  world  "),
            compute_line_hash(1, "helloworld")
        );
    }

    #[test]
    fn strips_trailing_cr() {
        assert_eq!(
            compute_line_hash(1, "hello\r"),
            compute_line_hash(1, "hello")
        );
    }
}

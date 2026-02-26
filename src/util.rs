use std::fs;
use std::io;
use std::path::Path;

/// Reads the file at the given `path` into a string, normalizing line endings and removing trailing newline.
/// 
/// - Replaces CRLF (\r\n) with LF (\n)
/// - Truncates trailing LF if present
/// 
/// Ensures platform-consistent text processing.
pub fn read_normalized(path: &Path) -> io::Result<String> {
    let mut content = fs::read_to_string(path)?;
    content = content.replace("\r\n", "\n");
    if content.ends_with('\n') {
        content.truncate(content.len() - 1);
    }
    Ok(content)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_crlf_normalization() {
        let path: PathBuf = "/tmp/hashline_util_crlf.txt".into();
        fs::write(&path, b"line1\r\nline2\r\n").unwrap();
        let content = read_normalized(&path).unwrap();
        assert_eq!(content, "line1\nline2");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_trailing_newline() {
        let path: PathBuf = "/tmp/hashline_util_trailing.txt".into();
        fs::write(&path, b"foo\nbar\n").unwrap();
        let content = read_normalized(&path).unwrap();
        assert_eq!(content, "foo\nbar");
        let _ = fs::remove_file(&path);
    }
}

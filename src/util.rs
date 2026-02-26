use std::fs;
use std::io;
use std::path::Path;

pub fn read_normalized(path: &Path) -> io::Result<String> {
    let mut content = fs::read_to_string(path)?;
    content = content.replace("\r\n", "\n");
    if content.ends_with('\n') {
        content.truncate(content.len() - 1);
    }
    Ok(content)
}

use crate::hash::compute_line_hash;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Parse a JSON file into a serde_json Value AST
pub fn parse_json_ast(file_path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

/// Compute a hash anchor for a JSON value at a given path
/// Returns the path with a 2-character hash of the canonical JSON representation
pub fn compute_json_anchor(path: &str, value: &Value) -> String {
    // Serialize to canonical JSON (compact, sorted keys)
    let canonical = serde_json::to_string(value).unwrap_or_default();
    // Use the existing hashline hash function for consistency
    let hash = compute_line_hash(0, &canonical);
    format!("{}:{}", path, hash)
}

/// Format JSON AST with inline anchor comments
/// Returns pretty-printed JSON with // $.path:hash comments before each value
pub fn format_json_anchors(ast: &Value) -> String {
    format_json_with_anchors(ast, "$")
}

/// JSON-specific edit operations
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum JsonEdit {
    SetPath { set_path: SetPathOp },
    InsertAtPath { insert_at_path: InsertAtPathOp },
    DeletePath { delete_path: DeletePathOp },
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SetPathOp {
    pub anchor: String,
    pub value: Value,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InsertAtPathOp {
    pub anchor: String,
    pub key: Option<String>, // None for array append, Some(key) for object
    pub value: Value,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeletePathOp {
    pub anchor: String,
}

/// Apply JSON edits to AST atomically
/// Returns error if any anchor doesn't match current value
pub fn apply_json_edits(
    ast: &mut Value,
    edits: &[JsonEdit],
) -> Result<(), Box<dyn std::error::Error>> {
    // First pass: validate all anchors
    for edit in edits {
        let (path, expected_hash) = match edit {
            JsonEdit::SetPath { set_path: op } => parse_anchor(&op.anchor)?,
            JsonEdit::InsertAtPath { insert_at_path: op } => parse_anchor(&op.anchor)?,
            JsonEdit::DeletePath { delete_path: op } => parse_anchor(&op.anchor)?,
        };
        let current_value = query_path(ast, &path)?;
        let current_hash = compute_line_hash(0, &serde_json::to_string(current_value)?);
        if current_hash != expected_hash {
            return Err(format!(
                "Hash mismatch at {}: expected {}, got {}",
                path, expected_hash, current_hash
            )
            .into());
        }
    }

    // Second pass: apply all edits
    for edit in edits {
        match edit {
            JsonEdit::SetPath { set_path: op } => {
                let (path, _) = parse_anchor(&op.anchor)?;
                set_path(ast, &path, op.value.clone())?;
            }
            JsonEdit::InsertAtPath { insert_at_path: op } => {
                let (path, _) = parse_anchor(&op.anchor)?;
                insert_at_path(ast, &path, op.key.as_deref(), op.value.clone())?;
            }
            JsonEdit::DeletePath { delete_path: op } => {
                let (path, _) = parse_anchor(&op.anchor)?;
                delete_path(ast, &path)?;
            }
        }
    }

    Ok(())
}

/// Parse anchor string into (path, hash)
fn parse_anchor(anchor: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    if let Some(colon_pos) = anchor.rfind(':') {
        let path = &anchor[..colon_pos];
        let hash = &anchor[colon_pos + 1..];
        if hash.len() == 2 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok((path.to_string(), hash.to_string()))
        } else {
            Err(format!("Invalid hash format in anchor: {}", anchor).into())
        }
    } else {
        Err(format!("Invalid anchor format, missing ':': {}", anchor).into())
    }
}

/// Query value at JSONPath (simplified implementation for basic paths)
fn query_path<'a>(ast: &'a Value, path: &str) -> Result<&'a Value, Box<dyn std::error::Error>> {
    if path == "$" {
        return Ok(ast);
    }

    if let Some(key) = path.strip_prefix("$.") {
        if let Some(obj) = ast.as_object() {
            if let Some(value) = obj.get(key) {
                Ok(value)
            } else {
                Err(format!("Key not found: {}", key).into())
            }
        } else {
            Err("Cannot query key on non-object".into())
        }
    } else {
        Err(format!("Unsupported path format: {}", path).into())
    }
}

/// Set value at JSONPath (basic implementation)
fn set_path(ast: &mut Value, path: &str, value: Value) -> Result<(), Box<dyn std::error::Error>> {
    if path == "$" {
        *ast = value;
        return Ok(());
    }

    if let Some(key) = path.strip_prefix("$.") {
        if let Some(obj) = ast.as_object_mut() {
            obj.insert(key.to_string(), value);
            Ok(())
        } else {
            Err("Cannot set path on non-object".into())
        }
    } else {
        Err(format!("Unsupported path format: {}", path).into())
    }
}

/// Insert value at path (basic implementation)
fn insert_at_path(
    ast: &mut Value,
    _path: &str,
    key: Option<&str>,
    value: Value,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(key) = key {
        // Insert into object
        if let Some(obj) = ast.as_object_mut() {
            obj.insert(key.to_string(), value);
            Ok(())
        } else {
            Err("Cannot insert key into non-object".into())
        }
    } else {
        // Append to array
        if let Some(arr) = ast.as_array_mut() {
            arr.push(value);
            Ok(())
        } else {
            Err("Cannot append to non-array".into())
        }
    }
}

/// Delete value at path (basic implementation)
fn delete_path(ast: &mut Value, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(key) = path.strip_prefix("$.") {
        if let Some(obj) = ast.as_object_mut() {
            obj.remove(key);
            Ok(())
        } else {
            Err("Cannot delete from non-object".into())
        }
    } else {
        Err(format!("Unsupported path format: {}", path).into())
    }
}

/// Recursive helper to format JSON with anchors
fn format_json_with_anchors(value: &Value, current_path: &str) -> String {
    match value {
        Value::Object(map) => {
            let mut result = "{\n".to_string();
            for (i, (key, val)) in map.iter().enumerate() {
                let path = format!("{}.{}", current_path, key);
                let anchor = compute_json_anchor(&path, val);
                result.push_str(&format!("  // {}\n", anchor));
                result.push_str(&format!(
                    "  \"{}\": {}",
                    key,
                    format_json_with_anchors(val, &path)
                ));
                if i < map.len() - 1 {
                    result.push(',');
                }
                result.push('\n');
            }
            result.push('}');
            result
        }
        Value::Array(arr) => {
            let mut result = "[\n".to_string();
            for (i, val) in arr.iter().enumerate() {
                let path = format!("{}[{}]", current_path, i);
                let anchor = compute_json_anchor(&path, val);
                result.push_str(&format!("  // {}\n", anchor));
                result.push_str(&format!("  {}", format_json_with_anchors(val, &path)));
                if i < arr.len() - 1 {
                    result.push(',');
                }
                result.push('\n');
            }
            result.push(']');
            result
        }
        _ => {
            // For primitives, just return the JSON representation
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_parse_valid_json() {
        let json = r#"{"name": "test", "value": 42}"#;
        let temp_path = PathBuf::from("test_valid.json");
        fs::write(&temp_path, json).unwrap();

        let ast = parse_json_ast(&temp_path).unwrap();
        assert_eq!(ast["name"], "test");
        assert_eq!(ast["value"], 42);

        fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_parse_invalid_json() {
        let invalid_json = r#"{"name": "test", "value":}"#;
        let temp_path = PathBuf::from("test_invalid.json");
        fs::write(&temp_path, invalid_json).unwrap();

        let result = parse_json_ast(&temp_path);
        assert!(result.is_err());

        fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_compute_json_anchor() {
        let value = serde_json::json!({"name": "test", "value": 42});
        let anchor = compute_json_anchor("$.test", &value);
        assert!(anchor.starts_with("$.test:"));
        assert_eq!(anchor.len(), 9); // path + : + 2 chars

        // Same value should produce same hash
        let anchor2 = compute_json_anchor("$.test", &value);
        assert_eq!(anchor, anchor2);
    }

    #[test]
    fn test_format_json_anchors() {
        let value = serde_json::json!({"name": "test", "value": 42});
        let formatted = format_json_anchors(&value);
        assert!(formatted.contains("// $.name:"));
        assert!(formatted.contains("// $.value:"));
        assert!(formatted.contains("\"name\": \"test\""));
        assert!(formatted.contains("\"value\": 42"));
    }
}

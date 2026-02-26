use crate::hash::compute_line_hash;
use serde_json::Value;
use std::fmt;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Error type (fix 3)
// ---------------------------------------------------------------------------

/// Typed error returned by `apply_json_edits`.
pub enum JsonError {
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    Other(String),
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::HashMismatch {
                path,
                expected,
                actual,
            } => write!(
                f,
                "Hash mismatch at {}: expected {}, got {}",
                path, expected, actual
            ),
            JsonError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl fmt::Debug for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for JsonError {}

impl From<String> for JsonError {
    fn from(s: String) -> Self {
        JsonError::Other(s)
    }
}

impl From<&str> for JsonError {
    fn from(s: &str) -> Self {
        JsonError::Other(s.to_string())
    }
}

impl From<serde_json::Error> for JsonError {
    fn from(e: serde_json::Error) -> Self {
        JsonError::Other(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Params struct (fix 5)
// ---------------------------------------------------------------------------

/// Parameters for `json-apply`: file path and list of edits.
#[derive(serde::Deserialize)]
pub struct JsonApplyParams {
    pub path: String,
    pub edits: Vec<JsonEdit>,
}

// ---------------------------------------------------------------------------
// Path segment parser (fix 1)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum PathSegment {
    Key(String),
    Index(usize),
}

/// Parse a JSONPath string into segments.
/// Supports: `$`, `$.a`, `$.a.b`, `$.a[0]`, `$.a[0].b`, etc.
fn parse_path_segments(path: &str) -> Result<Vec<PathSegment>, JsonError> {
    if path == "$" {
        return Ok(vec![]);
    }
    if !path.starts_with('$') {
        return Err(format!("Path must start with '$': {}", path).into());
    }

    let tail = &path[1..]; // everything after '$'
    let bytes = tail.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut segments = Vec::new();

    while i < len {
        match bytes[i] {
            b'.' => {
                i += 1; // skip '.'
                let start = i;
                while i < len && bytes[i] != b'.' && bytes[i] != b'[' {
                    i += 1;
                }
                let key = &tail[start..i];
                if key.is_empty() {
                    return Err(format!("Empty key segment in path: {}", path).into());
                }
                segments.push(PathSegment::Key(key.to_string()));
            }
            b'[' => {
                i += 1; // skip '['
                let start = i;
                while i < len && bytes[i] != b']' {
                    i += 1;
                }
                let idx_str = &tail[start..i];
                let idx: usize = idx_str.parse().map_err(|_| {
                    format!("Invalid array index '{}' in path: {}", idx_str, path)
                })?;
                if i < len && bytes[i] == b']' {
                    i += 1; // skip ']'
                }
                segments.push(PathSegment::Index(idx));
            }
            other => {
                return Err(format!(
                    "Unexpected character '{}' in path: {}",
                    other as char, path
                )
                .into());
            }
        }
    }

    Ok(segments)
}

/// Navigate immutably to the node identified by `segments`.
fn query_path_segments<'a>(
    ast: &'a Value,
    segments: &[PathSegment],
) -> Result<&'a Value, JsonError> {
    let mut current = ast;
    for (i, seg) in segments.iter().enumerate() {
        match seg {
            PathSegment::Key(key) => {
                current = current
                    .as_object()
                    .ok_or_else(|| {
                        JsonError::Other(format!(
                            "Expected object at segment {} but got non-object",
                            i
                        ))
                    })?
                    .get(key)
                    .ok_or_else(|| JsonError::Other(format!("Key not found: {}", key)))?;
            }
            PathSegment::Index(idx) => {
                let arr = current.as_array().ok_or_else(|| {
                    JsonError::Other(format!(
                        "Expected array at segment {} but got non-array",
                        i
                    ))
                })?;
                current = arr.get(*idx).ok_or_else(|| {
                    JsonError::Other(format!("Array index {} out of bounds", idx))
                })?;
            }
        }
    }
    Ok(current)
}

/// Navigate mutably to the node identified by `segments`.
fn query_path_segments_mut<'a>(
    ast: &'a mut Value,
    segments: &[PathSegment],
) -> Result<&'a mut Value, JsonError> {
    let mut current = ast;
    for (i, seg) in segments.iter().enumerate() {
        match seg {
            PathSegment::Key(key) => {
                current = current
                    .as_object_mut()
                    .ok_or_else(|| {
                        JsonError::Other(format!(
                            "Expected object at segment {} but got non-object",
                            i
                        ))
                    })?
                    .get_mut(key)
                    .ok_or_else(|| JsonError::Other(format!("Key not found: {}", key)))?;
            }
            PathSegment::Index(idx) => {
                let idx = *idx;
                let arr = current.as_array_mut().ok_or_else(|| {
                    JsonError::Other(format!(
                        "Expected array at segment {} but got non-array",
                        i
                    ))
                })?;
                current = arr
                    .get_mut(idx)
                    .ok_or_else(|| JsonError::Other(format!("Array index {} out of bounds", idx)))?;
            }
        }
    }
    Ok(current)
}

// ---------------------------------------------------------------------------
// Canonical JSON (fix 2)
// ---------------------------------------------------------------------------

/// Serialize `value` to canonical JSON with sorted object keys, recursively.
pub fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let pairs: Vec<String> = keys
                .into_iter()
                .map(|k| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap(),
                        canonical_json(&map[k])
                    )
                })
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        Value::Array(arr) => {
            let elems: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", elems.join(","))
        }
        _ => serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a JSON file into a serde_json Value AST.
pub fn parse_json_ast(file_path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

/// Compute a hash anchor for a JSON value at a given path.
/// Uses canonical JSON (sorted keys) for stable hashing.
pub fn compute_json_anchor(path: &str, value: &Value) -> String {
    let canonical = canonical_json(value);
    let hash = compute_line_hash(0, &canonical);
    format!("{}:{}", path, hash)
}

/// Format JSON AST with inline anchor comments.
pub fn format_json_anchors(ast: &Value) -> String {
    format_json_with_anchors_inner(ast, "$", 0)
}

/// JSON-specific edit operations.
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
    /// Object insertion: key name. Omit for array operations.
    pub key: Option<String>,
    /// Array insertion: 0-based index. Omit to append. Ignored when `key` is set.
    pub index: Option<usize>,
    pub value: Value,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeletePathOp {
    pub anchor: String,
}

/// Apply JSON edits to AST atomically.
/// Returns `JsonError::HashMismatch` if any anchor hash does not match the current value.
pub fn apply_json_edits(ast: &mut Value, edits: &[JsonEdit]) -> Result<(), JsonError> {
    // First pass: validate all anchors
    for edit in edits {
        let (path, expected_hash) = match edit {
            JsonEdit::SetPath { set_path: op } => parse_anchor(&op.anchor)?,
            JsonEdit::InsertAtPath { insert_at_path: op } => parse_anchor(&op.anchor)?,
            JsonEdit::DeletePath { delete_path: op } => parse_anchor(&op.anchor)?,
        };
        let segments = parse_path_segments(&path)?;
        let current_value = query_path_segments(ast, &segments)?;
        let current_hash = compute_line_hash(0, &canonical_json(current_value));
        if current_hash != expected_hash {
            return Err(JsonError::HashMismatch {
                path,
                expected: expected_hash,
                actual: current_hash,
            });
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
                insert_at_path(ast, &path, op.key.as_deref(), op.index, op.value.clone())?;
            }
            JsonEdit::DeletePath { delete_path: op } => {
                let (path, _) = parse_anchor(&op.anchor)?;
                delete_path(ast, &path)?;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn parse_anchor(anchor: &str) -> Result<(String, String), JsonError> {
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

fn set_path(ast: &mut Value, path: &str, value: Value) -> Result<(), JsonError> {
    let segments = parse_path_segments(path)?;
    if segments.is_empty() {
        *ast = value;
        return Ok(());
    }
    let (parent_segs, last) = segments.split_at(segments.len() - 1);
    let parent = query_path_segments_mut(ast, parent_segs)?;
    match &last[0] {
        PathSegment::Key(key) => {
            parent
                .as_object_mut()
                .ok_or_else(|| JsonError::Other("Expected object for set_path".to_string()))?
                .insert(key.clone(), value);
        }
        PathSegment::Index(idx) => {
            let arr = parent
                .as_array_mut()
                .ok_or_else(|| JsonError::Other("Expected array for set_path".to_string()))?;
            if *idx < arr.len() {
                arr[*idx] = value;
            } else {
                return Err(JsonError::Other(format!(
                    "Array index {} out of bounds in set_path",
                    idx
                )));
            }
        }
    }
    Ok(())
}

fn insert_at_path(
    ast: &mut Value,
    path: &str,
    key: Option<&str>,
    index: Option<usize>,
    value: Value,
) -> Result<(), JsonError> {
    let segments = parse_path_segments(path)?;
    let target = query_path_segments_mut(ast, &segments)?;
    if let Some(key) = key {
        target
            .as_object_mut()
            .ok_or_else(|| JsonError::Other("Cannot insert key into non-object".to_string()))?
            .insert(key.to_string(), value);
    } else {
        let arr = target
            .as_array_mut()
            .ok_or_else(|| JsonError::Other("Cannot insert into non-array".to_string()))?;
        match index {
            Some(idx) if idx <= arr.len() => arr.insert(idx, value),
            Some(idx) => {
                return Err(JsonError::Other(format!(
                    "Array insert index {} out of bounds (len {})",
                    idx,
                    arr.len()
                )))
            }
            None => arr.push(value),
        }
    }
    Ok(())
}

fn delete_path(ast: &mut Value, path: &str) -> Result<(), JsonError> {
    let segments = parse_path_segments(path)?;
    if segments.is_empty() {
        return Err(JsonError::Other("Cannot delete root".to_string()));
    }
    let (parent_segs, last) = segments.split_at(segments.len() - 1);
    let parent = query_path_segments_mut(ast, parent_segs)?;
    match &last[0] {
        PathSegment::Key(key) => {
            parent
                .as_object_mut()
                .ok_or_else(|| JsonError::Other("Expected object for delete_path".to_string()))?
                .remove(key);
        }
        PathSegment::Index(idx) => {
            let arr = parent
                .as_array_mut()
                .ok_or_else(|| JsonError::Other("Expected array for delete_path".to_string()))?;
            if *idx < arr.len() {
                arr.remove(*idx);
            } else {
                return Err(JsonError::Other(format!(
                    "Array index {} out of bounds in delete_path",
                    idx
                )));
            }
        }
    }
    Ok(())
}

/// Recursive formatter with proper indentation (fix 4).
fn format_json_with_anchors_inner(value: &Value, current_path: &str, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    match value {
        Value::Object(map) => {
            let mut result = "{\n".to_string();
            for (i, (key, val)) in map.iter().enumerate() {
                let path = format!("{}.{}", current_path, key);
                let anchor = compute_json_anchor(&path, val);
                result.push_str(&format!("{}  // {}\n", pad, anchor));
                result.push_str(&format!(
                    "{}  \"{}\": {}",
                    pad,
                    key,
                    format_json_with_anchors_inner(val, &path, indent + 1)
                ));
                if i < map.len() - 1 {
                    result.push(',');
                }
                result.push('\n');
            }
            result.push_str(&format!("{}}}", pad));
            result
        }
        Value::Array(arr) => {
            let mut result = "[\n".to_string();
            for (i, val) in arr.iter().enumerate() {
                let path = format!("{}[{}]", current_path, i);
                let anchor = compute_json_anchor(&path, val);
                result.push_str(&format!("{}  // {}\n", pad, anchor));
                result.push_str(&format!(
                    "{}  {}",
                    pad,
                    format_json_with_anchors_inner(val, &path, indent + 1)
                ));
                if i < arr.len() - 1 {
                    result.push(',');
                }
                result.push('\n');
            }
            result.push_str(&format!("{}]", pad));
            result
        }
        _ => serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".to_string()),
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
        let temp_path = PathBuf::from("/tmp/test_valid_json_rs.json");
        fs::write(&temp_path, json).unwrap();

        let ast = parse_json_ast(&temp_path).unwrap();
        assert_eq!(ast["name"], "test");
        assert_eq!(ast["value"], 42);

        fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn test_parse_invalid_json() {
        let invalid_json = r#"{"name": "test", "value":}"#;
        let temp_path = PathBuf::from("/tmp/test_invalid_json_rs.json");
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

    #[test]
    fn test_canonical_json_sorted_keys() {
        let a: Value = serde_json::from_str(r#"{"b": 1, "a": 2}"#).unwrap();
        let b: Value = serde_json::from_str(r#"{"a": 2, "b": 1}"#).unwrap();
        assert_eq!(canonical_json(&a), canonical_json(&b));
    }

    #[test]
    fn test_parse_path_segments_root() {
        let segs = parse_path_segments("$").unwrap();
        assert!(segs.is_empty());
    }

    #[test]
    fn test_parse_path_segments_nested() {
        let segs = parse_path_segments("$.a.b.c").unwrap();
        assert_eq!(
            segs,
            vec![
                PathSegment::Key("a".to_string()),
                PathSegment::Key("b".to_string()),
                PathSegment::Key("c".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_path_segments_array() {
        let segs = parse_path_segments("$.arr[0]").unwrap();
        assert_eq!(
            segs,
            vec![
                PathSegment::Key("arr".to_string()),
                PathSegment::Index(0),
            ]
        );
    }

    #[test]
    fn test_parse_path_segments_mixed() {
        let segs = parse_path_segments("$.users[0].name").unwrap();
        assert_eq!(
            segs,
            vec![
                PathSegment::Key("users".to_string()),
                PathSegment::Index(0),
                PathSegment::Key("name".to_string()),
            ]
        );
    }

    #[test]
    fn test_set_path_nested() {
        let mut ast = serde_json::json!({"a": {"b": 1}});
        set_path(&mut ast, "$.a.b", serde_json::json!(99)).unwrap();
        assert_eq!(ast["a"]["b"], 99);
    }

    #[test]
    fn test_set_path_array_index() {
        let mut ast = serde_json::json!({"arr": [1, 2, 3]});
        set_path(&mut ast, "$.arr[1]", serde_json::json!(42)).unwrap();
        assert_eq!(ast["arr"][1], 42);
    }

    #[test]
    fn test_insert_at_path_nested() {
        let mut ast = serde_json::json!({"a": {"b": 1}});
        insert_at_path(&mut ast, "$.a", Some("c"), None, serde_json::json!(3)).unwrap();
        assert_eq!(ast["a"]["c"], 3);
    }

    #[test]
    fn test_delete_path_nested() {
        let mut ast = serde_json::json!({"a": {"b": 1, "c": 2}});
        delete_path(&mut ast, "$.a.b").unwrap();
        assert!(ast["a"].get("b").is_none());
        assert_eq!(ast["a"]["c"], 2);
    }

    #[test]
    fn test_apply_json_edits_hash_mismatch_returns_typed_error() {
        let mut ast = serde_json::json!({"version": "1.0"});
        let edits = vec![JsonEdit::SetPath {
            set_path: SetPathOp {
                anchor: "$.version:ff".to_string(), // wrong hash
                value: serde_json::json!("2.0"),
            },
        }];
        let result = apply_json_edits(&mut ast, &edits);
        assert!(matches!(result, Err(JsonError::HashMismatch { .. })));
    }
}

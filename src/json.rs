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

impl From<Box<dyn std::error::Error>> for JsonError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        JsonError::Other(err.to_string())
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
                let idx: usize = idx_str
                    .parse()
                    .map_err(|_| format!("Invalid array index '{}' in path: {}", idx_str, path))?;
                if i < len && bytes[i] == b']' {
                    i += 1; // skip ']'
                }
                segments.push(PathSegment::Index(idx));
            }
            other => {
                return Err(
                    format!("Unexpected character '{}' in path: {}", other as char, path).into(),
                );
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
                    JsonError::Other(format!("Expected array at segment {} but got non-array", i))
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
                    JsonError::Other(format!("Expected array at segment {} but got non-array", i))
                })?;
                current = arr.get_mut(idx).ok_or_else(|| {
                    JsonError::Other(format!("Array index {} out of bounds", idx))
                })?;
            }
        }
    }
    Ok(current)
}

// ---------------------------------------------------------------------------
// Canonical hash (optimized, direct xxh32, zero string allocs)
pub fn compute_canonical_hash(value: &Value) -> String {
    let mut buf = Vec::new();
    hash_canonical(&mut buf, value).expect("hash_canonical failed");
    let h = xxhash_rust::xxh32::xxh32(&buf, 0) % 256u32;
    format!("{:02x}", h as u8)
}

fn hash_canonical<W: std::io::Write>(w: &mut W, value: &Value) -> std::io::Result<()> {
    match value {
        Value::Null => w.write_all(b"null")?,
        Value::Bool(true) => w.write_all(b"true")?,
        Value::Bool(false) => w.write_all(b"false")?,
        Value::Number(n) => w.write_all(n.to_string().as_bytes())?,
        Value::String(s) => {
            w.write_all(b"\"")?;
            for &b in s.as_bytes() {
                match b {
                    b'\"' => w.write_all(b"\\\"")?,
                    b'\\' => w.write_all(b"\\\\")?,
                    b'\n' => w.write_all(b"\\n")?,
                    b'\r' => w.write_all(b"\\r")?,
                    b'\t' => w.write_all(b"\\t")?,
                    b'\x08' => w.write_all(b"\\b")?,
                    b'\x0c' => w.write_all(b"\\f")?,
                    b if b < 0x20u8 => {
                        let s = format!("\\u{:04x}", b as u32);
                        w.write_all(s.as_bytes())?;
                    }
                    _ => w.write_all(&[b])?,
                }
            }
            w.write_all(b"\"")?
        }
        Value::Array(arr) => {
            w.write_all(b"[")?;
            let mut first = true;
            for v in arr.iter() {
                if !first {
                    w.write_all(b",")?;
                }
                first = false;
                hash_canonical(w, v)?;
            }
            w.write_all(b"]")?
        }
        Value::Object(map) => {
            w.write_all(b"{")?;
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            let mut first = true;
            for key in keys.iter() {
                if !first {
                    w.write_all(b",")?;
                }
                first = false;
                w.write_all(b"\"")?;
                for &b in key.as_bytes() {
                    match b {
                        b'\"' => w.write_all(b"\\\"")?,
                        b'\\' => w.write_all(b"\\\\")?,
                        b'\n' => w.write_all(b"\\n")?,
                        b'\r' => w.write_all(b"\\r")?,
                        b'\t' => w.write_all(b"\\t")?,
                        b'\x08' => w.write_all(b"\\b")?,
                        b'\x0c' => w.write_all(b"\\f")?,
                        b if b < 0x20u8 => {
                            let s = format!("\\u{:04x}", b as u32);
                            w.write_all(s.as_bytes())?;
                        }
                        _ => w.write_all(&[b])?,
                    }
                }
                w.write_all(b"\":")?;
                hash_canonical(w, map.get(key.as_str()).unwrap())?;
            }
            w.write_all(b"}")?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a JSON file into a serde_json Value AST.
pub fn parse_json_ast(file_path: &Path) -> Result<Value, JsonError> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| JsonError::from(Box::new(e) as Box<dyn std::error::Error>))?;
    let value = serde_json::from_str(&content)?;
    Ok(value)
}

/// Compute a hash anchor for a JSON value at a given path.
/// (stable canonical hash with sorted keys).
pub fn compute_json_anchor(path: &str, value: &Value) -> String {
    format!("{}:{}", path, compute_canonical_hash(value))
}

/// Format JSON AST with inline anchor comments.
pub fn format_json_anchors(ast: &Value) -> String {
    let mut buf = String::new();
    let _ = format_json_with_anchors_inner(&mut buf, ast, "$", 0);
    buf
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
        let current_hash = compute_canonical_hash(current_value);
        if current_hash != expected_hash {
            return Err(JsonError::HashMismatch {
                path,
                expected: expected_hash,
                actual: current_hash,
            });
        }
    }

    let mut cloned_ast = ast.clone();

    // Apply edits to clone atomically
    for edit in edits {
        match edit {
            JsonEdit::SetPath { set_path: op } => {
                let (path, _) = parse_anchor(&op.anchor)?;
                set_path(&mut cloned_ast, &path, op.value.clone())?;
            }
            JsonEdit::InsertAtPath { insert_at_path: op } => {
                let (path, _) = parse_anchor(&op.anchor)?;
                insert_at_path(
                    &mut cloned_ast,
                    &path,
                    op.key.as_deref(),
                    op.index,
                    op.value.clone(),
                )?;
            }
            JsonEdit::DeletePath { delete_path: op } => {
                let (path, _) = parse_anchor(&op.anchor)?;
                delete_path(&mut cloned_ast, &path)?;
            }
        }
    }

    *ast = cloned_ast;

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn parse_anchor(anchor: &str) -> Result<(String, String), JsonError> {
    if let Some(colon_pos) = anchor.rfind(':') {
        let path = &anchor[..colon_pos];
        let hash = &anchor[colon_pos + 1..];
        let len = hash.len();
        if (1..=16).contains(&len) && hash.chars().all(|c| c.is_ascii_alphanumeric()) {
            Ok((path.to_string(), hash.to_string()))
        } else {
            Err(format!(
                "Invalid hash format in anchor (must be 1-16 alphanumeric chars): {}",
                anchor
            )
            .into())
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

/// Formats a JSON value with anchor comments.
fn format_json_with_anchors_inner<W: std::fmt::Write>(
    w: &mut W,
    value: &Value,
    current_path: &str,
    indent: usize,
) -> std::fmt::Result {
    fn write_indent<W: std::fmt::Write>(w: &mut W, depth: usize) -> std::fmt::Result {
        for _ in 0..depth {
            w.write_str("  ")?;
        }
        Ok(())
    }
    fn render_value<W: std::fmt::Write>(
        w: &mut W,
        value: &Value,
        current_path: &str,
        indent: usize,
        needs_comma: bool,
    ) -> std::fmt::Result {
        match value {
            Value::Object(map) => {
                if map.is_empty() {
                    write_indent(w, indent)?;
                    if needs_comma {
                        writeln!(w, "{{}},")?;
                    } else {
                        writeln!(w, "{{}}")?;
                    }
                    return Ok(());
                }

                write_indent(w, indent)?;
                writeln!(w, "{{")?;
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort_unstable();
                for (index, key) in keys.iter().enumerate() {
                    let child = map.get(*key).unwrap();
                    let child_path = if current_path == "$" {
                        format!("$.{}", key)
                    } else {
                        format!("{}.{}", current_path, key)
                    };

                    write_indent(w, indent + 1)?;
                    writeln!(w, "// {}", compute_json_anchor(&child_path, child))?;
                    write_indent(w, indent + 1)?;
                    let key_repr = serde_json::to_string(key).map_err(|_| std::fmt::Error)?;
                    write!(w, "{}: ", key_repr)?;
                    let is_last = index + 1 == keys.len();
                    match child {
                        Value::Object(_) | Value::Array(_) => {
                            writeln!(w)?;
                            render_value(w, child, &child_path, indent + 1, !is_last)?;
                        }
                        _ => {
                            let value_repr =
                                serde_json::to_string(child).map_err(|_| std::fmt::Error)?;
                            if is_last {
                                writeln!(w, "{}", value_repr)?;
                            } else {
                                writeln!(w, "{},", value_repr)?;
                            }
                        }
                    }
                }
                write_indent(w, indent)?;
                if needs_comma {
                    writeln!(w, "}},")?;
                } else {
                    writeln!(w, "}}")?;
                }
                Ok(())
            }
            Value::Array(items) => {
                if items.is_empty() {
                    write_indent(w, indent)?;
                    if needs_comma {
                        writeln!(w, "[],")?;
                    } else {
                        writeln!(w, "[]")?;
                    }
                    return Ok(());
                }

                write_indent(w, indent)?;
                writeln!(w, "[")?;
                for (index, item) in items.iter().enumerate() {
                    let child_path = format!("{}[{}]", current_path, index);
                    write_indent(w, indent + 1)?;
                    writeln!(w, "// {}", compute_json_anchor(&child_path, item))?;
                    let is_last = index + 1 == items.len();
                    match item {
                        Value::Object(_) | Value::Array(_) => {
                            render_value(w, item, &child_path, indent + 1, !is_last)?;
                        }
                        _ => {
                            write_indent(w, indent + 1)?;
                            let value_repr =
                                serde_json::to_string(item).map_err(|_| std::fmt::Error)?;
                            if is_last {
                                writeln!(w, "{}", value_repr)?;
                            } else {
                                writeln!(w, "{},", value_repr)?;
                            }
                        }
                    }
                }
                write_indent(w, indent)?;
                if needs_comma {
                    writeln!(w, "],")?;
                } else {
                    writeln!(w, "]")?;
                }
                Ok(())
            }
            _ => {
                write_indent(w, indent)?;
                let value_repr = serde_json::to_string(value).map_err(|_| std::fmt::Error)?;
                if needs_comma {
                    writeln!(w, "{},", value_repr)
                } else {
                    writeln!(w, "{}", value_repr)
                }
            }
        }
    }

    write_indent(w, indent)?;
    writeln!(w, "// {}", compute_json_anchor(current_path, value))?;
    render_value(w, value, current_path, indent, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_valid_json() {
        let json = r#"{"name": "test", "value": 42}"#;
        let temp = NamedTempFile::new().unwrap();
        fs::write(temp.path(), json).unwrap();

        let ast = parse_json_ast(temp.path()).unwrap();
        assert_eq!(ast["name"], "test");
        assert_eq!(ast["value"], 42);
    }

    #[test]
    fn test_parse_invalid_json() {
        let invalid_json = r#"{"name": "test", "value":}"#;
        let temp = NamedTempFile::new().unwrap();
        fs::write(temp.path(), invalid_json).unwrap();

        let result = parse_json_ast(temp.path());
        assert!(result.is_err());
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
    fn test_canonical_hash_sorted_keys() {
        let a: Value = serde_json::from_str(r#"{"b": 1, "a": 2}"#).unwrap();
        let b: Value = serde_json::from_str(r#"{"a": 2, "b": 1}"#).unwrap();
        let hash_a = compute_canonical_hash(&a);
        let hash_b = compute_canonical_hash(&b);
        assert_eq!(hash_a.len(), 2);
        assert!(hash_a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn test_parse_anchor_accepts_long_hash() {
        let (path, hash) =
            parse_anchor("$.config.value:abcdEFGH12345678").expect("anchor should parse");
        assert_eq!(path, "$.config.value");
        assert_eq!(hash, "abcdEFGH12345678");
    }

    #[test]
    fn test_parse_anchor_rejects_overlong_hash() {
        assert!(parse_anchor("$.config.value:12345678901234567").is_err());
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
            vec![PathSegment::Key("arr".to_string()), PathSegment::Index(0),]
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

use hashline::json::{
    apply_json_edits, compute_json_anchor, format_json_anchors, parse_json_ast, DeletePathOp,
    InsertAtPathOp, JsonEdit, JsonError, SetPathOp,
};
use serde_json::{json, Value};
use std::path::Path;

fn load_small() -> Value {
    parse_json_ast(Path::new("tests/fixtures/json/small.json")).unwrap()
}

fn load_medium() -> Value {
    parse_json_ast(Path::new("tests/fixtures/json/medium.json")).unwrap()
}

// ---------------------------------------------------------------------------
// small.json tests
// ---------------------------------------------------------------------------

#[test]
fn json_read_small_has_anchors() {
    let ast = load_small();
    let output = format_json_anchors(&ast);

    assert!(output.contains("// $.version:"), "missing $.version anchor");
    assert!(output.contains("// $.name:"), "missing $.name anchor");
    assert!(output.contains("// $.scripts.build:"), "missing $.scripts.build anchor");
    assert!(output.contains("\"1.1.0\""), "missing actual version value");
}

#[test]
fn json_set_top_level_key() {
    let ast = load_small();
    let anchor = compute_json_anchor("$.version", &ast["version"]);
    let mut ast = ast;

    let result = apply_json_edits(
        &mut ast,
        &[JsonEdit::SetPath {
            set_path: SetPathOp {
                anchor,
                value: json!("2.0.0"),
            },
        }],
    );

    assert!(result.is_ok(), "set_path failed: {:?}", result.err());
    assert_eq!(ast["version"], "2.0.0");
}

#[test]
fn json_set_nested_key() {
    let ast = load_small();
    let anchor = compute_json_anchor("$.scripts.test", &ast["scripts"]["test"]);
    let mut ast = ast;

    let result = apply_json_edits(
        &mut ast,
        &[JsonEdit::SetPath {
            set_path: SetPathOp {
                anchor,
                value: json!("vitest"),
            },
        }],
    );

    assert!(result.is_ok(), "set_path nested failed: {:?}", result.err());
    assert_eq!(ast["scripts"]["test"], "vitest");
}

#[test]
fn json_delete_top_level_key() {
    let ast = load_small();
    let anchor = compute_json_anchor("$.license", &ast["license"]);
    let mut ast = ast;

    let result = apply_json_edits(
        &mut ast,
        &[JsonEdit::DeletePath {
            delete_path: DeletePathOp { anchor },
        }],
    );

    assert!(result.is_ok(), "delete_path failed: {:?}", result.err());
    assert!(ast.get("license").is_none(), "license key should be absent");
}

#[test]
fn json_delete_nested_key() {
    let ast = load_small();
    let anchor = compute_json_anchor("$.scripts.start", &ast["scripts"]["start"]);
    let mut ast = ast;

    let result = apply_json_edits(
        &mut ast,
        &[JsonEdit::DeletePath {
            delete_path: DeletePathOp { anchor },
        }],
    );

    assert!(result.is_ok(), "delete_path nested failed: {:?}", result.err());
    assert!(
        ast["scripts"].get("start").is_none(),
        "scripts.start should be removed"
    );
    // other scripts keys must still be present
    assert!(ast["scripts"].get("build").is_some(), "build should remain");
    assert!(ast["scripts"].get("test").is_some(), "test should remain");
}

#[test]
fn json_insert_into_object() {
    let ast = load_small();
    let anchor = compute_json_anchor("$.dependencies", &ast["dependencies"]);
    let mut ast = ast;

    let result = apply_json_edits(
        &mut ast,
        &[JsonEdit::InsertAtPath {
            insert_at_path: InsertAtPathOp {
                anchor,
                key: Some("lodash".to_string()),
                value: json!("^4.17.0"),
            },
        }],
    );

    assert!(result.is_ok(), "insert_at_path failed: {:?}", result.err());
    assert_eq!(ast["dependencies"]["lodash"], "^4.17.0");
    // existing key must survive
    assert_eq!(ast["dependencies"]["express"], "^4.17.1");
}

#[test]
fn json_hash_mismatch_returns_typed_error() {
    let mut ast = load_small();

    let result = apply_json_edits(
        &mut ast,
        &[JsonEdit::SetPath {
            set_path: SetPathOp {
                anchor: "$.version:ff".to_string(), // deliberately wrong hash
                value: json!("9.9.9"),
            },
        }],
    );

    match result {
        Err(JsonError::HashMismatch { path, .. }) => {
            assert_eq!(path, "$.version");
        }
        other => panic!("expected HashMismatch, got {:?}", other),
    }
}

#[test]
fn json_atomicity_first_ok_second_stale() {
    let ast = load_small();
    let version_anchor = compute_json_anchor("$.version", &ast["version"]);
    let original_version = ast["version"].clone();
    let mut ast = ast;

    // Second edit has wrong hash — the whole batch should fail.
    let result = apply_json_edits(
        &mut ast,
        &[
            JsonEdit::SetPath {
                set_path: SetPathOp {
                    anchor: version_anchor,
                    value: json!("3.0.0"),
                },
            },
            JsonEdit::SetPath {
                set_path: SetPathOp {
                    anchor: "$.name:ff".to_string(), // wrong hash
                    value: json!("hacked"),
                },
            },
        ],
    );

    assert!(result.is_err(), "expected error due to stale second edit");
    // Atomicity: version must be unchanged
    assert_eq!(
        ast["version"], original_version,
        "version should be unchanged (atomic rollback)"
    );
}

#[test]
fn json_canonical_hash_key_order_independence() {
    // Build two objects with same keys/values but different insertion order.
    let v1: Value = serde_json::from_str(r#"{"z": 1, "a": 2, "m": 3}"#).unwrap();
    let v2: Value = serde_json::from_str(r#"{"a": 2, "m": 3, "z": 1}"#).unwrap();

    let anchor1 = compute_json_anchor("$.test", &v1);
    let anchor2 = compute_json_anchor("$.test", &v2);

    assert_eq!(
        anchor1, anchor2,
        "anchors must be equal regardless of key insertion order"
    );
}

// ---------------------------------------------------------------------------
// medium.json tests
// ---------------------------------------------------------------------------

#[test]
fn json_set_deeply_nested() {
    let ast = load_medium();
    let anchor =
        compute_json_anchor("$.database.credentials.username", &ast["database"]["credentials"]["username"]);
    let mut ast = ast;

    let result = apply_json_edits(
        &mut ast,
        &[JsonEdit::SetPath {
            set_path: SetPathOp {
                anchor,
                value: json!("superuser"),
            },
        }],
    );

    assert!(result.is_ok(), "deeply nested set_path failed: {:?}", result.err());
    assert_eq!(ast["database"]["credentials"]["username"], "superuser");
}

#[test]
fn json_round_trip_read_then_apply() {
    let ast = load_medium();
    let output = format_json_anchors(&ast);

    // Find the anchor line for $.app.version.
    // format_json_anchors emits lines like:  "  // $.app.version:XX"
    let anchor = output
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("// $.app.version:") {
                Some(trimmed.trim_start_matches("// ").to_string())
            } else {
                None
            }
        })
        .expect("could not find $.app.version anchor in format_json_anchors output");

    let mut ast = ast;
    let result = apply_json_edits(
        &mut ast,
        &[JsonEdit::SetPath {
            set_path: SetPathOp {
                anchor,
                value: json!("3.0.0"),
            },
        }],
    );

    assert!(result.is_ok(), "round-trip apply failed: {:?}", result.err());
    assert_eq!(ast["app"]["version"], "3.0.0");
}

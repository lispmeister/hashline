use hashline::json::{
    apply_json_edits, compute_json_anchor, format_json_anchors, parse_json_ast, DeletePathOp,
    InsertAtPathOp, JsonEdit, JsonError, SetPathOp,
};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

fn load_small() -> Value {
    parse_json_ast(Path::new("tests/fixtures/json/small.json")).unwrap()
}

fn load_medium() -> Value {
    parse_json_ast(Path::new("tests/fixtures/json/medium.json")).unwrap()
}

fn load_large() -> Value {
    parse_json_ast(Path::new("tests/fixtures/json/large.json")).unwrap()
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
    assert!(
        output.contains("// $.scripts.build:"),
        "missing $.scripts.build anchor"
    );
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

    assert!(
        result.is_ok(),
        "delete_path nested failed: {:?}",
        result.err()
    );
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
                index: None,
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
fn json_atomicity_delete_then_set() {
    let ast = load_small();
    let scripts_anchor = compute_json_anchor("$.scripts", &ast["scripts"]);
    let test_anchor = compute_json_anchor("$.scripts.test", &ast["scripts"]["test"]);
    let original_test = ast["scripts"]["test"].clone();

    let mut ast2 = ast.clone();
    let edits = vec![
        JsonEdit::DeletePath {
            delete_path: DeletePathOp {
                anchor: scripts_anchor,
            },
        },
        JsonEdit::SetPath {
            set_path: SetPathOp {
                anchor: test_anchor,
                value: json!("vitest"),
            },
        },
    ];

    let result = apply_json_edits(&mut ast2, &edits);
    assert!(
        result.is_err(),
        "expected Err due to delete-then-set conflict"
    );

    // Atomicity: no changes applied
    assert!(ast2["scripts"].is_object());
    assert_eq!(ast2["scripts"]["test"], original_test);
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
    let anchor = compute_json_anchor(
        "$.database.credentials.username",
        &ast["database"]["credentials"]["username"],
    );
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

    assert!(
        result.is_ok(),
        "deeply nested set_path failed: {:?}",
        result.err()
    );
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

    assert!(
        result.is_ok(),
        "round-trip apply failed: {:?}",
        result.err()
    );
    assert_eq!(ast["app"]["version"], "3.0.0");
}

#[test]
fn json_insert_array_index() {
    let ast = load_medium();
    let anchor = compute_json_anchor("$.users", &ast["users"]);
    let mut ast = ast;

    let result = apply_json_edits(
        &mut ast,
        &[JsonEdit::InsertAtPath {
            insert_at_path: InsertAtPathOp {
                anchor,
                key: None,
                index: Some(1),
                value: json!({
                    "id": 99,
                    "name": "Eve",
                    "email": "eve@example.com",
                    "role": "admin",
                    "active": true
                }),
            },
        }],
    );

    assert!(
        result.is_ok(),
        "insert_at_path array index failed: {:?}",
        result.err()
    );

    let users = ast["users"].as_array().unwrap();
    assert_eq!(users.len(), 4);
    assert_eq!(users[1]["name"], "Eve");
    assert_eq!(users[2]["name"], "Bob Smith");
    assert_eq!(users[0]["name"], "Alice Johnson");
    assert_eq!(users[3]["name"], "Charlie Brown");
}

// ---------------------------------------------------------------------------
// large.json tests
// ---------------------------------------------------------------------------

#[test]
fn json_large_fixture_round_trip() {
    let mut ast = load_large();
    let anchor = compute_json_anchor("$.items[0].name", &ast["items"][0]["name"]);

    let result = apply_json_edits(
        &mut ast,
        &[JsonEdit::SetPath {
            set_path: SetPathOp {
                anchor,
                value: json!("Renamed Item 0"),
            },
        }],
    );

    assert!(
        result.is_ok(),
        "set_path on large fixture failed: {:?}",
        result.err()
    );
    assert_eq!(ast["items"][0]["name"], "Renamed Item 0");

    let formatted = format_json_anchors(&ast);
    assert!(
        formatted.contains("// $.metadata.version:"),
        "missing metadata version anchor"
    );
    assert!(
        formatted.contains("// $.items[99].name:"),
        "missing deep array anchor"
    );
}

#[test]
fn cli_json_roundtrip_special_keys() {
    let tmp = NamedTempFile::new().unwrap();
    fs::write(tmp.path(), r#"{"a.b": {"c d": 1}}"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_hashline"))
        .args(["json-read", tmp.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(r#"// $["a.b"]:"#));
    assert!(stdout.contains(r#"// $["a.b"]["c d"]:"#));

    let ast: Value = serde_json::from_str(&fs::read_to_string(tmp.path()).unwrap()).unwrap();
    let anchor_path = r#"$["a.b"]["c d"]"#;
    let anchor = compute_json_anchor(anchor_path, &ast["a.b"]["c d"]);
    let payload = json!({
        "path": tmp.path().to_str().unwrap(),
        "edits": [
            {"set_path": {"anchor": anchor, "value": 2}}
        ]
    });
    let payload_file = NamedTempFile::new().unwrap();
    fs::write(
        payload_file.path(),
        serde_json::to_string(&payload).unwrap(),
    )
    .unwrap();

    let apply_output = Command::new(env!("CARGO_BIN_EXE_hashline"))
        .args([
            "json-apply",
            "--input",
            payload_file.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(apply_output.status.success());

    let updated: Value = serde_json::from_str(&fs::read_to_string(tmp.path()).unwrap()).unwrap();
    assert_eq!(updated["a.b"]["c d"], 2);
}

#[test]
fn cli_json_apply_mismatch_reports_error() {
    let tmp = NamedTempFile::new().unwrap();
    fs::write(tmp.path(), r#"{"version": "1.0"}"#).unwrap();

    let payload = json!({
        "path": tmp.path().to_str().unwrap(),
        "edits": [
            {"set_path": {"anchor": "$.version:ff", "value": "2.0"}}
        ]
    });
    let payload_file = NamedTempFile::new().unwrap();
    fs::write(
        payload_file.path(),
        serde_json::to_string(&payload).unwrap(),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_hashline"))
        .args([
            "json-apply",
            "--input",
            payload_file.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("expected hash"));
    assert!(stderr.contains("current hash"));
    assert!(stderr.contains("updated anchor"));
}

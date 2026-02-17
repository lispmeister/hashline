//! LLM comparison tests: hashline edits vs raw search-and-replace.
//!
//! Loads fixture JSON files from tests/fixtures/ and applies edits both ways,
//! comparing results to the expected output.

use hashline::{apply_hashline_edits, HashlineEdit};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
struct RawEdit {
    old_text: String,
    new_text: String,
}

#[derive(Deserialize)]
struct Fixture {
    name: String,
    description: String,
    original_content: String,
    expected_content: Option<String>,
    hashline_edits: Vec<HashlineEdit>,
    #[serde(default)]
    hashline_should_fail: bool,
    raw_edit: Option<RawEdit>,
    raw_edits: Option<Vec<RawEdit>>,
    #[serde(default)]
    raw_edit_note: Option<String>,
    #[serde(default)]
    hashline_fail_reason: Option<String>,
}

/// Apply raw search-and-replace edit(s) to content.
/// Returns None if old_text not found.
fn apply_raw_edit(content: &str, edit: &RawEdit) -> Option<String> {
    if let Some(pos) = content.find(&edit.old_text) {
        let mut result = String::with_capacity(content.len());
        result.push_str(&content[..pos]);
        result.push_str(&edit.new_text);
        result.push_str(&content[pos + edit.old_text.len()..]);
        Some(result)
    } else {
        None
    }
}

fn apply_raw_edits(content: &str, edits: &[RawEdit]) -> Option<String> {
    let mut current = content.to_string();
    for edit in edits {
        current = apply_raw_edit(&current, edit)?;
    }
    Some(current)
}

fn load_fixtures() -> Vec<(String, Fixture)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut fixtures: Vec<(String, Fixture)> = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("tests/fixtures/ directory not found")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let content = fs::read_to_string(&path).unwrap();
        let fixture: Fixture = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e));
        fixtures.push((path.file_name().unwrap().to_string_lossy().into(), fixture));
    }
    fixtures
}

#[test]
fn comparison_all_fixtures() {
    let fixtures = load_fixtures();
    assert!(!fixtures.is_empty(), "No fixtures found");

    let mut results: Vec<(String, bool, bool, String)> = Vec::new();

    for (filename, fixture) in &fixtures {
        // --- Hashline mode ---
        let hashline_result =
            apply_hashline_edits(&fixture.original_content, &fixture.hashline_edits);

        let hashline_ok = if fixture.hashline_should_fail {
            hashline_result.is_err()
        } else {
            match &hashline_result {
                Ok(r) => {
                    fixture.expected_content.as_ref().is_some_and(|exp| r.content == *exp)
                }
                Err(_) => false,
            }
        };

        // --- Raw mode ---
        let raw_result = if let Some(edits) = &fixture.raw_edits {
            apply_raw_edits(&fixture.original_content, edits)
        } else if let Some(edit) = &fixture.raw_edit {
            apply_raw_edit(&fixture.original_content, edit)
        } else {
            None
        };

        let raw_ok = if fixture.hashline_should_fail {
            // For stale-content scenarios, raw mode should also fail (old_text not found)
            raw_result.is_none()
                || fixture
                    .expected_content
                    .as_ref()
                    .is_some_and(|exp| raw_result.as_ref().is_some_and(|r| r == exp))
        } else {
            match (&raw_result, &fixture.expected_content) {
                (Some(raw), Some(exp)) => raw == exp,
                _ => false,
            }
        };

        results.push((
            fixture.name.clone(),
            hashline_ok,
            raw_ok,
            filename.clone(),
        ));
    }

    // Print summary table
    println!("\n{}", "=".repeat(80));
    println!(
        "{:<45} {:>10} {:>10}",
        "Scenario", "Hashline", "Raw"
    );
    println!("{}", "-".repeat(80));

    let mut hashline_pass = 0;
    let mut raw_pass = 0;
    let total = results.len();

    for (name, h_ok, r_ok, _filename) in &results {
        let h_str = if *h_ok { "PASS" } else { "FAIL" };
        let r_str = if *r_ok { "PASS" } else { "FAIL" };
        println!("{:<45} {:>10} {:>10}", name, h_str, r_str);
        if *h_ok {
            hashline_pass += 1;
        }
        if *r_ok {
            raw_pass += 1;
        }
    }

    println!("{}", "-".repeat(80));
    println!(
        "{:<45} {:>7}/{:<2} {:>7}/{:<2}",
        "TOTAL", hashline_pass, total, raw_pass, total
    );
    println!("{}", "=".repeat(80));

    // Assert all hashline edits pass
    for (name, h_ok, _, filename) in &results {
        assert!(
            h_ok,
            "Hashline edit FAILED for '{}' ({})",
            name, filename
        );
    }
}

// Individual fixture tests for granularity
#[test]
fn fixture_01_simple_single_line() {
    run_fixture("01_simple_single_line.json");
}

#[test]
fn fixture_02_context_ambiguity() {
    run_fixture("02_context_ambiguity.json");
}

#[test]
fn fixture_03_many_similar_lines() {
    run_fixture("03_many_similar_lines.json");
}

#[test]
fn fixture_04_indentation_sensitive() {
    run_fixture("04_indentation_sensitive.json");
}

#[test]
fn fixture_05_stale_content() {
    run_fixture("05_stale_content.json");
}

#[test]
fn fixture_06_large_range_replacement() {
    run_fixture("06_large_range_replacement.json");
}

#[test]
fn fixture_07_insert_after() {
    run_fixture("07_insert_after.json");
}

#[test]
fn fixture_08_delete_line() {
    run_fixture("08_delete_line.json");
}

#[test]
fn fixture_09_multiple_edits() {
    run_fixture("09_multiple_edits.json");
}

#[test]
fn fixture_10_duplicate_code_blocks() {
    run_fixture("10_duplicate_code_blocks.json");
}

fn run_fixture(filename: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(filename);
    let content = fs::read_to_string(&path).unwrap();
    let fixture: Fixture = serde_json::from_str(&content).unwrap();

    let result = apply_hashline_edits(&fixture.original_content, &fixture.hashline_edits);

    if fixture.hashline_should_fail {
        assert!(
            result.is_err(),
            "Expected hashline edit to fail for '{}' but it succeeded",
            fixture.name
        );
    } else {
        let result = result.unwrap_or_else(|e| {
            panic!("Hashline edit failed for '{}': {}", fixture.name, e)
        });
        let expected = fixture.expected_content.as_ref().unwrap();
        assert_eq!(
            result.content, *expected,
            "Hashline result mismatch for '{}'",
            fixture.name
        );
    }
}

use clap::Parser;
use std::io::Read;
use std::path::Path;

use std::process;

mod cli;
mod edit;
mod error;
mod format;
mod hash;
mod heuristics;
mod json;
mod parse;
mod usage;
mod util;

use cli::{Cli, Commands};
use usage::{log_event, UsageEvent, UsageResult};
use util::read_normalized;

fn record_usage(
    command: &'static str,
    result: UsageResult,
    emit_updated: bool,
    used_input_file: bool,
) {
    let _ = log_event(UsageEvent {
        command,
        result,
        emit_updated,
        used_input_file,
    });
}

fn exit_with(
    code: i32,
    command: &'static str,
    result: UsageResult,
    emit_updated: bool,
    used_input_file: bool,
) -> ! {
    record_usage(command, result, emit_updated, used_input_file);
    process::exit(code);
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Read {
            file,
            start_line,
            lines,
        } => {
            let content = match read_normalized(Path::new(&file)) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading {}: {}", file, e);
                    exit_with(2, "read", UsageResult::Error, false, false);
                }
            };
            let all_lines: Vec<&str> = content.split('\n').collect();
            let start_idx = start_line.saturating_sub(1).min(all_lines.len());
            let end_idx = if let Some(n) = lines {
                (start_idx + n).min(all_lines.len())
            } else {
                all_lines.len()
            };
            let slice = &all_lines[start_idx..end_idx];
            if !slice.is_empty() {
                let sliced_content = slice.join("\n");
                println!("{}", format::format_hashlines(&sliced_content, start_line));
            }
            record_usage("read", UsageResult::Success, false, false);
        }
        Commands::Apply {
            input,
            emit_updated,
        } => {
            let used_input_file = input.is_some();

            let input_data = if let Some(ref path) = input {
                match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error reading input file {}: {}", path, e);
                        exit_with(
                            2,
                            "apply",
                            UsageResult::Error,
                            emit_updated,
                            used_input_file,
                        );
                    }
                }
            } else {
                let mut buf = String::new();
                if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
                    eprintln!("Error reading stdin: {}", e);
                    exit_with(
                        2,
                        "apply",
                        UsageResult::Error,
                        emit_updated,
                        used_input_file,
                    );
                }
                buf
            };

            let params: edit::HashlineParams = match serde_json::from_str(&input_data) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Invalid JSON input: {}", e);
                    exit_with(
                        2,
                        "apply",
                        UsageResult::Error,
                        emit_updated,
                        used_input_file,
                    );
                }
            };

            let content = match read_normalized(Path::new(&params.path)) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading {}: {}", params.path, e);
                    exit_with(
                        2,
                        "apply",
                        UsageResult::Error,
                        emit_updated,
                        used_input_file,
                    );
                }
            };

            // Anchor edits run first, then replace edits on the result
            let anchor_edits: Vec<_> = params
                .edits
                .iter()
                .filter(|e| !matches!(e, edit::HashlineEdit::Replace { .. }))
                .cloned()
                .collect();
            let replace_edits: Vec<_> = params
                .edits
                .iter()
                .filter(|e| matches!(e, edit::HashlineEdit::Replace { .. }))
                .cloned()
                .collect();

            let anchor_result = match edit::apply_hashline_edits(&content, &anchor_edits) {
                Ok(r) => r,
                Err(e) => {
                    if e.downcast_ref::<error::HashlineMismatchError>().is_some() {
                        eprintln!("{}", e);
                        exit_with(
                            1,
                            "apply",
                            UsageResult::Mismatch,
                            emit_updated,
                            used_input_file,
                        );
                    } else {
                        eprintln!("Error: {}", e);
                        exit_with(
                            2,
                            "apply",
                            UsageResult::Error,
                            emit_updated,
                            used_input_file,
                        );
                    }
                }
            };

            let mut final_content = anchor_result.content;
            let mut replace_first_changed = None;
            let mut replace_replacements = 0usize;
            if !replace_edits.is_empty() {
                match edit::apply_replace_edits(&final_content, &replace_edits) {
                    Ok(r) => {
                        replace_first_changed = r.first_changed_line;
                        replace_replacements = r.replacements;
                        final_content = r.content;
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        exit_with(
                            2,
                            "apply",
                            UsageResult::Error,
                            emit_updated,
                            used_input_file,
                        );
                    }
                }
            }

            let mut output = final_content;
            output.push('\n');
            if let Err(e) = std::fs::write(&params.path, &output) {
                eprintln!("Error writing {}: {}", params.path, e);
                exit_with(
                    2,
                    "apply",
                    UsageResult::Error,
                    emit_updated,
                    used_input_file,
                );
            }
            if !anchor_result.warnings.is_empty() {
                for w in &anchor_result.warnings {
                    eprintln!("Warning: {}", w);
                }
            }

            let had_anchor_changes = anchor_result.first_changed_line.is_some();
            let had_replace_changes = replace_replacements > 0;
            if emit_updated {
                let first_line = match (anchor_result.first_changed_line, replace_first_changed) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };
                if let Some(first_line) = first_line {
                    let updated = read_normalized(Path::new(&params.path)).unwrap_or_default();
                    let all_lines: Vec<&str> = updated.split('\n').collect();
                    let context = 2;
                    let start = first_line.saturating_sub(1 + context);
                    let edits_count = params.edits.len();
                    let end = all_lines
                        .len()
                        .min(start + (edits_count * 3).max(10) + context * 2);
                    let slice = &all_lines[start..end];
                    if !slice.is_empty() {
                        let sliced_content = slice.join("\n");
                        println!("---");
                        println!("{}", format::format_hashlines(&sliced_content, start + 1));
                    }
                }
            }

            if !had_anchor_changes && !had_replace_changes {
                println!("No changes applied.");
            }
            record_usage("apply", UsageResult::Success, emit_updated, used_input_file);
        }
        Commands::Hash { file } => {
            let content = match read_normalized(Path::new(&file)) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading {}: {}", file, e);
                    exit_with(2, "hash", UsageResult::Error, false, false);
                }
            };
            for (i, line) in content.split('\n').enumerate() {
                let num = i + 1;
                println!("{}:{}", num, hash::compute_line_hash(num, line));
            }

            for (i, line) in content.split('\n').enumerate() {
                let num = i + 1;
                println!("{}:{}", num, hash::compute_line_hash(num, line));
            }
            record_usage("hash", UsageResult::Success, false, false);
        }
        Commands::JsonRead { file } => {
            use std::path::Path;
            let ast = match json::parse_json_ast(Path::new(&file)) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Error parsing JSON {}: {}", file, e);
                    exit_with(2, "json-read", UsageResult::Error, false, false);
                }
            };
            println!("{}", json::format_json_anchors(&ast));
            record_usage("json-read", UsageResult::Success, false, false);
        }
        Commands::JsonApply {
            input,
            emit_updated,
        } => {
            let used_input_file = input.is_some();

            let input_data = if let Some(ref path) = input {
                match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error reading input file {}: {}", path, e);
                        exit_with(
                            2,
                            "json-apply",
                            UsageResult::Error,
                            emit_updated,
                            used_input_file,
                        );
                    }
                }
            } else {
                let mut buf = String::new();
                if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
                    eprintln!("Error reading stdin: {}", e);
                    exit_with(
                        2,
                        "json-apply",
                        UsageResult::Error,
                        emit_updated,
                        used_input_file,
                    );
                }
                buf
            };

            let params: json::JsonApplyParams = match serde_json::from_str(&input_data) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Invalid JSON input: {}", e);
                    exit_with(
                        2,
                        "json-apply",
                        UsageResult::Error,
                        emit_updated,
                        used_input_file,
                    );
                }
            };

            use std::path::Path;
            let mut ast = match json::parse_json_ast(Path::new(&params.path)) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Error parsing JSON {}: {}", params.path, e);
                    exit_with(
                        2,
                        "json-apply",
                        UsageResult::Error,
                        emit_updated,
                        used_input_file,
                    );
                }
            };

            if let Err(e) = json::apply_json_edits(&mut ast, &params.edits) {
                match e {
                    json::JsonError::HashMismatch {
                        ref path,
                        ref expected,
                        ref actual,
                    } => {
                        eprintln!("Hash mismatch for {}.", path);
                        eprintln!("  expected hash: {}", expected);
                        eprintln!("  current hash:  {}", actual);
                        eprintln!("  updated anchor: {}:{}", path, actual);
                        eprintln!(
                            "Re-run `hashline json-read {}` to refresh anchors.",
                            params.path
                        );
                        exit_with(
                            1,
                            "json-apply",
                            UsageResult::Mismatch,
                            emit_updated,
                            used_input_file,
                        );
                    }
                    json::JsonError::Other(msg) => {
                        eprintln!("Error: {}", msg);
                        exit_with(
                            2,
                            "json-apply",
                            UsageResult::Error,
                            emit_updated,
                            used_input_file,
                        );
                    }
                }
            }

            // Write back the modified JSON
            let output = match serde_json::to_string_pretty(&ast) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error serializing JSON: {}", e);
                    exit_with(
                        2,
                        "json-apply",
                        UsageResult::Error,
                        emit_updated,
                        used_input_file,
                    );
                }
            };
            if let Err(e) = std::fs::write(&params.path, output + "\n") {
                eprintln!("Error writing {}: {}", params.path, e);
                exit_with(
                    2,
                    "json-apply",
                    UsageResult::Error,
                    emit_updated,
                    used_input_file,
                );
            }

            if emit_updated {
                // Re-format with updated anchors
                println!("---");
                println!("{}", json::format_json_anchors(&ast));
            }
            record_usage(
                "json-apply",
                UsageResult::Success,
                emit_updated,
                used_input_file,
            );
        }
    }
}

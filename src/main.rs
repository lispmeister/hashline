use clap::Parser;
use std::io::Read;
use std::process;

mod cli;
mod edit;
mod error;
mod format;
mod hash;
mod heuristics;
mod parse;

use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Read {
            file,
            start_line,
            lines,
        } => {
            let content = match std::fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading {}: {}", file, e);
                    process::exit(2);
                }
            };
            // Normalize line endings
            let content = content.replace("\r\n", "\n");
            // Strip trailing newline for consistent formatting
            let content = if content.ends_with('\n') {
                &content[..content.len() - 1]
            } else {
                &content
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
        }
        Commands::Apply { input, emit_updated } => {
            let input_data = if let Some(ref path) = input {
                match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error reading input file {}: {}", path, e);
                        process::exit(2);
                    }
                }
            } else {
                let mut buf = String::new();
                if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
                    eprintln!("Error reading stdin: {}", e);
                    process::exit(2);
                }
                buf
            };

            let params: edit::HashlineParams = match serde_json::from_str(&input_data) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Invalid JSON input: {}", e);
                    process::exit(2);
                }
            };

            let content = match std::fs::read_to_string(&params.path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading {}: {}", params.path, e);
                    process::exit(2);
                }
            };
            let content = content.replace("\r\n", "\n");
            let content = if content.ends_with('\n') {
                // Preserve trailing newline awareness
                content[..content.len() - 1].to_string()
            } else {
                content
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
                        process::exit(1);
                    } else {
                        eprintln!("Error: {}", e);
                        process::exit(2);
                    }
                }
            };

            let final_content = if replace_edits.is_empty() {
                anchor_result.content
            } else {
                match edit::apply_replace_edits(&anchor_result.content, &replace_edits) {
                    Ok(r) => r.content,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(2);
                    }
                }
            };

            let mut output = final_content;
            output.push('\n');
            if let Err(e) = std::fs::write(&params.path, &output) {
                eprintln!("Error writing {}: {}", params.path, e);
                process::exit(2);
            }
            if !anchor_result.warnings.is_empty() {
                for w in &anchor_result.warnings {
                    eprintln!("Warning: {}", w);
                }
            }
            if let Some(first_line) = anchor_result.first_changed_line {
                println!("Applied successfully. First changed line: {}", first_line);
                if emit_updated {
                    // Re-read the written file and emit hashline-formatted output for the changed region
                    let updated = std::fs::read_to_string(&params.path).unwrap_or_default();
                    let updated = updated.replace("\r\n", "\n");
                    let updated = if updated.ends_with('\n') {
                        &updated[..updated.len() - 1]
                    } else {
                        &updated
                    };
                    let all_lines: Vec<&str> = updated.split('\n').collect();
                    // Emit a window around the changed region
                    let context = 2;
                    let start = first_line.saturating_sub(1 + context);
                    let edits_count = params.edits.len();
                    let end = all_lines.len().min(start + (edits_count * 3).max(10) + context * 2);
                    let slice = &all_lines[start..end];
                    if !slice.is_empty() {
                        let sliced_content = slice.join("\n");
                        println!("---");
                        println!("{}", format::format_hashlines(&sliced_content, start + 1));
                    }
                }
            } else if !replace_edits.is_empty() {
                println!("Applied successfully.");
            } else {
                println!("No changes applied.");
            }
        }
        Commands::Hash { file } => {
            let content = match std::fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading {}: {}", file, e);
                    process::exit(2);
                }
            };
            let content = content.replace("\r\n", "\n");
            let content = if content.ends_with('\n') {
                &content[..content.len() - 1]
            } else {
                &content
            };
            for (i, line) in content.split('\n').enumerate() {
                let num = i + 1;
                println!("{}:{}", num, hash::compute_line_hash(num, line));
            }
        }
    }
}

use clap::{Parser, Subcommand};
use std::io::Read;
use std::process;

mod hash;
mod format;
mod parse;
mod error;
mod heuristics;
mod edit;

#[derive(Parser)]
#[command(name = "hashline", about = "Line-addressable file editing with content hashes")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Read a file and output hashline-formatted content
    Read {
        /// File path to read
        file: String,
        /// Starting line number (1-indexed, default 1)
        #[arg(long, default_value_t = 1)]
        start_line: usize,
    },
    /// Apply hashline edits from stdin JSON to a file
    Apply,
    /// Output line hashes for a file
    Hash {
        /// File path to hash
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Read { file, start_line } => {
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
            println!("{}", format::format_hashlines(content, start_line));
        }
        Commands::Apply => {
            let mut input = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut input) {
                eprintln!("Error reading stdin: {}", e);
                process::exit(2);
            }

            let params: edit::HashlineParams = match serde_json::from_str(&input) {
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

            match edit::apply_hashline_edits(&content, &params.edits) {
                Ok(result) => {
                    // Write back with trailing newline
                    let mut output = result.content;
                    output.push('\n');
                    if let Err(e) = std::fs::write(&params.path, &output) {
                        eprintln!("Error writing {}: {}", params.path, e);
                        process::exit(2);
                    }
                    if !result.warnings.is_empty() {
                        for w in &result.warnings {
                            eprintln!("Warning: {}", w);
                        }
                    }
                    if let Some(line) = result.first_changed_line {
                        println!("Applied successfully. First changed line: {}", line);
                    } else {
                        println!("No changes applied.");
                    }
                }
                Err(e) => {
                    if e.downcast_ref::<error::HashlineMismatchError>().is_some() {
                        eprintln!("{}", e);
                        process::exit(1);
                    } else {
                        eprintln!("Error: {}", e);
                        process::exit(2);
                    }
                }
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

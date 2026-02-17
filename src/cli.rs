use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "hashline",
    version,
    about = "Line-addressable file editing with content hashes"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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

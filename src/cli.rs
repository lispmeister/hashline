use clap::{Parser, Subcommand, builder::RangedU64ValueParser};

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
        #[arg(long, default_value_t = 1, value_parser = RangedU64ValueParser::<usize>::new().range(1..=(u32::MAX as u64)))]
        start_line: usize,
        /// Maximum number of lines to output
        #[arg(long, value_parser = RangedU64ValueParser::<usize>::new().range(1..=(u32::MAX as u64)))]
        lines: Option<usize>,
    },
    /// Apply hashline edits from stdin JSON to a file
    Apply,
    /// Output line hashes for a file
    Hash {
        /// File path to hash
        file: String,
    },
}

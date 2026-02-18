use clap::{builder::RangedU64ValueParser, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "hashline",
    version,
    about = "Line-addressable file editing with content hashes",
    long_about = "Hashline tags each line of a file with a short content hash (LINE:HASH), \
allowing AI coding agents to reference lines by anchor rather than reproducing \
exact text. Hash mismatches after file changes are detected before any edit is \
applied, preventing silent corruption.\n\n\
Hash algorithm: xxHash32(whitespace_stripped_line, seed=0) % 256, formatted as 2 hex chars.\n\n\
Exit codes: 0 = success, 1 = hash mismatch (stderr has updated anchors), 2 = other error.",
    after_long_help = "AGENT WORKFLOW\n\
    Add the contents of HASHLINE_TEMPLATE.md to your project's CLAUDE.md,\n\
    AGENTS.md, or equivalent agent instructions file. The full workflow:\n\n\
    1. hashline read src/foo.rs\n\
       Output: LINE:HASH|content for each line. Collect anchors for lines to change.\n\n\
    2. cat << 'EOF' | hashline apply\n\
       {\"path\":\"src/foo.rs\",\"edits\":[{\"set_line\":{\"anchor\":\"4:01\",\"new_text\":\"...\"}}]}\n\
       EOF\n\
       Edits are atomic — all validate before any mutation. On hash mismatch\n\
       (exit 1), stderr shows updated LINE:HASH refs; retry with those anchors.\n\n\
    3. hashline read --start-line 3 --lines 5 src/foo.rs\n\
       Verify just the changed region without re-reading the whole file.\n\n\
EDIT OPERATIONS\n\
    set_line      Replace one line:    {\"set_line\":{\"anchor\":\"4:01\",\"new_text\":\"...\"}}\n\
    replace_lines Replace a range:     {\"replace_lines\":{\"start_anchor\":\"3:7f\",\"end_anchor\":\"5:0e\",\"new_text\":\"...\"}}\n\
    insert_after  Insert after anchor: {\"insert_after\":{\"anchor\":\"2:b2\",\"text\":\"...\"}}\n\
    replace       Exact substring:     {\"replace\":{\"old_text\":\"...\",\"new_text\":\"...\"}}\n\n\
    Use \"new_text\":\"\" in replace_lines to delete a range.\n\
    Use \\n in strings for multi-line content.\n\
    Batch multiple edits to one file in a single apply call.\n\
    replace edits run after all anchor edits and error on ambiguous matches."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Read a file and output hashline-formatted content
    #[command(
        long_about = "Read a file and output each line prefixed with its LINE:HASH anchor.\n\n\
Each line of output has the form:\n\n\
    LINENUM:HASH|CONTENT\n\n\
where HASH is a 2-char hex string derived from xxHash32 of the whitespace-stripped \
line content. Use --start-line and --lines to read a specific range — useful for \
verifying edits without re-reading an entire large file.",
        after_long_help = "EXAMPLES\n\
    Read the whole file:\n\
        hashline read src/main.rs\n\n\
    Read lines 50-74:\n\
        hashline read --start-line 50 --lines 25 src/main.rs\n\n\
    Read from line 100 to end of file:\n\
        hashline read --start-line 100 src/main.rs"
    )]
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
    #[command(
        long_about = "Read a JSON edit specification from stdin and apply it to the target file.\n\n\
All anchors are validated against the current file state before any changes are made \
(atomic apply). If any anchor hash does not match, no edits are applied and the \
correct updated LINE:HASH refs are printed to stderr.\n\n\
Input format:\n\
    {\"path\": \"<file>\", \"edits\": [<edit>, ...]}\n\n\
Supported edit operations: set_line, replace_lines, insert_after, replace.\n\
See hashline(1) for the full edit operation reference.\n\n\
Exit codes:\n\
    0  All edits applied successfully\n\
    1  Hash mismatch — stderr contains updated LINE:HASH anchors, retry with those\n\
    2  Other error (bad JSON, file not found, ambiguous replace match, etc.)",
        after_long_help = "EXAMPLES\n\
    Replace one line (use heredoc to avoid shell escaping issues):\n\
        cat << 'EOF' | hashline apply\n\
        {\"path\":\"src/main.rs\",\"edits\":[{\"set_line\":{\"anchor\":\"4:01\",\"new_text\":\"    println!(\\\"goodbye\\\");\"}}]}\n\
        EOF\n\n\
    Multiple edits in one call:\n\
        cat << 'EOF' | hashline apply\n\
        {\n\
          \"path\": \"src/main.rs\",\n\
          \"edits\": [\n\
            {\"set_line\":    {\"anchor\": \"4:01\", \"new_text\": \"    println!(\\\"goodbye\\\");\"}},\n\
            {\"insert_after\": {\"anchor\": \"5:0e\", \"text\": \"// end\"}}\n\
          ]\n\
        }\n\
        EOF\n\n\
    Delete a range of lines:\n\
        cat << 'EOF' | hashline apply\n\
        {\"path\":\"src/main.rs\",\"edits\":[{\"replace_lines\":{\"start_anchor\":\"3:7f\",\"end_anchor\":\"5:0e\",\"new_text\":\"\"}}]}\n\
        EOF"
    )]
    Apply,
    /// Output line hashes for a file
    #[command(
        long_about = "Output the LINE:HASH prefix for each line without the content. \
Useful for debugging hash mismatches or inspecting how hashline identifies lines.",
        after_long_help = "EXAMPLES\n\
    hashline hash src/main.rs"
    )]
    Hash {
        /// File path to hash
        file: String,
    },
}

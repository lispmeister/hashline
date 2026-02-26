AGENT WORKFLOW
    Add the contents of HASHLINE_TEMPLATE.md to your project's CLAUDE.md,
    AGENTS.md, or equivalent agent instructions file. The full workflow:

    1. hashline read src/foo.rs
    Output: LINE:HASH|content for each line. Collect anchors for lines to change.

    2. cat << 'EOF' | hashline apply
    {"path":"src/foo.rs","edits":[{"set_line":{"anchor":"4:01","new_text":"..."}}]}
    EOF
    Edits are atomic — all validate before any mutation. On hash mismatch
    (exit 1), stderr shows updated LINE:HASH refs; retry with those anchors.

    3. hashline read --start-line 3 --lines 5 src/foo.rs
    Verify just the changed region without re-reading the whole file.

    For JSON files:
    1. hashline json-read package.json
    Output: JSON with // $.path:hash anchors. Collect anchors for values to change.

    2. cat << 'EOF' | hashline json-apply
    {"path":"package.json","edits":[{"set_path":{"anchor":"$.version:a7","value":"1.2.0"}}]}
    EOF
    JSON edits are atomic and preserve valid JSON structure.

    EDIT OPERATIONS (Text Files)
    set_line      Replace one line:    {"set_line":{"anchor":"4:01","new_text":"..."}}
    replace_lines Replace a range:     {"replace_lines":{"start_anchor":"3:7f","end_anchor":"5:0e","new_text":"..."}}
    insert_after  Insert after anchor: {"insert_after":{"anchor":"2:b2","text":"..."}}
    replace       Exact substring:     {"replace":{"old_text":"...","new_text":"..."}}

    JSON OPERATIONS
    set_path      Set value at path:   {"set_path":{"anchor":"$.version:a7","value":"1.2.0"}}
    insert_at_path Insert at path:     {"insert_at_path":{"anchor":"$.deps:a1","key":"lodash","value":"^4.17.0"}}
    delete_path   Delete value:        {"delete_path":{"anchor":"$.scripts.test:3b"}}

    Use "new_text":"" in replace_lines to delete a range.
    Use \n in strings for multi-line content.
    Batch multiple edits to one file in a single apply call.
    replace edits run after all anchor edits and error on ambiguous matches.
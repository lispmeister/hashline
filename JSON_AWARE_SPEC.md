# JSON-Aware Hashline Feature Specification

> **Status**: Implemented. This document reflects the shipped implementation.
> The original planning tasks (J1–J7) are retained at the bottom for history.

## Overview

This feature extends Hashline to support JSON-specific editing operations, leveraging JSON structure and JSONPath-based anchors instead of line-based anchors. This provides semantic editing capabilities for JSON files while maintaining Hashline's core principles: atomicity, hash-based staleness detection, and correct exit codes.

## Core Concept

Instead of line-based `LINE:HASH` anchors, use `JSONPATH:VALUEHASH` anchors where JSONPath identifies the location and a 2-char hash of the serialized value prevents stale edits. Edits operate on the JSON AST, ensuring valid output and atomic operations.

## Anchor Format

- `$.users[0].name:8f` (JSONPath + 2-char hash of the value at that path)
- Hash computed on canonical JSON serialization (sorted keys, no whitespace) for consistency
- Examples:
  - Root object: `$:a3`
  - Nested key: `$.config.database.host:5b`
  - Array element: `$.users[2]:9f`

## Supported JSONPath Syntax

The implementation supports a subset of JSONPath sufficient for real-world JSON editing:

| Syntax | Example | Meaning |
|--------|---------|---------|
| `$` | `$` | Root value |
| `$.key` | `$.version` | Top-level object key |
| `$.a.b.c` | `$.database.host` | Nested object keys (arbitrary depth) |
| `$.arr[N]` | `$.users[0]` | Array element by index |
| Mixed | `$.users[0].name` | Object and array segments combined |

Keys containing `.` or `[` are not supported (the path would be ambiguous).

## Edit Operations

1. **`set_path`** — Set a value at a JSONPath
   ```json
   {"set_path": {"anchor": "$.version:5a", "value": "1.2.3"}}
   ```

2. **`insert_at_path`** — Insert into array or object
   ```json
   {"insert_at_path": {"anchor": "$.dependencies:a1", "key": "lodash", "value": "^4.17.0"}}
   ```
   - For objects: provide `"key"`. The anchor points to the object being modified.
   - For arrays, omit `"key"`:
     - Omit `"index"` to append: `{"anchor": "$.tags:3b", "value": "new-tag"}`
     - Provide `"index"` to insert before that position: `{"anchor": "$.tags:3b", "index": 0, "value": "first-tag"}`

3. **`delete_path`** — Remove value at path
   ```json
   {"delete_path": {"anchor": "$.scripts.test:3b"}}
   ```

## Atomicity and Safety

- Parse JSON once into AST at start
- Validate all anchors against current values (fail fast on first mismatch)
- Apply all edits in order only if all anchors validated
- Serialize back to JSON once at end
- Hash mismatches prevent stale edits

## CLI Interface

- `hashline json-read <file>` — Output JSON with path-based anchors
  - Format: JSONC (JSON with `// comment` anchors) for human and model readability
  - Example output:
    ```
    {
      // $.name:8f
      "name": "my-project",
      // $.version:5a
      "version": "1.0.0"
    }
    ```
  - Note: output is not valid strict JSON (uses `//` comments). Parse with a JSONC-aware tool if needed.

- `hashline json-apply [--input file] [--emit-updated]` — Apply edits
  - Reads JSON payload from stdin or `--input` file
  - `--emit-updated`: output fresh anchors after successful apply (avoids a separate re-read)
  - Exit codes: 0 success, 1 hash mismatch, 2 other error

## Error Handling

On hash mismatch (exit code 1), stderr shows the changed path with `>>>` and then the full re-anchored file so the agent can get fresh anchors:

```
1 anchor has changed since last read. Updated references (>>> marks changed values):

>>> $.version:c9
{
  // $.name:8f
  "name": "my-project",
  // $.version:c9
  "version": "1.1.0"
}
```

Copy the updated anchor (`$.version:c9`) into your edit and retry.

## Known Limitations

- **No heuristic recovery**: The JSON engine does not implement the heuristic recovery layer (merge detection, Unicode normalization, etc.) present in the text-file engine. If an anchor is stale the agent must re-read and retry.
- **Keys with `.` or `[`**: JSON keys containing these characters cannot be addressed — the path encoding is ambiguous.
- **Fail-fast validation**: Only the first mismatched anchor is reported per apply call.

## Integration with Existing Hashline

- JSON-aware commands coexist with existing `read`/`apply`
- Models choose based on file type; line-based editing still works on JSON files as a fallback
- Shared infrastructure: hashing algorithm (`xxHash32 % 256`), atomicity model, exit code convention

---

## Original Planning Tasks (history)

1. **Task J1: JSON Parsing & AST Setup** — ✅ Done (`parse_json_ast`, `serde_json` dep)
2. **Task J2: JSON Anchor Computation** — ✅ Done (`compute_json_anchor` with sorted-key canonical JSON)
3. **Task J3: JSON Formatting for Read** — ✅ Done (`format_json_anchors` with JSONC comments, proper indentation)
4. **Task J4: JSON Edit Operations Engine** — ✅ Done (`apply_json_edits`, real JSONPath traversal, `set_path`/`insert_at_path`/`delete_path`)
5. **Task J5: Hash Mismatch Detection** — ✅ Done (`JsonError::HashMismatch`, exit 1, updated anchor output)
6. **Task J6: CLI Subcommands** — ✅ Done (`json-read`, `json-apply`, `--input`, `--emit-updated`)
7. **Task J7: Documentation** — ✅ Done (`HASHLINE_TEMPLATE.md`, `HASHLINE_HOOKS.md`, this spec)

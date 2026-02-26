# JSON-Aware Hashline Feature Specification

## Overview

This feature extends Hashline to support JSON-specific editing operations, leveraging JSON structure and JSONPath-based anchors instead of line-based anchors. This provides semantic editing capabilities for JSON files while maintaining Hashline's core principles: atomicity, hash-based staleness detection, and heuristic recovery.

## Core Concept

Instead of line-based `LINE:HASH` anchors, use `JSONPATH:VALUEHASH` anchors where JSONPath identifies the location and a 2-char hash of the serialized value prevents stale edits. Edits operate on the JSON AST, ensuring valid output and atomic operations.

## Anchor Format

- `$.users[0].name:8f` (JSONPath + 2-char hash of the value at that path)
- Hash computed on canonical JSON serialization (sorted keys, no whitespace) for consistency
- Examples:
  - Root object: `$:a3`
  - Nested key: `$.config.database.host:5b`
  - Array element: `$.users[2]:9f`
  - Array append: `$.users[]:new` (for insertions)

## Edit Operations

1. `set_path` - Set a value at a JSONPath
   ```json
   {"set_path": {"anchor": "$.version:5a", "value": "1.2.3"}}
   ```

2. `insert_at_path` - Insert into array/object
   ```json
   {"insert_at_path": {"anchor": "$.dependencies:a1", "key": "lodash", "value": "^4.17.0"}}
   ```
   - For arrays: omit "key", insert at index or append
   - For objects: provide "key"

3. `delete_path` - Remove from path
   ```json
   {"delete_path": {"anchor": "$.scripts.test:3b"}}
   ```

4. `replace` - Fallback exact string replacement (same as current Hashline)
   ```json
   {"replace": {"old_string": "\"version\": \"1.0.0\"", "new_string": "\"version\": \"1.2.3\""}}
   ```

## Atomicity and Safety

- Parse JSON once into AST at start
- Validate all anchors against current values
- Apply all edits atomically to AST (all succeed or none apply)
- Serialize back to JSON once at end
- Hash mismatches prevent stale edits, showing updated `JSONPATH:NEW_HASH` anchors

## Heuristic Recovery (JSON-Adapted)

- Strip accidental JSONPath prefixes from model output
- Normalize Unicode hyphens/em-dashes in string values
- Detect value merges (model combines adjacent values)
- Undo pure formatting rewraps in JSON strings
- Normalize confusable Unicode in keys/values
- Strip boundary echo (model echoes surrounding structure)

## CLI Interface

- `hashline json-read <file>` - Output JSON with path-based anchors
  - Format: JSON with inline comments showing anchors
  - Example output:
    ```json
    {
      // $.name:8f
      "name": "my-project",
      // $.version:5a
      "version": "1.0.0"
    }
    ```

- `hashline json-apply [--input file.json] [--emit-updated]` - Apply edits
  - Accepts JSON payload with array of edits
  - `--input`: Read edits from file (avoids shell guard issues)
  - `--emit-updated`: Output fresh anchors after successful apply
  - Exit codes: 0 success, 1 hash mismatch, 2 error

## Error Handling

On hash mismatch (exit code 1), stderr shows current JSON state with `>>>` marking changed values:

```
1 value has changed since last read. Use the updated JSONPATH:HASH references shown below (>>> marks changed values).

    $.version:5a| "1.0.0"
>>> $.version:c9| "1.1.0"
```

## Independent Implementation Tasks

1. **Task J1: JSON Parsing & AST Setup**
   - Add `serde_json` dependency
   - Implement `parse_json_ast(file_path) -> Result<Value>`
   - Add JSONPath library (`jsonpath-rust`) for path resolution
   - Unit tests: Parse valid/invalid JSON files

2. **Task J2: JSON Anchor Computation**
   - Implement `compute_json_anchor(path: &str, value: &Value) -> String`
   - Hash on canonical JSON (sorted keys, compact)
   - Unit tests: Hash consistency, uniqueness

3. **Task J3: JSON Formatting for Read**
   - `format_json_anchors(ast: &Value) -> String`
   - Output JSON with inline `// $.path:hash` comments
   - Preserve readability
   - Integration tests: Round-trip preservation

4. **Task J4: JSON Edit Operations Engine**
   - Implement `apply_json_edits(ast: &mut Value, edits: &[JsonEdit]) -> Result<()>`
   - Validate anchors, apply operations atomically
   - Unit tests: Each operation on sample ASTs

5. **Task J5: Hash Mismatch Detection & Recovery**
   - Check value hashes before edits
   - Error reporting with updated anchors
   - Implement recovery heuristics
   - Integration tests: Stale edit scenarios

6. **Task J6: CLI Subcommands Implementation**
   - Add `json-read` and `json-apply` subcommands
   - Support flags and error handling
   - Manual tests: CLI integration

7. **Task J7: Documentation & Examples**
   - Update README and CLAUDE.md
   - Create example payloads

## Test Case Specifications

Test fixtures in `tests/fixtures/json/`:

### Small JSON (~10 lines, package.json-like)
- Structure: Simple object with strings, arrays, nested objects
- Test operations: set_path (version), insert_at_path (add dependency), delete_path (remove script)
- Edge cases: Empty objects, null values, array operations
- File: `small.json`

### Medium JSON (~100 lines, config file)
- Structure: Deeper nesting, arrays of objects, mixed types
- Test operations: Insert into array, update nested value, delete key
- Edge cases: Unicode strings, large numbers, boolean toggles
- File: `medium.json`

### Large JSON (~1000+ lines, data export)
- Structure: Complex API response (users, posts, comments)
- Test operations: Batch multiple edits, deep path updates
- Edge cases: Very deep nesting, large arrays
- Performance requirement: Parse/serialize < 100ms
- File: `large.json`

For each fixture:
- Original file
- Expected outputs after specific edits
- Invalid JSON for error testing
- Stale edit scenarios

## Integration with Existing Hashline

- JSON-aware commands coexist with existing `read`/`apply`
- Models can choose based on file type
- Fallback to line-based editing for non-JSON files
- Shared infrastructure: hashing algorithm, atomicity, error reporting
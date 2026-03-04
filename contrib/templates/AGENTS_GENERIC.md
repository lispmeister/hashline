# Hashline Rules (Generic Agent)

Paste this at the top of your agent instruction file.

- Never edit a file before running `hashline read` (or `hashline json-read` for JSON).
- For text files, apply edits with `hashline apply --emit-updated --input edits.json`.
- For JSON files, apply edits with `hashline json-apply --emit-updated --input json-edits.json`.
- Prefer anchor-based operations over raw substring replace.
- If a hash mismatch occurs, use updated anchors and retry.

Note: Generic integration is advisory-only unless your agent supports hook-style command gating.

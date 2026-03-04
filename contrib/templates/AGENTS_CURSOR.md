# Hashline Rules (Cursor)

Paste this at the top of your Cursor rules file.

- Read files with `hashline read` (or `hashline json-read` for JSON) before edits.
- Apply text edits with `hashline apply --emit-updated --input edits.json`.
- Apply JSON edits with `hashline json-apply --emit-updated --input json-edits.json`.
- Batch edits per target file in one apply call.
- If apply fails with mismatch, use updated anchors and retry.

Note: Cursor integration is currently advisory (no native hook enforcement yet).

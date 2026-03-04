# Hashline Rules (Windsurf)

Paste this at the top of your Windsurf rules file.

- Read files with `hashline read` (or `hashline json-read` for JSON) before edits.
- Use `hashline apply --emit-updated --input edits.json` for text changes.
- Use `hashline json-apply --emit-updated --input json-edits.json` for JSON changes.
- Keep one file per apply payload where possible.
- On mismatch, refresh anchors from stderr output and retry.

Note: Windsurf integration is currently advisory (no native hook enforcement yet).

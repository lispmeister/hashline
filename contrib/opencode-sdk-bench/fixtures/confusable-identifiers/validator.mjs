import fs from "node:fs"

const [targetPath] = process.argv.slice(2)
const text = fs.readFileSync(targetPath, "utf8")

const canonical = /function\s+normalizeUser\(value: string\): string\s*\{\s*return value\.trim\(\)\.toLowerCase\(\);\s*\}/m.test(text)
const confusableUntouched = /function\s+normalizeUѕer\(value: string\): string\s*\{\s*return value\.trim\(\);\s*\}/m.test(text)

const pass = canonical && confusableUntouched
const reason = pass ? "validator_scope_and_confusable_pass" : "validator_scope_or_confusable_fail"

process.stdout.write(JSON.stringify({ pass, reason }))

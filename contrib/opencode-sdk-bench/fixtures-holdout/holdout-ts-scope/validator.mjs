import fs from "node:fs"

const [targetPath] = process.argv.slice(2)
const text = fs.readFileSync(targetPath, "utf8")

const classMethodLower = /class\s+AuthParser\s*\{[\s\S]*?parseToken\(input: string\): string\s*\{\s*return input\.trim\(\)\.toLowerCase\(\);\s*\}/m.test(text)
const topLevelUntouched = /function\s+parseToken\(input: string\): string\s*\{\s*return input\.trim\(\);\s*\}/m.test(text)

const pass = classMethodLower && topLevelUntouched
const reason = pass ? "validator_class_scope_pass" : "validator_class_scope_fail"

process.stdout.write(JSON.stringify({ pass, reason }))

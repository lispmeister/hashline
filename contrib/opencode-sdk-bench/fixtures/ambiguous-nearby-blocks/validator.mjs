import fs from "node:fs"

const [targetPath] = process.argv.slice(2)
const text = fs.readFileSync(targetPath, "utf8")

const deployOk = /### Deploy Stage\nMode: production\nOwner: sre-team/m.test(text)
const buildUnchanged = /### Build Stage\nMode: draft\nOwner: dev-team/m.test(text)
const rollbackUnchanged = /### Rollback Stage\nMode: standby\nOwner: sre-team/m.test(text)

const pass = deployOk && buildUnchanged && rollbackUnchanged
const reason = pass ? "validator_targeted_block_pass" : "validator_targeted_block_fail"

process.stdout.write(JSON.stringify({ pass, reason }))

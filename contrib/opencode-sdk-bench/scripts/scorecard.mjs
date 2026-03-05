#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"

const runsDir = path.resolve("runs")
const files = fs.readdirSync(runsDir).filter((f) => f.endsWith(".json") && f !== "progress.json")

const byKey = new Map()

for (const file of files) {
  const report = JSON.parse(fs.readFileSync(path.join(runsDir, file), "utf8"))
  const model = report.model?.label || report.config?.model || "unknown"
  const mode = report.config?.mode || "unknown"
  const key = `${model}::${mode}`
  if (!byKey.has(key)) byKey.set(key, { model, mode, attempts: 0, task: 0, protocol: 0, overall: 0, corrupt: 0, cost: 0, ms: 0 })
  const row = byKey.get(key)
  for (const c of report.results || []) {
    for (const a of c.attempts || []) {
      row.attempts += 1
      row.task += a.taskPassed ? 1 : 0
      row.protocol += a.protocolPassed ? 1 : 0
      row.overall += a.overallPassed ? 1 : 0
      row.corrupt += a.corruptionDetected ? 1 : 0
      row.cost += a.metrics?.cost || 0
      row.ms += a.metrics?.durationMs || 0
    }
  }
}

const rows = [...byKey.values()].map((r) => {
  const n = Math.max(1, r.attempts)
  const correctness = r.task / n
  const safety = 1 - r.corrupt / n
  const efficiency = r.overall / n
  const reviewBurden = 1 - r.corrupt / n
  const utility = 0.35 * correctness + 0.35 * safety + 0.2 * efficiency + 0.1 * reviewBurden
  return {
    model: r.model,
    mode: r.mode,
    attempts: r.attempts,
    taskPassRate: +(100 * correctness).toFixed(1),
    protocolPassRate: +(100 * (r.protocol / n)).toFixed(1),
    overallPassRate: +(100 * (r.overall / n)).toFixed(1),
    corruptionRate: +(100 * (r.corrupt / n)).toFixed(1),
    avgMs: +(r.ms / n).toFixed(1),
    costPerSuccess: +(r.cost / Math.max(1, r.overall)).toFixed(6),
    utility: +(100 * utility).toFixed(1),
  }
})

rows.sort((a, b) => b.utility - a.utility)
console.log(JSON.stringify(rows, null, 2))

import fs from "node:fs"
import path from "node:path"
import { spawnSync } from "node:child_process"
import { createOpencode } from "@opencode-ai/sdk"
import { resolveModel } from "./model"
import { startEventCapture } from "./event-stream"
import { readHashlineLogTail } from "./hashline-log"
import type {
  AttemptResult,
  BenchmarkConfig,
  CaseResult,
  EventRecord,
  FixtureSet,
  FixtureSpec,
  RunReport,
  ScenarioFamily,
} from "./types"

export type AttemptProgress = {
  fixture: FixtureSpec
  caseIndex: number
  caseTotal: number
  attempt: number
  attemptTotal: number
  result: AttemptResult
}
export interface RunBenchmarkHooks {
  onAttemptComplete?: (progress: AttemptProgress) => void | Promise<void>
}
interface WorkTarget {
  workPath: string
}
interface EvalResult {
  taskPassed: boolean
  reason: string
  corruptionDetected: boolean
  corruptionReason?: string
  changedLines: number
  expectedChangedLines: number
  unexpectedChangedLines: number
}
interface ProtocolResult {
  protocolPassed: boolean
  reasons: string[]
}
function inferFamily(caseId: string): ScenarioFamily {
  if (caseId.includes("ambiguous") || caseId.includes("similar")) return "ambiguity"
  if (caseId.includes("deep") || caseId.includes("json-migration")) return "json_migration"
  if (caseId.includes("whitespace")) return "whitespace"
  if (caseId.includes("scope") || caseId.includes("class")) return "scope"
  if (caseId.includes("array")) return "array_targeting"
  if (caseId.includes("embedded") || caseId.includes("mixed")) return "multiformat"
  if (caseId.includes("stale")) return "stale_context"
  if (caseId.includes("large")) return "large_file"
  if (caseId.includes("confusable")) return "confusable_chars"
  if (caseId.includes("refactor") || caseId.includes("rename")) return "refactor"
  return "other"
}
function fixtureSetFromConfig(config: BenchmarkConfig): FixtureSet {
  return config.fixtureSet ?? "default"
}
function xorshift(seed: number): () => number {
  let x = seed || 2463534242
  return () => {
    x ^= x << 13
    x ^= x >>> 17
    x ^= x << 5
    return (x >>> 0) / 0x100000000
  }
}
function seededShuffle<T>(input: T[], seed: number): T[] {
  const arr = [...input]
  const rand = xorshift(seed)
  for (let i = arr.length - 1; i > 0; i -= 1) {
    const j = Math.floor(rand() * (i + 1))
    const tmp = arr[i]
    arr[i] = arr[j]
    arr[j] = tmp
  }
  return arr
}
function fixtureRoots(fixturesDir: string, set: FixtureSet): string[] {
  const holdout = path.join(path.dirname(fixturesDir), "fixtures-holdout")
  if (set === "holdout") return [holdout]
  if (set === "all") return [fixturesDir, holdout]
  return [fixturesDir]
}
function listFixtures(config: BenchmarkConfig): FixtureSpec[] {
  const items: FixtureSpec[] = []
  const roots = fixtureRoots(config.fixturesDir, fixtureSetFromConfig(config))

  for (const root of roots) {
    if (!fs.existsSync(root)) continue
    for (const caseId of fs.readdirSync(root).sort()) {
      const base = path.join(root, caseId)
    if (!fs.statSync(base).isDirectory()) continue
      const ext = fs
        .readdirSync(base)
        .find((f) => f.startsWith("original."))
        ?.split(".")
        .slice(1)
      .join(".")
      if (!ext) continue

      const family = inferFamily(caseId)
      items.push({
        size: "small",
        caseId,
        family,
        extension: ext,
        basePath: base,
        originalPath: path.join(base, `original.${ext}`),
        mutatedPath: path.join(base, `mutated.${ext}`),
        taskPath: path.join(base, "task.md"),
        tags: [family],
      })
    }
  }
  const sized = items.filter((f) => config.sizes.includes(f.size))
  if (!config.randomize) return sized
  return seededShuffle(sized, config.seed ?? 42)
}
function buildInstruction(mode: BenchmarkConfig["mode"]): string {
  if (mode === "hashline") {
    return [
      "Use hashline for all edits.",
      "Read with hashline read/json-read before edits.",
      "Apply edits with hashline apply/json-apply.",
      "On mismatch, retry using updated anchors.",
      "Do not modify files outside the requested target file.",
    ].join(" ")
  }
  if (mode === "patch") {
    return "Use patch-style editing only. Do not use hashline. Only change the requested target file."
  }
  return "Use direct string replacement style editing only. Do not use hashline. Only change the requested target file."
}
function changedLineSet(before: string, after: string): Set<number> {
  const a = before.split("\n")
  const b = after.split("\n")
  const max = Math.max(a.length, b.length)
  const set = new Set<number>()
  for (let i = 0; i < max; i += 1) {
    if ((a[i] ?? "") !== (b[i] ?? "")) set.add(i)
  }
  return set
}
function runValidatorIfPresent(fixture: FixtureSpec, targetPath: string): { ok: boolean; reason: string } | null {
  const validatorPath = path.join(fixture.basePath, "validator.mjs")
  if (!fs.existsSync(validatorPath)) return null
  const out = spawnSync("node", [validatorPath, targetPath, fixture.originalPath, fixture.mutatedPath], {
    encoding: "utf8",
  })
  if (out.status !== 0) {
    return { ok: false, reason: `validator_error:${(out.stderr || "").trim() || out.status}` }
  }

  try {
    const parsed = JSON.parse(out.stdout.trim())
    return { ok: Boolean(parsed.pass), reason: String(parsed.reason || "validator") }
  } catch {
    return { ok: false, reason: "validator_invalid_output" }
  }
}
function evaluate(fixture: FixtureSpec, targetPath: string): EvalResult {
  const actual = fs.readFileSync(targetPath, "utf8").trimEnd()
  const expected = fs.readFileSync(fixture.originalPath, "utf8").trimEnd()
  const mutated = fs.readFileSync(fixture.mutatedPath, "utf8").trimEnd()

  const validator = runValidatorIfPresent(fixture, targetPath)
  const taskPassed = validator ? validator.ok : actual === expected
  const reason = validator ? validator.reason : taskPassed ? "exact_match" : "content_differs"
  const expectedChanged = changedLineSet(mutated, expected)
  let unexpected = 0
  const actualChanged = changedLineSet(mutated, actual)
  for (const line of actualChanged) {
    if (!expectedChanged.has(line)) unexpected += 1
  }
  const corruptionDetected = !taskPassed && unexpected > 0
  const corruptionReason = corruptionDetected ? "changed_unexpected_lines" : undefined
  return {
    taskPassed,
    reason,
    corruptionDetected,
    corruptionReason,
    changedLines: actualChanged.size,
    expectedChangedLines: expectedChanged.size,
    unexpectedChangedLines: unexpected,
  }
}
function eventCommand(event: EventRecord): string | null {
  if (event.command) return event.command
  if (!event.message) return null

  const msg = String(event.message)
  const parts = msg.split(",")
  if (parts.length >= 2) return parts[1].trim()
  return null
}
function evaluateProtocol(mode: BenchmarkConfig["mode"], events: EventRecord[]): ProtocolResult {
  if (mode !== "hashline") {
    return { protocolPassed: true, reasons: [] }
  }
  const reasons: string[] = []
  const commands = events.map(eventCommand).filter((x): x is string => Boolean(x))
  let seenApply = false
  let seenRead = false

  for (const cmd of commands) {
    if (cmd === "read" || cmd === "json-read") {
      seenRead = true
      continue
    }
    if (cmd === "apply" || cmd === "json-apply") {
      seenApply = true
      if (!seenRead) reasons.push("apply_before_read")
      continue
    }
  }
  if (!seenApply) reasons.push("no_hashline_apply_observed")
  if (!seenRead) reasons.push("no_hashline_read_observed")

  return { protocolPassed: reasons.length === 0, reasons }
}
function createAttemptTarget(config: BenchmarkConfig, fixture: FixtureSpec, attempt: number): WorkTarget {
  const attemptDir = path.join(config.runsDir, "work", fixture.size, fixture.caseId, `attempt-${attempt}`)
  fs.mkdirSync(attemptDir, { recursive: true })
  const fileName = `target.${fixture.extension}`
  const workPath = path.join(attemptDir, fileName)
  fs.copyFileSync(fixture.mutatedPath, workPath)
  return { workPath }
}
function maybeDisturb(config: BenchmarkConfig, targetPath: string): void {
  if (!config.disturbance) return

  const chance = config.disturbanceProbability ?? 0.5
  if (Math.random() > chance) return

  setTimeout(() => {
    try {
      const content = fs.readFileSync(targetPath, "utf8")
      fs.writeFileSync(targetPath, `${content}\n`)
    } catch {
      // ignore disturbance errors
    }
  }, 900)
}
async function runAttempt(
  client: any,
  config: BenchmarkConfig,
  fixture: FixtureSpec,
  attempt: number,
  modeInstruction: string,
  model: { providerID: string; modelID: string },
  capture: ReturnType<typeof startEventCapture> extends Promise<infer T> ? T : never,
): Promise<AttemptResult> {
  const taskText = fs.readFileSync(fixture.taskPath, "utf8").trim()
  const target = createAttemptTarget(config, fixture, attempt)
  const prompt = [
    modeInstruction,
    "",
    `Target file: ${target.workPath}`,
    `Scenario family: ${fixture.family}`,
    "Apply the requested fix to the target file only.",
    "",
    taskText,
  ].join("\n")

  const started = Date.now()
  const session = await client.session.create({
    body: { title: `bench:${fixture.caseId}:${fixture.size}:a${attempt}` },
  })

  maybeDisturb(config, target.workPath)
  const response = await client.session.prompt({
    path: { id: session.data.id },
    body: {
      model,
      parts: [{ type: "text", text: prompt }],
    },
  })
  const ended = Date.now()
  const evalResult = evaluate(fixture, target.workPath)

  let events = capture.eventsForSession(session.data.id)
  if (events.length === 0) {
    events = readHashlineLogTail(300)
  }
  const protocol = evaluateProtocol(config.mode, events)
  const enforce = config.enforceProtocol ?? true
  const overallPassed = evalResult.taskPassed && (enforce ? protocol.protocolPassed : true)
  const retries = events.filter((e) => e.type.includes("retry")).length
  const errors = events.filter((e) => e.type.includes("error")).length
  const tokenBlock = response.data.info?.tokens ?? {}
  return {
    attempt,
    passed: overallPassed,
    reason: evalResult.reason,
    taskPassed: evalResult.taskPassed,
    protocolPassed: protocol.protocolPassed,
    overallPassed,
    protocolFailureReasons: protocol.reasons,
    corruptionDetected: evalResult.corruptionDetected,
    corruptionReason: evalResult.corruptionReason,
    sessionID: session.data.id,
    events,
    metrics: {
      durationMs: ended - started,
      retries,
      errorCount: errors,
      tokenInput: tokenBlock.input,
      tokenOutput: tokenBlock.output,
      tokenReasoning: tokenBlock.reasoning,
      tokenTotal: tokenBlock.total,
      cost: response.data.info?.cost,
      changedLines: evalResult.changedLines,
      expectedChangedLines: evalResult.expectedChangedLines,
      unexpectedChangedLines: evalResult.unexpectedChangedLines,
    },
  }
}
export async function runBenchmark(config: BenchmarkConfig, hooks: RunBenchmarkHooks = {}): Promise<RunReport> {
  const startedAt = new Date().toISOString()
  fs.mkdirSync(config.runsDir, { recursive: true })
  const opencode = await createOpencode({ config: {} })

  try {
    const model = await resolveModel(opencode.client as any, config.model)
    const capture = await startEventCapture(opencode.client as any)
    const fixtures = listFixtures(config)
    const modeInstruction = buildInstruction(config.mode)
    const onAttemptComplete = hooks.onAttemptComplete
    const results: CaseResult[] = []
    for (let caseIndex = 0; caseIndex < fixtures.length; caseIndex += 1) {
      const fixture = fixtures[caseIndex]
      const attempts: AttemptResult[] = []
      for (let i = 1; i <= config.repeats; i += 1) {
        const attemptResult = await runAttempt(
          opencode.client,
          config,
          fixture,
          i,
          modeInstruction,
          { providerID: model.providerID, modelID: model.modelID },
          capture,
        )

        attempts.push(attemptResult)
        if (onAttemptComplete) {
          await onAttemptComplete({
            fixture,
            caseIndex: caseIndex + 1,
            caseTotal: fixtures.length,
            attempt: i,
            attemptTotal: config.repeats,
            result: attemptResult,
          })
        }
      }
      results.push({
        size: fixture.size,
        caseId: fixture.caseId,
        family: fixture.family,
        mode: config.mode,
        model,
        attempts,
      })
    }

    capture.stop()
    const finishedAt = new Date().toISOString()
    return {
      startedAt,
      finishedAt,
      config,
      model,
      results,
    }
  } finally {
    opencode.server.close()
  }
}


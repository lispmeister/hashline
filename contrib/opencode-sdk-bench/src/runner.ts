import fs from "node:fs"
import path from "node:path"
import { createOpencode } from "@opencode-ai/sdk"
import { resolveModel } from "./model"
import { startEventCapture } from "./event-stream"
import { readHashlineLogTail } from "./hashline-log"
import type { AttemptResult, BenchmarkConfig, CaseResult, FixtureSpec, RunReport } from "./types"

export interface AttemptProgress {
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
  evalOriginalPath: string
}

function listFixtures(fixturesDir: string): FixtureSpec[] {
  const items: FixtureSpec[] = []

  if (!fs.existsSync(fixturesDir)) return items

  for (const caseId of fs.readdirSync(fixturesDir).sort()) {
    const base = path.join(fixturesDir, caseId)
    if (!fs.statSync(base).isDirectory()) continue

    const ext = fs
      .readdirSync(base)
      .find((f) => f.startsWith("original."))
      ?.split(".")
      .slice(1)
      .join(".")

    if (!ext) continue

    items.push({
      size: "small" as FixtureSpec["size"], // All fixtures are now "small" for cost savings
      caseId: caseId as FixtureSpec["caseId"],
      extension: ext,
      basePath: base,
      originalPath: path.join(base, `original.${ext}`),
      mutatedPath: path.join(base, `mutated.${ext}`),
      taskPath: path.join(base, "task.md"),
    })
  }

  return items
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

function evaluate(actualPath: string, expectedPath: string): { passed: boolean; reason: string } {
  const actual = fs.readFileSync(actualPath, "utf8").trimEnd()
  const expected = fs.readFileSync(expectedPath, "utf8").trimEnd()
  if (actual === expected) return { passed: true, reason: "exact_match" }
  return { passed: false, reason: "content_differs" }
}

function createAttemptTarget(config: BenchmarkConfig, fixture: FixtureSpec, attempt: number): WorkTarget {
  const attemptDir = path.join(config.runsDir, "work", fixture.size, fixture.caseId, `attempt-${attempt}`)
  fs.mkdirSync(attemptDir, { recursive: true })

  const fileName = `target.${fixture.extension}`
  const workPath = path.join(attemptDir, fileName)
  fs.copyFileSync(fixture.mutatedPath, workPath)

  return {
    workPath,
    evalOriginalPath: fixture.originalPath,
  }
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
    "Apply the requested fix to the target file only.",
    "",
    taskText,
  ].join("\n")

  const started = Date.now()

  const session = await client.session.create({
    body: { title: `bench:${fixture.caseId}:${fixture.size}:a${attempt}` },
  })

  const response = await client.session.prompt({
    path: { id: session.data.id },
    body: {
      model,
      parts: [{ type: "text", text: prompt }],
    },
  })

  const ended = Date.now()
  const evalResult = evaluate(target.workPath, target.evalOriginalPath)

  let events = capture.eventsForSession(session.data.id)
  if (events.length === 0) {
    events = readHashlineLogTail(300)
  }

  const retries = events.filter((e) => e.type.includes("retry")).length
  const errors = events.filter((e) => e.type.includes("error")).length

  const tokenBlock = response.data.info?.tokens ?? {}

  return {
    attempt,
    passed: evalResult.passed,
    reason: evalResult.reason,
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
    const fixtures = listFixtures(config.fixturesDir).filter((f) => config.sizes.includes(f.size))
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

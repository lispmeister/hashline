import http from "node:http"
import fs from "node:fs/promises"
import path from "node:path"
import { execSync, spawn } from "node:child_process"
import { fileURLToPath } from "node:url"
import Database from "better-sqlite3"
const ROOT_DIR = path.dirname(fileURLToPath(import.meta.url))
const WEB_DIR = path.join(ROOT_DIR, "web")
const RUNS_DIR = path.join(ROOT_DIR, "runs")
const PROGRESS_FILE = path.join(RUNS_DIR, "progress.json")
const LOG_FILE = path.join(RUNS_DIR, "active.log")
const DB_FILE = path.join(RUNS_DIR, "bench.sqlite")
const PORT = Number(process.env.PORT || 4177)
const RUN_LIMIT = 200
const LOG_TAIL_LIMIT = 400
const MIME_TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
}

let db = null
let activeRun = {
  running: false,
  startedAt: null,
  finishedAt: null,
  config: null,
  completedJobs: 0,
  totalJobs: 0,
  lastError: null,
  logs: [],
}

let activeChild = null
let progressPoller = null

async function pollProgress() {
  try {
    const progress = await loadProgress()
    if (progress && progress.caseId) {
      const pct = progress.totalAttempts > 0
        ? Math.round((progress.completedAttempts / progress.totalAttempts) * 100)
        : 0
      const costStr = progress.totalCost > 0 ? ` cost=$${progress.totalCost.toFixed(4)}` : ''
      appendLog(
        `[PROGRESS] case=${progress.caseId} size=${progress.size} ` +
        `attempt=${progress.currentAttempt}/${progress.attemptsPerCase} ` +
        `case=${progress.currentCase}/${progress.totalCases} ` +
        `passed=${progress.passCount} errors=${progress.errorCount} retries=${progress.retryCount}` +
        costStr +
        ` ${pct}% complete`
      )
    }
  } catch {
  }
}

function ensureRunsDir() {
  return fs.mkdir(RUNS_DIR, { recursive: true })
}

function nowIso() {
  return new Date().toISOString()
}
function sendJson(res, status, payload) {
  const body = JSON.stringify(payload)
  res.writeHead(status, {
    "Content-Type": "application/json; charset=utf-8",
    "Cache-Control": "no-store",
  })
  res.end(body)
}
function numberOrZero(value) {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : 0
}
function modelLabel(model) {
  if (!model) return "unknown"
  if (typeof model === "string") return model
  if (typeof model.label === "string" && model.label.length > 0) return model.label
  const providerID = model.providerID || model.provider || ""
  const modelID = model.modelID || model.id || ""
  if (providerID && modelID) return `${providerID}/${modelID}`
  return String(modelID || providerID || "unknown")
}
function elapsedFromDates(startedAt, finishedAt) {
  if (!startedAt || !finishedAt) return 0
  const start = Date.parse(startedAt)
  const end = Date.parse(finishedAt)
  if (!Number.isFinite(start) || !Number.isFinite(end)) return 0
  return Math.max(0, end - start)
}
function summarizeReport(report, fileName) {
  const results = Array.isArray(report.results) ? report.results : []
  const attempts = []
  for (const item of results) {
    const list = Array.isArray(item.attempts) ? item.attempts : []
    attempts.push(...list)
  }
  const totalAttempts = attempts.length
  const completedAttempts = totalAttempts
  const passCount = attempts.filter((attempt) => Boolean(attempt.passed)).length
  let errorCount = 0
  let retryCount = 0
  let totalCost = 0
  let elapsedTimeMs = elapsedFromDates(report.startedAt, report.finishedAt)
  for (const attempt of attempts) {
    const metrics = attempt.metrics || {}
    errorCount += numberOrZero(metrics.errorCount)
    retryCount += numberOrZero(metrics.retries)
    totalCost += numberOrZero(metrics.cost)
    if (elapsedTimeMs === 0) {
      elapsedTimeMs += numberOrZero(metrics.durationMs)
    }
  }

  return {
    file: fileName,
    mode: report.config?.mode || report.mode || "unknown",
    model: modelLabel(report.model || report.config?.model),
    totalAttempts,
    completedAttempts,
    passCount,
    errorCount,
    retryCount,
    totalCost,
    elapsedTimeMs,
    startedAt: report.startedAt || null,
    finishedAt: report.finishedAt || null,
  }
}
function normalizeProgress(value) {
  if (!value || typeof value !== "object") return null
  if (Array.isArray(value.results)) {
    return summarizeReport(value, "progress.json")
  }

  return {
    file: "progress.json",
    mode: value.mode || value.config?.mode || "unknown",
    model: modelLabel(value.model || value.config?.model),
    totalAttempts: numberOrZero(value.totalAttempts ?? value.total ?? value.attemptsTotal),
    completedAttempts: numberOrZero(value.completedAttempts ?? value.completed ?? value.attemptsCompleted),
    passCount: numberOrZero(value.passCount ?? value.passes),
    errorCount: numberOrZero(value.errorCount ?? value.errors),
    retryCount: numberOrZero(value.retryCount ?? value.retries),
    totalCost: numberOrZero(value.totalCost ?? value.cost),
    elapsedTimeMs: numberOrZero(value.elapsedTimeMs ?? value.elapsedMs ?? value.elapsedTime),
    startedAt: value.startedAt || null,
    finishedAt: value.finishedAt || null,
    status: value.status || null,
    caseId: value.caseId || null,
    size: value.size || null,
    currentAttempt: numberOrZero(value.currentAttempt),
    attemptsPerCase: numberOrZero(value.attemptsPerCase),
    currentCase: numberOrZero(value.currentCase),
    totalCases: numberOrZero(value.totalCases),
  }
}

function initDb() {
  if (db) return
  db = new Database(DB_FILE)
  db.pragma("journal_mode = WAL")
  db.exec(`
    CREATE TABLE IF NOT EXISTS runs (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      file TEXT UNIQUE NOT NULL,
      mode TEXT NOT NULL,
      model TEXT NOT NULL,
      total_attempts INTEGER NOT NULL,
      completed_attempts INTEGER NOT NULL,
      pass_count INTEGER NOT NULL,
      error_count INTEGER NOT NULL,
      retry_count INTEGER NOT NULL,
      elapsed_time_ms INTEGER NOT NULL,
      total_cost REAL DEFAULT 0,
      started_at TEXT,
      finished_at TEXT,
      inserted_at TEXT NOT NULL
    )
  `)
  
  // Migrate existing schema if needed
  try {
    db.exec(`ALTER TABLE runs ADD COLUMN total_cost REAL DEFAULT 0`)
  } catch (e) {
    // Column already exists
  }
}

function insertSummary(summary) {
  const stmt = db.prepare(`
    INSERT INTO runs (
      file, mode, model, total_attempts, completed_attempts, pass_count, error_count, retry_count, elapsed_time_ms,
      total_cost, started_at, finished_at, inserted_at
    ) VALUES (
      @file, @mode, @model, @totalAttempts, @completedAttempts, @passCount, @errorCount, @retryCount, @elapsedTimeMs,
      @totalCost, @startedAt, @finishedAt, @insertedAt
    )
    ON CONFLICT(file) DO UPDATE SET
      mode=excluded.mode,
      model=excluded.model,
      total_attempts=excluded.total_attempts,
      completed_attempts=excluded.completed_attempts,
      pass_count=excluded.pass_count,
      error_count=excluded.error_count,
      retry_count=excluded.retry_count,
      elapsed_time_ms=excluded.elapsed_time_ms,
      total_cost=excluded.total_cost,
      started_at=excluded.started_at,
      finished_at=excluded.finished_at,
      inserted_at=excluded.inserted_at
  `)

  stmt.run({
    ...summary,
    insertedAt: nowIso(),
  })
}

async function ingestRunFile(filePath) {
  const fileName = path.basename(filePath)
  if (!fileName.endsWith(".json") || fileName === "progress.json") return false
  const raw = await fs.readFile(filePath, "utf8")
  const report = JSON.parse(raw)
  const summary = summarizeReport(report, fileName)
  insertSummary(summary)
  return true
}

async function ingestAllRunsFromJson() {
  const entries = await fs.readdir(RUNS_DIR, { withFileTypes: true })
  const files = entries
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name)
    .filter((name) => name.endsWith(".json") && name !== "progress.json")
  let count = 0
  for (const file of files) {
    const ok = await ingestRunFile(path.join(RUNS_DIR, file))
    if (ok) count += 1
  }

  return count
}

async function resetDbAndRepopulate() {
  if (db) {
    db.close()
    db = null
  }

  await fs.rm(DB_FILE, { force: true })
  await fs.rm(`${DB_FILE}-wal`, { force: true })
  await fs.rm(`${DB_FILE}-shm`, { force: true })

  initDb()
  return ingestAllRunsFromJson()
}

function listRunsFromDb() {
  const stmt = db.prepare(`
    SELECT
      file,
      mode,
      model,
      total_attempts AS totalAttempts,
      completed_attempts AS completedAttempts,
      pass_count AS passCount,
      error_count AS errorCount,
      retry_count AS retryCount,
      total_cost AS totalCost,
      elapsed_time_ms AS elapsedTimeMs,
      started_at AS startedAt,
      finished_at AS finishedAt
    FROM runs
    ORDER BY COALESCE(started_at, inserted_at) DESC
    LIMIT ?
  `)

  return stmt.all(RUN_LIMIT)
}

function loadModelCatalog() {
  try {
    const out = execSync("opencode models --verbose", { encoding: "utf8", maxBuffer: 10 * 1024 * 1024 })
    const lines = out.split("\n")
    
    const providers = {}
    let currentModel = null
    let jsonBuffer = ""
    let braceDepth = 0
    
    for (const line of lines) {
      // Model identifier line (e.g., "opencode/big-pickle")
      if (/^[a-zA-Z0-9_-]+\/[a-zA-Z0-9._-]+$/.test(line.trim()) && braceDepth === 0) {
        // Process previous model if exists
        if (currentModel && jsonBuffer) {
          try {
            const data = JSON.parse(jsonBuffer)
            const provider = currentModel.split("/")[0]
            if (!providers[provider]) providers[provider] = []
            
            const isFree = data.cost && data.cost.input === 0 && data.cost.output === 0
            
            providers[provider].push({
              id: data.id,
              name: data.name || data.id,
              cost: data.cost || null,
              isFree,
            })
          } catch (e) {
            // Skip malformed JSON
          }
        }
        currentModel = line.trim()
        jsonBuffer = ""
        braceDepth = 0
        continue
      }
      
      // Track brace depth
      if (currentModel) {
        for (const char of line) {
          if (char === "{") braceDepth++
          if (char === "}") braceDepth--
        }
        jsonBuffer += line + "\n"
      }
    }
    
    // Process last model
    if (currentModel && jsonBuffer) {
      try {
        const data = JSON.parse(jsonBuffer)
        const provider = currentModel.split("/")[0]
        if (!providers[provider]) providers[provider] = []
        
        const isFree = data.cost && data.cost.input === 0 && data.cost.output === 0
        
        providers[provider].push({
          id: data.id,
          name: data.name || data.id,
          cost: data.cost || null,
          isFree,
        })
      } catch (e) {
        // Skip malformed JSON
      }
    }

    return Object.entries(providers)
      .map(([provider, models]) => ({ 
        provider, 
        models: models.sort((a, b) => a.id.localeCompare(b.id))
      }))
      .sort((a, b) => a.provider.localeCompare(b.provider))
  } catch (error) {
    return []
  }
}
async function loadProgress() {
  try {
    const raw = await fs.readFile(PROGRESS_FILE, "utf8")
    return normalizeProgress(JSON.parse(raw))
  } catch (error) {
    if (error && error.code === "ENOENT") return null
    throw error
  }
}

function appendLog(line) {
  const timestamped = `[${nowIso()}] ${line}`
  activeRun.logs.push(timestamped)
  if (activeRun.logs.length > LOG_TAIL_LIMIT) {
    activeRun.logs = activeRun.logs.slice(activeRun.logs.length - LOG_TAIL_LIMIT)
  }
  // Write to file asynchronously without blocking
  fs.appendFile(LOG_FILE, `${timestamped}\n`, "utf8").catch(() => {})
}

function parseBody(req) {
  return new Promise((resolve, reject) => {
    let data = ""
    req.on("data", (chunk) => {
      data += String(chunk)
      if (data.length > 1024 * 1024) {
        reject(new Error("Request too large"))
      }
    })
    req.on("end", () => {
      if (!data) return resolve({})
      try {
        resolve(JSON.parse(data))
      } catch (error) {
        reject(new Error("Invalid JSON body"))
      }
    })
    req.on("error", reject)
  })
}

function sanitizeList(list, fallback) {
  if (!Array.isArray(list) || list.length === 0) return fallback
  return list.map((x) => String(x).trim()).filter((x) => x.length > 0)
}

function startRunJob(config) {
  if (activeRun.running) {
    throw new Error("A run is already in progress")
  }

  // Clean up any stale OpenCode server processes before starting
  try {
    execSync("pkill -f 'opencode serve' 2>/dev/null || true", { encoding: "utf8" })
  } catch (e) {
    // Ignore cleanup errors
  }

  activeRun = {
    running: true,
    startedAt: nowIso(),
    finishedAt: null,
    config,
    completedJobs: 0,
    totalJobs: config.modes.length,
    lastError: null,
    logs: [],
  }
  activeChild = null

  fs.writeFile(LOG_FILE, "", "utf8").catch(() => {})

  progressPoller = setInterval(pollProgress, 3000)

  void (async () => {
    try {
      for (const mode of config.modes) {
        appendLog(
          `Starting mode=${mode} model=${config.model} sizes=${config.sizes.join(",")} repeats=${config.repeats}`
        )
        const reportPath = await runSingleBenchmark({ ...config, mode })
        appendLog(`Completed mode=${mode} model=${config.model} -> ${reportPath}`)
        await ingestRunFile(reportPath)
        activeRun.completedJobs += 1
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      activeRun.lastError = message
      appendLog(`Run failed: ${message}`)
    } finally {
      if (progressPoller) {
        clearInterval(progressPoller)
        progressPoller = null
      }
      activeRun.running = false
      activeRun.finishedAt = nowIso()
    }
  })()
}

function runSingleBenchmark(config) {
  return new Promise((resolve, reject) => {
    const args = [
      "run",
      "run",
      "--",
      "--mode",
      config.mode,
      "--sizes",
      config.sizes.join(","),
      "--repeats",
      String(config.repeats),
      "--model",
      config.model,
      "--progress",
      "runs/progress.json",
    ]

    const child = spawn("npm", args, { cwd: ROOT_DIR })
    activeChild = child
    let reportPath = null

    const onData = (chunk) => {
      const text = String(chunk)
      const lines = text.split("\n").map((line) => line.trim()).filter(Boolean)
      for (const line of lines) {
        // Add to in-memory log buffer immediately
        const timestamped = `[${nowIso()}] ${line}`
        activeRun.logs.push(timestamped)
        if (activeRun.logs.length > LOG_TAIL_LIMIT) {
          activeRun.logs = activeRun.logs.slice(activeRun.logs.length - LOG_TAIL_LIMIT)
        }
        
        // Also write to file (async, don't wait)
        fs.appendFile(LOG_FILE, `${timestamped}\n`, "utf8").catch(() => {})
        
        // Check for report path
        if (line.endsWith(".json") && line.includes(`${path.sep}runs${path.sep}`)) {
          reportPath = line
        }
      }
    }

    child.stdout.on("data", onData)
    child.stderr.on("data", onData)

    child.on("error", reject)
    child.on("exit", (code, signal) => {
      activeChild = null

      if (signal === "SIGTERM") {
        reject(new Error("Benchmark stopped by user"))
        return
      }

      if (code !== 0) {
        reject(new Error(`Benchmark process exited with code ${code}`))
        return
      }

      if (!reportPath) {
        reject(new Error("Benchmark completed but report path was not detected"))
        return
      }

      resolve(reportPath)
    })
  })
}

function stopRunJob() {
  if (!activeRun.running) return false
  if (activeChild) {
    activeChild.kill("SIGTERM")
  }
  return true
}
async function serveStatic(reqPath, res) {
  const decodedPath = decodeURIComponent(reqPath)
  const relativePath = decodedPath === "/" ? "index.html" : decodedPath.replace(/^\/+/, "")
  const normalizedPath = path.normalize(relativePath).replace(/^(\.\.(\/|\\|$))+/, "")
  const filePath = path.join(WEB_DIR, normalizedPath)
  if (!filePath.startsWith(WEB_DIR)) {
    res.writeHead(403)
    res.end("Forbidden")
    return
  }

  try {
    const ext = path.extname(filePath).toLowerCase()
    const data = await fs.readFile(filePath)
    res.writeHead(200, {
      "Content-Type": MIME_TYPES[ext] || "application/octet-stream",
      "Cache-Control": "no-store",
    })
    res.end(data)
  } catch (error) {
    if (error && error.code === "ENOENT") {
      res.writeHead(404)
      res.end("Not Found")
      return
    }
    res.writeHead(500)
    res.end("Internal Server Error")
  }
}

await ensureRunsDir()
initDb()

// Load existing logs from active.log into memory
try {
  const logContent = await fs.readFile(LOG_FILE, "utf8")
  const lines = logContent.split("\n").filter((line) => line.trim().length > 0)
  activeRun.logs = lines.slice(-LOG_TAIL_LIMIT) // Keep last N lines
} catch (error) {
  // No existing log file, start fresh
  activeRun.logs = []
}

const seedCount = db.prepare("SELECT COUNT(*) AS c FROM runs").get().c
if (seedCount === 0) {
  await ingestAllRunsFromJson()
}
const server = http.createServer(async (req, res) => {
  const requestUrl = new URL(req.url || "/", `http://${req.headers.host || "localhost"}`)
  try {
    if (requestUrl.pathname === "/api/models") {
      const providers = loadModelCatalog()
      sendJson(res, 200, { providers })
      return
    }
    if (requestUrl.pathname === "/api/progress") {
      const progress = await loadProgress()
      sendJson(res, 200, { progress })
      return
    }

    if (requestUrl.pathname === "/api/status") {
      sendJson(res, 200, {
        running: activeRun.running,
        startedAt: activeRun.startedAt,
        finishedAt: activeRun.finishedAt,
        config: activeRun.config,
        completedJobs: activeRun.completedJobs,
        totalJobs: activeRun.totalJobs,
        lastError: activeRun.lastError,
      })
      return
    }

    if (requestUrl.pathname === "/api/log") {
      const tail = Number.parseInt(requestUrl.searchParams.get("tail") || "120", 10)
      const limit = Number.isInteger(tail) && tail > 0 ? Math.min(tail, LOG_TAIL_LIMIT) : 120
      const lines = activeRun.logs.slice(Math.max(0, activeRun.logs.length - limit))
      sendJson(res, 200, { lines })
      return
    }
    if (requestUrl.pathname === "/api/runs") {
      const runs = listRunsFromDb()
      sendJson(res, 200, { runs })
      return
    }

    if (requestUrl.pathname === "/api/run/detail" && req.method === "GET") {
      const fileName = requestUrl.searchParams.get("file")
      if (!fileName) {
        sendJson(res, 400, { error: "Missing file parameter" })
        return
      }
      const filePath = path.join(RUNS_DIR, fileName)
      if (!filePath.startsWith(RUNS_DIR) || !fileName.endsWith(".json")) {
        sendJson(res, 403, { error: "Invalid file path" })
        return
      }
      try {
        const raw = await fs.readFile(filePath, "utf8")
        const report = JSON.parse(raw)
        sendJson(res, 200, { report })
      } catch (error) {
        sendJson(res, 404, { error: "File not found" })
      }
      return
    }

    if (requestUrl.pathname === "/api/run" && req.method === "POST") {
      const body = await parseBody(req)
      const model = String(body.model || "").trim()
      const modes = sanitizeList(body.modes, ["hashline", "raw_replace"])
      const sizes = sanitizeList(body.sizes, ["small", "mid", "large"])
      const repeats = Math.max(1, Number.parseInt(String(body.repeats ?? 1), 10) || 1)

      if (!model) {
        sendJson(res, 400, { error: "Select one model" })
        return
      }

      startRunJob({ model, modes, sizes, repeats })
      sendJson(res, 200, { ok: true })
      return
    }

    if (requestUrl.pathname === "/api/run/stop" && req.method === "POST") {
      const stopped = stopRunJob()
      sendJson(res, 200, { ok: true, stopped })
      return
    }

    if (requestUrl.pathname === "/api/admin/rebuild" && req.method === "POST") {
      const count = await resetDbAndRepopulate()
      sendJson(res, 200, { ok: true, ingested: count })
      return
    }

    if (requestUrl.pathname === "/api/admin/drop" && req.method === "POST") {
      if (db) {
        db.close()
        db = null
      }
      await fs.rm(DB_FILE, { force: true })
      await fs.rm(`${DB_FILE}-wal`, { force: true })
      await fs.rm(`${DB_FILE}-shm`, { force: true })
      initDb()
      sendJson(res, 200, { ok: true })
      return
    }
    await serveStatic(requestUrl.pathname, res)
  } catch (error) {
    sendJson(res, 500, { error: String(error && error.message ? error.message : error) })
  }
})
server.on("error", (error) => {
  if (error && error.code === "EADDRINUSE") {
    console.error(`Port ${PORT} is already in use. Stop the existing dashboard or set PORT.`)
    process.exit(1)
    return
  }

  console.error(String(error && error.message ? error.message : error))
  process.exit(1)
})
server.listen(PORT, () => {
  console.log(`http://localhost:${PORT}`)
})

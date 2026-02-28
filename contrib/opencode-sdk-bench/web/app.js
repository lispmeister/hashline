let allModels = []
function formatDuration(ms) {
  const value = Number(ms)
  if (!Number.isFinite(value) || value <= 0) return "0s"
  const totalSeconds = Math.floor(value / 1000)
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  if (minutes > 0) return `${minutes}m ${seconds}s`
  return `${seconds}s`
}
function rowsForStats(stats) {
  const costStr = stats.totalCost > 0 ? `$${stats.totalCost.toFixed(4)}` : "$0.00"
  return [
    ["mode", stats.mode],
    ["model", stats.model],
    ["total attempts", stats.totalAttempts],
    ["completed attempts", stats.completedAttempts],
    ["pass count", stats.passCount],
    ["error count", stats.errorCount],
    ["retry count", stats.retryCount],
    ["total cost", costStr],
    ["elapsed", formatDuration(stats.elapsedTimeMs)],
  ]
}
function checkedValues(selector) {
  return Array.from(document.querySelectorAll(selector))
    .filter((el) => el.checked)
    .map((el) => el.value)
}
function setRunMessage(text) {
  const node = document.getElementById("run-msg")
  if (node) node.textContent = text || ""
}
function setTab(tab) {
  for (const btn of document.querySelectorAll(".tab-btn")) {
    btn.classList.toggle("active", btn.dataset.tab === tab)
  }
  for (const panel of document.querySelectorAll(".tab")) {
    panel.classList.toggle("active", panel.id === `tab-${tab}`)
  }
}
function updateRunControls(status) {
  const modelSelect = document.getElementById("model-select")
  const hasModel = Boolean(modelSelect && modelSelect.value)
  const hasModes = checkedValues('input[name="mode"]').length > 0
  const hasSizes = checkedValues('input[name="size"]').length > 0
  const running = Boolean(status && status.running)
  const runBtn = document.getElementById("start-run")
  const stopBtn = document.getElementById("stop-run")


  if (runBtn) runBtn.disabled = running || !hasModel || !hasModes || !hasSizes
  if (stopBtn) stopBtn.disabled = !running
}
function renderStatus(status) {
  const root = document.getElementById("status")
  if (!root) return
  if (!status) {
    root.textContent = "No status available"
    updateRunControls(null)
    return
  }
  const text = status.running
    ? `Running (${status.completedJobs}/${status.totalJobs})`
    : status.lastError
      ? `Stopped with error: ${status.lastError}`
      : "Idle"
  updateRunControls(status)
}

function renderProgressBar(status, progress) {
  const fill = document.getElementById("progress-fill")
  const label = document.getElementById("progress-label")
  const tableRoot = document.getElementById("progress")

  if (!fill || !label) return

  if (!status || !status.running) {
    fill.style.width = "0%"
    label.textContent = ""
    return
  }

  // Calculate progress based on test cases, not modes
  let pct = 0
  if (progress && progress.totalAttempts > 0) {
    pct = Math.round((progress.completedAttempts / progress.totalAttempts) * 100)
  } else if (status.totalJobs > 0) {
    pct = Math.round((status.completedJobs / status.totalJobs) * 100)
  }
  fill.style.width = `${pct}%`

  if (progress) {
    const costStr = progress.totalCost > 0 ? ` | $${progress.totalCost.toFixed(4)}` : ''
    label.textContent = `${progress.completedAttempts || 0}/${progress.totalAttempts || 0} tests | ${progress.passCount || 0} passed | ${progress.errorCount || 0} errors${costStr}`
  } else {
    label.textContent = `${status.completedJobs}/${status.totalJobs} modes completed`
  }
}
function renderProgress(progress) {
  const root = document.getElementById("progress")
  if (!root) return
  if (!progress) {
    root.innerHTML = '<p class="muted">No progress file yet.</p>'
    return
  }
  const rows = rowsForStats(progress)
    .map(([label, value]) => `<tr><th>${label}</th><td>${value ?? "-"}</td></tr>`)
    .join("")
  root.innerHTML = `<table><tbody>${rows}</tbody></table>`
}
function renderStatsSummary(runs) {
  const root = document.getElementById("stats-summary")
  if (!root) return
  if (!runs.length) {
    root.innerHTML = '<p class="muted">No statistics available yet.</p>'
    return
  }

  // Group by mode
  const byMode = {}
  for (const run of runs) {
    if (!byMode[run.mode]) {
      byMode[run.mode] = { total: 0, passed: 0, errors: 0, retries: 0, cost: 0 }
    }
    byMode[run.mode].total += run.totalAttempts
    byMode[run.mode].passed += run.passCount
    byMode[run.mode].errors += run.errorCount
    byMode[run.mode].retries += run.retryCount
    byMode[run.mode].cost += run.totalCost || 0
  }

  // Group by model
  const byModel = {}
  for (const run of runs) {
    if (!byModel[run.model]) {
      byModel[run.model] = { total: 0, passed: 0, errors: 0, retries: 0, cost: 0 }
    }
    byModel[run.model].total += run.totalAttempts
    byModel[run.model].passed += run.passCount
    byModel[run.model].errors += run.errorCount
    byModel[run.model].retries += run.retryCount
    byModel[run.model].cost += run.totalCost || 0
  }

  const modeRows = Object.entries(byMode)
    .map(([mode, stats]) => {
      const passRate = stats.total > 0 ? Math.round((stats.passed / stats.total) * 100) : 0
      const costStr = stats.cost > 0 ? `$${stats.cost.toFixed(4)}` : "$0.00"
      return `
        <tr>
          <td><strong>${mode}</strong></td>
          <td>${stats.total}</td>
          <td>${stats.passed}</td>
          <td><strong>${passRate}%</strong></td>
          <td>${stats.errors}</td>
          <td>${stats.retries}</td>
          <td>${costStr}</td>
        </tr>
      `
    })
    .join("")

  const modelRows = Object.entries(byModel)
    .map(([model, stats]) => {
      const passRate = stats.total > 0 ? Math.round((stats.passed / stats.total) * 100) : 0
      const costStr = stats.cost > 0 ? `$${stats.cost.toFixed(4)}` : "$0.00"
      return `
        <tr>
          <td>${model}</td>
          <td>${stats.total}</td>
          <td>${stats.passed}</td>
          <td><strong>${passRate}%</strong></td>
          <td>${stats.errors}</td>
          <td>${stats.retries}</td>
          <td>${costStr}</td>
        </tr>
      `
    })
    .join("")

  root.innerHTML = `
    <h3>By Mode</h3>
    <table>
      <thead>
        <tr><th>Mode</th><th>Total</th><th>Passed</th><th>Pass Rate</th><th>Errors</th><th>Retries</th><th>Total Cost</th></tr>
      </thead>
      <tbody>${modeRows}</tbody>
    </table>
    <h3>By Model</h3>
    <table>
      <thead>
        <tr><th>Model</th><th>Total</th><th>Passed</th><th>Pass Rate</th><th>Errors</th><th>Retries</th><th>Total Cost</th></tr>
      </thead>
      <tbody>${modelRows}</tbody>
    </table>
  `
}

function renderRuns(runs) {
  cachedRuns = runs
  const root = document.getElementById("runs")
  if (!root) return
  if (!runs.length) {
    root.innerHTML = '<p class="muted">No runs stored in sqlite yet.</p>'
    return
  }
  const head = `
    <thead>
      <tr>
        <th>file</th>
        <th>mode</th>
        <th>model</th>
        <th>total</th>
        <th>pass</th>
        <th>errors</th>
        <th>retries</th>
        <th>cost</th>
        <th>elapsed</th>
        <th>started</th>
      </tr>
    </thead>
  `
  const body = runs
    .map((run) => {
      const costStr = run.totalCost > 0 ? `$${run.totalCost.toFixed(4)}` : "$0.00"
      return `
        <tr class="run-row" data-file="${run.file}">
          <td>${run.file}</td>
          <td>${run.mode}</td>
          <td>${run.model}</td>
          <td>${run.totalAttempts}</td>
          <td>${run.passCount}</td>
          <td>${run.errorCount}</td>
          <td>${run.retryCount}</td>
          <td>${costStr}</td>
          <td>${formatDuration(run.elapsedTimeMs)}</td>
          <td>${run.startedAt || "-"}</td>
        </tr>
      `
    })
    .join("")
  root.innerHTML = `<table>${head}<tbody>${body}</tbody></table>`

  // Add click handlers for drill-down
  for (const row of root.querySelectorAll(".run-row")) {
    row.addEventListener("click", () => {
      const file = row.dataset.file
      if (file) showRunDetail(file)
    })
  }
}

async function showRunDetail(fileName) {
  const modal = document.getElementById("detail-modal")
  const content = document.getElementById("detail-content")
  
  if (!modal || !content) return
  
  modal.classList.add("active")
  content.innerHTML = '<p class="muted">Loading...</p>'
  
  try {
    const payload = await getJson(`/api/run/detail?file=${encodeURIComponent(fileName)}`)
    const report = payload.report
    
    if (!report || !report.results) {
      content.innerHTML = '<p class="muted">No results found.</p>'
      return
    }
    
    const caseRows = report.results
      .map((caseResult) => {
        const attempts = caseResult.attempts || []
        const passedCount = attempts.filter(a => a.passed).length
        const totalAttempts = attempts.length
        const passRate = totalAttempts > 0 ? Math.round((passedCount / totalAttempts) * 100) : 0
        
        const attemptDetails = attempts
          .map(a => `
            <tr>
              <td>Attempt ${a.attempt}</td>
              <td>${a.passed ? '✓ Pass' : '✗ Fail'}</td>
              <td>${a.reason || '-'}</td>
              <td>${a.metrics?.durationMs || 0}ms</td>
              <td>${a.metrics?.retries || 0}</td>
              <td>${a.metrics?.errorCount || 0}</td>
            </tr>
          `)
          .join("")
        
        return `
          <div class="case-detail">
            <h4>${caseResult.caseId} (${caseResult.size}) - ${passedCount}/${totalAttempts} passed (${passRate}%)</h4>
            <table>
              <thead>
                <tr><th>Attempt</th><th>Result</th><th>Reason</th><th>Duration</th><th>Retries</th><th>Errors</th></tr>
              </thead>
              <tbody>${attemptDetails}</tbody>
            </table>
          </div>
        `
      })
      .join("")
    
    content.innerHTML = `
      <p><strong>File:</strong> ${fileName}</p>
      <p><strong>Mode:</strong> ${report.config?.mode || 'unknown'}</p>
      <p><strong>Model:</strong> ${report.model?.label || 'unknown'}</p>
      <p><strong>Started:</strong> ${report.startedAt || '-'}</p>
      <p><strong>Finished:</strong> ${report.finishedAt || '-'}</p>
      <hr />
      ${caseRows}
    `
  } catch (error) {
    content.innerHTML = `<p class="muted">Error loading details: ${String(error)}</p>`
  }
}

function closeDetailModal() {
  const modal = document.getElementById("detail-modal")
  if (modal) modal.classList.remove("active")
}

let cachedRuns = []

function exportAsCSV() {
  if (!cachedRuns.length) {
    alert("No runs available to export")
    return
  }
  
  const headers = ["file", "mode", "model", "totalAttempts", "passCount", "errorCount", "retryCount", "totalCost", "elapsedTimeMs", "startedAt"]
  const rows = cachedRuns.map(run => 
    headers.map(key => {
      const value = run[key] ?? ""
      // Escape quotes and wrap in quotes if contains comma or quote
      const escaped = String(value).replace(/"/g, '""')
      return escaped.includes(",") || escaped.includes('"') ? `"${escaped}"` : escaped
    }).join(",")
  )
  
  const csv = [headers.join(","), ...rows].join("\n")
  downloadFile(csv, "benchmark-runs.csv", "text/csv")
}

function exportAsJSON() {
  if (!cachedRuns.length) {
    alert("No runs available to export")
    return
  }
  
  const json = JSON.stringify(cachedRuns, null, 2)
  downloadFile(json, "benchmark-runs.json", "application/json")
}

function downloadFile(content, fileName, mimeType) {
  const blob = new Blob([content], { type: mimeType })
  const url = URL.createObjectURL(blob)
  const link = document.createElement("a")
  link.href = url
  link.download = fileName
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}
function renderLog(lines) {
  const root = document.getElementById("log")
  if (!root) return
  root.textContent = lines.join("\n")
  root.scrollTop = root.scrollHeight
}
function renderModels(providers) {
  const root = document.getElementById("models")
  if (!root) return
  if (!providers.length) {
    root.innerHTML = '<p class="muted">No models detected. Check `opencode models` output.</p>'
    updateRunControls(null)
    return
  }

  allModels = []
  for (const provider of providers) {
    for (const model of provider.models) {
      const fullName = `${provider.provider}/${model.id}`
      allModels.push({
        fullName,
        name: model.name,
        cost: model.cost,
        isFree: model.isFree,
      })
    }
  }
  allModels.sort((a, b) => a.fullName.localeCompare(b.fullName))

  function renderList(filter) {
    const filtered = filter
      ? allModels.filter((m) => m.fullName.toLowerCase().includes(filter.toLowerCase()))
      : allModels // Show all models by default

    const html = filtered
      .map((model) => {
        const costLabel = model.isFree 
          ? '<span style="color: #147d7f; font-weight: bold;">FREE</span>'
          : model.cost 
            ? `<span style="color: #5f6e83; font-size: 11px;">$${model.cost.input}/M in, $${model.cost.output}/M out</span>`
            : ''
        
        return `
          <label class="model-item">
            <input type="radio" name="model" value="${model.fullName}" />
            ${model.fullName} ${costLabel}
          </label>
        `
      })
      .join("")

    const listRoot = document.getElementById("model-list")
    if (listRoot) listRoot.innerHTML = html

    for (const el of listRoot.querySelectorAll('input[name="model"]')) {
      el.addEventListener("change", () => updateRunControls(null))
    }
  }

  root.innerHTML = `
    <input type="text" id="model-search" class="model-search" placeholder="Search models..." />
    <div id="model-list" class="model-list"></div>
  `

  const searchInput = document.getElementById("model-search")
  if (searchInput) {
    searchInput.addEventListener("input", (e) => renderList(e.target.value))
  }

  renderList("")
  updateRunControls(null)
}
async function getJson(url, options) {
  const response = await fetch(url, options)
  const data = await response.json()
  if (!response.ok) {
    throw new Error(data.error || `Request failed: ${response.status}`)
  }
  return data
}
async function refresh() {
  try {
    const [statusPayload, progressPayload, runsPayload, logPayload] = await Promise.all([
      getJson("/api/status"),
      getJson("/api/progress"),
      getJson("/api/runs"),
      getJson("/api/log?tail=200"),
    ])
    renderStatus(statusPayload)
    renderProgressBar(statusPayload, progressPayload.progress || null)
    renderProgress(progressPayload.progress || null)
    const runs = runsPayload.runs || []
    renderStatsSummary(runs)
    renderRuns(runs)
    renderLog(logPayload.lines || [])
  } catch (error) {
    const status = document.getElementById("status")
    if (status) status.textContent = `Connection error: ${String(error)}`
    
    const runs = document.getElementById("runs")
    if (runs && !runs.innerHTML) {
      runs.innerHTML = '<p class="muted">Waiting for server connection...</p>'
    }
  }
}
async function loadModels() {
  try {
    const payload = await getJson("/api/models")
    renderModels(payload.providers || [])
  } catch (error) {
    const node = document.getElementById("models")
    if (node) {
      node.innerHTML = `<p class="muted">Loading models... (${String(error)})</p>`
    }
    // Retry after 2 seconds if initial load fails
    setTimeout(loadModels, 2000)
  }
}
async function startRun() {
  const modelRadio = document.querySelector('input[name="model"]:checked')
  const model = modelRadio ? modelRadio.value : ""
  const modes = checkedValues('input[name="mode"]')
  const sizes = checkedValues('input[name="size"]')
  const repeatsNode = document.getElementById("repeats")
  const repeats = Number.parseInt(repeatsNode ? repeatsNode.value : "1", 10) || 1
  
  if (!model) {
    setRunMessage("Select a model first.")
    return
  }
  if (modes.length === 0 || sizes.length === 0) {
    setRunMessage("Select at least one mode and one size.")
    return
  }
  await getJson("/api/run", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ model, modes, sizes, repeats }),
  })
  setRunMessage(`Started run for ${model}`)
  await refresh()
}
async function stopRun() {
  const payload = await getJson("/api/run/stop", { method: "POST" })
  setRunMessage(payload.stopped ? "Stop signal sent." : "No active run to stop.")
  await refresh()
}
async function rebuildDb() {
  const msg = document.getElementById("admin-msg")
  if (!msg) return
  msg.textContent = "Rebuilding DB..."
  try {
    const payload = await getJson("/api/admin/rebuild", { method: "POST" })
    msg.textContent = `DB rebuilt from JSON (${payload.ingested} files).`
    await refresh()
  } catch (error) {
    msg.textContent = String(error)
  }
}
async function dropDb() {
  const msg = document.getElementById("admin-msg")
  if (!msg) return
  msg.textContent = "Dropping DB..."
  try {
    await getJson("/api/admin/drop", { method: "POST" })
    msg.textContent = "DB dropped."
    await refresh()
  } catch (error) {
    msg.textContent = String(error)
  }
}

for (const btn of document.querySelectorAll(".tab-btn")) {
  btn.addEventListener("click", () => setTab(btn.dataset.tab))
}
for (const el of document.querySelectorAll('input[name="mode"], input[name="size"]')) {
  el.addEventListener("change", () => updateRunControls(null))
}

const runBtn = document.getElementById("start-run")
if (runBtn) {
  runBtn.addEventListener("click", () => {
    startRun().catch((error) => setRunMessage(String(error)))
  })
}

const stopBtn = document.getElementById("stop-run")
if (stopBtn) {
  stopBtn.addEventListener("click", () => {
    stopRun().catch((error) => setRunMessage(String(error)))
  })
}
const rebuildBtn = document.getElementById("rebuild-db")
if (rebuildBtn) {
  rebuildBtn.addEventListener("click", () => {
    rebuildDb().catch(() => {})
  })
}
const dropBtn = document.getElementById("drop-db")
if (dropBtn) {
  dropBtn.addEventListener("click", () => {
    dropDb().catch(() => {})
  })
}

const exportCSVBtn = document.getElementById("export-csv")
if (exportCSVBtn) {
  exportCSVBtn.addEventListener("click", exportAsCSV)
}

const exportJSONBtn = document.getElementById("export-json")
if (exportJSONBtn) {
  exportJSONBtn.addEventListener("click", exportAsJSON)
}

let refreshInterval = null

function startAutoRefresh() {
  if (refreshInterval) return
  refreshInterval = setInterval(() => {
    const checkbox = document.getElementById("auto-refresh")
    if (checkbox && checkbox.checked) {
      refresh()
    }
  }, 2000)
}

function stopAutoRefresh() {
  if (refreshInterval) {
    clearInterval(refreshInterval)
    refreshInterval = null
  }
}

const autoRefreshCheckbox = document.getElementById("auto-refresh")
if (autoRefreshCheckbox) {
  autoRefreshCheckbox.addEventListener("change", (e) => {
    if (e.target.checked) {
      startAutoRefresh()
      refresh() // Immediate refresh when enabled
    } else {
      stopAutoRefresh()
    }
  })
}

loadModels()
refresh()
startAutoRefresh()

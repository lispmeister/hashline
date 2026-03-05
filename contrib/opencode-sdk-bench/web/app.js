let allModels = []
let cachedRuns = []
let refreshInterval = null
const formatDuration = (ms) => {
  const value = Number(ms)
  if (!Number.isFinite(value) || value <= 0) return "0s"
  const seconds = Math.floor(value / 1000)
  const minutes = Math.floor(seconds / 60)
  const rem = seconds % 60
  return minutes > 0 ? `${minutes}m ${rem}s` : `${rem}s`
}

const checkedValues = (selector) => Array.from(document.querySelectorAll(selector))
  .filter((el) => el.checked)
    .map((el) => el.value)
const setRunMessage = (text) => {
  const node = document.getElementById("run-msg")
  if (node) node.textContent = text || ""
}

const setTab = (tab) => {
  for (const btn of document.querySelectorAll(".tab-btn")) {
    btn.classList.toggle("active", btn.dataset.tab === tab)
  }
  for (const panel of document.querySelectorAll(".tab")) {
    panel.classList.toggle("active", panel.id === `tab-${tab}`)
  }
}

const updateRunControls = (status) => {
  const modelRadio = document.querySelector('input[name="model"]:checked')
  const hasModel = Boolean(modelRadio && modelRadio.value)
  const hasModes = checkedValues('input[name="mode"]').length > 0
  const hasSizes = checkedValues('input[name="size"]').length > 0
  const running = Boolean(status && status.running)
  const runBtn = document.getElementById("start-run")
  const stopBtn = document.getElementById("stop-run")
  if (runBtn) runBtn.disabled = running || !hasModel || !hasModes || !hasSizes
  if (stopBtn) stopBtn.disabled = !running
}

const renderStatus = (status) => {
  const root = document.getElementById("status")
  if (!root) return
  if (!status) {
    root.textContent = "No status available"
    updateRunControls(null)
    return
  }

  root.textContent = status.running
    ? `Running (${status.completedJobs}/${status.totalJobs})`
    : status.lastError
      ? `Stopped with error: ${status.lastError}`
      : status.blockedReason
        ? `Idle (blocked: ${status.blockedReason})`
        : "Idle"
}

const renderProgressBar = (status, progress) => {
  const fill = document.getElementById("progress-fill")
  const label = document.getElementById("progress-label")
  if (!fill || !label) return

  if (!status || !status.running) {
    fill.style.width = "0%"
    label.textContent = ""
    return
  }

  const total = progress?.totalAttempts || status.totalJobs || 0
  const done = progress?.completedAttempts || status.completedJobs || 0
  const pct = total > 0 ? Math.round((done / total) * 100) : 0
  fill.style.width = `${pct}%`
  label.textContent = progress
    ? `${done}/${total} tests | ${progress.passCount || 0} passed | ${progress.errorCount || 0} errors`
    : `${status.completedJobs}/${status.totalJobs} modes completed`
}

const renderProgress = (progress) => {
  const root = document.getElementById("progress")
  if (!root) return
  if (!progress) {
    root.innerHTML = '<p class="muted">No progress file yet.</p>'
    return
  }

  const rows = [
    ["mode", progress.mode],
    ["model", progress.model],
    ["total attempts", progress.totalAttempts],
    ["completed attempts", progress.completedAttempts],
    ["task pass", progress.taskPassCount ?? "-"],
    ["protocol pass", progress.protocolPassCount ?? "-"],
    ["overall pass", progress.overallPassCount ?? "-"],
    ["corruption", progress.corruptionCount ?? "-"],
    ["pass count", progress.passCount],
    ["error count", progress.errorCount],
    ["retry count", progress.retryCount],
    ["elapsed", formatDuration(progress.elapsedTimeMs)],
  ]
    .map(([k, v]) => `<tr><th>${k}</th><td>${v ?? "-"}</td></tr>`)
    .join("")
  root.innerHTML = `<table><tbody>${rows}</tbody></table>`
}

const renderRuns = (runs) => {
  cachedRuns = runs
  const root = document.getElementById("runs")
  if (!root) return
  if (!runs.length) {
    root.innerHTML = '<p class="muted">No runs stored in sqlite yet.</p>'
    return
  }

  const rows = runs.map((run) => `
    <tr class="run-row" data-file="${run.file}">
      <td>${run.file}</td>
      <td>${run.mode}</td>
      <td>${run.model}</td>
      <td>${run.totalAttempts}</td>
      <td>${run.passCount}</td>
      <td>${run.errorCount}</td>
      <td>${run.retryCount}</td>
      <td>${formatDuration(run.elapsedTimeMs)}</td>
    </tr>
  `).join("")

  root.innerHTML = `
    <table>
      <thead><tr><th>file</th><th>mode</th><th>model</th><th>total</th><th>pass</th><th>errors</th><th>retries</th><th>elapsed</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
  `
  for (const row of root.querySelectorAll(".run-row")) {
    row.addEventListener("click", () => {
      const file = row.dataset.file
      if (file) showRunDetail(file)
    })
  }
}

const renderLog = (lines) => {
  const root = document.getElementById("log")
  if (!root) return
  root.textContent = lines.join(String.fromCharCode(10))
  root.scrollTop = root.scrollHeight
}

const closeDetailModal = () => {
  const modal = document.getElementById("detail-modal")
  if (modal) modal.classList.remove("active")
}

const showRunDetail = async (fileName) => {
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
    content.innerHTML = `
      <p><strong>File:</strong> ${fileName}</p>
      <p><strong>Mode:</strong> ${report.config?.mode || "unknown"}</p>
      <p><strong>Model:</strong> ${report.model?.label || "unknown"}</p>
      <p><strong>Cases:</strong> ${report.results.length}</p>
    `
  } catch (error) {
    content.innerHTML = `<p class="muted">Error loading details: ${String(error)}</p>`
  }
}

const renderModels = (providers) => {
  const root = document.getElementById("models")
  if (!root) return
  allModels = []
  for (const provider of providers || []) {
    for (const model of provider.models || []) {
      allModels.push({ fullName: `${provider.provider}/${model.id}`, model })
    }
  }

  if (!allModels.length) {
    root.innerHTML = '<p class="muted">No models detected.</p>'
    updateRunControls(null)
    return
  }

  root.innerHTML = `
    <input type="text" id="model-search" class="model-search" placeholder="Search models..." />
    <div id="model-list" class="model-list"></div>
  `

  const renderList = (filter) => {
    const text = String(filter || "").toLowerCase()
    const filtered = text ? allModels.filter((m) => m.fullName.toLowerCase().includes(text)) : allModels
    const listRoot = document.getElementById("model-list")
    if (!listRoot) return
    listRoot.innerHTML = filtered.map((m) => `
      <label class="model-item">
        <input type="radio" name="model" value="${m.fullName}" />
        ${m.fullName}
      </label>
    `).join("")
      for (const el of listRoot.querySelectorAll('input[name="model"]')) {
      el.addEventListener("change", () => updateRunControls(null))
    }
  }

  const search = document.getElementById("model-search")
  if (search) search.addEventListener("input", (e) => renderList(e.target.value))
  renderList("")
  updateRunControls(null)
}

const getJson = async (url, options) => {
  const response = await fetch(url, options)
  const data = await response.json()
  if (!response.ok) throw new Error(data.error || `Request failed: ${response.status}`)
  return data
}

const refresh = async () => {
  try {
    const mode = (document.getElementById("filter-mode")?.value || "").trim()
    const model = (document.getElementById("filter-model")?.value || "").trim()
    const family = (document.getElementById("filter-family")?.value || "").trim()
    const runsUrl = `/api/runs?mode=${encodeURIComponent(mode)}&model=${encodeURIComponent(model)}&family=${encodeURIComponent(family)}`
    const [statusPayload, progressPayload, runsPayload, logPayload] = await Promise.all([
      getJson("/api/status"),
      getJson("/api/progress"),
      getJson(runsUrl),
      getJson("/api/log?tail=200"),
    ])
    renderStatus(statusPayload)
    renderProgressBar(statusPayload, progressPayload.progress || null)
    renderProgress(progressPayload.progress || null)
    renderRuns(runsPayload.runs || [])
    renderLog(logPayload.lines || [])
  } catch (error) {
    const status = document.getElementById("status")
    if (status) status.textContent = `Connection error: ${String(error)}`
  }
}

const loadModels = async () => {
  try {
    const payload = await getJson("/api/models")
    renderModels(payload.providers || [])
  } catch (error) {
    const node = document.getElementById("models")
    if (node) node.innerHTML = `<p class="muted">Loading models... (${String(error)})</p>`
    setTimeout(loadModels, 2000)
  }
}

const startRun = async () => {
  const modelRadio = document.querySelector('input[name="model"]:checked')
  const model = modelRadio ? modelRadio.value : ""
  const modes = checkedValues('input[name="mode"]')
  const sizes = checkedValues('input[name="size"]')
  const repeats = Number.parseInt(document.getElementById("repeats")?.value || "1", 10) || 1

  if (!model) return setRunMessage("Select a model first.")
  if (!modes.length || !sizes.length) return setRunMessage("Select at least one mode and one size.")

  const body = {
    model,
    modes,
    sizes,
    repeats,
    fixtureSet: document.getElementById("fixture-set")?.value || "default",
    randomize: Boolean(document.getElementById("randomize")?.checked),
    seed: Number.parseInt(document.getElementById("seed")?.value || "42", 10) || 42,
    disturbance: Boolean(document.getElementById("disturbance")?.checked),
    disturbanceProbability: Number.parseFloat(document.getElementById("disturbance-probability")?.value || "0.5"),
    enforceProtocol: Boolean(document.getElementById("enforce-protocol")?.checked),
  }
    await getJson("/api/run", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  })
    setRunMessage(`Started run for ${model}`)
  await refresh()
}

const stopRun = async () => {
  const payload = await getJson("/api/run/stop", { method: "POST" })
  setRunMessage(payload.stopped ? "Stop signal sent." : "No active run to stop.")
  await refresh()
}

const rebuildDb = async () => {
  const msg = document.getElementById("admin-msg")
  if (!msg) return
  msg.textContent = "Rebuilding DB..."
  const payload = await getJson("/api/admin/rebuild", { method: "POST" })
  msg.textContent = `DB rebuilt from JSON (${payload.ingested} files).`
  await refresh()
}

const dropDb = async () => {
  const msg = document.getElementById("admin-msg")
  if (!msg) return
  msg.textContent = "Dropping DB..."
  await getJson("/api/admin/drop", { method: "POST" })
  msg.textContent = "DB dropped."
  await refresh()
}

const exportAsCSV = () => {
  if (!cachedRuns.length) return
  const headers = ["file", "mode", "model", "totalAttempts", "passCount", "errorCount", "retryCount", "elapsedTimeMs"]
  const rows = cachedRuns.map((run) => headers.map((h) => JSON.stringify(run[h] ?? "")).join(","))
  const csv = [headers.join(","), ...rows].join(String.fromCharCode(10))
  downloadFile(csv, "benchmark-runs.csv", "text/csv")
}

const exportAsJSON = () => {
  if (!cachedRuns.length) return
  downloadFile(JSON.stringify(cachedRuns, null, 2), "benchmark-runs.json", "application/json")
}

const downloadFile = (content, fileName, mimeType) => {
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

for (const btn of document.querySelectorAll(".tab-btn")) {
  btn.addEventListener("click", () => setTab(btn.dataset.tab))
}
for (const el of document.querySelectorAll('input[name="mode"], input[name="size"]')) {
  el.addEventListener("change", () => updateRunControls(null))
}
const filterMode = document.getElementById("filter-mode")
if (filterMode) filterMode.addEventListener("input", () => refresh())
const filterModel = document.getElementById("filter-model")
if (filterModel) filterModel.addEventListener("input", () => refresh())
const filterFamily = document.getElementById("filter-family")
if (filterFamily) filterFamily.addEventListener("input", () => refresh())
const runBtn = document.getElementById("start-run")
if (runBtn) runBtn.addEventListener("click", () => startRun().catch((e) => setRunMessage(String(e))))
const stopBtn = document.getElementById("stop-run")
if (stopBtn) stopBtn.addEventListener("click", () => stopRun().catch((e) => setRunMessage(String(e))))
const rebuildBtn = document.getElementById("rebuild-db")
if (rebuildBtn) rebuildBtn.addEventListener("click", () => rebuildDb().catch((e) => setRunMessage(String(e))))
const dropBtn = document.getElementById("drop-db")
if (dropBtn) dropBtn.addEventListener("click", () => dropDb().catch((e) => setRunMessage(String(e))))
const exportCSVBtn = document.getElementById("export-csv")
if (exportCSVBtn) exportCSVBtn.addEventListener("click", exportAsCSV)
const exportJSONBtn = document.getElementById("export-json")
if (exportJSONBtn) exportJSONBtn.addEventListener("click", exportAsJSON)
const detailModalClose = document.getElementById("detail-modal-close")
if (detailModalClose) detailModalClose.addEventListener("click", closeDetailModal)
const detailModal = document.getElementById("detail-modal")
if (detailModal) detailModal.addEventListener("click", (event) => {
  if (event.target === detailModal) closeDetailModal()
})

const startAutoRefresh = () => {
  if (refreshInterval) return
  refreshInterval = setInterval(() => {
    const checkbox = document.getElementById("auto-refresh")
    if (checkbox && checkbox.checked) refresh()
  }, 2000)
}

const stopAutoRefresh = () => {
  if (!refreshInterval) return
  clearInterval(refreshInterval)
  refreshInterval = null
}
const autoRefreshCheckbox = document.getElementById("auto-refresh")
if (autoRefreshCheckbox) {
  autoRefreshCheckbox.addEventListener("change", (event) => {
    if (event.target.checked) {
      startAutoRefresh()
      refresh()
    } else {
      stopAutoRefresh()
    }
  })
}
loadModels()
refresh()
startAutoRefresh()

import { spawnSync, spawn } from "node:child_process"

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    stdio: "inherit",
    shell: false,
    ...options,
  })
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status ?? "unknown"}`)
  }
}

async function waitFor(url, timeoutMs) {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(url)
      if (res.ok) return
    } catch {
      // retry
    }
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error(`Timed out waiting for ${url}`)
}

async function smokeServer() {
  const port = Number(process.env.BENCH_TEST_PORT || 4277)
  const child = spawn("node", ["server.js"], {
    env: { ...process.env, PORT: String(port) },
    stdio: ["ignore", "pipe", "pipe"],
  })

  let stderr = ""
  child.stderr.on("data", (chunk) => {
    stderr += String(chunk)
  })

  try {
    await waitFor(`http://127.0.0.1:${port}/api/status`, 10000)

    const statusRes = await fetch(`http://127.0.0.1:${port}/api/status`)
    if (!statusRes.ok) throw new Error(`status endpoint failed: ${statusRes.status}`)

    const progressRes = await fetch(`http://127.0.0.1:${port}/api/progress`)
    if (!progressRes.ok) throw new Error(`progress endpoint failed: ${progressRes.status}`)

    const runsRes = await fetch(`http://127.0.0.1:${port}/api/runs`)
    if (!runsRes.ok) throw new Error(`runs endpoint failed: ${runsRes.status}`)
  } finally {
    child.kill("SIGTERM")
    await new Promise((resolve) => child.once("exit", resolve))
    if (stderr.trim().length > 0) {
      process.stderr.write(stderr)
    }
  }
}

async function main() {
  run("npm", ["run", "build"])
  run("npm", ["run", "run", "--", "--help"])
  await smokeServer()
}

main().catch((error) => {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`)
  process.exit(1)
})

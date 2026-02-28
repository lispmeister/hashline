import { execSync, spawn } from "node:child_process"
import path from "node:path"

function pidsListening(port: number): number[] {
  try {
    const out = execSync(`lsof -nP -iTCP:${port} -sTCP:LISTEN -t`, { encoding: "utf8" }).trim()
    if (!out) return []
    return out
      .split("\n")
      .map((line) => Number.parseInt(line.trim(), 10))
      .filter((n) => Number.isInteger(n) && n > 0)
  } catch {
    return []
  }
}

function commandForPid(pid: number): string {
  try {
    return execSync(`ps -p ${pid} -o command=`, { encoding: "utf8" }).trim()
  } catch {
    return ""
  }
}

function stopDashboardIfRunning(): { restarted: boolean } {
  const pids = pidsListening(4177)
  if (pids.length === 0) return { restarted: false }

  for (const pid of pids) {
    const cmd = commandForPid(pid)
    if (cmd.includes("node server.js") || cmd.includes("opencode-sdk-bench/server.js")) {
      process.stdout.write(`Stopping dashboard process ${pid} before benchmark run\n`)
      try {
        process.kill(pid, "SIGTERM")
      } catch {
        // ignore
      }
    }
  }

  return { restarted: true }
}

function ensureOpencodePortFree(): void {
  const pids = pidsListening(4096)
  if (pids.length === 0) return

  const detail = pids.map((pid) => `${pid}:${commandForPid(pid)}`).join(" | ")
  throw new Error(`OpenCode server port 4096 is busy. Stop existing process and retry. ${detail}`)
}

function runCli(args: string[], cwd: string): Promise<number> {
  const tsxBin = path.join(cwd, "node_modules", ".bin", "tsx")
  return new Promise((resolve, reject) => {
    const child = spawn(tsxBin, ["src/cli.ts", ...args], {
      cwd,
      stdio: "inherit",
      env: process.env,
    })

    child.on("exit", (code) => resolve(code ?? 1))
    child.on("error", reject)
  })
}

function restartDashboard(cwd: string): void {
  process.stdout.write("Restarting dashboard on port 4177\n")
  const child = spawn("node", ["server.js"], {
    cwd,
    detached: true,
    stdio: "ignore",
    env: process.env,
  })
  child.unref()
}

async function main(): Promise<void> {
  const cwd = process.cwd()
  const args = process.argv.slice(2)

  const dashboard = stopDashboardIfRunning()
  ensureOpencodePortFree()

  try {
    const code = await runCli(args, cwd)
    process.exitCode = code
  } finally {
    if (dashboard.restarted) {
      restartDashboard(cwd)
    }
  }
}

void main()

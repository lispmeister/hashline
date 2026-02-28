import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import type { EventRecord } from "./types";

function defaultLogPath(): string {
  if (process.platform === "win32") {
    const appData = process.env.APPDATA;
    if (!appData) return "";
    return path.join(appData, "hashline", "usage.log");
  }
  return path.join(os.homedir(), ".local", "state", "hashline", "usage.log");
}

export function readHashlineLogTail(lines = 500): EventRecord[] {
  const logPath = process.env.HASHLINE_USAGE_LOG || defaultLogPath();
  if (!logPath || !fs.existsSync(logPath)) return [];

  const raw = fs.readFileSync(logPath, "utf8");
  const entries = raw.trim().split("\n").slice(-lines);

  return entries.map((line) => {
    const lower = line.toLowerCase();
    const isError = lower.includes(",1,") || lower.includes(",2,") || lower.includes("error");
    const isRetry = lower.includes("hash mismatch") || lower.includes("retry");

    return {
      timestamp: Date.now(),
      type: isError ? "hashline_log:error" : isRetry ? "hashline_log:retry" : "hashline_log:event",
      message: line,
      raw: line,
    };
  });
}

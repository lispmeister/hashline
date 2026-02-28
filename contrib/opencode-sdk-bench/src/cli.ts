import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { runBenchmark } from "./runner";
import type { BenchmarkConfig, BenchmarkMode, FixtureSize } from "./types";

interface CliOptions {
  mode: BenchmarkMode;
  sizes: FixtureSize[];
  repeats: number;
  model?: string;
  progressPath?: string;
}

type ProgressStatus = "running" | "done" | "error";

interface ProgressState {
  status: ProgressStatus;
  mode: BenchmarkMode;
  model: string;
  currentCase: number;
  totalCases: number;
  currentAttempt: number;
  attemptsPerCase: number;
  completedAttempts: number;
  totalAttempts: number;
  passCount: number;
  errorCount: number;
  retryCount: number;
  elapsedTimeMs: number;
  totalCost: number;
  startedAt: string;
  caseId?: string;
  size?: FixtureSize;
  reportPath?: string;
  error?: string;
}

const ALL_SIZES: FixtureSize[] = ["small", "mid", "large"];
const VALID_MODES: BenchmarkMode[] = ["hashline", "raw_replace", "patch"];

function sanitizeFilenamePart(input: string): string {
  return input.replace(/[^a-zA-Z0-9._-]/g, "_");
}

function runTimestampForFile(): string {
  return new Date().toISOString().replace(/[:.]/g, "-");
}

function parseSizes(raw: string): FixtureSize[] {
  const parsed = raw
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item.length > 0);

  if (parsed.length === 0) {
    throw new Error("--sizes requires at least one value");
  }

  const invalid = parsed.filter((size) => !ALL_SIZES.includes(size as FixtureSize));
  if (invalid.length > 0) {
    throw new Error(`Invalid size(s): ${invalid.join(", ")}. Expected small,mid,large.`);
  }

  return [...new Set(parsed)] as FixtureSize[];
}

function requireNext(argv: string[], index: number, flag: string): string {
  const value = argv[index + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`Missing value for ${flag}`);
  }
  return value;
}

function parseArgs(argv: string[]): CliOptions {
  let mode: BenchmarkMode = "hashline";
  let sizes: FixtureSize[] = [...ALL_SIZES];
  let repeats = 1;
  let model: string | undefined;
  let progressPath: string | undefined;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];

    if (arg === "--mode") {
      const value = requireNext(argv, i, "--mode");
      if (!VALID_MODES.includes(value as BenchmarkMode)) {
        throw new Error(`Invalid --mode '${value}'. Expected hashline|raw_replace|patch.`);
      }
      mode = value as BenchmarkMode;
      i += 1;
      continue;
    }

    if (arg === "--sizes") {
      sizes = parseSizes(requireNext(argv, i, "--sizes"));
      i += 1;
      continue;
    }

    if (arg === "--repeats") {
      const raw = requireNext(argv, i, "--repeats");
      const parsed = Number.parseInt(raw, 10);
      if (!Number.isInteger(parsed) || parsed < 1) {
        throw new Error(`Invalid --repeats '${raw}'. Expected integer >= 1.`);
      }
      repeats = parsed;
      i += 1;
      continue;
    }

    if (arg === "--model") {
      model = requireNext(argv, i, "--model");
      i += 1;
      continue;
    }

    if (arg === "--progress") {
      progressPath = requireNext(argv, i, "--progress");
      i += 1;
      continue;
    }

    if (arg === "--help" || arg === "-h") {
      throw new Error("HELP");
    }

    throw new Error(`Unknown argument: ${arg}`);
  }

  return { mode, sizes, repeats, model, progressPath };
}

function writeJson(filePath: string, data: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(data, null, 2)}\n`, "utf8");
}

function usage(): string {
  return [
    "Usage: tsx contrib/opencode-sdk-bench/src/cli.ts [options]",
    "",
    "Options:",
    "  --mode <hashline|raw_replace|patch>",
    "  --sizes <small,mid,large>",
    "  --repeats <n>",
    "  --model <provider/model>",
    "  --progress <path>",
  ].join("\n");
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  let opts: CliOptions;

  try {
    opts = parseArgs(args);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message === "HELP") {
      process.stdout.write(`${usage()}\n`);
      return;
    }
    process.stderr.write(`${message}\n\n${usage()}\n`);
    process.exitCode = 1;
    return;
  }

  const sourceDir = path.dirname(fileURLToPath(import.meta.url));
  const benchDir = path.resolve(sourceDir, "..");
  const runsDir = path.join(benchDir, "runs");
  const progressPath = opts.progressPath ? path.resolve(process.cwd(), opts.progressPath) : undefined;
  const modelLabel = opts.model ?? "auto";

  const config: BenchmarkConfig = {
    workspaceDir: benchDir,
    fixturesDir: path.join(benchDir, "fixtures"),
    runsDir,
    mode: opts.mode,
    repeats: opts.repeats,
    sizes: opts.sizes,
    model: opts.model,
  };

  const startedAt = new Date().toISOString();
  let passCount = 0;
  let errorCount = 0;
  let retryCount = 0;
  let totalCost = 0;

  try {
    const report = await runBenchmark(config, {
      onAttemptComplete: async (progress) => {
        passCount += progress.result.passed ? 1 : 0;
        errorCount += progress.result.metrics.errorCount;
        retryCount += progress.result.metrics.retries;
        totalCost += progress.result.metrics.cost ?? 0;

        if (!progressPath) return;

        const totalAttempts = progress.caseTotal * progress.attemptTotal;
        const completedAttempts = (progress.caseIndex - 1) * progress.attemptTotal + progress.attempt;

        const state: ProgressState = {
          status: "running",
          mode: opts.mode,
          model: modelLabel,
          currentCase: progress.caseIndex,
          totalCases: progress.caseTotal,
          currentAttempt: progress.attempt,
          attemptsPerCase: progress.attemptTotal,
          completedAttempts,
          totalAttempts,
          passCount,
          errorCount,
          retryCount,
          totalCost,
          elapsedTimeMs: Date.now() - Date.parse(startedAt),
          startedAt,
          caseId: progress.fixture.caseId,
          size: progress.fixture.size,
        };

        writeJson(progressPath, state);
      },
    });

    const timestamp = runTimestampForFile();
    const reportModel = sanitizeFilenamePart(report.model.label);
    const reportName = `${timestamp}-${opts.mode}-${reportModel}.json`;
    const reportPath = path.join(runsDir, reportName);

    writeJson(reportPath, report);

    if (progressPath) {
      const doneState: ProgressState = {
        status: "done",
        mode: opts.mode,
        model: report.model.label,
        currentCase: report.results.length,
        totalCases: report.results.length,
        currentAttempt: opts.repeats,
        attemptsPerCase: opts.repeats,
        completedAttempts: report.results.length * opts.repeats,
        totalAttempts: report.results.length * opts.repeats,
        passCount,
        errorCount,
        retryCount,
        totalCost,
        elapsedTimeMs: Date.now() - Date.parse(startedAt),
        startedAt,
        reportPath,
      };

      writeJson(progressPath, doneState);
    }

    process.stdout.write(`${reportPath}\n`);
  } catch (error) {
    if (progressPath) {
      const state: ProgressState = {
        status: "error",
        mode: opts.mode,
        model: modelLabel,
        currentCase: 0,
        totalCases: 0,
        currentAttempt: 0,
        attemptsPerCase: opts.repeats,
        completedAttempts: 0,
        totalAttempts: 0,
        passCount,
        errorCount,
        retryCount,
        totalCost,
        elapsedTimeMs: Date.now() - Date.parse(startedAt),
        startedAt,
        error: error instanceof Error ? error.message : String(error),
      };

      writeJson(progressPath, state);
    }

    throw error;
  }
}

void main();

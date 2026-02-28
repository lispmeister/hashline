export type BenchmarkMode = "hashline" | "raw_replace" | "patch";

export type FixtureSize = "small" | "mid" | "large";

export type FixtureCase =
  | "markdown_text"
  | "markdown_embedded_json"
  | "markdown_embedded_typescript"
  | "typescript"
  | "typescript_embedded_json"
  | "json"
  | "rust"
  | "rust_embedded_json"
  | "polyglot_complex";

export interface BenchmarkConfig {
  workspaceDir: string;
  fixturesDir: string;
  runsDir: string;
  mode: BenchmarkMode;
  repeats: number;
  sizes: FixtureSize[];
  model?: string;
}

export interface ModelRef {
  providerID: string;
  modelID: string;
  label: string;
}

export interface FixtureSpec {
  size: FixtureSize;
  caseId: FixtureCase;
  extension: string;
  basePath: string;
  originalPath: string;
  mutatedPath: string;
  taskPath: string;
}

export interface EventRecord {
  timestamp: number;
  type: string;
  sessionID?: string;
  message?: string;
  raw: unknown;
}

export interface AttemptMetrics {
  durationMs: number;
  retries: number;
  errorCount: number;
  tokenInput?: number;
  tokenOutput?: number;
  tokenReasoning?: number;
  tokenTotal?: number;
  cost?: number;
}

export interface AttemptResult {
  attempt: number;
  passed: boolean;
  reason: string;
  metrics: AttemptMetrics;
  sessionID: string;
  events: EventRecord[];
}

export interface CaseResult {
  size: FixtureSize;
  caseId: FixtureCase;
  mode: BenchmarkMode;
  model: ModelRef;
  attempts: AttemptResult[];
}

export interface RunReport {
  startedAt: string;
  finishedAt: string;
  config: BenchmarkConfig;
  model: ModelRef;
  results: CaseResult[];
}

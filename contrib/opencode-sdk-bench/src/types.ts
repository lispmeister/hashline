export type BenchmarkMode = "hashline" | "raw_replace" | "patch";
export type FixtureSize = "small" | "mid" | "large";

export type FixtureSet = "default" | "holdout" | "all";

export type ScenarioFamily =
  | "ambiguity"
  | "json_migration"
  | "whitespace"
  | "scope"
  | "array_targeting"
  | "multiformat"
  | "stale_context"
  | "large_file"
  | "confusable_chars"
  | "refactor"
  | "other";
export interface BenchmarkConfig {
  workspaceDir: string;
  fixturesDir: string;
  runsDir: string;
  mode: BenchmarkMode;
  repeats: number;
  sizes: FixtureSize[];
  model?: string;
  disturbance?: boolean;
  disturbanceProbability?: number;
  fixtureSet?: FixtureSet;
  randomize?: boolean;
  seed?: number;
  enforceProtocol?: boolean;
}
export interface ModelRef {
  providerID: string;
  modelID: string;
  label: string;
}
export interface FixtureSpec {
  size: FixtureSize;
  caseId: string;
  family: ScenarioFamily;
  extension: string;
  basePath: string;
  originalPath: string;
  mutatedPath: string;
  taskPath: string;
  tags: string[];
}
export interface EventRecord {
  timestamp: number;
  type: string;
  sessionID?: string;
  message?: string;
  command?: string;
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
  changedLines?: number;
  expectedChangedLines?: number;
  unexpectedChangedLines?: number;
}
export interface AttemptResult {
  attempt: number;
  passed: boolean;
  reason: string;
  taskPassed: boolean;
  protocolPassed: boolean;
  overallPassed: boolean;
  protocolFailureReasons: string[];
  corruptionDetected: boolean;
  corruptionReason?: string;
  metrics: AttemptMetrics;
  sessionID: string;
  events: EventRecord[];
}
export interface CaseResult {
  size: FixtureSize;
  caseId: string;
  family: ScenarioFamily;
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


# Known Issues (2026-03-04)

## 1) Benchmark-metric expansion branch is unstable

Recent edits for protocol/safety metric expansion are not fully stabilized yet:
- `contrib/opencode-sdk-bench/src/cli.ts`
- `contrib/opencode-sdk-bench/src/runner.ts`
- `contrib/opencode-sdk-bench/web/app.js`

Impact:
- Build and dashboard behavior may be unreliable until syntax/runtime stabilization is completed.

## 2) OpenCode permission prompts can still block unattended runs

Some environments still trigger interactive permission prompts from the OpenCode stack.

Symptoms:
- run status remains active without progress,
- no meaningful attempt progression,
- logs indicate permission requests.

Mitigations:
- pre-grant required permissions in an interactive setup pass,
- ensure non-interactive/headless settings are configured where supported,
- verify model/provider configuration before long benchmark sweeps.


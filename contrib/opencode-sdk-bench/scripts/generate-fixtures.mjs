#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const outDir = path.join(root, "contrib", "opencode-sdk-bench", "fixtures");

const sizes = {
  small: 1,
  mid: 6,
  large: 30,
};

function writeFile(filePath, content) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content, "utf8");
}

function repeatBlock(block, n) {
  return Array.from({ length: n }, () => block).join("\n");
}

function markdownText(mult) {
  const block = `## Release Note\n\nHashline uses anchors for deterministic edits.\n\n- keep atomic apply\n- keep retry diagnostics\n- keep JSON-aware edits\n`;
  const original = `# Changelog Draft\n\n${repeatBlock(block, mult)}\nFinal status: PASS`;
  const mutated = original.replace("Final status: PASS", "Final status: FAIL");
  const task = "Fix the file by restoring the final status line to `Final status: PASS`.";
  return { ext: "md", original, mutated, task };
}

function markdownEmbeddedJson(mult) {
  const jsonBlock = '{\n  "feature": "hashline",\n  "enabled": true,\n  "maxRetries": 2\n}';
  const original = `# SDK Config Note\n\nUse the following payload:\n\n\`\`\`json\n${jsonBlock}\n\`\`\`\n\n${repeatBlock("This block is informational.", mult)}`;
  const mutated = original.replace('"enabled": true', '"enabled": false');
  const task = "Inside the embedded JSON code block, restore `\"enabled\": true`.";
  return { ext: "md", original, mutated, task };
}

function markdownEmbeddedTs(mult) {
  const tsBlock = "function shouldRun(flag: boolean): boolean {\n  return flag;\n}";
  const original = `# Runner Snippet\n\n\`\`\`ts\n${tsBlock}\n\`\`\`\n\n${repeatBlock("The snippet is used by docs and tests.", mult)}`;
  const mutated = original.replace("return flag;", "return !flag;");
  const task = "Fix the embedded TypeScript snippet so `shouldRun` returns `flag` again.";
  return { ext: "md", original, mutated, task };
}

function typescriptCase(mult) {
  const block = `export function sum(values: number[]): number {\n  return values.reduce((acc, v) => acc + v, 0);\n}\n`;
  const original = `${repeatBlock(block, mult)}\nexport function isReady(input: boolean): boolean {\n  return input;\n}\n`;
  const mutated = original.replace("return input;", "return !input;");
  const task = "Restore `isReady` so it returns `input` without negation.";
  return { ext: "ts", original, mutated, task };
}

function typescriptEmbeddedJson(mult) {
  const original = `const configJson = String.raw\`{\n  \"mode\": \"strict\",\n  \"allowWrites\": true,\n  \"retries\": 3\n}\`;\n\n${repeatBlock("export const marker = \"ts-json\";", mult)}\n`;
  const mutated = original.replace('\"allowWrites\": true', '\"allowWrites\": false');
  const task = "Inside the JSON string literal, restore `\"allowWrites\": true`.";
  return { ext: "ts", original, mutated, task };
}

function jsonCase(mult) {
  const items = Array.from({ length: Math.max(3, mult * 4) }, (_, i) => ({
    id: i + 1,
    enabled: true,
    name: `item-${i + 1}`,
  }));
  const originalObj = {
    version: "1.0.0",
    defaults: { safeMode: true, maxRetries: 2 },
    items,
  };
  const mutatedObj = structuredClone(originalObj);
  mutatedObj.defaults.safeMode = false;
  const original = `${JSON.stringify(originalObj, null, 2)}\n`;
  const mutated = `${JSON.stringify(mutatedObj, null, 2)}\n`;
  const task = "Restore `defaults.safeMode` back to `true`.";
  return { ext: "json", original, mutated, task };
}

function rustCase(mult) {
  const block = `fn normalize(v: i32) -> i32 {\n    if v < 0 {\n        return 0;\n    }\n    v\n}\n`;
  const original = `${repeatBlock(block, mult)}\nfn should_log(flag: bool) -> bool {\n    flag\n}\n`;
  const mutated = original.replace("    flag", "    !flag");
  const task = "Fix `should_log` so it returns `flag` and not `!flag`.";
  return { ext: "rs", original, mutated, task };
}

function rustEmbeddedJson(mult) {
  const original = `const RAW: &str = r#"{\n  \"pipeline\": \"bench\",\n  \"emitUpdated\": true,\n  \"attempts\": 3\n}"#;\n\n${repeatBlock("fn noop() {}", mult)}\n`;
  const mutated = original.replace('"emitUpdated": true', '"emitUpdated": false');
  const task = "In the embedded JSON string, restore `\"emitUpdated\": true`.";
  return { ext: "rs", original, mutated, task };
}

function polyglotComplex(mult) {
  const original = `# Polyglot Scenario\n\n## JSON block\n\n\`\`\`json\n{\n  \"enabled\": true,\n  \"limit\": 5\n}\n\`\`\`\n\n## TypeScript block\n\n\`\`\`ts\nexport function choose(v: number) {\n  return v > 0;\n}\n\`\`\`\n\n## Rust block\n\n\`\`\`rust\nfn ok(flag: bool) -> bool {\n    flag\n}\n\`\`\`\n\n${repeatBlock("Context paragraph for parser stress.", mult)}\n`;
  const mutated = original
    .replace('"enabled": true', '"enabled": false')
    .replace("return v > 0;", "return v < 0;")
    .replace("    flag", "    !flag");
  const task = "Restore the three regressions: JSON `enabled: true`, TS condition `v > 0`, and Rust return `flag`.";
  return { ext: "md", original, mutated, task };
}

const cases = {
  markdown_text: markdownText,
  markdown_embedded_json: markdownEmbeddedJson,
  markdown_embedded_typescript: markdownEmbeddedTs,
  typescript: typescriptCase,
  typescript_embedded_json: typescriptEmbeddedJson,
  json: jsonCase,
  rust: rustCase,
  rust_embedded_json: rustEmbeddedJson,
  polyglot_complex: polyglotComplex,
};

for (const [size, mult] of Object.entries(sizes)) {
  for (const [caseId, fn] of Object.entries(cases)) {
    const sample = fn(mult);
    const base = path.join(outDir, size, caseId);
    writeFile(path.join(base, `original.${sample.ext}`), sample.original);
    writeFile(path.join(base, `mutated.${sample.ext}`), sample.mutated);
    writeFile(path.join(base, "task.md"), `${sample.task}\n`);
  }
}

console.log(`Generated fixtures in ${outDir}`);

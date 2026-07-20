const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const outputDir = path.join(
  repoRoot,
  "docs",
  "reports",
  "evidence",
  "frames",
  "v205-conpty",
);
const scenarioScript = path.join(__dirname, "capture-scenario.js");
const modes = [
  "responsive",
  "decline",
  "approve",
  "cancel",
  "nonzero",
  "invalid",
  "binary",
  "long",
];

fs.mkdirSync(outputDir, { recursive: true });

for (const mode of modes) {
  const result = spawnSync(process.execPath, [scenarioScript, mode], {
    cwd: repoRoot,
    env: process.env,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    throw new Error(`ConPTY scenario failed: ${mode} (exit ${result.status})`);
  }
}

const scenarios = modes.map((mode) => {
  const filename = `${mode}.json`;
  const fullPath = path.join(outputDir, filename);
  const raw = fs.readFileSync(fullPath);
  const evidence = JSON.parse(raw.toString("utf8"));
  if (evidence.error) {
    throw new Error(`ConPTY evidence contains an error: ${mode}: ${evidence.error}`);
  }
  if (!evidence.checkpoints.every((checkpoint) => checkpoint.matched !== false)) {
    throw new Error(`ConPTY evidence contains an unmatched checkpoint: ${mode}`);
  }
  return {
    mode,
    file: filename,
    sha256: crypto.createHash("sha256").update(raw).digest("hex"),
    output_bytes: evidence.output_bytes,
    checkpoint_count: evidence.checkpoints.length,
    started_at: evidence.checkpoints[0]?.timestamp ?? null,
    finished_at: evidence.checkpoints.at(-1)?.timestamp ?? null,
  };
});

const manifest = {
  schema_version: 1,
  generated_at: new Date().toISOString(),
  version: "2.0.5",
  provider: "DeepSeek",
  model: "deepseek-chat",
  terminal: "Windows ConPTY",
  collector: "scripts/conpty/v205/capture.js",
  scenarios,
};
const manifestPath = path.join(outputDir, "manifest.json");
fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

const persisted = fs.readFileSync(manifestPath, "utf8");
const forbidden = [
  /github_pat_[A-Za-z0-9_]+/,
  /gh[pousr]_[A-Za-z0-9_]+/,
  /Bearer\s+[A-Za-z0-9._-]+/i,
  /\bsk-[A-Za-z0-9_-]+/,
];
for (const pattern of forbidden) {
  if (pattern.test(persisted)) {
    throw new Error(`Secret-like value reached manifest: ${pattern}`);
  }
}

console.log(JSON.stringify({ ok: true, manifest: manifestPath, scenarios }));

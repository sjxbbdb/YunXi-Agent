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
  "v208-conpty",
);
const scenarioScript = path.join(__dirname, "capture-scenario.js");
const manifestOnly = process.argv.includes("--manifest-only");
const modes = ["responsive-density", "semantic-low-color"];

fs.mkdirSync(outputDir, { recursive: true });

if (!manifestOnly) {
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
}

const scenarios = modes.map((mode) => {
  const file = `${mode}.json`;
  const fullPath = path.join(outputDir, file);
  const raw = fs.readFileSync(fullPath);
  const evidence = JSON.parse(raw.toString("utf8"));
  if (evidence.error) {
    throw new Error(`scenario contains error: ${mode}: ${evidence.error}`);
  }
  if (!evidence.checkpoints.every((checkpoint) => checkpoint.matched !== false)) {
    throw new Error(`scenario contains unmatched checkpoint: ${mode}`);
  }
  return {
    mode,
    file,
    sha256: crypto.createHash("sha256").update(raw).digest("hex"),
    output_bytes: evidence.output_bytes,
    checkpoint_count: evidence.checkpoints.length,
    started_at: evidence.checkpoints[0]?.timestamp ?? null,
    finished_at: evidence.checkpoints.at(-1)?.timestamp ?? null,
  };
});

const manifest = {
  schema_version: 3,
  generated_at: new Date().toISOString(),
  version: "2.0.8",
  provider: "DeepSeek",
  model: "deepseek-chat",
  terminal: "Windows ConPTY",
  collector: "scripts/conpty/v208/capture.js",
  scenarios,
};
const manifestPath = path.join(outputDir, "manifest.json");
fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
assertNoSecrets(fs.readFileSync(manifestPath, "utf8"), "manifest");

console.log(JSON.stringify({ ok: true, manifest: manifestPath, scenarios }));

function assertNoSecrets(text, source) {
  for (const pattern of [
    /github_pat_[A-Za-z0-9_]+/,
    /gh[pousr]_[A-Za-z0-9_]+/,
    /Bearer\s+[A-Za-z0-9._-]+/i,
    /\bsk-[A-Za-z0-9_-]+/,
  ]) {
    if (pattern.test(text)) {
      throw new Error(`secret-like value found in ${source}: ${pattern}`);
    }
  }
}

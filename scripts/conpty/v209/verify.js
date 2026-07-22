const crypto = require("crypto");
const fs = require("fs");
const path = require("path");

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const inputDir = path.resolve(
  optionValue("--input-dir") ||
    path.join(repoRoot, "docs", "reports", "evidence", "frames", "v209-conpty"),
);
const forbiddenSecrets = [
  /github_pat_[A-Za-z0-9_]+/,
  /gh[pousr]_[A-Za-z0-9_]+/,
  /Bearer\s+[A-Za-z0-9._-]+/i,
  /\bsk-[A-Za-z0-9_-]+/,
];

function optionValue(name) {
  const index = process.argv.indexOf(name);
  if (index < 0) return undefined;
  const value = process.argv[index + 1];
  if (!value || value.startsWith("--")) throw new Error(`${name} requires a path`);
  return value;
}

function sha256(raw) {
  return crypto.createHash("sha256").update(raw).digest("hex");
}

function assertNoSecrets(text, source) {
  for (const pattern of forbiddenSecrets) {
    if (pattern.test(text)) throw new Error(`secret-like value found in ${source}: ${pattern}`);
  }
}

function directoryFingerprint(directory) {
  return fs
    .readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => {
      const raw = fs.readFileSync(path.join(directory, entry.name));
      return `${entry.name}:${raw.length}:${sha256(raw)}`;
    })
    .sort()
    .join("\n");
}

function verify() {
  const before = directoryFingerprint(inputDir);
  const manifestPath = path.join(inputDir, "manifest.json");
  const manifestRaw = fs.readFileSync(manifestPath);
  const manifest = JSON.parse(manifestRaw.toString("utf8"));
  if (
    manifest.schema_version !== 1 ||
    manifest.version !== "2.0.9" ||
    manifest.terminal !== "Windows ConPTY" ||
    manifest.collector !== "scripts/conpty/v209/capture.js" ||
    manifest.evidence !== "resilience.json"
  ) {
    throw new Error("v2.0.9 manifest metadata is invalid");
  }
  assertNoSecrets(manifestRaw.toString("utf8"), "manifest.json");

  const evidencePath = path.join(inputDir, manifest.evidence);
  const evidenceRaw = fs.readFileSync(evidencePath);
  if (sha256(evidenceRaw) !== manifest.sha256) {
    throw new Error("v2.0.9 evidence SHA-256 mismatch");
  }
  const evidenceText = evidenceRaw.toString("utf8");
  assertNoSecrets(evidenceText, manifest.evidence);
  const evidence = JSON.parse(evidenceText);
  if (
    evidence.schema_version !== 1 ||
    evidence.version !== "2.0.9" ||
    evidence.terminal !== "Windows ConPTY"
  ) {
    throw new Error("v2.0.9 evidence metadata is invalid");
  }

  const expectedModes = [
    "plain-one-shot",
    "plain-pipe",
    "ci",
    "no-tui",
    "forced-tui-fallback",
    "json",
    "jsonl",
  ];
  if (JSON.stringify(evidence.mode_matrix.map((entry) => entry.label)) !== JSON.stringify(expectedModes)) {
    throw new Error("v2.0.9 mode matrix is incomplete or reordered");
  }
  if (!evidence.mode_matrix.every((entry) => entry.status === 0 && entry.stdout_bytes > 0)) {
    throw new Error("v2.0.9 mode matrix contains a failed path");
  }
  if (
    JSON.stringify(evidence.terminal_recovery.map((entry) => entry.label)) !==
      JSON.stringify(["normal-exit", "ctrl-c-exit"]) ||
    !evidence.terminal_recovery.every(
      (entry) => entry.exit_code === 0 && entry.output_bytes > 0 && entry.focus_disable_action_unit_tested,
    )
  ) {
    throw new Error("v2.0.9 terminal recovery evidence is invalid");
  }
  if (
    evidence.provider_recovery?.requests < 2 ||
    evidence.provider_recovery?.exit_code !== 0 ||
    evidence.provider_recovery?.recovered !== true
  ) {
    throw new Error("v2.0.9 provider recovery evidence is invalid");
  }
  if (
    evidence.stream_cancellation?.requests < 2 ||
    evidence.stream_cancellation?.streamed_bytes < 300 * 1024 ||
    evidence.stream_cancellation?.exit_code !== 0 ||
    evidence.stream_cancellation?.recovered !== true
  ) {
    throw new Error("v2.0.9 stream cancellation evidence is invalid");
  }
  const after = directoryFingerprint(inputDir);
  if (after !== before) throw new Error("read-only verifier changed formal evidence");
  return { ok: true, read_only: true, input: inputDir, sha256: manifest.sha256 };
}

console.log(JSON.stringify(verify()));

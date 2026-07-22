const crypto = require("crypto");
const fs = require("fs");
const path = require("path");

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const inputDir = path.resolve(
  optionValue("--input-dir") ||
    path.join(repoRoot, "docs", "reports", "evidence", "frames", "v210-conpty"),
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
  const manifestRaw = fs.readFileSync(path.join(inputDir, "manifest.json"));
  const manifest = JSON.parse(manifestRaw.toString("utf8"));
  if (
    manifest.schema_version !== 1 ||
    manifest.version !== "2.1.0" ||
    manifest.terminal !== "Windows ConPTY" ||
    manifest.collector !== "scripts/conpty/v210/capture.js" ||
    manifest.evidence !== "integrated.json"
  ) {
    throw new Error("v2.1.0 manifest metadata is invalid");
  }
  assertNoSecrets(manifestRaw.toString("utf8"), "manifest.json");
  const evidenceRaw = fs.readFileSync(path.join(inputDir, manifest.evidence));
  if (sha256(evidenceRaw) !== manifest.sha256) throw new Error("v2.1.0 evidence SHA-256 mismatch");
  const evidenceText = evidenceRaw.toString("utf8");
  assertNoSecrets(evidenceText, manifest.evidence);
  const evidence = JSON.parse(evidenceText);
  if (
    evidence.schema_version !== 1 ||
    evidence.version !== "2.1.0" ||
    evidence.terminal !== "Windows ConPTY" ||
    evidence.baseline?.version !== "2.1.0"
  ) {
    throw new Error("v2.1.0 evidence metadata is invalid");
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
  if (
    JSON.stringify(evidence.baseline.mode_matrix.map((entry) => entry.label)) !==
      JSON.stringify(expectedModes) ||
    !evidence.baseline.mode_matrix.every((entry) => entry.status === 0 && entry.stdout_bytes > 0)
  ) {
    throw new Error("v2.1.0 non-TUI mode matrix is invalid");
  }
  if (
    !evidence.baseline.terminal_recovery.every((entry) => entry.exit_code === 0) ||
    evidence.baseline.provider_recovery?.recovered !== true ||
    evidence.baseline.stream_cancellation?.recovered !== true ||
    evidence.baseline.stream_cancellation?.streamed_bytes < 300 * 1024
  ) {
    throw new Error("v2.1.0 baseline recovery contract is invalid");
  }
  const smoke = evidence.integrated_smoke;
  if (
    smoke?.exit_code !== 0 ||
    smoke?.wide_characters_visible !== true ||
    smoke?.copy_boundary_quiet !== true ||
    smoke?.mouse_sequences_sent < 2 ||
    smoke?.resized_cols !== 58 ||
    smoke?.resized_rows !== 18 ||
    smoke?.narrow_footer_visible !== true ||
    smoke?.ansi_reset_action_unit_tested !== true ||
    smoke?.alternate_screen_restored !== true ||
    smoke?.cursor_restored !== true
  ) {
    throw new Error("v2.1.0 integrated smoke evidence is invalid");
  }
  for (const golden of evidence.goldens) {
    const raw = fs.readFileSync(path.join(repoRoot, ...golden.path.split("/")));
    if (raw.length !== golden.bytes || sha256(raw) !== golden.sha256) {
      throw new Error(`golden mismatch: ${golden.path}`);
    }
    if (
      golden.path.endsWith("vt100_lifecycle_v210.txt") &&
      !raw.toString("utf8").includes("<ESC>[0m")
    ) {
      throw new Error("VT100 golden does not prove the ANSI reset action");
    }
  }
  const after = directoryFingerprint(inputDir);
  if (after !== before) throw new Error("read-only verifier changed formal evidence");
  return { ok: true, read_only: true, input: inputDir, sha256: manifest.sha256 };
}

console.log(JSON.stringify(verify()));

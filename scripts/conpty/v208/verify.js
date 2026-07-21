const crypto = require("crypto");
const fs = require("fs");
const path = require("path");

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const outputDir = path.join(
  repoRoot,
  "docs",
  "reports",
  "evidence",
  "frames",
  "v208-conpty",
);
const manifestPath = path.join(outputDir, "manifest.json");
const manifestRaw = fs.readFileSync(manifestPath);
const manifest = JSON.parse(manifestRaw.toString("utf8"));
const modes = ["responsive-density", "semantic-low-color"];
const requiredLabels = {
  "responsive-density": [
    "startup",
    "conversation-80",
    "conversation-200",
    "conversation-58",
    "exit",
  ],
  "semantic-low-color": [
    "startup",
    "approval-required",
    "approval-narrow",
    "approval-declined",
    "approval-turn-cancelled",
    "provider-error",
    "stream-started",
    "turn-cancelled",
    "exit",
  ],
};
const forbidden = [
  /github_pat_[A-Za-z0-9_]+/,
  /gh[pousr]_[A-Za-z0-9_]+/,
  /Bearer\s+[A-Za-z0-9._-]+/i,
  /\bsk-[A-Za-z0-9_-]+/,
];

if (
  manifest.schema_version !== 3 ||
  manifest.version !== "2.0.8" ||
  manifest.provider !== "DeepSeek" ||
  manifest.model !== "deepseek-chat" ||
  manifest.terminal !== "Windows ConPTY" ||
  manifest.collector !== "scripts/conpty/v208/capture.js" ||
  JSON.stringify(manifest.scenarios.map((entry) => entry.mode)) !==
    JSON.stringify(modes)
) {
  throw new Error("manifest metadata is invalid");
}
assertNoSecrets(manifestRaw.toString("utf8"), "manifest");

for (const entry of manifest.scenarios) {
  const raw = fs.readFileSync(path.join(outputDir, entry.file));
  const hash = crypto.createHash("sha256").update(raw).digest("hex");
  if (hash !== entry.sha256) {
    throw new Error(`SHA-256 mismatch for ${entry.file}`);
  }
  const text = raw.toString("utf8");
  assertNoSecrets(text, entry.file);
  const evidence = JSON.parse(text);
  if (
    evidence.schema_version !== 3 ||
    evidence.version !== "2.0.8" ||
    evidence.provider !== "DeepSeek" ||
    evidence.model !== "deepseek-chat" ||
    evidence.terminal !== "Windows ConPTY" ||
    evidence.mode !== entry.mode ||
    evidence.output_bytes <= 0
  ) {
    throw new Error(`scenario metadata mismatch: ${entry.mode}`);
  }
  const labels = new Set(evidence.checkpoints.map((item) => item.label));
  for (const label of requiredLabels[entry.mode]) {
    if (!labels.has(label)) {
      throw new Error(`missing checkpoint ${entry.mode}:${label}`);
    }
  }
  if (!evidence.checkpoints.every((checkpoint) => checkpoint.matched !== false)) {
    throw new Error(`unmatched checkpoint: ${entry.mode}`);
  }
  const checkpoint = (label) =>
    evidence.checkpoints.find((candidate) => candidate.label === label);

  if (entry.mode === "responsive-density") {
    for (const [label, cols, rows] of [
      ["conversation-80", 80, 24],
      ["conversation-200", 200, 40],
      ["conversation-58", 58, 18],
    ]) {
      const frame = checkpoint(label);
      if (
        frame?.cols !== cols ||
        frame?.rows !== rows ||
        frame?.regions_ordered !== true ||
        frame?.composer_visible !== true
      ) {
        throw new Error(`responsive frame is invalid: ${label}`);
      }
    }
    if (checkpoint("conversation-58")?.diagnostics_deprioritized !== true) {
      throw new Error("58-column frame retained low-priority diagnostics");
    }
    if (!text.includes("YUNXI_V208_CONVERSATION_OK")) {
      throw new Error("real conversation marker is missing");
    }
  } else {
    if (evidence.color_capability !== "monochrome") {
      throw new Error("low-color scenario did not use monochrome capability");
    }
    const approval = checkpoint("approval-required");
    if (
      approval?.approval_redundancy !== true ||
      approval?.style_stats?.colored_cells !== 0 ||
      approval?.style_stats?.background_cells !== 0 ||
      approval?.style_stats?.inverse_cells <= 0
    ) {
      throw new Error("monochrome Approval still depends on color");
    }
    if (checkpoint("provider-error")?.error_text_visible !== true) {
      throw new Error("provider error text is missing");
    }
    if (checkpoint("turn-cancelled")?.cancel_text_visible !== true) {
      throw new Error("cancel text is missing");
    }
    for (const marker of [
      "Approval required",
      "default: Decline",
      "safe default",
      "risk:",
    ]) {
      if (!text.includes(marker)) {
        throw new Error(`missing low-color semantic marker: ${marker}`);
      }
    }
  }
}

console.log(JSON.stringify({ ok: true, manifest: manifestPath, scenarios: modes.length }));

function assertNoSecrets(text, source) {
  for (const pattern of forbidden) {
    if (pattern.test(text)) {
      throw new Error(`secret-like value found in ${source}: ${pattern}`);
    }
  }
}

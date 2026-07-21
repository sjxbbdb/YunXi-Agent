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
  "v207-hotfix-conpty",
);
const manifestPath = path.join(outputDir, "manifest.json");
const manifestRaw = fs.readFileSync(manifestPath);
const manifest = JSON.parse(manifestRaw.toString("utf8"));

const expectedModes = ["approval-freeze", "details-scroll"];
const requiredLabels = {
  "approval-freeze": [
    "startup",
    "composer-footer",
    "history-response",
    "history-response-idle",
    "approval-overlay",
    "approval-footer",
    "approval-before-mouse",
    "approval-after-mouse",
    "approval-narrow",
    "approval-restored",
    "approval-turn-idle",
    "exit",
  ],
  "details-scroll": [
    "startup",
    "composer-footer",
    "history-focus",
    "history-footer",
    "composer-focus-restored",
    "debug-enabled",
    "approval-overlay",
    "tool-response",
    "tool-response-idle",
    "tool-turn-idle",
    "details-open",
    "details-footer",
    "details-wheel-scroll",
    "details-page-scroll",
    "details-narrow",
    "details-closed",
    "details-restored",
    "exit",
  ],
};
const requiredText = {
  "approval-freeze": [
    "v2.0.7-hotfix",
    "YUNXI_HOTFIX_HISTORY_OK",
    "Approval",
    "Tab select",
    "Esc decline",
  ],
  "details-scroll": [
    "v2.0.7-hotfix",
    "wheel/drag/PgUp/PgDown scroll",
    "Details",
    "wheel/PgUp/PgDown scroll",
    "YUNXI_HOTFIX_DETAIL_LINE_001",
  ],
};
const forbidden = [
  /github_pat_[A-Za-z0-9_]+/,
  /gh[pousr]_[A-Za-z0-9_]+/,
  /Bearer\s+[A-Za-z0-9._-]+/i,
  /\bsk-[A-Za-z0-9_-]+/,
];

if (
  manifest.schema_version !== 2 ||
  manifest.version !== "2.0.7-hotfix" ||
  manifest.provider !== "DeepSeek" ||
  manifest.model !== "deepseek-chat" ||
  manifest.terminal !== "Windows ConPTY" ||
  manifest.collector !== "scripts/conpty/v207-hotfix/capture.js"
) {
  throw new Error("manifest metadata is invalid");
}
if (
  JSON.stringify(manifest.scenarios.map((entry) => entry.mode)) !==
  JSON.stringify(expectedModes)
) {
  throw new Error("manifest scenario order or membership is invalid");
}
assertNoSecrets(manifestRaw.toString("utf8"), "manifest");

for (const entry of manifest.scenarios) {
  const fullPath = path.join(outputDir, entry.file);
  const raw = fs.readFileSync(fullPath);
  const text = raw.toString("utf8");
  const hash = crypto.createHash("sha256").update(raw).digest("hex");
  if (hash !== entry.sha256) {
    throw new Error(`SHA-256 mismatch for ${entry.file}`);
  }
  assertNoSecrets(text, entry.file);

  const evidence = JSON.parse(text);
  if (evidence.error) {
    throw new Error(`scenario contains error: ${entry.mode}: ${evidence.error}`);
  }
  if (
    evidence.schema_version !== 2 ||
    evidence.version !== "2.0.7-hotfix" ||
    evidence.provider !== "DeepSeek" ||
    evidence.model !== "deepseek-chat" ||
    evidence.terminal !== "Windows ConPTY" ||
    evidence.mode !== entry.mode ||
    evidence.output_bytes <= 0
  ) {
    throw new Error(`scenario metadata mismatch: ${entry.mode}`);
  }
  if (
    entry.output_bytes !== evidence.output_bytes ||
    entry.checkpoint_count !== evidence.checkpoints.length
  ) {
    throw new Error(`manifest summary mismatch: ${entry.mode}`);
  }

  const checkpoint = (label) =>
    evidence.checkpoints.find((candidate) => candidate.label === label);
  const labels = new Set(evidence.checkpoints.map((candidate) => candidate.label));
  for (const label of requiredLabels[entry.mode]) {
    if (!labels.has(label)) {
      throw new Error(`missing checkpoint ${entry.mode}:${label}`);
    }
  }
  if (!evidence.checkpoints.every((candidate) => candidate.matched !== false)) {
    throw new Error(`unmatched checkpoint in ${entry.mode}`);
  }
  const evidenceContent = [
    JSON.stringify(evidence.actions),
    ...evidence.checkpoints.map((candidate) => candidate.screen),
  ].join("\n");
  for (const marker of requiredText[entry.mode]) {
    if (!evidenceContent.includes(marker)) {
      throw new Error(`missing evidence marker ${entry.mode}:${marker}`);
    }
  }

  if (entry.mode === "approval-freeze") {
    if (checkpoint("approval-after-mouse")?.transcript_unchanged !== true) {
      throw new Error("Approval mouse actions did not preserve the transcript frame");
    }
    if (checkpoint("approval-footer")?.wheel_not_advertised !== true) {
      throw new Error("Approval footer advertises transcript mouse scrolling");
    }
    const narrow = checkpoint("approval-narrow");
    if (narrow?.cols !== 58 || narrow?.rows !== 18 || narrow?.footer_matches !== true) {
      throw new Error("Approval narrow frame is invalid");
    }
    if (evidence.actions.filter((action) => action.action === "mouse").length < 5) {
      throw new Error("Approval scenario did not exercise wheel/click/drag/release");
    }
  } else {
    if (checkpoint("details-wheel-scroll")?.details_changed !== true) {
      throw new Error("Details wheel did not change only the details frame");
    }
    if (checkpoint("details-page-scroll")?.details_changed !== true) {
      throw new Error("Details PgDown did not change the details frame");
    }
    if (checkpoint("details-restored")?.transcript_anchor_restored !== true) {
      throw new Error("Closing Details did not restore the transcript frame");
    }
    const narrow = checkpoint("details-narrow");
    if (narrow?.cols !== 58 || narrow?.rows !== 18 || narrow?.footer_matches !== true) {
      throw new Error("Details narrow frame is invalid");
    }
  }
}

console.log(
  JSON.stringify({
    ok: true,
    manifest: manifestPath,
    scenarios: expectedModes.length,
  }),
);

function assertNoSecrets(text, source) {
  for (const pattern of forbidden) {
    if (pattern.test(text)) {
      throw new Error(`secret-like value found in ${source}: ${pattern}`);
    }
  }
}

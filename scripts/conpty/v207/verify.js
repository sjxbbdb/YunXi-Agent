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
  "v207-conpty",
);
const manifestPath = path.join(outputDir, "manifest.json");
const manifestRaw = fs.readFileSync(manifestPath);
const manifest = JSON.parse(manifestRaw.toString("utf8"));

const expectedModes = [
  "ordinary",
  "multiline",
  "crlf",
  "long",
  "ime",
  "stream-cancel",
  "approval-restore",
  "final-single",
];
const requiredLabels = {
  ordinary: ["startup", "ordinary-response", "ordinary-response-idle", "exit"],
  multiline: [
    "startup",
    "multiline-composer",
    "multiline-response",
    "multiline-response-idle",
    "exit",
  ],
  crlf: [
    "startup",
    "crlf-normalized-composer",
    "crlf-response",
    "crlf-response-idle",
    "exit",
  ],
  long: [
    "startup",
    "long-composer-bounded",
    "long-response",
    "long-response-idle",
    "exit",
  ],
  ime: [
    "startup",
    "ime-composer",
    "ime-response",
    "ime-response-idle",
    "exit",
  ],
  "stream-cancel": [
    "startup",
    "streaming-draft-visible",
    "stream-cancelled-draft-restored",
    "stream-draft-response",
    "stream-draft-response-idle",
    "exit",
  ],
  "approval-restore": [
    "startup",
    "approval-overlay",
    "approval-turn-finished",
    "approval-draft-restored",
    "approval-draft-response",
    "approval-draft-response-idle",
    "exit",
  ],
  "final-single": [
    "startup",
    "final-response",
    "final-response-idle",
    "final-single-cell",
    "exit",
  ],
};
const requiredText = {
  ordinary: ["v2.0.7", "YUNXI_INPUT_ORDINARY_OK"],
  multiline: [
    "YUNXI_INPUT_MULTILINE_OK",
    "This is the second pasted line.",
  ],
  crlf: ["YUNXI_INPUT_CRLF_OK", "CRLF_SECOND_LINE", "CR_THIRD_LINE"],
  long: ["YUNXI_INPUT_LONG_OK", "_TAIL", "Enter submit"],
  ime: ["YUNXI_INPUT_IME_OK", "已提交中文输入", "组合字符"],
  "stream-cancel": [
    "typing saves draft",
    "YUNXI_STREAM_DRAFT_OK",
    "Ctrl+C cancel",
  ],
  "approval-restore": [
    "> Decline",
    "probe-overlay.ps1",
    "YUNXI_OVERLAY_DRAFT_OK",
  ],
  "final-single": ["YUNXI_FINAL_SINGLE_OK"],
};
const forbidden = [
  /github_pat_[A-Za-z0-9_]+/,
  /gh[pousr]_[A-Za-z0-9_]+/,
  /Bearer\s+[A-Za-z0-9._-]+/i,
  /\bsk-[A-Za-z0-9_-]+/,
];

if (
  manifest.schema_version !== 1 ||
  manifest.version !== "2.0.7" ||
  manifest.provider !== "DeepSeek" ||
  manifest.model !== "deepseek-chat" ||
  manifest.terminal !== "Windows ConPTY" ||
  manifest.collector !== "scripts/conpty/v207/capture.js"
) {
  throw new Error("manifest metadata is invalid");
}
if (
  JSON.stringify(manifest.scenarios.map((entry) => entry.mode)) !==
  JSON.stringify(expectedModes)
) {
  throw new Error("manifest scenario order or membership is invalid");
}
for (const pattern of forbidden) {
  if (pattern.test(manifestRaw.toString("utf8"))) {
    throw new Error(`secret-like value found in manifest: ${pattern}`);
  }
}

for (const entry of manifest.scenarios) {
  const fullPath = path.join(outputDir, entry.file);
  const raw = fs.readFileSync(fullPath);
  const text = raw.toString("utf8");
  const hash = crypto.createHash("sha256").update(raw).digest("hex");
  if (hash !== entry.sha256) {
    throw new Error(`SHA-256 mismatch for ${entry.file}`);
  }
  for (const pattern of forbidden) {
    if (pattern.test(text)) {
      throw new Error(`secret-like value found in ${entry.file}: ${pattern}`);
    }
  }

  const evidence = JSON.parse(text);
  if (evidence.error) {
    throw new Error(`scenario contains error: ${entry.mode}: ${evidence.error}`);
  }
  if (
    evidence.version !== "2.0.7" ||
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

  const labels = new Set(
    evidence.checkpoints.map((checkpoint) => checkpoint.label),
  );
  for (const label of requiredLabels[entry.mode]) {
    if (!labels.has(label)) {
      throw new Error(`missing checkpoint ${entry.mode}:${label}`);
    }
  }
  if (!evidence.checkpoints.every((checkpoint) => checkpoint.matched !== false)) {
    throw new Error(`unmatched checkpoint in ${entry.mode}`);
  }
  const evidenceContent = [
    JSON.stringify(evidence.actions),
    ...evidence.checkpoints.map((checkpoint) => checkpoint.screen),
  ].join("\n");
  for (const marker of requiredText[entry.mode]) {
    if (!evidenceContent.includes(marker)) {
      throw new Error(`missing evidence marker ${entry.mode}:${marker}`);
    }
  }

  const action = (name) =>
    evidence.actions.find((candidate) => candidate.action === name);
  const checkpoint = (label) =>
    evidence.checkpoints.find((candidate) => candidate.label === label);
  if (entry.mode === "multiline") {
    if (!action("bracketed_paste")?.value.includes("\n")) {
      throw new Error("multiline paste did not preserve a newline action");
    }
  } else if (entry.mode === "crlf") {
    const value = action("bracketed_paste")?.value ?? "";
    if (!value.includes("\r\n") || !/\r(?!\n)/.test(value)) {
      throw new Error("CRLF scenario did not exercise CRLF and lone CR input");
    }
    if (checkpoint("crlf-normalized-composer").screen.includes("\r")) {
      throw new Error("carriage return reached the rendered composer");
    }
  } else if (entry.mode === "long") {
    if ((action("bracketed_paste")?.value.length ?? 0) < 1800) {
      throw new Error("long paste payload is below the required test length");
    }
    const frame = checkpoint("long-composer-bounded");
    if (frame.cols !== 80 || frame.rows !== 24) {
      throw new Error("long composer evidence was not captured at 80x24");
    }
  } else if (entry.mode === "ime") {
    const value = action("ime_committed_text")?.value ?? "";
    if (!value.includes("👨‍👩‍👧‍👦") || !value.includes("e\u0301")) {
      throw new Error("IME evidence is missing emoji ZWJ or combining input");
    }
  } else if (entry.mode === "stream-cancel") {
    if (!action("queued_draft_paste") || !action("key")?.value) {
      throw new Error("stream cancellation did not record queued draft and key input");
    }
  } else if (entry.mode === "approval-restore") {
    if (!action("queued_draft_paste")) {
      throw new Error("approval scenario did not queue a composer draft");
    }
  } else if (entry.mode === "final-single") {
    if (checkpoint("final-single-cell").assistant_marker_occurrences !== 1) {
      throw new Error("final response was rendered more than once");
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

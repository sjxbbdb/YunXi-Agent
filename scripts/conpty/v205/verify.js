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
  "v205-conpty",
);
const manifest = JSON.parse(
  fs.readFileSync(path.join(outputDir, "manifest.json"), "utf8"),
);

const expectedModes = [
  "responsive",
  "decline",
  "approve",
  "cancel",
  "nonzero",
  "invalid",
  "binary",
  "long",
];
const requiredLabels = {
  responsive: [
    "startup",
    "resize-100x30",
    "resize-120x40",
    "resize-200x50",
    "resize-80x24",
    "responsive-live-response",
    "exit",
  ],
  decline: [
    "startup",
    "decline-approval-pane",
    "declined",
    "decline-turn-done",
    "decline-next-turn",
    "exit",
  ],
  approve: [
    "startup",
    "approve-approval-pane",
    "approved",
    "approve-turn-done",
    "approve-next-turn",
    "exit",
  ],
  cancel: [
    "startup",
    "cancel-approval-pane",
    "cancelled",
    "cancel-turn-done",
    "cancel-next-turn",
    "exit",
  ],
  nonzero: [
    "startup",
    "nonzero-approval-pane",
    "nonzero-exit",
    "nonzero-turn-done",
    "nonzero-next-turn",
    "exit",
  ],
  invalid: [
    "startup",
    "debug-enabled",
    "invalid-approval-pane",
    "invalid-completed",
    "invalid-turn-done",
    "invalid-details",
    "invalid-next-turn",
    "exit",
  ],
  binary: [
    "startup",
    "debug-enabled",
    "binary-approval-pane",
    "binary-completed",
    "binary-turn-done",
    "binary-details",
    "binary-next-turn",
    "exit",
  ],
  long: [
    "startup",
    "debug-enabled",
    "long-approval-pane",
    "long-completed",
    "long-turn-done",
    "long-details-tail",
    "long-details",
    "long-next-turn",
    "exit",
  ],
};
const requiredText = {
  responsive: ["YUNXI_RESPONSIVE_LIVE_OK", "100x30", "120x40", "200x50"],
  decline: ["> Decline", "YX-APPROVAL-001", "YUNXI_DECLINE_NEXT_OK"],
  approve: ["> Decline", "shell: completed", "YUNXI_APPROVE_NEXT_OK"],
  cancel: [
    "> Decline",
    "shell: cancelled",
    "YX-CANCEL-001",
    "YUNXI_CANCEL_NEXT_OK",
  ],
  nonzero: ["YX-TOOL-001", "cmd /c exit 7", "YUNXI_NONZERO_NEXT_OK"],
  invalid: [
    "replacement_count=1",
    "integrity=Lossy",
    "YUNXI_INVALID_NEXT_OK",
  ],
  binary: [
    "binary output omitted",
    "replacement_count=1",
    "integrity=Lossy",
    "YUNXI_BINARY_NEXT_OK",
  ],
  long: [
    "truncated=true",
    "integrity=Partial",
    "line(s) hidden",
    "YUNXI_LONG_NEXT_OK",
  ],
};
const forbidden = [
  /github_pat_[A-Za-z0-9_]+/,
  /gh[pousr]_[A-Za-z0-9_]+/,
  /Bearer\s+[A-Za-z0-9._-]+/i,
  /\bsk-[A-Za-z0-9_-]+/,
];

if (JSON.stringify(manifest.scenarios.map((entry) => entry.mode)) !== JSON.stringify(expectedModes)) {
  throw new Error("manifest scenario order or membership is invalid");
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
    evidence.version !== "2.0.5" ||
    evidence.provider !== "DeepSeek" ||
    evidence.model !== "deepseek-chat" ||
    evidence.terminal !== "Windows ConPTY"
  ) {
    throw new Error(`scenario metadata mismatch: ${entry.mode}`);
  }
  const labels = new Set(evidence.checkpoints.map((checkpoint) => checkpoint.label));
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
  const normalized = evidenceContent.replace(/[^\x21-\x7E]+/g, "");
  for (const marker of requiredText[entry.mode]) {
    const normalizedMarker = marker.replace(/[^\x21-\x7E]+/g, "");
    if (!normalized.includes(normalizedMarker)) {
      throw new Error(`missing evidence marker ${entry.mode}:${marker}`);
    }
  }
}

console.log(
  JSON.stringify({
    ok: true,
    manifest: path.join(outputDir, "manifest.json"),
    scenarios: expectedModes.length,
  }),
);

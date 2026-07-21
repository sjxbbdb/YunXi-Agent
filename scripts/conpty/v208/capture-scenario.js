const fs = require("fs");
const path = require("path");
const pty = require("node-pty");
const { Terminal } = require("@xterm/headless");

const mode = process.argv[2];
if (!new Set(["responsive-density", "semantic-low-color"]).has(mode)) {
  throw new Error("usage: node capture-scenario.js responsive-density|semantic-low-color");
}

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const binary = path.join(repoRoot, "target", "release", "yunxi.exe");
const workspace = path.join(
  __dirname,
  ".work",
  mode,
  "very-long-workspace-name-for-density-evidence",
);
const outputDir = path.join(
  repoRoot,
  "docs",
  "reports",
  "evidence",
  "frames",
  "v208-conpty",
);
const evidencePath = path.join(outputDir, `${mode}.json`);
const monochrome = mode === "semantic-low-color";

if (!fs.existsSync(binary)) {
  throw new Error(`release binary is missing: ${binary}`);
}
fs.mkdirSync(workspace, { recursive: true });
fs.mkdirSync(outputDir, { recursive: true });

let cols = mode === "responsive-density" ? 80 : 100;
let rows = mode === "responsive-density" ? 24 : 30;
const checkpoints = [];
const actions = [];
const terminal = new Terminal({ cols, rows, scrollback: 4000, allowProposedApi: true });
const childEnv = {
  ...process.env,
  TERM: monochrome ? "xterm" : "xterm-256color",
  YUNXI_PROVIDER_STREAM: "true",
};
if (monochrome) {
  childEnv.NO_COLOR = "1";
  delete childEnv.COLORTERM;
} else {
  childEnv.COLORTERM = "truecolor";
  delete childEnv.NO_COLOR;
}

const child = pty.spawn(
  binary,
  ["--backend", "yunxi", "--provider-live", "--model", "deepseek-chat", "--cwd", workspace],
  {
    name: "xterm-256color",
    cols,
    rows,
    cwd: workspace,
    env: childEnv,
    useConpty: true,
  },
);

let outputBytes = 0;
let childExited = false;
child.onData((data) => {
  outputBytes += Buffer.byteLength(data, "utf8");
  terminal.write(data);
});
child.onExit(() => {
  childExited = true;
});

const sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

function screen() {
  const lines = [];
  for (let index = 0; index < terminal.rows; index += 1) {
    const line = terminal.buffer.active.getLine(index);
    lines.push(line ? line.translateToString(true) : "");
  }
  return lines.join("\n");
}

function styleStats() {
  const stats = {
    visible_cells: 0,
    colored_cells: 0,
    background_cells: 0,
    bold_cells: 0,
    inverse_cells: 0,
  };
  for (let y = 0; y < terminal.rows; y += 1) {
    const line = terminal.buffer.active.getLine(y);
    if (!line) continue;
    for (let x = 0; x < terminal.cols; x += 1) {
      const cell = line.getCell(x);
      if (!cell || !cell.getChars()) continue;
      stats.visible_cells += 1;
      if (!cell.isFgDefault()) stats.colored_cells += 1;
      if (!cell.isBgDefault()) stats.background_cells += 1;
      if (cell.isBold()) stats.bold_cells += 1;
      if (cell.isInverse()) stats.inverse_cells += 1;
    }
  }
  return stats;
}

function sanitize(value) {
  return value
    .replace(/github_pat_[A-Za-z0-9_]+/g, "[redacted]")
    .replace(/gh[pousr]_[A-Za-z0-9_]+/g, "[redacted]")
    .replace(/(Authorization:|Bearer\s+|api[-_ ]?key\s*[:=])[^\s]+/gi, "$1[redacted]")
    .replace(/\bsk-[A-Za-z0-9_-]+/g, "[redacted]");
}

function regionState(value = screen()) {
  const lines = value.split("\n");
  const header = lines.findIndex((line) => line.includes("YunXi"));
  const transcript = lines.findIndex((line) => line.includes("Transcript"));
  const bottom = lines.findIndex(
    (line) => line.includes("Composer") || line.includes("Approval required"),
  );
  return {
    header_row: header,
    transcript_row: transcript,
    bottom_row: bottom,
    regions_ordered: header >= 0 && transcript > header && bottom > transcript,
    composer_visible: value.includes("Composer") && value.includes("Enter submit"),
  };
}

function recordAction(action, value = "") {
  actions.push({ action, value: sanitize(value), timestamp: new Date().toISOString() });
}

function checkpoint(label, extra = {}) {
  checkpoints.push({
    label,
    cols,
    rows,
    timestamp: new Date().toISOString(),
    screen: sanitize(screen()),
    style_stats: styleStats(),
    ...extra,
  });
}

async function waitFor(label, predicate, timeoutMilliseconds = 120000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMilliseconds) {
    const value = screen();
    if (predicate(value)) {
      checkpoint(label, { matched: true });
      return value;
    }
    await sleep(250);
  }
  checkpoint(label, { matched: false, timeout: true });
  throw new Error(`timeout waiting for ${label}`);
}

function writeKey(label, value) {
  recordAction("key", label);
  child.write(value);
}

async function submitPrompt(value) {
  recordAction("prompt", value);
  child.write(value);
  await sleep(250);
  child.write("\r");
}

async function runCommand(value) {
  recordAction("command", value);
  child.write(value);
  await sleep(200);
  child.write("\r");
}

function resize(nextCols, nextRows) {
  cols = nextCols;
  rows = nextRows;
  terminal.resize(cols, rows);
  child.resize(cols, rows);
  recordAction("resize", `${cols}x${rows}`);
}

async function waitForIdle(label) {
  return waitFor(label, (value) => value.includes("Enter submit"));
}

async function exitCleanly() {
  await runCommand("/exit");
  for (let attempt = 0; attempt < 30 && !childExited; attempt += 1) {
    await sleep(500);
    child.write("\r");
  }
  checkpoint("exit", { matched: childExited, exited: childExited });
  if (!childExited) throw new Error("YunXi did not exit after /exit");
}

async function runResponsiveDensity() {
  await waitFor("startup", (value) => value.includes("v2.0.8") && value.includes("yunxi>"));
  await submitPrompt(
    "Reply exactly YUNXI_V208_CONVERSATION_OK followed by one short companion sentence. Do not call tools.",
  );
  await waitFor("conversation-response", (value) =>
    value.includes("[assistant]") &&
    value.includes("YUNXI_V208_CONVERSATION_OK") &&
    value.includes("Enter submit"),
  );
  await waitForIdle("conversation-idle");

  resize(80, 24);
  await sleep(500);
  checkpoint("conversation-80", regionState());
  resize(200, 40);
  await sleep(500);
  checkpoint("conversation-200", regionState());
  resize(58, 18);
  await sleep(500);
  const narrow = screen();
  checkpoint("conversation-58", {
    ...regionState(narrow),
    diagnostics_deprioritized:
      !narrow.includes("backend=") &&
      !narrow.includes("source=") &&
      !narrow.includes("debug"),
  });
}

async function runSemanticLowColor() {
  await waitFor("startup", (value) => value.includes("v2.0.8") && value.includes("yunxi>"));
  await submitPrompt(
    "Call the shell tool now and run exactly powershell -NoProfile -Command \"Write-Output YUNXI_V208_TOOL_OK\". Return the actual result.",
  );
  const approval = await waitFor("approval-overlay", (value) =>
    value.includes("Approval required") &&
    value.includes("default: Decline") &&
    value.includes("safe default") &&
    value.includes("risk:"),
  );
  checkpoint("approval-required", {
    approval_redundancy:
      approval.includes("Approval required") &&
      approval.includes("default: Decline") &&
      approval.includes("safe default") &&
      approval.includes("risk:"),
  });
  resize(58, 18);
  await sleep(500);
  const narrow = screen();
  checkpoint("approval-narrow", {
    approval_redundancy:
      narrow.includes("Approval required") &&
      narrow.includes("Decline") &&
      narrow.includes("risk:"),
  });
  resize(100, 30);
  writeKey("N decline", "n");
  await waitFor("approval-declined", (value) =>
    value.toLowerCase().includes("declined") && value.includes("Enter submit"),
  );
  writeKey("Ctrl+C settle declined turn", "\u0003");
  await waitFor("approval-turn-cancelled", (value) =>
    value.toLowerCase().includes("cancel") && value.includes("Enter submit"),
  );

  await runCommand("/model yunxi-v208-invalid-model");
  await waitFor("invalid-model-selected", (value) =>
    value.includes("model: yunxi-v208-invalid-model") && value.includes("Enter submit"),
  );
  await submitPrompt("Reply with YUNXI_V208_ERROR_PROBE.");
  const providerError = await waitFor("provider-error-visible", (value) =>
    value.includes("YX-PROVIDER-001") && value.includes("Enter submit"),
  );
  checkpoint("provider-error", {
    error_text_visible: providerError.includes("YX-PROVIDER-001"),
  });

  await runCommand("/model deepseek-chat");
  await waitFor("valid-model-restored", (value) =>
    value.includes("model: deepseek-chat") && value.includes("Enter submit"),
  );
  await submitPrompt(
    "Do not call tools. Start your response exactly with YUNXI_V208_CANCEL_PROBE, then write a long plain-text companion letter of at least 2000 words from your own language until cancelled.",
  );
  await waitFor("stream-started", (value) => value.includes("[assistant*]"));
  writeKey("Ctrl+C cancel", "\u0003");
  const cancelled = await waitFor("cancel-visible", (value) =>
    value.toLowerCase().includes("cancel") && value.includes("Enter submit"),
  );
  checkpoint("turn-cancelled", {
    cancel_text_visible: cancelled.toLowerCase().includes("cancel"),
  });
}

async function run() {
  if (mode === "responsive-density") {
    await runResponsiveDensity();
  } else {
    await runSemanticLowColor();
  }
  await exitCleanly();
  const evidence = {
    schema_version: 3,
    version: "2.0.8",
    provider: "DeepSeek",
    model: "deepseek-chat",
    terminal: "Windows ConPTY",
    color_capability: monochrome ? "monochrome" : "full",
    mode,
    command: child.process,
    output_bytes: outputBytes,
    actions,
    checkpoints,
  };
  fs.writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  terminal.dispose();
  console.log(JSON.stringify({ ok: true, mode, output_bytes: outputBytes, evidence: evidencePath }));
  process.exit(0);
}

run().catch((error) => {
  const evidence = {
    schema_version: 3,
    version: "2.0.8",
    provider: "DeepSeek",
    model: "deepseek-chat",
    terminal: "Windows ConPTY",
    color_capability: monochrome ? "monochrome" : "full",
    mode,
    output_bytes: outputBytes,
    actions,
    checkpoints,
    error: String(error.message || error),
  };
  fs.writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  if (!childExited) child.kill();
  terminal.dispose();
  console.error(error.stack || error);
  process.exit(1);
});

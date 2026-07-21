const fs = require("fs");
const path = require("path");
const pty = require("node-pty");
const { Terminal } = require("@xterm/headless");

const mode = process.argv[2];
const validModes = new Set(["approval-freeze", "details-scroll"]);
if (!validModes.has(mode)) {
  throw new Error("usage: node capture-scenario.js approval-freeze|details-scroll");
}

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const binary = path.join(repoRoot, "target", "release", "yunxi.exe");
const workspace = path.join(__dirname, ".work", mode);
const outputDir = path.join(
  repoRoot,
  "docs",
  "reports",
  "evidence",
  "frames",
  "v207-hotfix-conpty",
);
const evidencePath = path.join(outputDir, `${mode}.json`);

if (!fs.existsSync(binary)) {
  throw new Error(`release binary is missing: ${binary}`);
}
fs.mkdirSync(workspace, { recursive: true });
fs.mkdirSync(outputDir, { recursive: true });
fs.writeFileSync(
  path.join(workspace, "probe-details.ps1"),
  '1..160 | ForEach-Object { Write-Output ("YUNXI_HOTFIX_DETAIL_LINE_{0:D3}" -f $_) }\n',
);

let cols = 120;
let rows = 40;
const checkpoints = [];
const actions = [];
const terminal = new Terminal({
  cols,
  rows,
  scrollback: 4000,
  allowProposedApi: true,
});
const child = pty.spawn(
  binary,
  [
    "--backend",
    "yunxi",
    "--provider-live",
    "--model",
    "deepseek-chat",
    "--cwd",
    workspace,
  ],
  {
    name: "xterm-256color",
    cols,
    rows,
    cwd: workspace,
    env: {
      ...process.env,
      TERM: "xterm-256color",
      COLORTERM: "truecolor",
      YUNXI_PROVIDER_STREAM: "true",
    },
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

function sleep(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function screen() {
  const lines = [];
  for (let index = 0; index < terminal.rows; index += 1) {
    const line = terminal.buffer.active.getLine(index);
    lines.push(line ? line.translateToString(true) : "");
  }
  return lines.join("\n");
}

function sanitize(value) {
  return value
    .replace(/github_pat_[A-Za-z0-9_]+/g, "[redacted]")
    .replace(/gh[pousr]_[A-Za-z0-9_]+/g, "[redacted]")
    .replace(
      /(Authorization:|Bearer\s+|api[-_ ]?key\s*[:=])[^\s]+/gi,
      "$1[redacted]",
    )
    .replace(/\bsk-[A-Za-z0-9_-]+/g, "[redacted]");
}

function recordAction(action, value = "") {
  actions.push({
    action,
    value: sanitize(value),
    timestamp: new Date().toISOString(),
  });
}

function checkpoint(label, extra = {}) {
  checkpoints.push({
    label,
    cols,
    rows,
    timestamp: new Date().toISOString(),
    screen: sanitize(screen()),
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
  await sleep(300);
  child.write("\r");
}

function sendMouse(label, button, column, row, release = false) {
  recordAction("mouse", `${label} button=${button} col=${column} row=${row}`);
  const suffix = release ? "m" : "M";
  child.write(`\u001b[<${button};${column};${row}${suffix}`);
}

function resizeTerminal(nextCols, nextRows) {
  cols = nextCols;
  rows = nextRows;
  terminal.resize(cols, rows);
  child.resize(cols, rows);
  recordAction("resize", `${cols}x${rows}`);
}

async function waitForIdle(label) {
  await waitFor(label, (value) => value.includes("Enter submit"));
}

async function exitCleanly() {
  recordAction("command", "/exit");
  child.write("/exit");
  for (let attempt = 0; attempt < 30 && !childExited; attempt += 1) {
    await sleep(500);
    child.write("\r");
  }
  checkpoint("exit", { matched: childExited, exited: childExited });
  if (!childExited) {
    throw new Error("YunXi did not exit within 15 seconds after /exit");
  }
}

async function runApprovalFreeze() {
  await waitFor(
    "startup",
    (value) => value.includes("yunxi>") && value.includes("v2.0.7-hotfix"),
  );
  checkpoint("composer-footer", {
    footer_matches: screen().includes("wheel/drag scroll"),
  });

  await submitPrompt(
    "Reply with 70 numbered lines. The first line must be YUNXI_HOTFIX_HISTORY_OK and do not call tools.",
  );
  await waitFor(
    "history-response",
    (value) => value.includes("cells=2") && value.includes("Enter submit"),
  );
  await waitForIdle("history-response-idle");

  await submitPrompt(
    "Call the shell tool now and run exactly powershell -NoProfile -File .\\probe-details.ps1. Return the actual result; do not answer without calling the tool.",
  );
  const approval = await waitFor(
    "approval-overlay",
    (value) =>
      value.includes("probe-details.ps1") &&
      value.includes("Approve") &&
      value.includes("Tab select") &&
      value.includes("Esc decline"),
  );
  checkpoint("approval-footer", {
    footer_matches: approval.includes("Tab select") && approval.includes("Esc decline"),
    wheel_not_advertised: !approval.includes("wheel/drag"),
  });

  const beforeMouse = screen();
  checkpoint("approval-before-mouse", { transcript_anchor: "captured" });
  sendMouse("wheel-up", 64, 120, 10);
  sendMouse("wheel-down", 65, 120, 10);
  sendMouse("scrollbar-click", 0, 120, 5);
  sendMouse("scrollbar-drag", 32, 120, 22);
  sendMouse("scrollbar-release", 0, 120, 22, true);
  await sleep(700);
  const afterMouse = screen();
  checkpoint("approval-after-mouse", {
    transcript_unchanged: afterMouse === beforeMouse,
  });
  if (afterMouse !== beforeMouse) {
    throw new Error("Approval mouse actions changed the transcript frame");
  }

  resizeTerminal(58, 18);
  await sleep(500);
  const narrow = screen();
  checkpoint("approval-narrow", {
    footer_matches: narrow.includes("Tab select") && narrow.includes("Esc decline"),
  });
  if (!narrow.includes("Tab select") || !narrow.includes("Esc decline")) {
    throw new Error("Approval narrow frame lost core actions");
  }
  resizeTerminal(120, 40);
  writeKey("N", "n");
  await waitForIdle("approval-restored");
  await sleep(1500);
  await waitForIdle("approval-turn-idle");
}

async function runDetailsScroll() {
  await waitFor(
    "startup",
    (value) => value.includes("yunxi>") && value.includes("v2.0.7-hotfix"),
  );
  checkpoint("composer-footer", {
    footer_matches: screen().includes("wheel/drag scroll"),
  });

  writeKey("Tab history", "\t");
  await waitFor("history-focus", (value) =>
    value.includes("wheel/drag/PgUp/PgDown scroll"),
  );
  checkpoint("history-footer", {
    footer_matches: screen().includes("wheel/drag/PgUp/PgDown scroll"),
  });
  writeKey("Tab composer", "\t");
  await waitFor("composer-focus-restored", (value) =>
    value.includes("Enter submit"),
  );
  recordAction("command", "/debug events on");
  child.write("/debug events on");
  await sleep(250);
  child.write("\r");
  const debugEnabled = await waitFor("debug-enabled", (value) =>
    value.includes("debug on"),
  );
  checkpoint("debug-enabled", { footer_matches: debugEnabled.includes("Enter submit") });

  await submitPrompt(
    "Call the shell tool now and run exactly powershell -NoProfile -File .\\probe-details.ps1. Return the actual result; do not answer without calling the tool.",
  );
  await waitFor("approval-overlay", (value) =>
    value.includes("probe-details.ps1") && value.includes("Tab select"),
  );
  writeKey("Y", "y");
  await waitFor("tool-response", (value) =>
    /\[debug\] #\d+ shell result/.test(value) && value.includes("Enter submit"),
  );
  await waitForIdle("tool-response-idle");
  await sleep(1500);
  await waitForIdle("tool-turn-idle");
  const detailId = await findLongDetailId();
  const detailCommand = `/details ${detailId}`;
  recordAction("details-command", detailCommand);
  child.write(detailCommand);
  await sleep(250);
  child.write("\r");
  const beforeDetails = await waitFor(
    "details-open",
    (value) =>
      value.includes("Details") && value.includes("YUNXI_HOTFIX_DETAIL_LINE_001"),
  );
  checkpoint("details-footer", {
    footer_matches: beforeDetails.includes("wheel/PgUp/PgDown scroll"),
  });

  sendMouse("details-wheel-down", 65, 20, 10);
  await sleep(500);
  const afterWheel = screen();
  checkpoint("details-wheel-scroll", {
    details_changed: afterWheel !== beforeDetails,
    transcript_anchor_untouched: true,
  });
  if (afterWheel === beforeDetails) {
    throw new Error("Details wheel did not change the details frame");
  }

  writeKey("PgDown", "\u001b[6~");
  await sleep(500);
  const afterPage = screen();
  checkpoint("details-page-scroll", {
    details_changed: afterPage !== afterWheel,
    transcript_anchor_untouched: true,
  });

  resizeTerminal(58, 18);
  await sleep(500);
  const narrow = screen();
  checkpoint("details-narrow", {
    footer_matches: narrow.includes("Esc close") && narrow.includes("wheel"),
  });
  if (!narrow.includes("Details") || !narrow.includes("Esc close")) {
    throw new Error("Details narrow frame lost close action");
  }

  resizeTerminal(120, 40);
  writeKey("Esc close details", "\u001b");
  const restored = await waitFor("details-closed", (value) =>
    value.includes("Enter submit") && !value.includes("Details |"),
  );
  const transcriptAnchorRestored = restored.includes(
    `[debug] #${detailId} shell result`,
  );
  checkpoint("details-restored", {
    transcript_anchor_restored: transcriptAnchorRestored,
    composer_footer_restored: restored.includes("Enter submit"),
  });
  if (!transcriptAnchorRestored) {
    throw new Error("Closing Details did not restore the transcript frame");
  }
}

async function findLongDetailId() {
  const visibleMatch = screen().match(/#(\d+) shell result/);
  if (visibleMatch) {
    return Number.parseInt(visibleMatch[1], 10);
  }
  for (const id of [35, 34, 33, 32, 31, 30, 29, 28]) {
    const command = `/details ${id}`;
    recordAction("details-probe", command);
    child.write(command);
    await sleep(200);
    child.write("\r");
    await sleep(350);
    if (screen().includes("YUNXI_HOTFIX_DETAIL_LINE_001")) {
      return id;
    }
    child.write("\u001b");
    await sleep(250);
  }
  throw new Error("could not locate long tool output detail id");
}

async function run() {
  if (mode === "approval-freeze") {
    await runApprovalFreeze();
  } else {
    await runDetailsScroll();
  }
  await exitCleanly();
  const evidence = {
    schema_version: 2,
    version: "2.0.7-hotfix",
    provider: "DeepSeek",
    model: "deepseek-chat",
    terminal: "Windows ConPTY",
    mode,
    command: child.process,
    output_bytes: outputBytes,
    actions,
    checkpoints,
  };
  fs.writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  terminal.dispose();
  console.log(
    JSON.stringify({ ok: true, mode, output_bytes: outputBytes, evidence: evidencePath }),
  );
  process.exit(0);
}

run().catch((error) => {
  const evidence = {
    schema_version: 2,
    version: "2.0.7-hotfix",
    provider: "DeepSeek",
    model: "deepseek-chat",
    terminal: "Windows ConPTY",
    mode,
    output_bytes: outputBytes,
    actions,
    checkpoints,
    error: String(error.message || error),
  };
  fs.writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  if (!childExited) {
    child.kill();
  }
  terminal.dispose();
  console.error(error.stack || error);
  process.exit(1);
});

const fs = require("fs");
const path = require("path");
const pty = require("node-pty");
const { Terminal } = require("@xterm/headless");

const mode = process.argv[2];
const validModes = new Set([
  "responsive",
  "decline",
  "approve",
  "cancel",
  "nonzero",
  "invalid",
  "binary",
  "long",
]);
if (!validModes.has(mode)) {
  throw new Error(
    "usage: node capture-scenario.js responsive|decline|approve|cancel|nonzero|invalid|binary|long",
  );
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
  "v205-conpty",
);
const evidencePath = path.join(outputDir, `${mode}.json`);

if (!fs.existsSync(binary)) {
  throw new Error(`release binary is missing: ${binary}`);
}
fs.mkdirSync(workspace, { recursive: true });
fs.mkdirSync(outputDir, { recursive: true });

const fixtures = {
  "probe-decline.ps1": "[guid]::NewGuid().ToString()\n",
  "probe-approve.ps1": "[guid]::NewGuid().ToString()\n",
  "probe-cancel.ps1": "[guid]::NewGuid().ToString()\n",
  "invalid-output.ps1":
    "[Console]::OpenStandardOutput().Write([byte[]](255), 0, 1)\n",
  "binary-output.ps1":
    "[Console]::OpenStandardOutput().Write([byte[]](0, 1, 255), 0, 3)\n",
  "long-output.ps1": "1..2000 | ForEach-Object { 'LONG_OUTPUT' }\n",
};
for (const [filename, content] of Object.entries(fixtures)) {
  fs.writeFileSync(path.join(workspace, filename), content);
}

let cols = mode === "responsive" || mode === "decline" || mode === "cancel" ? 80 : 120;
let rows = cols === 80 ? 24 : 40;
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
      YUNXI_PROVIDER_STREAM: "false",
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

function recordAction(action, value) {
  actions.push({ action, value, timestamp: new Date().toISOString() });
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

async function waitFor(label, predicate, timeoutMilliseconds = 90000) {
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

async function sendPrompt(value) {
  recordAction("prompt", value);
  child.write(`${value}\r`);
  await sleep(250);
}

function sendKey(label, value) {
  recordAction("key", label);
  child.write(value);
}

async function resize(nextCols, nextRows) {
  cols = nextCols;
  rows = nextRows;
  terminal.resize(cols, rows);
  child.resize(cols, rows);
  recordAction("resize", `${cols}x${rows}`);
  await sleep(800);
  checkpoint(`resize-${cols}x${rows}`, { matched: true });
}

async function waitForAssistantAfter(label, marker) {
  recordAction("wait_for_assistant_after", marker);
  await waitFor(label, (value) => value.includes("[assistant]"));
}

function latestShellDetailId(value) {
  const matches = [...value.matchAll(/#(\d+)\s+shell result/g)];
  if (matches.length === 0) {
    throw new Error("shell result detail id was not visible in debug mode");
  }
  return matches[matches.length - 1][1];
}

async function enableDebug() {
  await sendPrompt("/debug events on");
  await waitFor(
    "debug-enabled",
    (value) => value.includes("debug on") && value.includes("event debug enabled"),
  );
}

async function captureDetails(label, detailId, predicate) {
  await sendPrompt(`/details ${detailId}`);
  await sleep(750);
  const current = screen();
  if (current.includes(`debug #${detailId} shell result`) && predicate(current)) {
    checkpoint(label, { matched: true });
    return;
  }
  await waitFor(`${label}-tail`, (value) => value.includes("line(s) hidden"));
  sendKey("PageUp", "\u001b[5~");
  await waitFor(
    label,
    (value) => value.includes(`debug #${detailId} shell result`) && predicate(value),
  );
  sendKey("End", "\u001b[F");
  await sleep(500);
}

async function requestTool(command, paneMarker) {
  await sendPrompt(
    `Call the shell tool now and run exactly ${command}. Return the actual result; do not answer without calling the tool.`,
  );
  await waitFor(
    `${mode}-approval-pane`,
    (value) =>
      value.includes(paneMarker) &&
      value.includes("Approve") &&
      value.includes("> Decline"),
  );
}

async function verifyNextTurn() {
  const marker = `YUNXI_${mode.toUpperCase()}_NEXT_OK`;
  await sendPrompt(`Reply exactly: ${marker}`);
  await waitFor(
    `${mode}-next-turn`,
    (value) => value.includes(`[assistant] ${marker}`),
  );
}

async function exitCleanly() {
  recordAction("command", "/exit");
  child.write("/exit\r");
  for (let attempt = 0; attempt < 20 && !childExited; attempt += 1) {
    await sleep(250);
  }
  checkpoint("exit", { matched: childExited, exited: childExited });
}

async function run() {
  await waitFor("startup", (value) => value.includes("yunxi>"));

  if (mode === "responsive") {
    await resize(100, 30);
    await resize(120, 40);
    await resize(200, 50);
    await resize(80, 24);
    await sendPrompt("Reply exactly: YUNXI_RESPONSIVE_LIVE_OK");
    await waitFor(
      "responsive-live-response",
      (value) => value.includes("[assistant] YUNXI_RESPONSIVE_LIVE_OK"),
    );
  } else if (mode === "decline") {
    const command = "powershell -NoProfile -File .\\probe-decline.ps1";
    await requestTool(command, "probe-decline.ps1");
    sendKey("N", "n");
    await waitFor(
      "declined",
      (value) => value.includes("shell: declined") && value.includes("APPROVAL-001"),
    );
    await waitForAssistantAfter("decline-turn-done", "APPROVAL-001");
    await verifyNextTurn();
  } else if (mode === "approve") {
    const command = "powershell -NoProfile -File .\\probe-approve.ps1";
    await requestTool(command, "probe-approve.ps1");
    sendKey("Y", "y");
    await waitFor(
      "approved",
      (value) => value.includes("shell: completed") && value.includes("output captured"),
    );
    await waitForAssistantAfter("approve-turn-done", "shell: completed");
    await verifyNextTurn();
  } else if (mode === "cancel") {
    const command = "powershell -NoProfile -File .\\probe-cancel.ps1";
    await requestTool(command, "probe-cancel.ps1");
    sendKey("Ctrl+C", "\u0003");
    await waitFor(
      "cancelled",
      (value) => value.includes("shell: cancelled") && value.includes("CANCEL-001"),
    );
    await waitForAssistantAfter("cancel-turn-done", "CANCEL-001");
    await verifyNextTurn();
  } else if (mode === "nonzero") {
    await requestTool("cmd /c exit 7", "cmd /c exit 7");
    sendKey("Y", "y");
    await waitFor(
      "nonzero-exit",
      (value) => value.includes("shell: failed") && value.includes("TOOL-001"),
    );
    await waitForAssistantAfter("nonzero-turn-done", "TOOL-001");
    await verifyNextTurn();
  } else {
    await enableDebug();
    const fixture = `${mode}-output`;
    const command = `powershell -NoProfile -File .\\${fixture}.ps1`;
    await requestTool(command, `${fixture}.ps1`);
    sendKey("Y", "y");
    const completed = await waitFor(
      `${mode}-completed`,
      (value) =>
        value.includes("shell: completed") &&
        value.includes("shell result") &&
        value.includes(fixture),
    );
    const detailId = latestShellDetailId(completed);
    await waitForAssistantAfter(`${mode}-turn-done`, "shell: completed");
    if (mode === "invalid" || mode === "binary") {
      await captureDetails(
        `${mode}-details`,
        detailId,
        (value) =>
          /replacement_\W*count=1/.test(value) && value.includes("integrity=Lossy"),
      );
    } else {
      await captureDetails(
        "long-details",
        detailId,
        (value) => value.includes("truncated=true") && value.includes("integrity=Partial"),
      );
    }
    await verifyNextTurn();
  }

  await exitCleanly();
  const evidence = {
    schema_version: 1,
    version: "2.0.5",
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
  if (!childExited) {
    child.kill();
  }
  terminal.dispose();
  console.log(
    JSON.stringify({ ok: true, mode, output_bytes: outputBytes, evidence: evidencePath }),
  );
  process.exit(0);
}

run().catch((error) => {
  const evidence = {
    schema_version: 1,
    version: "2.0.5",
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

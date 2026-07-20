const fs = require("fs");
const path = require("path");
const pty = require("node-pty");
const { Terminal } = require("@xterm/headless");

const mode = process.argv[2];
const validModes = new Set([
  "ordinary",
  "multiline",
  "crlf",
  "long",
  "ime",
  "stream-cancel",
  "approval-restore",
  "final-single",
]);
if (!validModes.has(mode)) {
  throw new Error(
    "usage: node capture-scenario.js ordinary|multiline|crlf|long|ime|stream-cancel|approval-restore|final-single",
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
  "v206-conpty",
);
const evidencePath = path.join(outputDir, `${mode}.json`);

if (!fs.existsSync(binary)) {
  throw new Error(`release binary is missing: ${binary}`);
}
fs.mkdirSync(workspace, { recursive: true });
fs.mkdirSync(outputDir, { recursive: true });
fs.writeFileSync(
  path.join(workspace, "probe-overlay.ps1"),
  "Write-Output 'YUNXI_OVERLAY_TOOL_OK'\n",
);

let cols = mode === "long" ? 80 : 120;
let rows = mode === "long" ? 24 : 40;
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

function recordAction(action, value) {
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

function writeKey(label, value) {
  recordAction("key", label);
  child.write(value);
}

async function writeText(label, value) {
  recordAction(label, value);
  child.write(value);
  await sleep(300);
}

async function pasteText(value, action = "bracketed_paste") {
  recordAction(action, value);
  child.write(`\u001b[200~${value}\u001b[201~`);
  await sleep(300);
}

async function submitPrompt(value) {
  recordAction("prompt", value);
  child.write(value);
  await sleep(300);
  child.write("\r");
  await sleep(250);
}

async function submitPromptWithImmediateDraft(prompt, draft) {
  recordAction("prompt", prompt);
  child.write(prompt);
  await sleep(300);
  child.write("\r");
  await pasteText(draft, "queued_draft_paste");
}

async function waitForAssistant(label, marker) {
  await waitFor(label, (value) => value.includes(`[assistant] ${marker}`));
  await waitFor(`${label}-idle`, (value) =>
    value.includes("Enter submit"),
  );
}

function countOccurrences(value, marker) {
  return value.split(marker).length - 1;
}

async function exitCleanly() {
  recordAction("command", "/exit");
  child.write("/exit");
  await sleep(300);
  child.write("\r");
  for (let attempt = 0; attempt < 60 && !childExited; attempt += 1) {
    await sleep(250);
  }
  checkpoint("exit", { matched: childExited, exited: childExited });
  if (!childExited) {
    throw new Error("YunXi did not exit within 15 seconds after /exit");
  }
}

async function run() {
  await waitFor(
    "startup",
    (value) => value.includes("yunxi>") && value.includes("v2.0.6"),
  );

  if (mode === "ordinary") {
    await submitPrompt("Reply exactly: YUNXI_INPUT_ORDINARY_OK");
    await waitForAssistant("ordinary-response", "YUNXI_INPUT_ORDINARY_OK");
  } else if (mode === "multiline") {
    const prompt =
      "Reply exactly: YUNXI_INPUT_MULTILINE_OK\nThis is the second pasted line.";
    await pasteText(prompt);
    await waitFor(
      "multiline-composer",
      (value) =>
        value.includes("YUNXI_INPUT_MULTILINE_OK") &&
        value.includes("This is the second pasted line.") &&
        value.includes("cells=0") &&
        value.includes("Enter submit"),
    );
    writeKey("Enter", "\r");
    await waitForAssistant("multiline-response", "YUNXI_INPUT_MULTILINE_OK");
  } else if (mode === "crlf") {
    const prompt =
      "Reply exactly: YUNXI_INPUT_CRLF_OK\r\nCRLF_SECOND_LINE\rCR_THIRD_LINE";
    await pasteText(prompt);
    await waitFor(
      "crlf-normalized-composer",
      (value) =>
        value.includes("YUNXI_INPUT_CRLF_OK") &&
        value.includes("CRLF_SECOND_LINE") &&
        value.includes("CR_THIRD_LINE") &&
        value.includes("cells=0") &&
        !value.includes("\r"),
    );
    writeKey("Enter", "\r");
    await waitForAssistant("crlf-response", "YUNXI_INPUT_CRLF_OK");
  } else if (mode === "long") {
    const longToken = `LONG_${"0123456789".repeat(180)}_TAIL`;
    const prompt = `Reply exactly: YUNXI_INPUT_LONG_OK\n${longToken}`;
    await pasteText(prompt);
    await waitFor(
      "long-composer-bounded",
      (value) =>
        value.includes("_TAIL") &&
        value.includes("cells=0") &&
        value.includes("Enter submit") &&
        value.split("\n").length === rows,
    );
    writeKey("Enter", "\r");
    await waitForAssistant("long-response", "YUNXI_INPUT_LONG_OK");
  } else if (mode === "ime") {
    const prompt =
      "Reply exactly: YUNXI_INPUT_IME_OK - 已提交中文输入，家庭👨‍👩‍👧‍👦，e\u0301组合字符";
    await writeText("ime_committed_text", prompt);
    await waitFor(
      "ime-composer",
      (value) =>
        value.includes("已提交中文输入") &&
        value.includes("家庭") &&
        value.includes("组合字符") &&
        value.includes("Enter submit"),
    );
    writeKey("Enter", "\r");
    await waitForAssistant("ime-response", "YUNXI_INPUT_IME_OK");
  } else if (mode === "stream-cancel") {
    const sourcePrompt =
      "Do not call or search for any tool. This is only a writing task. Directly write 300 numbered lines, start with YUNXI_STREAM_CANCEL_SOURCE, and do not stop early.";
    const draft = "Reply exactly: YUNXI_STREAM_DRAFT_OK";
    await submitPrompt(sourcePrompt);
    await pasteText(draft, "queued_draft_paste");
    await waitFor(
      "streaming-draft-visible",
      (value) =>
        value.includes("Ctrl+C cancel") &&
        value.includes("typing saves draft") &&
        value.includes("YUNXI_STREAM_DRAFT_OK"),
    );
    writeKey("Ctrl+C", "\u0003");
    await waitFor(
      "stream-cancelled-draft-restored",
      (value) =>
        value.includes("Enter submit") && value.includes("YUNXI_STREAM_DRAFT_OK"),
    );
    writeKey("Enter", "\r");
    await waitForAssistant("stream-draft-response", "YUNXI_STREAM_DRAFT_OK");
  } else if (mode === "approval-restore") {
    const toolPrompt =
      "Call the shell tool now and run exactly powershell -NoProfile -File .\\probe-overlay.ps1. Return the actual result; do not answer without calling the tool.";
    const draft = "Reply exactly: YUNXI_OVERLAY_DRAFT_OK";
    await submitPromptWithImmediateDraft(toolPrompt, draft);
    await waitFor(
      "approval-overlay",
      (value) =>
        value.includes("probe-overlay.ps1") &&
        value.includes("Approve") &&
        value.includes("> Decline"),
    );
    writeKey("N", "n");
    await waitFor("approval-turn-finished", (value) =>
      value.includes("[assistant]"),
    );
    await waitFor(
      "approval-draft-restored",
      (value) =>
        value.includes("Enter submit") && value.includes("YUNXI_OVERLAY_DRAFT_OK"),
    );
    writeKey("Enter", "\r");
    await waitForAssistant("approval-draft-response", "YUNXI_OVERLAY_DRAFT_OK");
  } else if (mode === "final-single") {
    const marker = "YUNXI_FINAL_SINGLE_OK";
    await submitPrompt(`Reply exactly: ${marker}`);
    await waitForAssistant("final-response", marker);
    await sleep(1500);
    const current = screen();
    checkpoint("final-single-cell", {
      matched: countOccurrences(current, `[assistant] ${marker}`) === 1,
      assistant_marker_occurrences: countOccurrences(
        current,
        `[assistant] ${marker}`,
      ),
    });
    if (countOccurrences(current, `[assistant] ${marker}`) !== 1) {
      throw new Error("final response was not rendered exactly once");
    }
  }

  await exitCleanly();
  const evidence = {
    schema_version: 1,
    version: "2.0.6",
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
    version: "2.0.6",
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

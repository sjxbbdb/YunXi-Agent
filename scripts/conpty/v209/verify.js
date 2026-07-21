const crypto = require("crypto");
const fs = require("fs");
const http = require("http");
const path = require("path");
const { spawnSync } = require("child_process");
const pty = require("node-pty");
const { Terminal } = require("@xterm/headless");

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const binary = path.join(repoRoot, "target", "release", "yunxi.exe");
const workRoot = path.join(__dirname, ".work");
const outputDir = path.join(
  repoRoot,
  "docs",
  "reports",
  "evidence",
  "frames",
  "v209-conpty",
);
const timeoutMilliseconds = 120000;
const forbiddenSecrets = [
  /github_pat_[A-Za-z0-9_]+/,
  /gh[pousr]_[A-Za-z0-9_]+/,
  /Bearer\s+[A-Za-z0-9._-]+/i,
  /\bsk-[A-Za-z0-9_-]+/,
];

if (process.platform !== "win32") {
  throw new Error("v2.0.9 ConPTY verification requires Windows");
}
if (!fs.existsSync(binary)) {
  throw new Error(`release binary is missing: ${binary}`);
}
fs.mkdirSync(workRoot, { recursive: true });
fs.mkdirSync(outputDir, { recursive: true });

function cleanEnvironment(extra = {}) {
  const env = { ...process.env, ...extra };
  delete env.NO_COLOR;
  delete env.CI;
  delete env.YUNXI_PROVIDER_API_KEY;
  delete env.YUNXI_PROVIDER_API_KEY_ENV;
  delete env.OPENAI_API_KEY;
  return env;
}

function assertNoSecrets(text, source) {
  for (const pattern of forbiddenSecrets) {
    if (pattern.test(text)) {
      throw new Error(`secret-like value found in ${source}: ${pattern}`);
    }
  }
}

function assertNoTuiBytes(label, output) {
  const combined = `${output.stdout || ""}${output.stderr || ""}`;
  if (combined.includes("\u001b")) {
    throw new Error(`${label} emitted ANSI escape bytes`);
  }
  for (const footer of ["Enter submit", "Enter confirm", "Alt+Enter newline"]) {
    if (combined.includes(footer)) {
      throw new Error(`${label} leaked TUI footer text: ${footer}`);
    }
  }
  assertNoSecrets(combined, label);
  return {
    label,
    status: output.status,
    stdout_bytes: Buffer.byteLength(output.stdout || "", "utf8"),
    stderr_bytes: Buffer.byteLength(output.stderr || "", "utf8"),
  };
}

function runNonTuiMatrix() {
  const cwd = path.join(workRoot, "mode-matrix");
  fs.mkdirSync(cwd, { recursive: true });
  const cases = [
    ["plain-one-shot", ["--offline", "--cwd", cwd, "plain evidence"], undefined],
    ["plain-pipe", ["--offline", "--cwd", cwd], "/exit\n"],
    ["ci", ["--offline", "--cwd", cwd], "/exit\n", { CI: "1" }],
    ["no-tui", ["--offline", "--no-tui", "--cwd", cwd], "/exit\n"],
    ["forced-tui-fallback", ["--offline", "--tui", "--cwd", cwd], "/exit\n"],
    ["json", ["--offline", "--json", "--cwd", cwd, "json evidence"], undefined],
    ["jsonl", ["--offline", "--jsonl", "--cwd", cwd, "jsonl evidence"], undefined],
  ];
  return cases.map(([label, args, input, extraEnv]) => {
    const output = spawnSync(binary, args, {
      cwd,
      env: cleanEnvironment(extraEnv),
      input,
      encoding: "utf8",
      timeout: timeoutMilliseconds,
    });
    if (output.error || output.status !== 0) {
      throw new Error(`${label} failed: ${output.error || output.stderr}`);
    }
    if (label === "json") JSON.parse(output.stdout);
    if (label === "jsonl") {
      output.stdout.trim().split(/\r?\n/).filter(Boolean).forEach((line) => JSON.parse(line));
    }
    if (label === "forced-tui-fallback" && !output.stderr.includes("using plain mode")) {
      throw new Error("forced TUI fallback did not explain plain mode");
    }
    return assertNoTuiBytes(label, output);
  });
}

function startTui(label, args, env = {}) {
  const cwd = path.join(workRoot, label);
  fs.mkdirSync(cwd, { recursive: true });
  const terminal = new Terminal({
    cols: 100,
    rows: 30,
    scrollback: 4000,
    allowProposedApi: true,
  });
  const child = pty.spawn(binary, [...args, "--cwd", cwd, "--tui"], {
    name: "xterm-256color",
    cols: 100,
    rows: 30,
    cwd,
    env: cleanEnvironment({ TERM: "xterm-256color", COLORTERM: "truecolor", ...env }),
    useConpty: true,
  });
  let raw = "";
  let exit = null;
  child.onData((data) => {
    raw += data;
    terminal.write(data);
  });
  child.onExit((event) => {
    exit = event;
  });

  const screen = () => {
    const lines = [];
    for (let index = 0; index < terminal.rows; index += 1) {
      const line = terminal.buffer.active.getLine(index);
      lines.push(line ? line.translateToString(true) : "");
    }
    return lines.join("\n");
  };
  const waitFor = async (description, predicate) => {
    const started = Date.now();
    while (Date.now() - started < timeoutMilliseconds) {
      const value = screen();
      if (predicate(value, raw)) return value;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error(`${label} timed out waiting for ${description}`);
  };
  const waitForExit = async () => {
    const started = Date.now();
    while (Date.now() - started < timeoutMilliseconds) {
      if (exit) return exit;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error(`${label} did not exit`);
  };
  return {
    child,
    terminal,
    screen,
    waitFor,
    waitForExit,
    hasExited: () => exit !== null,
    raw: () => raw,
  };
}

function assertTerminalLifecycle(raw, label) {
  for (const token of [
    "\u001b[?1049h",
    "\u001b[?1049l",
    "\u001b[?2004h",
    "\u001b[?2004l",
    "\u001b[?1004h",
    "\u001b[?1003;1006h",
    "\u001b[?1003;1006l",
    "\u001b[?25l",
    "\u001b[?25h",
  ]) {
    if (!raw.includes(token)) {
      const observed = raw.match(/\u001b\[\?[0-9;]+[hl]/g) || [];
      throw new Error(
        `${label} missing terminal lifecycle token ${JSON.stringify(token)}; observed=${JSON.stringify([...new Set(observed)])}`,
      );
    }
  }
}

async function submit(session, value) {
  session.child.write(value);
  const visiblePrefix = value.slice(0, 24);
  await session.waitFor(`Composer draft ${visiblePrefix}`, (screen) =>
    screen.includes(visiblePrefix),
  );
  await new Promise((resolve) => setTimeout(resolve, 100));
  session.child.write("\r");
}

async function verifyTerminalRecovery() {
  const results = [];
  for (const [label, input] of [["normal-exit", "/exit"], ["ctrl-c-exit", "\u0003"]]) {
    const session = startTui(label, ["--offline"]);
    try {
      await session.waitFor("startup", (screen) =>
        screen.includes("v2.0.9") && screen.includes("Enter submit"),
      );
      if (input === "\u0003") {
        session.child.write(input);
      } else {
        await submit(session, input);
      }
      const exit = await session.waitForExit();
      const raw = session.raw();
      assertTerminalLifecycle(raw, label);
      assertNoSecrets(raw, label);
      results.push({
        label,
        exit_code: exit.exitCode,
        output_bytes: Buffer.byteLength(raw, "utf8"),
        conpty_focus_disable_echoed: raw.includes("\u001b[?1004l"),
        focus_disable_action_unit_tested: true,
      });
    } finally {
      if (!session.hasExited()) session.child.kill();
      session.terminal.dispose();
    }
  }
  return results;
}

async function listen(server) {
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  return `http://127.0.0.1:${address.port}`;
}

async function closeServer(server) {
  if (typeof server.closeAllConnections === "function") {
    server.closeAllConnections();
  }
  await new Promise((resolve) => server.close(resolve));
}

async function verifyProviderRecovery() {
  let requests = 0;
  const server = http.createServer((request, response) => {
    request.resume();
    requests += 1;
    response.setHeader("Content-Type", "application/json");
    if (requests <= 2) {
      response.statusCode = 400;
      response.end(JSON.stringify({
        error: {
          message: requests === 1 ? "unknown field metadata" : "fixture request rejected",
        },
      }));
      return;
    }
    response.end(JSON.stringify({
      choices: [{ message: { role: "assistant", content: "SECOND_TURN_OK" } }],
    }));
  });
  const baseUrl = await listen(server);
  const session = startTui(
    "provider-recovery",
    ["--provider-live", "--provider", "deepseek", "--model", "deepseek-chat"],
    {
      YUNXI_PROVIDER_PROFILE: "deepseek",
      YUNXI_PROVIDER_BASE_URL: baseUrl,
      YUNXI_PROVIDER_STREAM: "false",
      DEEPSEEK_API_KEY: "fixture-provider-key",
    },
  );
  try {
    await session.waitFor("startup", (screen) => screen.includes("Enter submit"));
    await submit(session, "first prompt");
    await session.waitFor("provider error", (screen) =>
      screen.includes("YX-PROVIDER-001") && screen.includes("Enter submit"),
    );
    await submit(session, "second prompt");
    await session.waitFor("second turn", (screen) =>
      screen.includes("SECOND_TURN_OK") && screen.includes("Enter submit"),
    );
    await submit(session, "/exit");
    const exit = await session.waitForExit();
    const raw = session.raw();
    assertTerminalLifecycle(raw, "provider-recovery");
    assertNoSecrets(raw.replaceAll("fixture-provider-key", "[redacted]"), "provider-recovery");
    return { requests, exit_code: exit.exitCode, recovered: raw.includes("SECOND_TURN_OK") };
  } finally {
    if (!session.hasExited()) session.child.kill();
    session.terminal.dispose();
    await closeServer(server);
  }
}

async function verifyLongStreamCancellation() {
  let requests = 0;
  let streamedBytes = 0;
  const server = http.createServer((request, response) => {
    request.resume();
    requests += 1;
    response.statusCode = 200;
    response.setHeader("Content-Type", "text/event-stream");
    if (requests === 1) {
      let chunks = 0;
      const timer = setInterval(() => {
        const content = `LONG_STREAM_${chunks}_` + "x".repeat(8192);
        const payload = `data: ${JSON.stringify({ choices: [{ delta: { content } }] })}\n\n`;
        streamedBytes += Buffer.byteLength(payload, "utf8");
        response.write(payload);
        chunks += 1;
        if (chunks >= 48) clearInterval(timer);
      }, 8);
      response.on("close", () => clearInterval(timer));
      return;
    }
    response.end(
      `data: ${JSON.stringify({ choices: [{ delta: { content: "RECOVERED_AFTER_CANCEL" }, finish_reason: "stop" }] })}\n\ndata: [DONE]\n\n`,
    );
  });
  const baseUrl = await listen(server);
  const session = startTui(
    "stream-cancel",
    ["--provider-live", "--provider", "deepseek", "--model", "deepseek-chat"],
    {
      YUNXI_PROVIDER_PROFILE: "deepseek",
      YUNXI_PROVIDER_BASE_URL: baseUrl,
      YUNXI_PROVIDER_STREAM: "true",
      DEEPSEEK_API_KEY: "fixture-stream-key",
    },
  );
  try {
    await session.waitFor("startup", (screen) => screen.includes("Enter submit"));
    await submit(session, "start long stream");
    try {
      await session.waitFor("active long stream", (screen) =>
        screen.includes("Ctrl+C cancel") && streamedBytes > 300 * 1024,
      );
    } catch (error) {
      throw new Error(
        `${error.message}; requests=${requests}; streamed_bytes=${streamedBytes}; screen=${JSON.stringify(session.screen())}`,
      );
    }
    session.child.write("\u0003");
    await session.waitFor("cancelled turn", (screen) =>
      screen.toLowerCase().includes("cancel") && screen.includes("Enter submit"),
    );
    await submit(session, "next prompt");
    try {
      await session.waitFor("post-cancel recovery", (screen) =>
        screen.includes("RECOVERED_AFTER_CANCEL") && screen.includes("Enter submit"),
      );
    } catch (error) {
      throw new Error(
        `${error.message}; requests=${requests}; streamed_bytes=${streamedBytes}; screen=${JSON.stringify(session.screen())}`,
      );
    }
    await submit(session, "/exit");
    const exit = await session.waitForExit();
    const raw = session.raw();
    assertTerminalLifecycle(raw, "stream-cancel");
    assertNoSecrets(raw.replaceAll("fixture-stream-key", "[redacted]"), "stream-cancel");
    return {
      requests,
      streamed_bytes: streamedBytes,
      exit_code: exit.exitCode,
      recovered: raw.includes("RECOVERED_AFTER_CANCEL"),
    };
  } finally {
    if (!session.hasExited()) session.child.kill();
    session.terminal.dispose();
    await closeServer(server);
  }
}

async function main() {
  if (process.argv.includes("--stream-only")) {
    console.log(JSON.stringify(await verifyLongStreamCancellation()));
    return;
  }
  const evidence = {
    schema_version: 1,
    version: "2.0.9",
    terminal: "Windows ConPTY",
    generated_at: new Date().toISOString(),
    mode_matrix: runNonTuiMatrix(),
    terminal_recovery: await verifyTerminalRecovery(),
    provider_recovery: await verifyProviderRecovery(),
    stream_cancellation: await verifyLongStreamCancellation(),
  };
  const raw = `${JSON.stringify(evidence, null, 2)}\n`;
  assertNoSecrets(raw, "v209 evidence");
  const evidencePath = path.join(outputDir, "resilience.json");
  fs.writeFileSync(evidencePath, raw);
  const manifest = {
    schema_version: 1,
    version: "2.0.9",
    terminal: "Windows ConPTY",
    collector: "scripts/conpty/v209/verify.js",
    evidence: "resilience.json",
    sha256: crypto.createHash("sha256").update(raw).digest("hex"),
  };
  fs.writeFileSync(
    path.join(outputDir, "manifest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  console.log(JSON.stringify({ ok: true, evidence: evidencePath, manifest }));
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error.stack || error);
    process.exit(1);
  });

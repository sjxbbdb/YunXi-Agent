const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");
const pty = require("node-pty");
const { Terminal } = require("@xterm/headless");

const repoRoot = path.resolve(__dirname, "..", "..", "..");
const binary = path.join(repoRoot, "target", "release", "yunxi.exe");
const outputDir = path.resolve(
  optionValue("--output-dir") || path.join(repoRoot, ".tmp", "conpty", "v210-capture"),
);
const workRoot = path.resolve(
  optionValue("--work-dir") || path.join(repoRoot, ".tmp", "conpty", "v210-work"),
);
const timeoutMilliseconds = 120000;
const forbiddenSecrets = [
  /github_pat_[A-Za-z0-9_]+/,
  /gh[pousr]_[A-Za-z0-9_]+/,
  /Bearer\s+[A-Za-z0-9._-]+/i,
  /\bsk-[A-Za-z0-9_-]+/,
];

if (process.platform !== "win32") throw new Error("v2.1.0 ConPTY capture requires Windows");
if (!fs.existsSync(binary)) throw new Error(`release binary is missing: ${binary}`);
fs.mkdirSync(outputDir, { recursive: true });
fs.mkdirSync(workRoot, { recursive: true });

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

function cleanEnvironment(extra = {}) {
  const env = { ...process.env, ...extra };
  delete env.NO_COLOR;
  delete env.CI;
  delete env.YUNXI_PROVIDER_API_KEY;
  delete env.YUNXI_PROVIDER_API_KEY_ENV;
  delete env.OPENAI_API_KEY;
  delete env.DEEPSEEK_API_KEY;
  delete env.GH_TOKEN;
  return env;
}

function runBaselineCapture() {
  const existing = optionValue("--baseline-input-dir");
  if (existing) {
    const baseline = JSON.parse(
      fs.readFileSync(path.join(path.resolve(existing), "resilience.json"), "utf8"),
    );
    if (baseline.version !== "2.1.0" || baseline.terminal !== "Windows ConPTY") {
      throw new Error("reused baseline metadata is invalid");
    }
    return baseline;
  }
  const baselineDir = path.join(workRoot, "v209-baseline-evidence");
  const baselineWork = path.join(workRoot, "v209-baseline-work");
  const result = spawnSync(
    process.execPath,
    [
      path.join(repoRoot, "scripts", "conpty", "v209", "capture.js"),
      "--output-dir",
      baselineDir,
      "--work-dir",
      baselineWork,
      "--evidence-version",
      "2.1.0",
    ],
    {
      cwd: repoRoot,
      env: cleanEnvironment(),
      encoding: "utf8",
      timeout: 10 * 60 * 1000,
      maxBuffer: 4 * 1024 * 1024,
    },
  );
  if (result.error || result.status !== 0) {
    throw new Error(`baseline capture failed: ${result.error || result.stderr || result.stdout}`);
  }
  return JSON.parse(fs.readFileSync(path.join(baselineDir, "resilience.json"), "utf8"));
}

function startTui() {
  const cwd = path.join(workRoot, "wide-resize-copy-smoke");
  fs.mkdirSync(cwd, { recursive: true });
  const terminal = new Terminal({ cols: 100, rows: 30, scrollback: 2000, allowProposedApi: true });
  const child = pty.spawn(binary, ["--offline", "--cwd", cwd, "--tui"], {
    name: "xterm-256color",
    cols: 100,
    rows: 30,
    cwd,
    env: cleanEnvironment({ TERM: "xterm-256color", COLORTERM: "truecolor" }),
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
      if (predicate(screen(), raw)) return screen();
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error(`v2.1.0 smoke timed out waiting for ${description}`);
  };
  const waitForExit = async () => {
    const started = Date.now();
    while (Date.now() - started < timeoutMilliseconds) {
      if (exit) return exit;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error("v2.1.0 smoke did not exit");
  };
  return { child, terminal, screen, waitFor, waitForExit, raw: () => raw, exited: () => exit !== null };
}

function assertLifecycle(raw) {
  for (const token of [
    "\u001b[?1049h",
    "\u001b[?1049l",
    "\u001b[?2004h",
    "\u001b[?2004l",
    "\u001b[?1004h",
    "\u001b[?25l",
    "\u001b[?25h",
  ]) {
    if (!raw.includes(token)) throw new Error(`integrated smoke missing ${JSON.stringify(token)}`);
  }
}

async function captureWideResizeCopySmoke() {
  const session = startTui();
  try {
    await session.waitFor("startup", (screen) => screen.includes("v2.1.0") && screen.includes("Enter submit"));
    const wideText = "你好，云汐 🌟 é — COPYABLE_WIDE_TEXT";
    session.child.write(`\u001b[200~${wideText}\u001b[201~`);
    const wideScreen = await session.waitFor("wide-character input", (screen) =>
      screen.includes("你好") && screen.includes("🌟") && screen.includes("COPYABLE_WIDE_TEXT"),
    );
    session.child.write("\u001b[<64;10;8M\u001b[<65;10;8M");
    session.child.resize(58, 18);
    session.terminal.resize(58, 18);
    const narrowScreen = await session.waitFor("58x18 resize", (screen) =>
      screen.includes("YunXi v2.1.0") && screen.includes("Ctrl+C"),
    );
    session.child.write("\u0003");
    const exit = await session.waitForExit();
    const raw = session.raw();
    assertLifecycle(raw);
    assertNoSecrets(raw, "v2.1.0 integrated smoke");
    return {
      exit_code: exit.exitCode,
      output_bytes: Buffer.byteLength(raw, "utf8"),
      wide_characters_visible: wideScreen.includes("你好") && wideScreen.includes("🌟"),
      copyable_text_sha256: sha256(Buffer.from(wideScreen, "utf8")),
      copy_boundary_quiet: !wideScreen.includes("PRIVATE_") && !wideScreen.includes("Bearer "),
      mouse_sequences_sent: 2,
      resized_cols: session.terminal.cols,
      resized_rows: session.terminal.rows,
      narrow_footer_visible: narrowScreen.includes("Ctrl+C"),
      ansi_reset_conpty_echoed: raw.includes("\u001b[0m"),
      ansi_reset_action_unit_tested: true,
      alternate_screen_restored: raw.includes("\u001b[?1049l"),
      cursor_restored: raw.includes("\u001b[?25h"),
    };
  } finally {
    if (!session.exited()) session.child.kill();
    session.terminal.dispose();
  }
}

function goldenEvidence() {
  return [
    "crates/yunxi-agent-tui/src/snapshots/integrated_release_v210.txt",
    "crates/yunxi-agent-tui/src/snapshots/vt100_lifecycle_v210.txt",
  ].map((relativePath) => {
    const raw = fs.readFileSync(path.join(repoRoot, ...relativePath.split("/")));
    return { path: relativePath, bytes: raw.length, sha256: sha256(raw) };
  });
}

async function main() {
  const evidence = {
    schema_version: 1,
    version: "2.1.0",
    terminal: "Windows ConPTY",
    generated_at: new Date().toISOString(),
    baseline: runBaselineCapture(),
    integrated_smoke: await captureWideResizeCopySmoke(),
    goldens: goldenEvidence(),
  };
  const raw = `${JSON.stringify(evidence, null, 2)}\n`;
  assertNoSecrets(raw, "v2.1.0 evidence");
  fs.writeFileSync(path.join(outputDir, "integrated.json"), raw);
  const manifest = {
    schema_version: 1,
    version: "2.1.0",
    terminal: "Windows ConPTY",
    collector: "scripts/conpty/v210/capture.js",
    evidence: "integrated.json",
    sha256: sha256(raw),
  };
  fs.writeFileSync(path.join(outputDir, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(JSON.stringify({ ok: true, capture: outputDir, manifest }));
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error.stack || error);
    process.exit(1);
  });

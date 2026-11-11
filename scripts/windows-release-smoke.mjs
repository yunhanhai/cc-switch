// Run only on a fresh GitHub-hosted Windows runner. This deliberately uses the
// real Windows profile: HOME/APPDATA overrides do not isolate KnownFolder APIs.
import assert from "node:assert/strict";
import { spawn, execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { createServer as createTcpServer } from "node:net";
import { promisify } from "node:util";
import { mkdir, readFile, writeFile, stat, cp } from "node:fs/promises";
import { resolve, join, relative, isAbsolute } from "node:path";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const args = Object.fromEntries(
  process.argv.slice(2).reduce((pairs, value, index, values) => {
    if (value.startsWith("--")) pairs.push([value.slice(2), values[index + 1]]);
    return pairs;
  }, []),
);
const delay = (ms) => new Promise((done) => setTimeout(done, ms));
const exists = async (path) =>
  stat(path).then(
    () => true,
    () => false,
  );
const sha256 = async (path) =>
  createHash("sha256")
    .update(await readFile(path))
    .digest("hex");
const psQuote = (value) => `'${value.replaceAll("'", "''")}'`;
const evidence = {
  startedAt: new Date().toISOString(),
  checks: [],
  status: "running",
};
let output;
let installed = false;
let currentApp;
let mock;
let msi;
let expectedVersion;

async function powershell(code) {
  const encoded = Buffer.from(
    `$ErrorActionPreference='Stop'; ${code}`,
    "utf16le",
  ).toString("base64");
  const result = await execFileAsync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-EncodedCommand", encoded],
    {
      windowsHide: true,
      maxBuffer: 2 * 1024 * 1024,
      timeout: 120_000,
    },
  );
  return result.stdout.trim();
}

function check(name, details = {}) {
  evidence.checks.push({ name, status: "passed", ...details });
  process.stdout.write(`PASS ${name}\n`);
}

async function until(action, description, timeout = 60_000) {
  const deadline = Date.now() + timeout;
  let last;
  while (Date.now() < deadline) {
    try {
      const result = await action();
      if (result) return result;
    } catch (error) {
      last = error;
    }
    await delay(400);
  }
  throw new Error(
    `Timed out: ${description}${last ? ` (${last.message})` : ""}`,
  );
}

async function freshRunnerGuard() {
  assert.equal(process.platform, "win32", "Windows only");
  assert.equal(
    process.env.GITHUB_ACTIONS,
    "true",
    "Never run against a personal Windows profile",
  );
  assert.equal(
    process.env.RUNNER_ENVIRONMENT,
    "github-hosted",
    "Self-hosted runners may contain live credentials",
  );
  assert.equal(process.env.RUNNER_OS, "Windows");
  assert.ok(
    process.env.RUNNER_TEMP &&
      process.env.GITHUB_WORKSPACE &&
      process.env.USERPROFILE,
  );
  for (const key of [
    "CC_SWITCH_TEST_HOME",
    "HERMES_HOME",
    "OPENCODE_DB",
    "XDG_DATA_HOME",
    "XDG_CONFIG_HOME",
    "PI_CODING_AGENT_DIR",
    "PI_CODING_AGENT_SESSION_DIR",
    "CLAUDE_CONFIG_DIR",
    "CODEX_HOME",
  ])
    assert.ok(
      !process.env[key],
      `Unexpected external configuration override: ${key}`,
    );
  for (const path of [
    ".cc-switch",
    ".cc-switch-dev",
    ".yunswitch",
    ".yunswitch-dev",
    ".claude",
    ".claude.json",
    ".codex",
    ".gemini",
    ".grok",
    ".openclaw",
    ".hermes",
    ".pi",
    ".config/opencode",
  ])
    assert.ok(
      !(await exists(join(process.env.USERPROFILE, path))),
      `Profile is not fresh: ${path}`,
    );
  const existing = await powershell(
    "@(Get-Process cc-switch -ErrorAction SilentlyContinue).Count",
  );
  assert.equal(
    Number(existing),
    0,
    "An existing app may contain live credentials",
  );
  const registryPresent = await powershell(
    String.raw`[bool]((Test-Path 'HKCU:\Software\Classes\yunswitch') -or (Test-Path 'HKCU:\Software\Classes\ccswitch'))`,
  );
  assert.equal(
    registryPresent,
    "False",
    "An existing protocol registration must not be replaced",
  );
  for (const key of ["APPDATA", "LOCALAPPDATA"]) {
    assert.ok(process.env[key]);
    const child = relative(process.env.USERPROFILE, process.env[key]);
    assert.ok(
      child && !child.startsWith("..") && !isAbsolute(child),
      `${key} points outside this runner profile`,
    );
  }
  assert.ok(
    !(await exists(join(process.env.APPDATA, "io.github.yunhanhai.yunswitch"))),
    "Existing Tauri Store",
  );
  assert.ok(
    !(await exists(
      join(process.env.LOCALAPPDATA, "io.github.yunhanhai.yunswitch"),
    )),
    "Existing WebView profile",
  );
  check("fresh GitHub-hosted Windows profile");
}

class Cdp {
  constructor(socket) {
    this.socket = socket;
    this.nextId = 0;
    this.pending = new Map();
    socket.addEventListener("message", ({ data }) => {
      const message = JSON.parse(String(data));
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      clearTimeout(pending.timer);
      if (message.error)
        pending.reject(new Error(JSON.stringify(message.error)));
      else pending.resolve(message.result);
    });
    socket.addEventListener("close", () => {
      for (const pending of this.pending.values()) {
        clearTimeout(pending.timer);
        pending.reject(new Error("WebView CDP connection closed"));
      }
      this.pending.clear();
    });
  }
  async send(method, params = {}) {
    const id = ++this.nextId;
    return new Promise((resolvePromise, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`CDP ${method} timed out`));
      }, 30_000);
      this.pending.set(id, { resolve: resolvePromise, reject, timer });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async evaluate(expression) {
    const result = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (result.exceptionDetails)
      throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
  invoke(command, payload = {}) {
    return this.evaluate(
      `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(payload)})`,
    );
  }
  async screenshot(filename) {
    const result = await this.send("Page.captureScreenshot", { format: "png" });
    await writeFile(join(output, filename), Buffer.from(result.data, "base64"));
  }
  close() {
    this.socket.close();
  }
}

async function freePort() {
  const server = createTcpServer();
  await new Promise((done, reject) =>
    server.once("error", reject).listen(0, "127.0.0.1", done),
  );
  const { port } = server.address();
  await new Promise((done) => server.close(done));
  return port;
}

async function launch(exe, label) {
  const port = await freePort();
  const childEnvironment = Object.fromEntries(
    Object.entries(process.env).filter(
      ([key]) => !/(TOKEN|PASSWORD|SECRET|SIGNING_PRIVATE_KEY)$/i.test(key),
    ),
  );
  const processHandle = spawn(exe, [], {
    cwd: resolve(exe, ".."),
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
    env: {
      ...childEnvironment,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,
    },
  });
  const chunks = [];
  processHandle.stdout.on("data", (chunk) => chunks.push(chunk));
  processHandle.stderr.on("data", (chunk) => chunks.push(chunk));
  let exited = false;
  let exitCode;
  processHandle.once("exit", (code) => {
    exited = true;
    exitCode = code;
  });
  processHandle.once("error", (error) => {
    chunks.push(Buffer.from(error.message));
    exited = true;
  });
  currentApp = {
    processHandle,
    label,
    chunks,
    exited: () => exited,
    exitCode: () => exitCode,
  };
  const target = await until(async () => {
    assert.ok(!exited, `${label} exited before renderer initialization`);
    const response = await fetch(`http://127.0.0.1:${port}/json/list`, {
      signal: AbortSignal.timeout(2000),
    });
    const targets = await response.json();
    return targets.find(
      (item) => item.type === "page" && /tauri\.localhost/.test(item.url),
    );
  }, `${label} WebView target`);
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((done, reject) => {
    socket.addEventListener("open", done, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  const cdp = new Cdp(socket);
  currentApp.cdp = cdp;
  await until(
    () =>
      cdp.evaluate(
        "!!window.__TAURI_INTERNALS__ && !!document.querySelector('#root')?.children.length && document.body.innerText.includes('YunSwitch')",
      ),
    `${label} rendered UI`,
  );
  assert.equal(
    await cdp.invoke("get_init_error"),
    null,
    `${label} backend initialization failed`,
  );
  check(`${label} backend and renderer initialize`);
  return cdp;
}

async function verifySingleInstance(exe) {
  const second = spawn(exe, [], { windowsHide: true, stdio: "ignore" });
  let done = false;
  let result;
  second.once("exit", (code) => {
    done = true;
    result = code;
  });
  second.once("error", (error) => {
    done = true;
    result = error;
  });
  try {
    await until(() => done, "second application instance exits", 15_000);
    assert.equal(result, 0);
    assert.ok(!currentApp.exited());
    assert.equal(await currentApp.cdp.invoke("plugin:app|name"), "YunSwitch");
    check("second installed instance hands off to the existing instance");
  } finally {
    if (!done) second.kill();
  }
}

async function exitApp() {
  const app = currentApp;
  if (!app) return;
  if (!app.exited()) {
    await app.cdp?.invoke("plugin:process|exit", { code: 0 }).catch(() => {});
    await until(() => app.exited(), `${app.label} graceful exit`, 20_000);
  }
  assert.equal(app.exitCode(), 0, `${app.label} exit status`);
  app.cdp?.close();
  await writeFile(
    join(output, `${app.label}-process.log`),
    Buffer.concat(app.chunks),
  );
  currentApp = undefined;
  check(`${app.label} exits normally`);
}

async function verifyIdentity(cdp, portable, version = expectedVersion) {
  assert.equal(await cdp.invoke("plugin:app|name"), "YunSwitch");
  assert.equal(
    await cdp.invoke("plugin:app|identifier"),
    "io.github.yunhanhai.yunswitch",
  );
  assert.equal(await cdp.invoke("plugin:app|version"), version);
  assert.equal(await cdp.invoke("is_portable_mode"), portable);
  const identity = await cdp.invoke("get_runtime_directory_identity");
  assert.equal(
    resolve(identity.defaultAppConfigDir),
    resolve(process.env.USERPROFILE, ".yunswitch"),
  );
  assert.equal(identity.isolatedConfigRoot, null);
  assert.equal(await cdp.invoke("get_app_config_dir_override"), null);
  assert.equal(
    resolve(await cdp.invoke("get_app_config_path")),
    resolve(process.env.USERPROFILE, ".yunswitch", "config.json"),
  );
  const config = await cdp.invoke("get_global_proxy_config");
  assert.equal(config.listenPort, 15731);
  assert.equal(config.listenAddress, "127.0.0.1");
  assert.equal(await cdp.invoke("is_app_updater_configured"), false);
  const query = "v1/import?resource=provider&app=claude&name=Smoke";
  assert.equal(
    (await cdp.invoke("parse_deeplink", { url: `yunswitch://${query}` })).name,
    "Smoke",
  );
  await assert.rejects(() =>
    cdp.invoke("parse_deeplink", { url: `ccswitch://${query}` }),
  );
  check(
    portable ? "portable production identity" : "installed production identity",
    { identity, config },
  );
}

async function exerciseProxy(cdp) {
  const requests = [];
  mock = createServer(async (request, response) => {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    requests.push({
      path: request.url,
      authorization: request.headers.authorization,
      body: JSON.parse(Buffer.concat(chunks).toString()),
    });
    response.writeHead(200, { "content-type": "application/json" });
    response.end(
      JSON.stringify({
        id: "msg_smoke",
        type: "message",
        role: "assistant",
        model: "claude-sonnet-4-5",
        content: [{ type: "text", text: "YunSwitch smoke response" }],
        stop_reason: "end_turn",
        stop_sequence: null,
        usage: { input_tokens: 1, output_tokens: 4 },
      }),
    );
  });
  await new Promise((done) => mock.listen(0, "127.0.0.1", done));
  const provider = {
    id: "yunswitch-ci-smoke",
    name: "CI synthetic local provider",
    category: "custom",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: `http://127.0.0.1:${mock.address().port}`,
        ANTHROPIC_AUTH_TOKEN: "fake-yunswitch-ci-key",
      },
    },
  };
  assert.equal(
    await cdp.invoke("add_provider", {
      app: "claude",
      provider,
      addToLive: false,
    }),
    true,
  );
  await cdp.invoke("switch_provider", { app: "claude", id: provider.id });
  assert.equal(
    await cdp.invoke("get_current_provider", { app: "claude" }),
    provider.id,
  );
  const info = await cdp.invoke("start_proxy_server");
  assert.equal(info.port, 15731);
  const response = await fetch("http://127.0.0.1:15731/v1/messages", {
    method: "POST",
    signal: AbortSignal.timeout(20_000),
    headers: {
      "content-type": "application/json",
      "anthropic-version": "2023-06-01",
      "x-api-key": "fake-client-key",
    },
    body: JSON.stringify({
      model: "claude-sonnet-4-5",
      max_tokens: 32,
      messages: [{ role: "user", content: "synthetic smoke request" }],
    }),
  });
  assert.equal(
    response.status,
    200,
    await response.text().then((text) => {
      evidence.proxyResponse = text;
      return text;
    }),
  );
  const body = JSON.parse(evidence.proxyResponse);
  assert.equal(body.content[0].text, "YunSwitch smoke response");
  assert.equal(requests.length, 1);
  assert.equal(requests[0].path, "/v1/messages");
  assert.equal(requests[0].authorization, "Bearer fake-yunswitch-ci-key");
  assert.equal(requests[0].body.messages[0].content, "synthetic smoke request");
  await cdp.invoke("stop_proxy_server");
  assert.equal((await cdp.invoke("get_proxy_status")).running, false);
  check("local proxy forwards a synthetic request and upstream credential", {
    requests: requests.length,
  });
  await new Promise((done) => mock.close(done));
  mock = undefined;
}

async function exerciseMigration(cdp) {
  const repoRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
  const source = await readFile(
    join(repoRoot, "src-tauri/src/database/tests.rs"),
    "utf8",
  );
  const schema = source.match(
    /const V3_8_SCHEMA_V1_SQL: &str = r#"([\s\S]*?)"#;/,
  )?.[1];
  assert.ok(schema, "Legacy schema fixture must remain available");
  const config = JSON.stringify({
    env: {
      ANTHROPIC_AUTH_TOKEN: "fake-legacy-key",
      ANTHROPIC_BASE_URL: "http://127.0.0.1:1",
    },
  });
  const sql = `-- CC Switch SQLite 导出\nPRAGMA foreign_keys=OFF;\nPRAGMA user_version=1;\nBEGIN TRANSACTION;\n${schema}\nINSERT INTO providers (id,app_type,name,settings_config,meta,is_current) VALUES ('legacy-ci','claude','Synthetic legacy provider','${config}','{}',1);\nCOMMIT;\nPRAGMA foreign_keys=ON;\n`;
  const input = join(output, "synthetic-legacy-v1.sql");
  const after = join(output, "migrated-schema.sql");
  await writeFile(input, sql);
  const beforeHash = await sha256(input);
  const imported = await cdp.invoke("import_config_from_file", {
    filePath: input,
  });
  assert.equal(imported.success, true);
  const providers = await cdp.invoke("get_providers", { app: "claude" });
  assert.equal(providers["legacy-ci"].name, "Synthetic legacy provider");
  assert.deepEqual(providers["legacy-ci"].settingsConfig, JSON.parse(config));
  assert.equal(
    (await cdp.invoke("export_config_to_file", { filePath: after })).success,
    true,
  );
  const migrated = await readFile(after, "utf8");
  assert.match(migrated, /PRAGMA user_version=20;/);
  assert.match(migrated, /legacy-ci/);
  assert.equal(
    await sha256(input),
    beforeHash,
    "Import must not alter the input backup",
  );
  check("synthetic legacy schema v1 imports and migrates to v20", {
    inputSha256: beforeHash,
    inputUnchanged: true,
  });
}

async function installOrUninstall(action, packagePath = msi, label) {
  const log = join(
    output,
    `${label || (action === "/i" ? "install" : "uninstall")}.log`,
  );
  // Do not pass AUTOLAUNCHAPP: both "0" and an empty trailing assignment
  // can be interpreted as a present property and launch an extra instance.
  const result = await execFileAsync(
    "msiexec.exe",
    [
      action,
      packagePath,
      "/qn",
      "/norestart",
      "/l*v",
      log,
      "WIXUI_EXITDIALOGOPTIONALCHECKBOX=0",
    ],
    {
      windowsHide: true,
      timeout: 180_000,
    },
  ).catch((error) => {
    if (error.code === 3010) return { rebootRequired: true };
    throw error;
  });
  check(
    action === "/i"
      ? "per-user MSI installs silently"
      : "per-user MSI uninstalls silently",
    result.rebootRequired ? { rebootRequired: true } : {},
  );
}

async function readMsiProperties(packagePath) {
  return JSON.parse(
    await powershell(`
      $installer = New-Object -ComObject WindowsInstaller.Installer
      $database = $installer.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $installer, @(${psQuote(packagePath)}, 0))
      $view = $database.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $database, @('SELECT Property, Value FROM Property'))
      $view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null) | Out-Null
      $properties = @{}
      while ($record = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)) {
        $key = $record.GetType().InvokeMember('StringData', 'GetProperty', $null, $record, @(1))
        if ($key -in @('ProductCode', 'ProductVersion', 'UpgradeCode', 'ProductName', 'Manufacturer')) {
          $properties[$key] = $record.GetType().InvokeMember('StringData', 'GetProperty', $null, $record, @(2))
        }
      }
      $properties | ConvertTo-Json -Compress
    `),
  );
}

async function productState(productCode) {
  return Number(
    await powershell(`
      $installer = New-Object -ComObject WindowsInstaller.Installer
      $installer.GetType().InvokeMember('ProductState', 'GetProperty', $null, $installer, @(${psQuote(productCode)}))
    `),
  );
}

async function verifyUpgrade(previous, current, installedExe, portableExe) {
  installed = true;
  const currentMsi = msi;
  // Cleanup must uninstall the version currently installed if a later step fails.
  msi = previous.path;
  await installOrUninstall("/i", previous.path, "previous-version-install");
  assert.equal(await productState(previous.properties.ProductCode), 5);
  let cdp = await launch(installedExe, "installed-previous-version");
  await verifyIdentity(cdp, false, previous.version);
  assert.equal(
    (await cdp.invoke("get_providers", { app: "claude" }))["legacy-ci"].name,
    "Synthetic legacy provider",
  );
  const marker = {
    id: "upgrade-ci",
    name: "Synthetic provider created by previous YunSwitch version",
    category: "custom",
    settingsConfig: {
      env: {
        ANTHROPIC_AUTH_TOKEN: "fake-upgrade-key",
        ANTHROPIC_BASE_URL: "http://127.0.0.1:1",
      },
    },
  };
  assert.equal(
    await cdp.invoke("add_provider", {
      app: "claude",
      provider: marker,
      addToLive: false,
    }),
    true,
  );
  const providersBefore = await cdp.invoke("get_providers", { app: "claude" });
  const settingsBefore = await cdp.invoke("get_settings");
  await cdp.screenshot("installed-previous-version.png");
  await exitApp();
  const databasePath = join(
    process.env.USERPROFILE,
    ".yunswitch",
    "cc-switch.db",
  );
  const databaseBefore = await sha256(databasePath);
  msi = currentMsi;
  await installOrUninstall("/i", currentMsi, "upgrade-install");
  assert.equal(
    await sha256(databasePath),
    databaseBefore,
    "MSI upgrade must not modify the application database",
  );
  assert.equal(await productState(previous.properties.ProductCode), -1);
  assert.equal(await productState(current.ProductCode), 5);
  assert.equal(await sha256(installedExe), await sha256(portableExe));
  const upgradeLogBytes = await readFile(join(output, "upgrade-install.log"));
  const upgradeLog = upgradeLogBytes.toString(
    upgradeLogBytes[0] === 0xff && upgradeLogBytes[1] === 0xfe
      ? "utf16le"
      : "utf8",
  );
  assert.ok(
    upgradeLog.includes(previous.properties.ProductCode),
    "Upgrade log must reference the replaced MSI product",
  );
  assert.match(upgradeLog, /RemoveExistingProducts/);
  cdp = await launch(installedExe, "installed-after-upgrade");
  await verifyIdentity(cdp, false);
  assert.deepEqual(
    await cdp.invoke("get_providers", { app: "claude" }),
    providersBefore,
    "MSI upgrade must retain providers created by the previous version",
  );
  assert.deepEqual(await cdp.invoke("get_settings"), settingsBefore);
  const command = await powershell(
    String.raw`(Get-Item 'HKCU:\Software\Classes\yunswitch\shell\open\command').GetValue('')`,
  );
  const registeredExe = command.match(/^"([^"]+)"\s+"%1"$/)?.[1];
  assert.ok(registeredExe);
  assert.equal(await sha256(registeredExe), await sha256(installedExe));
  assert.ok(
    await exists(
      join(
        process.env.APPDATA,
        "Microsoft",
        "Windows",
        "Start Menu",
        "Programs",
        "YunSwitch",
        "YunSwitch.lnk",
      ),
    ),
    "Upgrade must retain the YunSwitch Start Menu shortcut",
  );
  const upgradeLicense = await readFile(
    join(process.env.LOCALAPPDATA, "Programs", "YunSwitch", "LICENSE.rtf"),
    "utf8",
  );
  assert.match(upgradeLicense, /Jason Young/);
  assert.match(upgradeLicense, /yunhanhai/);
  if (args["require-third-party-notices"] === "true") {
    await verifyThirdPartyNotices(
      join(process.env.LOCALAPPDATA, "Programs", "YunSwitch"),
      "upgraded MSI",
    );
  }
  await verifySingleInstance(installedExe);
  await cdp.screenshot("installed-after-upgrade.png");
  await exitApp();
  check(
    "previous prerelease upgrades to current MSI and retains synthetic data",
    {
      previousVersion: previous.version,
      currentVersion: expectedVersion,
      previousProductCode: previous.properties.ProductCode,
      currentProductCode: current.ProductCode,
      previousProductRemoved: true,
      databaseUnchangedDuringInstall: true,
      databaseBeforeUpgradeSha256: databaseBefore,
      providerIds: Object.keys(providersBefore).sort(),
    },
  );
}

async function verifyThirdPartyNotices(packageRoot, label) {
  const repoRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
  for (const path of [
    "LICENSE",
    "THIRD-PARTY-NOTICES.md",
    "licenses/CodexBar-MIT.txt",
    "licenses/LobeHub-Icons-MIT.txt",
    "licenses/AgentRouter-Responses-Shim-MIT.txt",
  ]) {
    const expected = await readFile(join(repoRoot, path));
    const actual = await readFile(join(packageRoot, path));
    assert.deepEqual(actual, expected, `${label}: bundled ${path} differs`);
  }
  check(`${label} includes exact third-party notices and license copies`);
}

async function verifyNoticeUninstall(packageRoot, label) {
  if (args["require-third-party-notices"] !== "true") return;
  for (const path of ["LICENSE", "THIRD-PARTY-NOTICES.md", "licenses"]) {
    assert.ok(
      !(await exists(join(packageRoot, path))),
      `${label}: ${path} remains`,
    );
  }
  assert.equal(
    await powershell(
      String.raw`[bool](Test-Path 'HKCU:\Software\yunhanhai\YunSwitch\ThirdPartyNotices')`,
    ),
    "False",
  );
  check(`${label} removes third-party notices, license directory and registry`);
}

async function main() {
  await freshRunnerGuard();
  assert.ok(
    args.msi && args.zip && args.output,
    "Usage: --msi file --zip file --output directory",
  );
  msi = resolve(args.msi);
  const zip = resolve(args.zip);
  output = resolve(args.output);
  const outputRelative = relative(process.env.RUNNER_TEMP, output);
  assert.ok(
    outputRelative &&
      !outputRelative.startsWith("..") &&
      !isAbsolute(outputRelative),
    "Evidence must stay in RUNNER_TEMP",
  );
  assert.ok(!(await exists(output)), "Evidence directory must be new");
  await mkdir(output, { recursive: true });
  const repoRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
  const pkg = JSON.parse(
    await readFile(join(repoRoot, "package.json"), "utf8"),
  );
  const tauri = JSON.parse(
    await readFile(join(repoRoot, "src-tauri/tauri.conf.json"), "utf8"),
  );
  expectedVersion = pkg.version;
  assert.match(expectedVersion, /^\d+\.\d+\.\d+-yunswitch\.\d+$/);
  assert.equal(tauri.version, expectedVersion);
  const currentMsiProperties = await readMsiProperties(msi);
  assert.equal(currentMsiProperties.ProductName, "YunSwitch");
  assert.equal(currentMsiProperties.Manufacturer, "yunhanhai");
  assert.equal(
    currentMsiProperties.ProductVersion,
    tauri.bundle.windows.wix.version,
  );
  assert.equal(
    currentMsiProperties.UpgradeCode.toLowerCase(),
    `{${tauri.bundle.windows.wix.upgradeCode.toLowerCase()}}`,
  );
  evidence.artifacts = {
    msi: {
      path: msi,
      sha256: await sha256(msi),
      properties: currentMsiProperties,
    },
    zip: { path: zip, sha256: await sha256(zip) },
  };
  evidence.expectedVersion = expectedVersion;
  let previous;
  if (args["previous-msi"]) {
    assert.ok(args["previous-version"] && args["previous-msi-sha256"]);
    const previousPath = resolve(args["previous-msi"]);
    const previousSha256 = await sha256(previousPath);
    assert.equal(previousSha256, args["previous-msi-sha256"].toLowerCase());
    assert.match(args["previous-version"], /^\d+\.\d+\.\d+-yunswitch\.\d+$/);
    assert.notEqual(args["previous-version"], expectedVersion);
    const properties = await readMsiProperties(previousPath);
    assert.equal(properties.ProductName, "YunSwitch");
    assert.equal(properties.Manufacturer, "yunhanhai");
    assert.equal(properties.UpgradeCode, currentMsiProperties.UpgradeCode);
    assert.notEqual(properties.ProductCode, currentMsiProperties.ProductCode);
    assert.equal(
      properties.ProductVersion,
      args["previous-version"].replace("-yunswitch.", "."),
    );
    previous = {
      path: previousPath,
      version: args["previous-version"],
      sha256: previousSha256,
      properties,
    };
    evidence.previousMsi = previous;
    check("previous published MSI checksum and independent identity match", {
      version: previous.version,
      sha256: previousSha256,
      properties,
    });
  }
  const portableRoot = join(output, "portable");
  await powershell(
    `Expand-Archive -LiteralPath ${psQuote(zip)} -DestinationPath ${psQuote(portableRoot)}`,
  );
  const portableExe = join(portableRoot, "cc-switch.exe");
  assert.ok(await exists(portableExe));
  assert.ok(await exists(join(portableRoot, "portable.ini")));
  const license = await readFile(join(portableRoot, "LICENSE"), "utf8");
  assert.match(license, /MIT License/);
  assert.match(license, /Jason Young/);
  assert.match(
    await readFile(join(portableRoot, "README.txt"), "utf8"),
    /\.yunswitch/,
  );
  check("portable ZIP contains binary, marker, license and data instructions");
  if (args["require-third-party-notices"] === "true") {
    await verifyThirdPartyNotices(portableRoot, "portable ZIP");
  }
  let cdp = await launch(portableExe, "portable-first-run");
  await verifyIdentity(cdp, true);
  await until(
    () =>
      cdp.evaluate(
        "[...document.querySelectorAll('[role=dialog]')].some(d => /Welcome to YunSwitch|欢迎使用 YunSwitch/.test(d.innerText))",
      ),
    "first-run welcome dialog",
  );
  await cdp.screenshot("portable-first-run.png");
  assert.equal(
    await cdp.evaluate(
      "(() => {const button=[...document.querySelectorAll('[role=dialog] button')].find(b => /^(Got it|我知道了)$/.test(b.innerText.trim())); if(!button)return false; button.click(); return true;})()",
    ),
    true,
  );
  await until(
    async () =>
      (await cdp.invoke("get_settings")).firstRunNoticeConfirmed === true,
    "welcome confirmation persistence",
  );
  check("first-run UI renders and its confirmation persists");
  await exerciseProxy(cdp);
  await exitApp();
  // A fresh installation may create tool directories. They must remain inside
  // this disposable runner profile; the upstream app database stays absent.
  assert.ok(!(await exists(join(process.env.USERPROFILE, ".cc-switch"))));
  cdp = await launch(portableExe, "portable-relaunch");
  await verifyIdentity(cdp, true);
  assert.equal(
    (await cdp.invoke("get_settings")).firstRunNoticeConfirmed,
    true,
  );
  assert.equal(
    await cdp.invoke("get_current_provider", { app: "claude" }),
    "yunswitch-ci-smoke",
  );
  await cdp.screenshot("portable-relaunch.png");
  check("portable relaunch retains provider and welcome state");
  await exitApp();
  installed = true;
  await installOrUninstall("/i");
  const installedRoot = join(process.env.LOCALAPPDATA, "Programs", "YunSwitch");
  const installedExe = join(installedRoot, "cc-switch.exe");
  assert.ok(await exists(installedExe));
  assert.equal(await sha256(installedExe), await sha256(portableExe));
  assert.match(
    await readFile(join(installedRoot, "LICENSE.rtf"), "utf8"),
    /Jason Young/,
  );
  const command = await powershell(
    String.raw`(Get-Item 'HKCU:\Software\Classes\yunswitch\shell\open\command').GetValue('')`,
  );
  const registeredExe = command.match(/^"([^"]+)"\s+"%1"$/)?.[1];
  assert.ok(registeredExe, `Unexpected protocol command: ${command}`);
  // [!Path] in MSI may expand to an 8.3 short path; verify the actual file.
  assert.equal(await sha256(registeredExe), await sha256(installedExe));
  evidence.protocolCommand = command;
  assert.ok(command.includes('"%1"'));
  assert.ok(
    await exists(
      join(
        process.env.APPDATA,
        "Microsoft",
        "Windows",
        "Start Menu",
        "Programs",
        "YunSwitch",
        "YunSwitch.lnk",
      ),
    ),
  );
  assert.equal(
    await powershell(
      String.raw`[bool](Test-Path 'HKCU:\Software\Classes\ccswitch')`,
    ),
    "False",
  );
  check("MSI registers its independent protocol, shortcut and license");
  if (args["require-third-party-notices"] === "true") {
    await verifyThirdPartyNotices(installedRoot, "installed MSI");
  }
  cdp = await launch(installedExe, "installed");
  await verifyIdentity(cdp, false);
  await verifySingleInstance(installedExe);
  await exerciseMigration(cdp);
  await cdp.screenshot("installed-after-migration.png");
  await exitApp();
  await installOrUninstall("/x");
  installed = false;
  await verifyNoticeUninstall(installedRoot, "first uninstall");
  if (previous) {
    await verifyUpgrade(
      previous,
      currentMsiProperties,
      installedExe,
      portableExe,
    );
    await installOrUninstall("/x", msi, "upgraded-version-uninstall");
    installed = false;
    await verifyNoticeUninstall(installedRoot, "upgraded uninstall");
  }
  assert.ok(!(await exists(installedExe)));
  assert.equal(
    await powershell(
      String.raw`[bool](Test-Path 'HKCU:\Software\Classes\yunswitch')`,
    ),
    "False",
  );
  assert.ok(
    !(await exists(
      join(
        process.env.APPDATA,
        "Microsoft",
        "Windows",
        "Start Menu",
        "Programs",
        "YunSwitch",
        "YunSwitch.lnk",
      ),
    )),
  );
  assert.ok(
    await exists(join(process.env.USERPROFILE, ".yunswitch", "cc-switch.db")),
  );
  assert.ok(!(await exists(join(process.env.USERPROFILE, ".cc-switch"))));
  check("uninstall removes installation and protocol, preserves own database");
  cdp = await launch(portableExe, "portable-after-uninstall");
  assert.equal(
    (await cdp.invoke("get_providers", { app: "claude" }))["legacy-ci"].name,
    "Synthetic legacy provider",
  );
  assert.equal(await cdp.invoke("is_portable_mode"), true);
  if (previous) {
    assert.equal(
      (await cdp.invoke("get_providers", { app: "claude" }))["upgrade-ci"].name,
      "Synthetic provider created by previous YunSwitch version",
    );
  }
  await exitApp();
  check("portable launch after uninstall reads the retained migrated database");
  await cp(
    join(process.env.USERPROFILE, ".yunswitch", "logs"),
    join(output, "app-logs"),
    { recursive: true },
  );
  evidence.status = "passed";
}

try {
  await main();
} catch (error) {
  evidence.status = "failed";
  evidence.error = error.stack || String(error);
  process.stderr.write(`${evidence.error}\n`);
  process.exitCode = 1;
} finally {
  if (currentApp) {
    try {
      await exitApp();
    } catch (error) {
      currentApp?.processHandle.kill();
      evidence.cleanupError = String(error);
      if (output && currentApp)
        await writeFile(
          join(output, `${currentApp.label}-process.log`),
          Buffer.concat(currentApp.chunks),
        );
    }
  }
  mock?.close();
  if (installed) {
    try {
      await installOrUninstall("/x");
    } catch (error) {
      evidence.uninstallCleanupError = String(error);
    }
  }
  if (output) {
    evidence.finishedAt = new Date().toISOString();
    await writeFile(
      join(output, "results.json"),
      `${JSON.stringify(evidence, null, 2)}\n`,
    );
  }
}

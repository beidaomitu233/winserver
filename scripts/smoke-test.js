const http = require("http");
const net = require("net");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawn } = require("child_process");

const ROOT = path.resolve(__dirname, "..");

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function getFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

function request(port, pathname, options = {}) {
  return new Promise((resolve, reject) => {
    const body = options.body === undefined ? "" : JSON.stringify(options.body);
    const req = http.request(
      {
        host: "127.0.0.1",
        port,
        path: pathname,
        method: options.method || "GET",
        headers: body ? {
          "Content-Type": "application/json",
          "Content-Length": Buffer.byteLength(body)
        } : undefined,
        timeout: 1200
      },
      (res) => {
        let body = "";
        res.setEncoding("utf8");
        res.on("data", (chunk) => {
          body += chunk;
        });
        res.on("end", () => {
          resolve({ statusCode: res.statusCode, body });
        });
      }
    );
    req.on("timeout", () => {
      req.destroy(new Error(`Timed out requesting ${pathname}`));
    });
    req.on("error", reject);
    if (body) req.write(body);
    req.end();
  });
}

async function waitForState(port, child, output) {
  let lastError;
  for (let attempt = 0; attempt < 60; attempt += 1) {
    if (child.exitCode !== null) {
      throw new Error(`Server exited before it became ready.\n${output()}`);
    }
    try {
      const response = await request(port, "/api/state");
      if (response.statusCode === 200) return JSON.parse(response.body);
      lastError = new Error(`HTTP ${response.statusCode}`);
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
  throw new Error(`Server did not become ready: ${lastError ? lastError.message : "unknown error"}\n${output()}`);
}

function stop(child) {
  return new Promise((resolve) => {
    if (child.exitCode !== null) {
      resolve();
      return;
    }
    const timer = setTimeout(resolve, 1500);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve();
    });
    child.kill();
  });
}

async function main() {
  const port = await getFreePort();
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), "xpcn-smoke-"));
  const startupDir = fs.mkdtempSync(path.join(os.tmpdir(), "xpcn-startup-"));
  const logs = [];
  const child = spawn(process.execPath, ["server.js"], {
    cwd: ROOT,
    env: { ...process.env, XPCN_PORT: String(port), XPCN_DATA_DIR: dataDir, XPCN_STARTUP_DIR: startupDir },
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true
  });

  const output = () => logs.join("").trim();
  child.stdout.on("data", (chunk) => logs.push(chunk.toString()));
  child.stderr.on("data", (chunk) => logs.push(chunk.toString()));

  try {
    const state = await waitForState(port, child, output);
    assert(Array.isArray(state.services), "state.services should be an array");
    assert(Array.isArray(state.software), "state.software should be an array");
    assert(Array.isArray(state.configFiles), "state.configFiles should be an array");
    assert(state.configFiles.some((item) => item.id === "php.ini"), "php.ini config entry should exist");
    assert(state.systemSettings && state.systemSettings.port === port, "system settings should expose the running port");

    const homepage = await request(port, "/");
    assert(homepage.statusCode === 200, "homepage should return HTTP 200");
    assert(homepage.body.includes("XP.CN 小皮"), "homepage should contain the product name");
    assert(homepage.body.includes("app.js"), "homepage should load app.js");

    const sitePath = path.join(dataDir, "www", "smoke.local");
    const createSite = await request(port, "/api/sites", {
      method: "POST",
      body: { domain: "smoke.local", port: "8088", path: sitePath }
    });
    assert(createSite.statusCode === 200, "creating a site record should return HTTP 200");
    const siteState = JSON.parse(createSite.body).state;
    const siteIndex = siteState.websites.findIndex((item) => item.domain === "smoke.local");
    assert(siteIndex >= 0, "created site should appear in state");

    const deleteSite = await request(port, `/api/sites/${siteIndex}`, { method: "DELETE" });
    assert(deleteSite.statusCode === 200, "removing a site record should return HTTP 200");
    assert(!JSON.parse(deleteSite.body).state.websites.some((item) => item.domain === "smoke.local"), "removed site should disappear from state");

    const createFtp = await request(port, "/api/ftp", {
      method: "POST",
      body: { user: "smoke_ftp", path: path.join(dataDir, "ftp"), permission: "读写" }
    });
    assert(createFtp.statusCode === 200, "creating an FTP record should return HTTP 200");
    const ftpState = JSON.parse(createFtp.body).state;
    const ftpIndex = ftpState.ftpAccounts.findIndex((item) => item.user === "smoke_ftp");
    assert(ftpIndex >= 0, "created FTP record should appear in state");

    const deleteFtp = await request(port, `/api/ftp/${ftpIndex}`, { method: "DELETE" });
    assert(deleteFtp.statusCode === 200, "removing an FTP record should return HTTP 200");

    const deleteRoot = await request(port, "/api/databases/0", { method: "DELETE" });
    assert(deleteRoot.statusCode === 500, "root database record should be protected");
    assert(deleteRoot.body.includes("root"), "root protection response should mention root");

    const settingsOn = await request(port, "/api/settings/system", {
      method: "POST",
      body: {
        autostart: true,
        startSuiteOnLaunch: true,
        phpMyAdminUrl: "http://127.0.0.1:18113/phpmyadmin"
      }
    });
    assert(settingsOn.statusCode === 200, "saving system settings should return HTTP 200");
    const enabledSettings = JSON.parse(settingsOn.body).systemSettings;
    assert(enabledSettings.autostart === true, "autostart should be enabled");
    assert(enabledSettings.startSuiteOnLaunch === true, "startSuiteOnLaunch should be enabled");
    assert(enabledSettings.phpMyAdminUrl.endsWith("/phpmyadmin"), "phpMyAdmin URL should be saved");
    assert(fs.existsSync(enabledSettings.autostartPath), "autostart command should be created");

    const settingsOff = await request(port, "/api/settings/system", {
      method: "POST",
      body: { autostart: false }
    });
    assert(settingsOff.statusCode === 200, "disabling autostart should return HTTP 200");
    const disabledSettings = JSON.parse(settingsOff.body).systemSettings;
    assert(disabledSettings.autostart === false, "autostart should be disabled");
    assert(!fs.existsSync(enabledSettings.autostartPath), "autostart command should be removed");

    console.log(`Smoke test passed on http://127.0.0.1:${port}`);
  } finally {
    await stop(child);
    fs.rmSync(dataDir, { recursive: true, force: true });
    fs.rmSync(startupDir, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(error.stack || error.message);
  process.exit(1);
});

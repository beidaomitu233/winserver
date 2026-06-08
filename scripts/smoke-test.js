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
  const hostsPath = path.join(dataDir, "hosts");
  fs.writeFileSync(hostsPath, "127.0.0.1 localhost\n", "utf8");
  const logs = [];
  const child = spawn(process.execPath, ["server.js"], {
    cwd: ROOT,
    env: {
      ...process.env,
      XPCN_PORT: String(port),
      XPCN_DATA_DIR: dataDir,
      XPCN_STARTUP_DIR: startupDir,
      XPCN_HOSTS_PATH: hostsPath,
      XPCN_SERVICE_DRY_RUN: "1"
    },
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true
  });

  const output = () => logs.join("").trim();
  child.stdout.on("data", (chunk) => logs.push(chunk.toString()));
  child.stderr.on("data", (chunk) => logs.push(chunk.toString()));

  try {
    const state = await waitForState(port, child, output);
    assert(Array.isArray(state.services), "state.services should be an array");
    assert(state.services.every((service) => service.running === false), "dry-run state should not detect real local processes");
    assert(state.services.every((service) => service.type === "square"), "stopped services should render as square");
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
    assert(fs.readFileSync(hostsPath, "utf8").includes("127.0.0.1 smoke.local # XP.CN smoke.local"), "created site should be synced to hosts");

    const duplicateSite = await request(port, "/api/sites", {
      method: "POST",
      body: { domain: "SMOKE.local", port: "8088", path: path.join(dataDir, "www", "smoke-duplicate.local") }
    });
    assert(duplicateSite.statusCode === 500, "creating a duplicate site domain and port should fail");
    assert(duplicateSite.body.includes("已存在"), "duplicate site response should explain the conflict");

    const secondSite = await request(port, "/api/sites", {
      method: "POST",
      body: { domain: "smoke-second.local", port: "8090", path: path.join(dataDir, "www", "smoke-second.local") }
    });
    assert(secondSite.statusCode === 200, "creating a second unique site should return HTTP 200");
    const secondSiteState = JSON.parse(secondSite.body).state;
    const secondSiteIndex = secondSiteState.websites.findIndex((item) => item.domain === "smoke-second.local");
    assert(secondSiteIndex >= 0, "second created site should appear in state");

    const conflictingEdit = await request(port, `/api/sites/${secondSiteIndex}`, {
      method: "PUT",
      body: { domain: "smoke.local", port: "8088", path: path.join(dataDir, "www", "smoke-conflict.local") }
    });
    assert(conflictingEdit.statusCode === 500, "editing a site into an existing domain and port should fail");
    assert(conflictingEdit.body.includes("已存在"), "conflicting edit response should explain the conflict");

    const deleteSecondSite = await request(port, `/api/sites/${secondSiteIndex}`, { method: "DELETE" });
    assert(deleteSecondSite.statusCode === 200, "removing the second site record should return HTTP 200");

    const editedSitePath = path.join(dataDir, "www", "smoke-edited.local");
    const updateSite = await request(port, `/api/sites/${siteIndex}`, {
      method: "PUT",
      body: { domain: "smoke-edited.local", port: "8089", path: editedSitePath }
    });
    assert(updateSite.statusCode === 200, "editing a site record should return HTTP 200");
    const updatedSite = JSON.parse(updateSite.body).state.websites[siteIndex];
    assert(updatedSite.domain === "smoke-edited.local", "edited site domain should be saved");
    assert(updatedSite.port === "8089", "edited site port should be saved");
    assert(fs.existsSync(editedSitePath), "edited site directory should be created");
    const hostsAfterEdit = fs.readFileSync(hostsPath, "utf8");
    assert(!hostsAfterEdit.includes("smoke.local # XP.CN smoke.local"), "old site domain should be removed from hosts");
    assert(hostsAfterEdit.includes("127.0.0.1 smoke-edited.local # XP.CN smoke-edited.local"), "edited site domain should be synced to hosts");

    const deleteSite = await request(port, `/api/sites/${siteIndex}`, { method: "DELETE" });
    assert(deleteSite.statusCode === 200, "removing a site record should return HTTP 200");
    assert(!JSON.parse(deleteSite.body).state.websites.some((item) => item.domain === "smoke-edited.local"), "removed site should disappear from state");
    assert(!fs.readFileSync(hostsPath, "utf8").includes("smoke-edited.local # XP.CN smoke-edited.local"), "removed site should be removed from hosts");

    const createFtp = await request(port, "/api/ftp", {
      method: "POST",
      body: { user: "smoke_ftp", path: path.join(dataDir, "ftp"), permission: "读写" }
    });
    assert(createFtp.statusCode === 200, "creating an FTP record should return HTTP 200");
    const ftpState = JSON.parse(createFtp.body).state;
    const ftpIndex = ftpState.ftpAccounts.findIndex((item) => item.user === "smoke_ftp");
    assert(ftpIndex >= 0, "created FTP record should appear in state");

    const editedFtpPath = path.join(dataDir, "ftp-edited");
    const updateFtp = await request(port, `/api/ftp/${ftpIndex}`, {
      method: "PUT",
      body: { user: "smoke_ftp_edited", path: editedFtpPath, permission: "只读" }
    });
    assert(updateFtp.statusCode === 200, "editing an FTP record should return HTTP 200");
    const updatedFtp = JSON.parse(updateFtp.body).state.ftpAccounts[ftpIndex];
    assert(updatedFtp.user === "smoke_ftp_edited", "edited FTP user should be saved");
    assert(updatedFtp.permission === "只读", "edited FTP permission should be saved");
    assert(fs.existsSync(editedFtpPath), "edited FTP directory should be created");

    const deleteFtp = await request(port, `/api/ftp/${ftpIndex}`, { method: "DELETE" });
    assert(deleteFtp.statusCode === 200, "removing an FTP record should return HTTP 200");

    const openExistingFolder = await request(port, "/api/open/folder", {
      method: "POST",
      body: { path: editedFtpPath }
    });
    assert(openExistingFolder.statusCode === 200, "opening an existing folder should return HTTP 200 in dry-run mode");
    assert(JSON.parse(openExistingFolder.body).message.includes("已验证目录"), "dry-run folder open should validate the folder");

    const missingFolderPath = path.join(dataDir, "missing-folder");
    const openMissingFolder = await request(port, "/api/open/folder", {
      method: "POST",
      body: { path: missingFolderPath }
    });
    assert(openMissingFolder.statusCode === 500, "opening a missing folder should fail");
    assert(!fs.existsSync(missingFolderPath), "opening a missing folder should not create it");

    const existingFilePath = path.join(dataDir, "open-file.txt");
    fs.writeFileSync(existingFilePath, "open file smoke", "utf8");
    const openExistingFile = await request(port, "/api/open/file", {
      method: "POST",
      body: { path: existingFilePath }
    });
    assert(openExistingFile.statusCode === 200, "opening an existing file should return HTTP 200 in dry-run mode");
    assert(JSON.parse(openExistingFile.body).message.includes("已验证文件"), "dry-run file open should validate the file");

    const missingFilePath = path.join(dataDir, "missing-file.txt");
    const openMissingFile = await request(port, "/api/open/file", {
      method: "POST",
      body: { path: missingFilePath }
    });
    assert(openMissingFile.statusCode === 500, "opening a missing file should fail");
    assert(!fs.existsSync(missingFilePath), "opening a missing file should not create it");

    const deleteRoot = await request(port, "/api/databases/0", { method: "DELETE" });
    assert(deleteRoot.statusCode === 500, "root database record should be protected");
    assert(deleteRoot.body.includes("root"), "root protection response should mention root");

    const exportRoot = await request(port, "/api/databases/0/export", { method: "POST", body: {} });
    assert(exportRoot.statusCode === 200, "exporting a database record should return HTTP 200");
    const exported = JSON.parse(exportRoot.body);
    assert(exported.path.endsWith(".sql"), "database export should return an SQL file path");
    assert(fs.existsSync(exported.path), "database export file should exist");
    assert(fs.readFileSync(exported.path, "utf8").includes("database: root"), "dry-run export should contain the database name");
    const backups = await request(port, "/api/databases/backups");
    assert(backups.statusCode === 200, "listing database backups should return HTTP 200");
    assert(JSON.parse(backups.body).backups.some((item) => item.path === exported.path), "database backup list should include the exported file");

    const redisOn = await request(port, "/api/services/redis/auto", {
      method: "POST",
      body: { auto: true }
    });
    assert(redisOn.statusCode === 200, "enabling redis auto should return HTTP 200");
    assert(JSON.parse(redisOn.body).service.auto === true, "redis should be in the suite");

    const suiteStop = await request(port, "/api/suite/stop", { method: "POST", body: {} });
    assert(suiteStop.statusCode === 200, "suite stop should return HTTP 200");
    assert(JSON.parse(suiteStop.body).results.some((line) => line.includes("Redis7.2.4")), "suite should include redis after enabling auto");

    const redisOff = await request(port, "/api/services/redis/auto", {
      method: "POST",
      body: { auto: false }
    });
    assert(redisOff.statusCode === 200, "disabling redis auto should return HTTP 200");
    assert(JSON.parse(redisOff.body).service.auto === false, "redis should be removed from the suite");

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
    assert(enabledSettings.phpMyAdminUrl === "http://127.0.0.1:18113/phpmyadmin", "phpMyAdmin URL should be saved exactly");
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

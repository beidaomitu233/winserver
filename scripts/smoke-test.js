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
  const phpStudyRoot = path.join(dataDir, "phpstudy_pro");
  const apacheVhostsDir = path.join(phpStudyRoot, "Extensions", "Apache2.4.39", "conf", "vhosts");
  const nginxVhostsDir = path.join(phpStudyRoot, "Extensions", "Nginx1.15.11", "conf", "vhosts");
  const ftpRoot = path.join(phpStudyRoot, "Extensions", "FTP0.9.60");
  const ftpConfigPath = path.join(ftpRoot, "FileZilla Server.xml");
  const hostsPath = path.join(dataDir, "hosts");
  fs.mkdirSync(apacheVhostsDir, { recursive: true });
  fs.mkdirSync(nginxVhostsDir, { recursive: true });
  fs.mkdirSync(ftpRoot, { recursive: true });
  fs.writeFileSync(path.join(apacheVhostsDir, "Listen.conf"), "Listen 80\n", "utf8");
  fs.writeFileSync(ftpConfigPath, [
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>",
    "<FileZillaServer>",
    "  <Settings />",
    "  <Users>",
    "    <User Name=\"manual_user\"><Option Name=\"Comments\">keep me</Option></User>",
    "  </Users>",
    "</FileZillaServer>",
    ""
  ].join(os.EOL), "utf8");
  fs.writeFileSync(hostsPath, "127.0.0.1 localhost\n", "utf8");
  const logs = [];
  const child = spawn(process.execPath, ["server.js"], {
    cwd: ROOT,
    env: {
      ...process.env,
      XPCN_PORT: String(port),
      XPCN_DATA_DIR: dataDir,
      XPCN_PHPSTUDY: phpStudyRoot,
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
    assert(state.services.some((service) => service.id === "mariadb"), "MariaDB service should be exposed");
    assert(Array.isArray(state.software), "state.software should be an array");
    assert(state.software.some((item) => item.id === "mariadb"), "MariaDB software should be exposed");
    assert(Array.isArray(state.configFiles), "state.configFiles should be an array");
    assert(state.configFiles.some((item) => item.id === "php.ini"), "php.ini config entry should exist");
    assert(state.configFiles.some((item) => item.id === "mariadb.ini"), "MariaDB config entry should exist");
    assert(state.systemSettings && state.systemSettings.port === port, "system settings should expose the running port");
    assert(state.systemSettings.editablePathLabels && state.systemSettings.editablePathLabels.phpStudyRoot, "path setting labels should be exposed");
    assert(state.systemSettings.editablePathLabels.mariadbRoot, "MariaDB path setting label should be exposed");

    const homepage = await request(port, "/");
    assert(homepage.statusCode === 200, "homepage should return HTTP 200");
    assert(homepage.body.includes("XP.CN 小皮"), "homepage should contain the product name");
    assert(homepage.body.includes("app.js"), "homepage should load app.js");

    const staticTraversal = await request(port, "/..%2Fserver.js");
    assert(staticTraversal.statusCode === 403, "static file traversal should be forbidden");
    const staticBadEncoding = await request(port, "/%E0%A4%A");
    assert(staticBadEncoding.statusCode === 400, "bad static URL encoding should return HTTP 400");

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
    const smokeApacheVhost = path.join(apacheVhostsDir, "smoke.local_8088.conf");
    const smokeNginxVhost = path.join(nginxVhostsDir, "smoke.local_8088.conf");
    assert(fs.existsSync(smokeApacheVhost), "created site should write an Apache vhost");
    assert(fs.existsSync(smokeNginxVhost), "created site should write an Nginx vhost");

    const siteConfig = await request(port, `/api/sites/${siteIndex}/config`);
    assert(siteConfig.statusCode === 200, "site config endpoint should return HTTP 200");
    const siteConfigBody = JSON.parse(siteConfig.body);
    assert(siteConfigBody.id === "site:smoke.local_8088.conf", "site config id should use the stable vhost name");
    assert(siteConfigBody.path.replace(/\\/g, "/").endsWith("/smoke.local_8088.conf"), "site config path should point to the created Apache vhost");
    assert(siteConfigBody.content.includes("ServerName smoke.local"), "site config content should include the created domain");

    const updatedSiteConfigContent = `${siteConfigBody.content}\n# smoke site config edit\n`;
    const saveSiteConfig = await request(port, `/api/config-files/${encodeURIComponent(siteConfigBody.id)}`, {
      method: "POST",
      body: { content: updatedSiteConfigContent }
    });
    assert(saveSiteConfig.statusCode === 200, "saving a dynamic site config should return HTTP 200");
    assert(fs.readFileSync(smokeApacheVhost, "utf8").includes("# smoke site config edit"), "saving a dynamic site config should update the Apache vhost file");

    const invalidCreatePort = await request(port, "/api/sites", {
      method: "POST",
      body: { domain: "invalid-port.local", port: "70000", path: path.join(dataDir, "www", "invalid-port.local") }
    });
    assert(invalidCreatePort.statusCode === 500, "creating a site with an invalid port should fail");
    assert(invalidCreatePort.body.includes("1-65535"), "invalid create port response should explain the valid range");

    const invalidCreateDomain = await request(port, "/api/sites", {
      method: "POST",
      body: { domain: "http://bad.local", port: "8091", path: path.join(dataDir, "www", "bad-domain.local") }
    });
    assert(invalidCreateDomain.statusCode === 500, "creating a site with an invalid domain should fail");
    assert(invalidCreateDomain.body.includes("域名格式"), "invalid create domain response should explain the domain format");

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
    const secondApacheVhost = path.join(apacheVhostsDir, "smoke-second.local_8090.conf");
    assert(fs.existsSync(secondApacheVhost), "second created site should write a stable vhost name");

    const conflictingEdit = await request(port, `/api/sites/${secondSiteIndex}`, {
      method: "PUT",
      body: { domain: "smoke.local", port: "8088", path: path.join(dataDir, "www", "smoke-conflict.local") }
    });
    assert(conflictingEdit.statusCode === 500, "editing a site into an existing domain and port should fail");
    assert(conflictingEdit.body.includes("已存在"), "conflicting edit response should explain the conflict");

    const invalidEditPort = await request(port, `/api/sites/${secondSiteIndex}`, {
      method: "PUT",
      body: { domain: "smoke-second.local", port: "0", path: path.join(dataDir, "www", "smoke-invalid-port.local") }
    });
    assert(invalidEditPort.statusCode === 500, "editing a site to an invalid port should fail");
    assert(invalidEditPort.body.includes("1-65535"), "invalid edit port response should explain the valid range");

    const invalidEditDomain = await request(port, `/api/sites/${secondSiteIndex}`, {
      method: "PUT",
      body: { domain: "bad/domain.local", port: "8090", path: path.join(dataDir, "www", "smoke-invalid-domain.local") }
    });
    assert(invalidEditDomain.statusCode === 500, "editing a site to an invalid domain should fail");
    assert(invalidEditDomain.body.includes("域名格式"), "invalid edit domain response should explain the domain format");

    const deleteSecondSite = await request(port, `/api/sites/${secondSiteIndex}`, { method: "DELETE" });
    assert(deleteSecondSite.statusCode === 200, "removing the second site record should return HTTP 200");
    assert(!fs.existsSync(secondApacheVhost), "removed second site should remove its stable vhost file");

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
    assert(!fs.existsSync(smokeApacheVhost), "editing a site should remove the old Apache vhost");
    assert(fs.existsSync(path.join(apacheVhostsDir, "smoke-edited.local_8089.conf")), "editing a site should write the new Apache vhost");
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
    const ftpConfigAfterCreate = fs.readFileSync(ftpConfigPath, "utf8");
    assert(ftpConfigAfterCreate.includes("XP.CN managed account: smoke_ftp"), "created FTP account should be synced to FileZilla config");
    assert(ftpConfigAfterCreate.includes(`Permission Dir="${path.join(dataDir, "ftp").replace(/\\/g, "/")}"`), "created FTP account should sync the configured directory");
    assert(ftpConfigAfterCreate.includes('<Option Name="FileWrite">1</Option>'), "read-write FTP account should allow file writes");
    assert(ftpConfigAfterCreate.includes("manual_user"), "syncing FTP accounts should preserve unmanaged FileZilla users");

    const duplicateFtp = await request(port, "/api/ftp", {
      method: "POST",
      body: { user: "SMOKE_FTP", path: path.join(dataDir, "ftp-duplicate"), permission: "读写" }
    });
    assert(duplicateFtp.statusCode === 500, "creating a duplicate FTP user should fail");
    assert(duplicateFtp.body.includes("已存在"), "duplicate FTP response should explain the conflict");

    const secondFtp = await request(port, "/api/ftp", {
      method: "POST",
      body: { user: "smoke_ftp_second", path: path.join(dataDir, "ftp-second"), permission: "只读" }
    });
    assert(secondFtp.statusCode === 200, "creating a second unique FTP record should return HTTP 200");
    const secondFtpState = JSON.parse(secondFtp.body).state;
    const secondFtpIndex = secondFtpState.ftpAccounts.findIndex((item) => item.user === "smoke_ftp_second");
    assert(secondFtpIndex >= 0, "second FTP record should appear in state");

    const invalidFtpUser = await request(port, "/api/ftp", {
      method: "POST",
      body: { user: "bad user", path: path.join(dataDir, "ftp-invalid"), permission: "读写" }
    });
    assert(invalidFtpUser.statusCode === 500, "creating FTP with an invalid user should fail");
    assert(invalidFtpUser.body.includes("只能包含"), "invalid FTP user response should explain the allowed characters");

    const conflictingFtpEdit = await request(port, `/api/ftp/${secondFtpIndex}`, {
      method: "PUT",
      body: { user: "smoke_ftp", path: path.join(dataDir, "ftp-conflict"), permission: "只读" }
    });
    assert(conflictingFtpEdit.statusCode === 500, "editing FTP into an existing user should fail");
    assert(conflictingFtpEdit.body.includes("已存在"), "conflicting FTP edit response should explain the conflict");

    const deleteSecondFtp = await request(port, `/api/ftp/${secondFtpIndex}`, { method: "DELETE" });
    assert(deleteSecondFtp.statusCode === 200, "removing the second FTP record should return HTTP 200");

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
    const ftpConfigAfterEdit = fs.readFileSync(ftpConfigPath, "utf8");
    assert(!ftpConfigAfterEdit.includes("XP.CN managed account: smoke_ftp<"), "editing FTP user should remove the old managed user block");
    assert(ftpConfigAfterEdit.includes("XP.CN managed account: smoke_ftp_edited"), "editing FTP user should sync the new managed user block");
    assert(ftpConfigAfterEdit.includes('<Option Name="FileWrite">0</Option>'), "read-only FTP account should disable file writes");

    const deleteFtp = await request(port, `/api/ftp/${ftpIndex}`, { method: "DELETE" });
    assert(deleteFtp.statusCode === 200, "removing an FTP record should return HTTP 200");
    const ftpConfigAfterDelete = fs.readFileSync(ftpConfigPath, "utf8");
    assert(!ftpConfigAfterDelete.includes("XP.CN managed account: smoke_ftp_edited"), "removing FTP should clean the managed FileZilla user");
    assert(ftpConfigAfterDelete.includes("manual_user"), "removing FTP should preserve unmanaged FileZilla users");

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

    const duplicateDatabase = await request(port, "/api/databases", {
      method: "POST",
      body: { db: "ROOT", user: "root_duplicate", pass: "123456" }
    });
    assert(duplicateDatabase.statusCode === 500, "creating a duplicate database record should fail");
    assert(duplicateDatabase.body.includes("已存在"), "duplicate database response should explain the conflict");

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

    const unknownSoftwareAction = await request(port, "/api/software/nginx/launch", {
      method: "POST",
      body: {}
    });
    assert(unknownSoftwareAction.statusCode === 404, "unknown software action should fail with HTTP 404");
    assert(unknownSoftwareAction.body.includes("软件操作不存在"), "unknown software action should explain the invalid action");

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

    const newPhpStudyRoot = path.join(dataDir, "custom_phpstudy");
    const customRedisRoot = path.join(dataDir, "custom_redis");
    const pathsUpdate = await request(port, "/api/settings/paths", {
      method: "POST",
      body: {
        phpStudyRoot: newPhpStudyRoot,
        wwwRoot: path.join(phpStudyRoot, "WWW"),
        apacheRoot: path.join(phpStudyRoot, "Extensions", "Apache2.4.39"),
        nginxRoot: path.join(phpStudyRoot, "Extensions", "Nginx1.15.11"),
        mysql57Root: path.join(phpStudyRoot, "Extensions", "MySQL5.7.26"),
        mysql80Root: path.join(phpStudyRoot, "Extensions", "MySQL8.0.12"),
        mariadbRoot: path.join(phpStudyRoot, "Extensions", "MariaDB10.11"),
        phpRoot: path.join(phpStudyRoot, "Extensions", "php", "php7.3.4nts"),
        ftpRoot: path.join(phpStudyRoot, "Extensions", "FTP0.9.60"),
        redisRoot: customRedisRoot,
        minioRoot: path.join(dataDir, "custom_minio")
      }
    });
    assert(pathsUpdate.statusCode === 200, "saving local paths should return HTTP 200");
    const updatedPathsState = JSON.parse(pathsUpdate.body).state;
    const updatedPaths = updatedPathsState.systemSettings.paths;
    assert(updatedPaths.phpStudyRoot.replace(/\\/g, "/").endsWith("/custom_phpstudy"), "phpStudy root should be saved");
    assert(updatedPaths.apacheRoot.replace(/\\/g, "/").endsWith("/custom_phpstudy/Extensions/Apache2.4.39"), "default Apache path should follow a changed phpStudy root");
    assert(updatedPaths.mariadbRoot.replace(/\\/g, "/").endsWith("/custom_phpstudy/Extensions/MariaDB10.11"), "default MariaDB path should follow a changed phpStudy root");
    assert(updatedPaths.redisRoot.replace(/\\/g, "/").endsWith("/custom_redis"), "custom Redis path should be saved");
    const apacheService = updatedPathsState.services.find((item) => item.id === "apache");
    const mariadbService = updatedPathsState.services.find((item) => item.id === "mariadb");
    const mariadbSoftware = updatedPathsState.software.find((item) => item.id === "mariadb");
    const redisSoftware = updatedPathsState.software.find((item) => item.id === "redis");
    const httpdConfig = updatedPathsState.configFiles.find((item) => item.id === "httpd.conf");
    const mariadbConfig = updatedPathsState.configFiles.find((item) => item.id === "mariadb.ini");
    assert(apacheService.configFile.replace(/\\/g, "/").endsWith("/custom_phpstudy/Extensions/Apache2.4.39/conf/httpd.conf"), "Apache service config path should be refreshed");
    assert(mariadbService.configFile.replace(/\\/g, "/").endsWith("/custom_phpstudy/Extensions/MariaDB10.11/my.ini"), "MariaDB service config path should be refreshed");
    assert(mariadbSoftware.executable.replace(/\\/g, "/").endsWith("/custom_phpstudy/Extensions/MariaDB10.11/bin/mysqld.exe"), "MariaDB software executable should be refreshed");
    assert(redisSoftware.executable.replace(/\\/g, "/").endsWith("/custom_redis/redis-server.exe"), "Redis software executable should be refreshed");
    assert(httpdConfig.path.replace(/\\/g, "/").endsWith("/custom_phpstudy/Extensions/Apache2.4.39/conf/httpd.conf"), "config file path should be refreshed");
    assert(mariadbConfig.path.replace(/\\/g, "/").endsWith("/custom_phpstudy/Extensions/MariaDB10.11/my.ini"), "MariaDB config file path should be refreshed");

    const invalidPathsUpdate = await request(port, "/api/settings/paths", {
      method: "POST",
      body: { phpStudyRoot: "relative/phpstudy" }
    });
    assert(invalidPathsUpdate.statusCode === 500, "saving relative local paths should fail");
    assert(invalidPathsUpdate.body.includes("绝对路径"), "invalid path response should explain absolute path requirement");

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

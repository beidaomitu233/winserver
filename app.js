const services = [
  { name: "Apache2.4.39", type: "square", running: false, auto: true },
  { name: "FTP0.9.60", type: "square", running: false, auto: true },
  { name: "MySQL5.7.26", type: "square", running: false, auto: true },
  { name: "MySQL8.0.12", type: "square", running: false, auto: true },
  { name: "MariaDB10.11", type: "square", running: false, auto: false },
  { name: "Nginx1.15.11", type: "square", running: false, auto: true },
  { name: "Redis7.2.4", type: "square", running: false, auto: false },
  { name: "MinIO2025.04", type: "square", running: false, auto: false }
];

const websites = [
  { domain: "localhost", port: "80", path: "D:/phpstudy_pro/WWW", status: "正常", expire: "2035-12-03" },
  { domain: "minio.local", port: "9000", path: "D:/phpstudy_pro/minio/data", status: "正常", expire: "2035-12-03" }
];

const databases = [
  { db: "root", user: "root", pass: "******", status: "正常" },
  { db: "xh_blog", user: "xinghuiTec", pass: "******", status: "正常" },
  { db: "housego", user: "xinghuiTec1", pass: "******", status: "正常" },
  { db: "ruoyi", user: "ruoyi", pass: "******", status: "正常" },
  { db: "xinghui_admin", user: "xinghuitec", pass: "******", status: "正常" },
  { db: "getvideotext", user: "xinghuitec2", pass: "******", status: "正常" },
  { db: "meiyeai", user: "beidaomitu", pass: "******", status: "正常" },
  { db: "diagram_gen", user: "beidao", pass: "******", status: "正常" },
  { db: "meiyeAICRM", user: "meiyecrm", pass: "******", status: "正常" },
  { db: "xinghuicom", user: "xinghuiteccom", pass: "******", status: "正常" }
];

const ftpAccounts = [
  { user: "localhost", path: "D:/phpstudy_pro/WWW", permission: "读写", status: "正常" },
  { user: "uploads", path: "D:/phpstudy_pro/uploads", permission: "只写", status: "正常" }
];
const databaseBackups = [];

const softwareTabs = ["全部", "系统环境", "安全", "网站程序", "工具"];
const categoryTabs = ["全部", "Web Servers", "数据库", "对象存储", "文件服务", "php", "redis", "composer"];
const software = [
  { name: "Apache2.4.39", category: "Web Servers", group: "全部", desc: "web服务，发布包内置", icon: "monitor", installed: true },
  { name: "Nginx1.15.11", category: "Web Servers", group: "全部", desc: "web服务，发布包内置", icon: "monitor", installed: true },
  { name: "Apache2.4.43", category: "Web Servers", group: "全部", desc: "web服务，发布包内置", icon: "monitor", installed: false },
  { name: "Nginx1.16.1", category: "Web Servers", group: "全部", desc: "web服务，发布包内置", icon: "monitor", installed: false },
  { name: "Nginx1.25.2", category: "Web Servers", group: "全部", desc: "web服务，发布包内置", icon: "monitor", installed: false },
  { name: "MySQL8.0.12", category: "数据库", group: "系统环境", desc: "数据库服务", icon: "db", installed: true },
  { name: "MySQL5.7.26", category: "数据库", group: "系统环境", desc: "数据库服务", icon: "db", installed: true },
  { name: "MariaDB10.11", category: "数据库", group: "系统环境", desc: "MariaDB 数据库服务", icon: "db", installed: false },
  { name: "Redis7.2.4", category: "redis", group: "系统环境", desc: "缓存、Session、队列与分布式锁服务", icon: "redis", installed: true },
  { name: "MinIO RELEASE", category: "对象存储", group: "工具", desc: "兼容 S3 API 的本地对象存储服务", icon: "minio", installed: true },
  { name: "MySQL5.0.96", category: "数据库", group: "系统环境", desc: "数据库服务", icon: "db", installed: false },
  { name: "MySQL5.1.60", category: "数据库", group: "系统环境", desc: "数据库服务", icon: "db", installed: false },
  { name: "Redis6.2.14", category: "redis", group: "系统环境", desc: "高性能键值缓存服务", icon: "redis", installed: false },
  { name: "FileZilla Server", category: "文件服务", group: "工具", desc: "FTP 文件服务", icon: "monitor", installed: false },
  { name: "php7.3.4nts", category: "php", group: "系统环境", desc: "PHP 运行环境", icon: "monitor", installed: true },
  { name: "composer2.7", category: "composer", group: "工具", desc: "PHP 依赖管理工具", icon: "monitor", installed: true }
];

const configs = ["php.ini", "httpd.conf", "nginx.conf", "vhosts.conf", "mysql.ini", "mariadb.ini", "redis.conf", "minio.env", "hosts"];
const logLines = [
  "2026-06-07 21:09:04 MySQL8.0.12 已启动",
  "2026-06-07 21:09:04 MySQL8.0.12 正在启动....."
];

const configFiles = configs.map((id) => ({ id, label: id, path: "", exists: false }));
let activeSoftwareTab = "全部";
let activeCategory = "全部";
let activeConfig = "php.ini";
let activeView = "home";
let activeSettings = "config";
let modalType = "";
let modalIndex = -1;
let backendOnline = false;
const configContents = {};
const systemSettings = {
  autostart: false,
  startSuiteOnLaunch: false,
  phpMyAdminUrl: "http://127.0.0.1/phpmyadmin",
  port: "",
  dataDir: "",
  configPath: "",
  autostartPath: "",
  autostartInstalled: false,
  paths: {},
  editablePathLabels: {}
};

const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => Array.from(root.querySelectorAll(selector));
const canUseBackend = location.protocol === "http:" || location.protocol === "https:";

function escapeHtml(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

async function api(path, options = {}) {
  const response = await fetch(path, {
    headers: { "Content-Type": "application/json", ...(options.headers || {}) },
    ...options
  });
  const text = await response.text();
  const data = text ? JSON.parse(text) : {};
  if (!response.ok) throw new Error(data.error || text || "请求失败");
  return data;
}

function replaceArray(target, items) {
  target.splice(0, target.length, ...(items || []));
}

function applyState(state) {
  backendOnline = true;
  const dynamicConfigFiles = configFiles.filter((item) => String(item.id || "").startsWith("site:"));
  replaceArray(services, state.services || []);
  replaceArray(websites, state.websites || []);
  replaceArray(databases, state.databases || []);
  replaceArray(databaseBackups, state.databaseBackups || []);
  replaceArray(ftpAccounts, state.ftpAccounts || []);
  replaceArray(software, state.software || []);
  const nextConfigFiles = [...(state.configFiles || configFiles)];
  dynamicConfigFiles.forEach((file) => {
    if (!nextConfigFiles.some((item) => item.id === file.id)) nextConfigFiles.push(file);
  });
  replaceArray(configFiles, nextConfigFiles);
  replaceArray(configs, configFiles.map((item) => item.id));
  replaceArray(logLines, Array.isArray(state.logs) ? state.logs : logLines);
  Object.assign(systemSettings, state.systemSettings || {});
  if (!configs.includes(activeConfig)) activeConfig = configs[0] || "php.ini";
  const version = $(".version");
  if (version && state.version) version.innerHTML = `<span>ⓘ</span> 版本：${escapeHtml(state.version)}`;
  renderAll();
  if (activeView === "settings") loadConfigFile(activeConfig);
}

function mergeConfigFileMeta(file) {
  if (!file?.id) return;
  const meta = { id: file.id, label: file.label || file.id, path: file.path || "", exists: !!file.exists };
  const index = configFiles.findIndex((item) => item.id === file.id);
  if (index >= 0) configFiles[index] = { ...configFiles[index], ...meta };
  else configFiles.push(meta);
  replaceArray(configs, configFiles.map((item) => item.id));
}

async function refreshState() {
  if (!canUseBackend) return;
  try {
    const state = await api("/api/state");
    applyState(state);
  } catch (error) {
    backendOnline = false;
    addLog(`后端未连接：${error.message}`);
  }
}

async function postApi(path, body = {}) {
  const result = await api(path, { method: "POST", body: JSON.stringify(body) });
  if (result.state) applyState(result.state);
  if (result.file) mergeConfigFileMeta(result.file);
  else if (result.systemSettings) {
    Object.assign(systemSettings, result.systemSettings);
    renderQuickStatus();
    renderRows("database");
    if (activeView === "settings") renderSettings();
    if (result.message) addLog(result.message);
  } else if (result.service) {
    const index = services.findIndex((item) => item.id === result.service.id);
    if (index >= 0) services[index] = { ...services[index], auto: !!result.service.auto };
    renderQuickStatus();
    renderServices();
    if (result.message) addLog(result.message);
  } else if (result.message) addLog(result.message);
  return result;
}

async function deleteApi(path) {
  const result = await api(path, { method: "DELETE" });
  if (result.state) applyState(result.state);
  else if (result.message) addLog(result.message);
  return result;
}

async function runBackend(action, fallback) {
  try {
    if (backendOnline || canUseBackend) {
      await action();
      return;
    }
  } catch (error) {
    addLog(`操作失败：${error.message}`);
    return;
  }
  if (fallback) fallback();
}

async function openLocal(kind, payload, fallback) {
  if (canUseBackend) {
    await runBackend(() => postApi(`/api/open/${kind}`, payload));
    return;
  }
  if (fallback) fallback();
}

function renderAll() {
  renderQuickStatus();
  renderServices();
  renderLogs();
  renderLegend();
  renderRows("website");
  renderRows("database");
  renderBackups();
  renderRows("ftp");
  renderSoftwareTabs();
  renderSoftware();
  renderSettings();
}

function autoServices(items = services) {
  return items.filter((item) => item.auto);
}

function suiteRunning(items = services) {
  return autoServices(items).some((item) => item.running);
}

function softwareInstallState(item) {
  const canInstall = !item.installed && item.installable !== false;
  return {
    canInstall,
    text: item.installed ? "已安装" : canInstall ? "安装" : "需配置",
    title: item.installed ? "已安装" : canInstall ? "下载安装" : (item.installNote || "请先在 data/config.json 中配置下载地址")
  };
}

function renderQuickStatus() {
  const suiteButton = $('[data-action="suite-toggle"]');
  const suiteDot = suiteButton?.parentElement.querySelector(".status-dot");
  const autostartButton = $('[data-action="autostart-toggle"]');
  const autostartDot = autostartButton?.parentElement.querySelector(".status-dot");
  const anyRunning = suiteRunning();
  if (suiteButton) suiteButton.textContent = anyRunning ? "停止" : "启动";
  if (suiteDot) {
    suiteDot.classList.toggle("running", anyRunning);
    suiteDot.classList.toggle("stopped", !anyRunning);
  }
  if (autostartButton) autostartButton.textContent = systemSettings.autostart ? "停用" : "启用";
  if (autostartDot) {
    autostartDot.classList.toggle("running", !!systemSettings.autostart);
    autostartDot.classList.toggle("stopped", !systemSettings.autostart);
  }
}

function nowStamp() {
  const date = new Date();
  const pad = (value) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function addLog(text) {
  logLines.unshift(`${nowStamp()} ${text}`);
  renderLogs();
}

function renderServices() {
  const list = $("#serviceList");
  list.innerHTML = services
    .map((service, index) => {
      const shape = service.running ? "triangle" : "square";
      const shapeClass = shape === "triangle" ? "shape-triangle" : "shape-square";
      const disabled = service.running ? "" : " disabled";
      const missing = service.installed === false ? " service-missing" : "";
      return `
        <div class="service-row${missing}">
          <div class="service-name">${escapeHtml(service.name)}${service.installed === false ? '<small>未安装</small>' : ""}</div>
          <div class="state-cell">
            <span class="${shapeClass}"></span>
            <button class="auto-badge ${service.auto ? "is-active" : ""}" type="button" data-service="${index}" data-service-action="auto" title="${service.auto ? "移出一键套件" : "加入一键套件"}">A</button>
          </div>
          <button class="primary" type="button" data-service="${index}" data-service-action="toggle">${service.running ? "停止" : "启动"}</button>
          <button class="ghost" type="button" data-service="${index}" data-service-action="restart"${disabled}>重启</button>
          <button class="primary" type="button" data-service="${index}" data-service-action="config">配置</button>
        </div>
      `;
    })
    .join("");
}

function renderLogs() {
  $("#logLines").innerHTML = logLines.map((line) => `<div>${line}</div>`).join("");
}

function renderLegend() {
  const activeServices = services.filter((service) => service.name.includes("Apache") || service.running || service.name.includes("Redis") || service.name.includes("MinIO"));
  $("#statusLegend").innerHTML = activeServices
    .slice(0, 4)
    .map((service) => {
      const shape = service.running ? "shape-triangle" : "shape-square";
      return `<span class="legend-item"><i class="${shape}"></i>${service.name.replace("2.4.39", "").replace("7.2.4", "").replace("2025.04", "")}</span>`;
    })
    .join("");
}

function siteUrl(site) {
  const sitePort = String(site.port || "");
  const protocol = sitePort === "443" ? "https" : "http";
  const port = sitePort && !["80", "443"].includes(sitePort) ? `:${sitePort}` : "";
  return `${protocol}://${site.domain}${port}/`;
}

function phpMyAdminUrl() {
  return systemSettings.phpMyAdminUrl || "http://127.0.0.1/phpmyadmin";
}

function renderRows(kind, query = "") {
  const q = query.trim().toLowerCase();
  const configsByKind = {
    website: {
      target: "#websiteRows",
      data: websites,
      cells: (item, index, displayIndex) => `
        <td>${displayIndex + 1}</td><td>${escapeHtml(item.domain)}</td><td>${escapeHtml(item.port)}</td><td class="path">${escapeHtml(item.path)}</td>
        <td class="status-normal">${escapeHtml(item.status)}</td><td>${escapeHtml(item.expire)}</td>
        <td><div class="row-actions">
          <button class="manage-button" type="button" data-open-url="${escapeHtml(siteUrl(item))}">打开</button>
          <button class="manage-button" type="button" data-open-folder="${escapeHtml(item.path)}">目录</button>
          <button class="manage-button" type="button" data-edit-record="site" data-record-index="${index}">编辑</button>
          <button class="manage-button" type="button" data-open-site-config="${index}">配置</button>
          <button class="manage-button danger" type="button" data-remove-record="sites" data-record-index="${index}" data-record-label="${escapeHtml(item.domain)}">移除</button>
        </div></td>
      `
    },
    database: {
      target: "#databaseRows",
      data: databases,
      cells: (item, index, displayIndex) => `
        <td>${displayIndex + 1}</td><td>${escapeHtml(item.db)}</td><td>${escapeHtml(item.user)}</td><td>${escapeHtml(item.pass)}</td>
        <td class="status-normal">${escapeHtml(item.status)}</td>
        <td><div class="row-actions">
          <button class="manage-button" type="button" data-open-url="${escapeHtml(phpMyAdminUrl())}">phpMyAdmin</button>
          <button class="manage-button" type="button" data-export-database="${index}" data-record-label="${escapeHtml(item.db)}">导出</button>
          <button class="manage-button" type="button" data-row-note="数据库 ${escapeHtml(item.db)} 可通过 phpMyAdmin 或 mysql 客户端做导入、导出、删除等高风险操作。">说明</button>
          <button class="manage-button danger" type="button" data-remove-record="databases" data-record-index="${index}" data-record-label="${escapeHtml(item.db)}">移除</button>
        </div></td>
      `
    },
    ftp: {
      target: "#ftpRows",
      data: ftpAccounts,
      cells: (item, index, displayIndex) => `
        <td>${displayIndex + 1}</td><td>${escapeHtml(item.user)}</td><td class="path">${escapeHtml(item.path)}</td><td>${escapeHtml(item.permission)}</td>
        <td class="status-normal">${escapeHtml(item.status)}</td>
        <td><div class="row-actions">
          <button class="manage-button" type="button" data-open-folder="${escapeHtml(item.path)}">目录</button>
          <button class="manage-button" type="button" data-edit-record="ftp" data-record-index="${index}">编辑</button>
          <button class="manage-button" type="button" data-row-note="FTP 账号 ${escapeHtml(item.user)} 已同步到 FileZilla Server.xml。移除记录时只会清理 XP.CN 托管的账号配置，不会删除本机目录。">说明</button>
          <button class="manage-button danger" type="button" data-remove-record="ftp" data-record-index="${index}" data-record-label="${escapeHtml(item.user)}">移除</button>
        </div></td>
      `
    }
  };

  const config = configsByKind[kind];
  const rows = config.data
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => JSON.stringify(item).toLowerCase().includes(q));
  $(config.target).innerHTML = rows.map(({ item, index }, displayIndex) => `<tr>${config.cells(item, index, displayIndex)}</tr>`).join("");
}

function formatBytes(size) {
  const value = Number(size) || 0;
  if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  if (value >= 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${value} B`;
}

function renderBackups() {
  const list = $("#backupList");
  if (!list) return;
  if (!databaseBackups.length) {
    list.innerHTML = '<div class="backup-empty">暂无备份</div>';
    return;
  }
  list.innerHTML = databaseBackups
    .slice(0, 6)
    .map((item) => `
      <div class="backup-row">
        <span class="backup-name" title="${escapeHtml(item.path)}">${escapeHtml(item.name)}</span>
        <span>${formatBytes(item.size)}</span>
        <button class="manage-button" type="button" data-open-file="${escapeHtml(item.path)}">打开</button>
        <button class="manage-button" type="button" data-open-folder="${escapeHtml(item.path.replace(/[/\\\\][^/\\\\]+$/, ""))}">目录</button>
      </div>
    `)
    .join("");
}

function renderSoftwareTabs() {
  $("#softwareTabs").innerHTML = softwareTabs
    .map((tab) => `<button class="tab-button ${tab === activeSoftwareTab ? "is-active" : ""}" type="button" data-software-tab="${tab}">${tab}</button>`)
    .join("");
  $("#categoryTabs").innerHTML = categoryTabs
    .map((tab) => `<button class="category-button ${tab === activeCategory ? "is-active" : ""}" type="button" data-category="${tab}">${tab}</button>`)
    .join("");
}

function softwareIcon(icon) {
  if (icon === "db") return '<span class="db-icon"></span>';
  if (icon === "redis") return '<span class="redis-icon"></span>';
  if (icon === "minio") return '<span class="minio-icon"></span>';
  return '<span class="monitor-icon"></span>';
}

function renderSoftware() {
  const rows = software.filter((item) => {
    const tabMatch = activeSoftwareTab === "全部" || item.group === activeSoftwareTab;
    const categoryMatch = activeCategory === "全部" || item.category === activeCategory;
    return tabMatch && categoryMatch;
  });

  $("#softwareList").innerHTML = rows
    .map((item) => {
      const index = software.indexOf(item);
      const installState = softwareInstallState(item);
      return `
        <div class="software-row">
          <div class="software-icon">${softwareIcon(item.icon)}</div>
          <div class="software-name">${escapeHtml(item.name)}</div>
          <div class="software-desc" title="${escapeHtml(item.executable || item.installDir || item.desc)}">${escapeHtml(item.desc)}</div>
          <button class="install-button" type="button" data-software-index="${index}" title="${escapeHtml(installState.title)}" ${installState.canInstall ? "" : "disabled"}>${installState.text}</button>
          ${item.serviceId || item.installed ? `<button class="primary" type="button" data-open-settings="${index}">设置</button>` : "<span></span>"}
        </div>
      `;
    })
    .join("");
}

function renderSettings() {
  $$(".settings-tab").forEach((item) => item.classList.toggle("is-active", item.dataset.settings === activeSettings));
  const showConfigTabs = activeSettings === "config";
  $("#configTabs").hidden = !showConfigTabs;
  if (activeSettings === "system") {
    renderSystemSettings();
    return;
  }
  if (activeSettings === "files") {
    renderFileLocations();
    return;
  }

  $("#configTabs").innerHTML = configs
    .map((item) => {
      const meta = configFiles.find((file) => file.id === item);
      const label = meta?.label || item;
      const missing = meta && !meta.exists ? " is-missing" : "";
      return `<button class="config-tab ${item === activeConfig ? "is-active" : ""}${missing}" type="button" data-config="${item}">${escapeHtml(label)}</button>`;
    })
    .join("");
  const meta = configFiles.find((file) => file.id === activeConfig) || { id: activeConfig, label: activeConfig, path: "" };
  const content = configContents[activeConfig] ?? "";
  $("#settingsContent").innerHTML = `
    <div class="config-head">
      <div>
        <p class="config-version">• ${escapeHtml(meta.label || meta.id)}</p>
        <p class="config-path">${escapeHtml(meta.path || "后端启动后显示真实路径")}</p>
      </div>
      <div class="config-actions">
        <button class="ghost" type="button" data-action="reload-config">刷新</button>
        <button class="primary" type="button" data-action="save-config">保存</button>
      </div>
    </div>
    <textarea class="config-editor" id="configEditor" spellcheck="false">${escapeHtml(content)}</textarea>
  `;
}

function renderSystemSettings() {
  $("#settingsContent").innerHTML = `
    <div class="config-head">
      <div>
        <p class="config-version">• 系统设置</p>
        <p class="config-path">保存到 ${escapeHtml(systemSettings.configPath || "data/config.json")}</p>
      </div>
      <div class="config-actions">
        <button class="primary" type="button" data-action="save-system-settings">保存</button>
      </div>
    </div>
    <div class="settings-form">
      <label class="toggle-row">
        <input type="checkbox" id="settingAutostart" ${systemSettings.autostart ? "checked" : ""} />
        <span>开机自启偏好</span>
      </label>
      <label class="toggle-row">
        <input type="checkbox" id="settingStartSuite" ${systemSettings.startSuiteOnLaunch ? "checked" : ""} />
        <span>管理台启动后自动启动套件</span>
      </label>
      <label class="settings-field" for="settingPhpMyAdmin">
        <span>phpMyAdmin 地址</span>
        <input id="settingPhpMyAdmin" type="url" value="${escapeHtml(systemSettings.phpMyAdminUrl || "")}" placeholder="http://127.0.0.1/phpmyadmin" />
      </label>
      <div class="settings-readonly">
        <div><span>管理台端口</span><strong>${escapeHtml(systemSettings.port || "")}</strong></div>
        <div><span>数据目录</span><strong>${escapeHtml(systemSettings.dataDir || "")}</strong></div>
        <div><span>开机自启脚本</span><strong>${escapeHtml(systemSettings.autostartPath || "未找到 Startup 目录")}</strong></div>
      </div>
    </div>
  `;
}

function renderFileLocations() {
  const paths = systemSettings.paths || {};
  const labels = Object.keys(systemSettings.editablePathLabels || {}).length ? systemSettings.editablePathLabels : {
    phpStudyRoot: "phpStudy 根目录",
    wwwRoot: "网站根目录",
    apacheRoot: "Apache",
    nginxRoot: "Nginx",
    mysql57Root: "MySQL 5.7",
    mysql80Root: "MySQL 8.0",
    mariadbRoot: "MariaDB",
    phpRoot: "PHP",
    ftpRoot: "FTP",
    redisRoot: "Redis",
    minioRoot: "MinIO"
  };
  const entries = Object.entries(labels).map(([key, label]) => [key, label, paths[key] || ""]);
  $("#settingsContent").innerHTML = `
    <div class="config-head">
      <div>
        <p class="config-version">• 文件位置</p>
        <p class="config-path">保存后会同步更新服务启动路径、软件安装目录和配置文件路径。</p>
      </div>
      <div class="config-actions">
        <button class="primary" type="button" data-action="save-path-settings">保存</button>
      </div>
    </div>
    <div class="path-list">
      ${entries.map(([key, label, value]) => `
        <div class="path-row">
          <label for="settingPath-${escapeHtml(key)}">${escapeHtml(label)}</label>
          <input id="settingPath-${escapeHtml(key)}" data-path-key="${escapeHtml(key)}" type="text" value="${escapeHtml(value)}" />
          <button class="manage-button" type="button" data-open-folder="${escapeHtml(value || "")}" ${value ? "" : "disabled"}>目录</button>
        </div>
      `).join("")}
    </div>
  `;
}

async function loadConfigFile(id = activeConfig) {
  if (!canUseBackend) return;
  try {
    const file = await api(`/api/config-files/${encodeURIComponent(id)}`);
    configContents[id] = file.content || "";
    mergeConfigFileMeta(file);
    renderSettings();
  } catch (error) {
    addLog(`读取配置失败：${error.message}`);
  }
}

async function openSiteConfig(index) {
  if (!canUseBackend) {
    activeConfig = "vhosts.conf";
    setView("settings");
    return;
  }
  try {
    const file = await api(`/api/sites/${index}/config`);
    mergeConfigFileMeta(file);
    configContents[file.id] = file.content || "";
    activeConfig = file.id;
    activeSettings = "config";
    setView("settings");
    renderSettings();
  } catch (error) {
    addLog(`读取网站配置失败：${error.message}`);
  }
}

function setView(view) {
  activeView = view;
  $$(".nav-item").forEach((item) => item.classList.toggle("is-active", item.dataset.view === view));
  $$(".view").forEach((item) => item.classList.toggle("is-visible", item.id === `view-${view}`));
  if (view === "settings" && activeSettings === "config") loadConfigFile(activeConfig);
}

function field(label, name, value = "", type = "text") {
  return `
    <div class="field">
      <label for="${name}">${label}</label>
      <input id="${name}" name="${name}" type="${type}" value="${value}" required />
    </div>
  `;
}

function selectField(label, name, value = "", options = []) {
  return `
    <div class="field">
      <label for="${name}">${label}</label>
      <select id="${name}" name="${name}" required>
        ${options.map((option) => `<option value="${escapeHtml(option)}" ${option === value ? "selected" : ""}>${escapeHtml(option)}</option>`).join("")}
      </select>
    </div>
  `;
}

function openModal(type, title = "", initial = {}, index = -1) {
  modalType = type;
  modalIndex = index;
  const modalTitle = $("#modalTitle");
  const fields = $("#modalFields");

  const templates = {
    site: {
      title: "创建网站",
      html: field("域名", "domain", initial.domain || "demo.local") + field("端口", "port", initial.port || "80") + field("根目录", "path", initial.path || "D:/phpstudy_pro/WWW/demo")
    },
    "site-edit": {
      title: "编辑网站",
      html: field("域名", "domain", initial.domain || "") + field("端口", "port", initial.port || "80") + field("根目录", "path", initial.path || "")
    },
    database: {
      title: "创建数据库",
      html: field("数据库", "db", "demo_app") + field("用户", "user", "demo_app") + field("密码", "pass", "123456", "password")
    },
    ftp: {
      title: "创建FTP",
      html: field("用户名", "user", initial.user || "demo_ftp") + field("根目录", "path", initial.path || "D:/phpstudy_pro/WWW/demo") + selectField("权限", "permission", initial.permission || "读写", ["读写", "只读", "只写"])
    },
    "ftp-edit": {
      title: "编辑FTP",
      html: field("用户名", "user", initial.user || "") + field("根目录", "path", initial.path || "") + selectField("权限", "permission", initial.permission || "读写", ["读写", "只读", "只写"])
    },
    root: {
      title: "修改root密码",
      html: field("新密码", "pass", "", "password") + '<p class="modal-note">确认后会调用本机 mysql.exe 修改 root 密码，并同步保存到 data/config.json。</p>'
    },
    config: {
      title: title || "配置",
      html: '<p class="modal-note">配置文件请在“设置 / 配置文件”中读取、编辑并保存。服务路径可在 data/config.json 中调整。</p>'
    }
  };

  modalTitle.textContent = templates[type].title;
  fields.innerHTML = templates[type].html;
  $("#modalBackdrop").classList.add("is-open");
  $("#modalBackdrop").setAttribute("aria-hidden", "false");
  const firstInput = $("input", fields);
  if (firstInput) firstInput.select();
}

function closeModal() {
  $("#modalBackdrop").classList.remove("is-open");
  $("#modalBackdrop").setAttribute("aria-hidden", "true");
  $("#modalForm").reset();
  modalIndex = -1;
}

async function handleModalSubmit(event) {
  event.preventDefault();
  const data = Object.fromEntries(new FormData(event.currentTarget).entries());
  const submit = event.submitter;
  if (submit) submit.disabled = true;
  try {
    if (canUseBackend) {
      if (modalType === "site") await postApi("/api/sites", data);
      if (modalType === "site-edit") await api(`/api/sites/${modalIndex}`, { method: "PUT", body: JSON.stringify(data) }).then((result) => result.state ? applyState(result.state) : result);
      if (modalType === "database") await postApi("/api/databases", data);
      if (modalType === "ftp") await postApi("/api/ftp", data);
      if (modalType === "ftp-edit") await api(`/api/ftp/${modalIndex}`, { method: "PUT", body: JSON.stringify(data) }).then((result) => result.state ? applyState(result.state) : result);
      if (modalType === "root") await postApi("/api/databases/root-password", data);
      closeModal();
      return;
    }

    if (modalType === "site") {
      websites.push({ domain: data.domain, port: data.port, path: data.path, status: "正常", expire: "2035-12-03" });
      renderRows("website");
      addLog(`网站 ${data.domain} 已创建`);
    }
    if (modalType === "site-edit" && websites[modalIndex]) {
      websites[modalIndex] = { ...websites[modalIndex], domain: data.domain, port: data.port, path: data.path };
      renderRows("website");
      addLog(`网站 ${data.domain} 已更新`);
    }
    if (modalType === "database") {
      databases.push({ db: data.db, user: data.user, pass: "******", status: "正常" });
      renderRows("database");
      addLog(`数据库 ${data.db} 已创建`);
    }
    if (modalType === "ftp") {
      ftpAccounts.push({ user: data.user, path: data.path, permission: data.permission, status: "正常" });
      renderRows("ftp");
      addLog(`FTP账号 ${data.user} 已创建`);
    }
    if (modalType === "ftp-edit" && ftpAccounts[modalIndex]) {
      ftpAccounts[modalIndex] = { ...ftpAccounts[modalIndex], user: data.user, path: data.path, permission: data.permission };
      renderRows("ftp");
      addLog(`FTP账号 ${data.user} 已更新`);
    }
    if (modalType === "root") addLog("root 密码已修改");
    closeModal();
  } catch (error) {
    addLog(`操作失败：${error.message}`);
  } finally {
    if (submit) submit.disabled = false;
  }
}

function bindEvents() {
  document.addEventListener("click", async (event) => {
    const nav = event.target.closest(".nav-item");
    if (nav) setView(nav.dataset.view);

    const serviceButton = event.target.closest("[data-service-action]");
    if (serviceButton) {
      const service = services[Number(serviceButton.dataset.service)];
      const action = serviceButton.dataset.serviceAction;
      if (action === "config") {
        const match = configFiles.find((file) => file.path && service.configFile && file.path.toLowerCase() === service.configFile.toLowerCase());
        if (match) {
          activeConfig = match.id;
          setView("settings");
        } else {
          openModal("config", `${service.name} 配置`);
        }
        return;
      }

      if (action === "auto") {
        const previous = !!service.auto;
        const next = !previous;
        if (canUseBackend && service.id) {
          service.auto = next;
          renderQuickStatus();
          renderServices();
          try {
            await postApi(`/api/services/${encodeURIComponent(service.id)}/auto`, { auto: next });
          } catch (error) {
            service.auto = previous;
            renderQuickStatus();
            renderServices();
            addLog(`操作失败：${error.message}`);
          }
        } else {
          service.auto = next;
          addLog(`${service.name} 已${next ? "加入" : "移出"}一键套件`);
          renderQuickStatus();
          renderServices();
        }
        return;
      }

      if (canUseBackend && service.id) {
        const next = action === "toggle" ? (service.running ? "stop" : "start") : "restart";
        await runBackend(() => postApi(`/api/services/${encodeURIComponent(service.id)}/${next}`));
      } else {
        if (action === "toggle") {
          service.running = !service.running;
          addLog(`${service.name} ${service.running ? "已启动" : "已停止"}`);
        }
        if (action === "restart" && service.running) addLog(`${service.name} 已重启`);
        renderServices();
        renderLegend();
      }
    }

    const openType = event.target.closest("[data-open-modal]");
    if (openType) openModal(openType.dataset.openModal);

    const configTarget = event.target.closest("[data-config-target]");
    if (configTarget) openModal("config", `${configTarget.dataset.configTarget} 管理`);

    const openFolder = event.target.closest("[data-open-folder]");
    if (openFolder) {
      await openLocal("folder", { path: openFolder.dataset.openFolder }, () => addLog(`目录：${openFolder.dataset.openFolder}`));
    }

    const openFile = event.target.closest("[data-open-file]");
    if (openFile) {
      await openLocal("file", { path: openFile.dataset.openFile }, () => addLog(`文件：${openFile.dataset.openFile}`));
    }

    const openUrlButton = event.target.closest("[data-open-url]");
    if (openUrlButton) {
      const url = openUrlButton.dataset.openUrl;
      await openLocal("url", { url }, () => window.open(url, "_blank"));
    }

    const openConfig = event.target.closest("[data-open-config]");
    if (openConfig) {
      activeConfig = openConfig.dataset.openConfig;
      setView("settings");
    }

    const openSiteConfigButton = event.target.closest("[data-open-site-config]");
    if (openSiteConfigButton) {
      const index = Number(openSiteConfigButton.dataset.openSiteConfig);
      if (Number.isInteger(index)) await openSiteConfig(index);
    }

    const rowNote = event.target.closest("[data-row-note]");
    if (rowNote) {
      openModal("config", "操作说明");
      $("#modalFields").innerHTML = `<p class="modal-note">${escapeHtml(rowNote.dataset.rowNote)}</p>`;
    }

    const exportDatabase = event.target.closest("[data-export-database]");
    if (exportDatabase) {
      const index = Number(exportDatabase.dataset.exportDatabase);
      if (!Number.isInteger(index)) return;
      if (canUseBackend) {
        await runBackend(() => postApi(`/api/databases/${index}/export`));
      } else {
        addLog(`数据库 ${exportDatabase.dataset.recordLabel || index} 已准备导出`);
      }
    }

    const editRecord = event.target.closest("[data-edit-record]");
    if (editRecord) {
      const index = Number(editRecord.dataset.recordIndex);
      if (!Number.isInteger(index)) return;
      if (editRecord.dataset.editRecord === "site" && websites[index]) {
        openModal("site-edit", "", websites[index], index);
      }
      if (editRecord.dataset.editRecord === "ftp" && ftpAccounts[index]) {
        openModal("ftp-edit", "", ftpAccounts[index], index);
      }
    }

    const removeRecord = event.target.closest("[data-remove-record]");
    if (removeRecord) {
      const kind = removeRecord.dataset.removeRecord;
      const index = Number(removeRecord.dataset.recordIndex);
      const label = removeRecord.dataset.recordLabel || "该记录";
      if (!Number.isInteger(index)) return;
      const confirmed = window.confirm(`只会从管理台移除记录，不会删除本机文件或数据库。\n确认移除 ${label}？`);
      if (!confirmed) return;
      if (canUseBackend) {
        await runBackend(() => deleteApi(`/api/${kind}/${index}`));
      } else {
        const local = {
          sites: { label: "网站", items: websites, view: "website" },
          databases: { label: "数据库", items: databases, view: "database" },
          ftp: { label: "FTP账号", items: ftpAccounts, view: "ftp" }
        }[kind];
        if (local && local.items[index]) {
          local.items.splice(index, 1);
          addLog(`${local.label} ${label} 已从管理台移除`);
          renderRows(local.view);
        }
      }
    }

    const closeButton = event.target.closest('[data-action="close-modal"]');
    if (closeButton) closeModal();

    const clearLog = event.target.closest('[data-action="clear-log"]');
    if (clearLog) {
      if (canUseBackend) await runBackend(() => postApi("/api/logs/clear"));
      else {
        logLines.length = 0;
        renderLogs();
      }
    }

    const logTab = event.target.closest(".log-tab");
    if (logTab) {
      $$(".log-tab").forEach((item) => item.classList.toggle("is-active", item === logTab));
      $("#logTitle").textContent = logTab.dataset.logTab;
    }

    const quickAction = event.target.closest("[data-action]");
    if (quickAction?.dataset.action === "suite-toggle") {
      const anyRunning = suiteRunning();
      if (canUseBackend) {
        await runBackend(() => postApi(`/api/suite/${anyRunning ? "stop" : "start"}`));
      } else {
        autoServices().forEach((item) => {
          item.running = !anyRunning;
        });
        quickAction.textContent = anyRunning ? "启动" : "停止";
        addLog(`WNMP 套件${anyRunning ? "已停止" : "已启动"}`);
        renderServices();
        renderLegend();
      }
    }
    if (quickAction?.dataset.action === "autostart-toggle") {
      const next = !systemSettings.autostart;
      if (canUseBackend) {
        await runBackend(() => postApi("/api/settings/system", { autostart: next }));
      } else {
        systemSettings.autostart = next;
        addLog(`开机自启偏好已${next ? "启用" : "关闭"}`);
        renderQuickStatus();
        if (activeView === "settings") renderSettings();
      }
    }
    if (quickAction?.dataset.action === "open-db-tool") {
      const url = phpMyAdminUrl();
      addLog("数据库工具已打开");
      if (canUseBackend) await runBackend(() => postApi("/api/open/url", { url }));
      else window.open(url, "_blank");
    }
    if (quickAction?.dataset.action === "show-all") {
      activeSoftwareTab = "全部";
      activeCategory = "全部";
      renderSoftwareTabs();
      renderSoftware();
    }
    if (quickAction?.dataset.action === "refresh-backups") {
      if (canUseBackend) {
        await runBackend(async () => {
          const data = await api("/api/databases/backups");
          replaceArray(databaseBackups, data.backups || []);
          renderBackups();
        });
      } else {
        renderBackups();
      }
    }

    const swTab = event.target.closest("[data-software-tab]");
    if (swTab) {
      activeSoftwareTab = swTab.dataset.softwareTab;
      renderSoftwareTabs();
      renderSoftware();
    }

    const cat = event.target.closest("[data-category]");
    if (cat) {
      activeCategory = cat.dataset.category;
      renderSoftwareTabs();
      renderSoftware();
    }

    const softwareButton = event.target.closest("[data-software-index]");
    if (softwareButton) {
      const item = software[Number(softwareButton.dataset.softwareIndex)];
      if (item.installed) return;
      if (item.installable === false) {
        addLog(item.installNote || `${item.name} 未配置下载地址`);
        return;
      }
      if (canUseBackend && item.id) await runBackend(() => postApi(`/api/software/${encodeURIComponent(item.id)}/install`));
      else {
        item.installed = true;
        addLog(`${item.name} 已安装`);
        renderSoftware();
      }
    }

    const softwareSetting = event.target.closest("[data-open-settings]");
    if (softwareSetting) {
      const item = software[Number(softwareSetting.dataset.openSettings)];
      const service = services.find((entry) => entry.id && entry.id === item?.serviceId);
      const match = configFiles.find((file) => file.path && service?.configFile && file.path.toLowerCase() === service.configFile.toLowerCase());
      if (match) {
        activeConfig = match.id;
        setView("settings");
      } else {
        openModal("config", item ? `${item.name} 设置` : "软件设置");
      }
    }

    const config = event.target.closest("[data-config]");
    if (config) {
      activeConfig = config.dataset.config;
      renderSettings();
      await loadConfigFile(activeConfig);
    }

    const settingsTab = event.target.closest("[data-settings]");
    if (settingsTab) {
      activeSettings = settingsTab.dataset.settings;
      renderSettings();
      if (activeSettings === "config") await loadConfigFile(activeConfig);
    }

    if (quickAction?.dataset.action === "reload-config") {
      await loadConfigFile(activeConfig);
    }
    if (quickAction?.dataset.action === "save-config") {
      const editor = $("#configEditor");
      const content = editor ? editor.value : "";
      configContents[activeConfig] = content;
      await runBackend(() => postApi(`/api/config-files/${encodeURIComponent(activeConfig)}`, { content }));
    }
    if (quickAction?.dataset.action === "save-system-settings") {
      const payload = {
        autostart: !!$("#settingAutostart")?.checked,
        startSuiteOnLaunch: !!$("#settingStartSuite")?.checked,
        phpMyAdminUrl: $("#settingPhpMyAdmin")?.value || phpMyAdminUrl()
      };
      await runBackend(() => postApi("/api/settings/system", payload), () => {
        Object.assign(systemSettings, payload);
        addLog("系统设置已保存");
        renderQuickStatus();
        renderSettings();
      });
    }
    if (quickAction?.dataset.action === "save-path-settings") {
      const payload = {};
      $$("[data-path-key]").forEach((input) => {
        payload[input.dataset.pathKey] = input.value;
      });
      await runBackend(() => postApi("/api/settings/paths", payload), () => {
        systemSettings.paths = { ...(systemSettings.paths || {}), ...payload };
        addLog("本机路径已保存");
        renderSettings();
      });
    }

    if (event.target === $("#modalBackdrop")) closeModal();
  });

  document.addEventListener("input", (event) => {
    const search = event.target.closest("[data-search]");
    if (search) renderRows(search.dataset.search, search.value);
    if (event.target.id === "configEditor") configContents[activeConfig] = event.target.value;
  });

  $("#modalForm").addEventListener("submit", handleModalSubmit);
}

function init() {
  renderServices();
  renderLogs();
  renderLegend();
  renderRows("website");
  renderRows("database");
  renderBackups();
  renderRows("ftp");
  renderSoftwareTabs();
  renderSoftware();
  renderSettings();
  bindEvents();
  const initialView = new URLSearchParams(window.location.search).get("view") || window.location.hash.replace("#", "");
  if (["home", "website", "database", "ftp", "software", "settings"].includes(initialView)) {
    setView(initialView);
  }
  refreshState();
}

init();

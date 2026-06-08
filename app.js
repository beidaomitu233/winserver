const services = [
  { name: "Apache2.4.39", type: "square", running: false, auto: true },
  { name: "FTP0.9.60", type: "square", running: false, auto: true },
  { name: "MySQL5.7.26", type: "square", running: false, auto: true },
  { name: "MySQL8.0.12", type: "triangle", running: true, auto: true },
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
  { name: "Redis7.2.4", category: "redis", group: "系统环境", desc: "缓存、Session、队列与分布式锁服务", icon: "redis", installed: true },
  { name: "MinIO RELEASE", category: "对象存储", group: "工具", desc: "兼容 S3 API 的本地对象存储服务", icon: "minio", installed: true },
  { name: "MySQL5.0.96", category: "数据库", group: "系统环境", desc: "数据库服务", icon: "db", installed: false },
  { name: "MySQL5.1.60", category: "数据库", group: "系统环境", desc: "数据库服务", icon: "db", installed: false },
  { name: "Redis6.2.14", category: "redis", group: "系统环境", desc: "高性能键值缓存服务", icon: "redis", installed: false },
  { name: "FileZilla Server", category: "文件服务", group: "工具", desc: "FTP 文件服务", icon: "monitor", installed: false },
  { name: "php7.3.4nts", category: "php", group: "系统环境", desc: "PHP 运行环境", icon: "monitor", installed: true },
  { name: "composer2.7", category: "composer", group: "工具", desc: "PHP 依赖管理工具", icon: "monitor", installed: true }
];

const configs = ["php.ini", "httpd.conf", "nginx.conf", "vhosts.conf", "mysql.ini", "redis.conf", "minio.env", "hosts"];
const logLines = [
  "2026-06-07 21:09:04 MySQL8.0.12 已启动",
  "2026-06-07 21:09:04 MySQL8.0.12 正在启动....."
];

const configFiles = configs.map((id) => ({ id, label: id, path: "", exists: false }));
let activeSoftwareTab = "全部";
let activeCategory = "全部";
let activeConfig = "php.ini";
let activeView = "home";
let modalType = "";
let backendOnline = false;
const configContents = {};

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
  replaceArray(services, state.services || []);
  replaceArray(websites, state.websites || []);
  replaceArray(databases, state.databases || []);
  replaceArray(ftpAccounts, state.ftpAccounts || []);
  replaceArray(software, state.software || []);
  replaceArray(configFiles, state.configFiles || configFiles);
  replaceArray(configs, configFiles.map((item) => item.id));
  replaceArray(logLines, Array.isArray(state.logs) ? state.logs : logLines);
  if (!configs.includes(activeConfig)) activeConfig = configs[0] || "php.ini";
  const version = $(".version");
  if (version && state.version) version.innerHTML = `<span>ⓘ</span> 版本：${escapeHtml(state.version)}`;
  renderAll();
  if (activeView === "settings") loadConfigFile(activeConfig);
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
  renderRows("ftp");
  renderSoftwareTabs();
  renderSoftware();
  renderSettings();
}

function renderQuickStatus() {
  const suiteButton = $('[data-action="suite-toggle"]');
  const suiteDot = suiteButton?.parentElement.querySelector(".status-dot");
  const autoServices = services.filter((item) => item.auto);
  const anyRunning = autoServices.some((item) => item.running);
  if (suiteButton) suiteButton.textContent = anyRunning ? "停止" : "启动";
  if (suiteDot) {
    suiteDot.classList.toggle("running", anyRunning);
    suiteDot.classList.toggle("stopped", !anyRunning);
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
      const shape = service.running ? "triangle" : service.type;
      const shapeClass = shape === "triangle" ? "shape-triangle" : "shape-square";
      const disabled = service.running ? "" : " disabled";
      const missing = service.installed === false ? " service-missing" : "";
      return `
        <div class="service-row${missing}">
          <div class="service-name">${escapeHtml(service.name)}${service.installed === false ? '<small>未安装</small>' : ""}</div>
          <div class="state-cell">
            <span class="${shapeClass}"></span>
            <span class="auto-badge">${service.auto ? "A" : ""}</span>
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

function renderRows(kind, query = "") {
  const q = query.trim().toLowerCase();
  const configsByKind = {
    website: {
      target: "#websiteRows",
      data: websites,
      cells: (item, index) => `
        <td>${index + 1}</td><td>${escapeHtml(item.domain)}</td><td>${escapeHtml(item.port)}</td><td class="path">${escapeHtml(item.path)}</td>
        <td class="status-normal">${escapeHtml(item.status)}</td><td>${escapeHtml(item.expire)}</td>
        <td><div class="row-actions">
          <button class="manage-button" type="button" data-open-folder="${escapeHtml(item.path)}">目录</button>
          <button class="manage-button" type="button" data-open-config="vhosts.conf">配置</button>
        </div></td>
      `
    },
    database: {
      target: "#databaseRows",
      data: databases,
      cells: (item, index) => `
        <td>${index + 1}</td><td>${escapeHtml(item.db)}</td><td>${escapeHtml(item.user)}</td><td>${escapeHtml(item.pass)}</td>
        <td class="status-normal">${escapeHtml(item.status)}</td>
        <td><div class="row-actions">
          <button class="manage-button" type="button" data-open-url="http://127.0.0.1/phpmyadmin">phpMyAdmin</button>
          <button class="manage-button" type="button" data-row-note="数据库 ${escapeHtml(item.db)} 可通过 phpMyAdmin 或 mysql 客户端做导入、导出、删除等高风险操作。">说明</button>
        </div></td>
      `
    },
    ftp: {
      target: "#ftpRows",
      data: ftpAccounts,
      cells: (item, index) => `
        <td>${index + 1}</td><td>${escapeHtml(item.user)}</td><td class="path">${escapeHtml(item.path)}</td><td>${escapeHtml(item.permission)}</td>
        <td class="status-normal">${escapeHtml(item.status)}</td>
        <td><div class="row-actions">
          <button class="manage-button" type="button" data-open-folder="${escapeHtml(item.path)}">目录</button>
          <button class="manage-button" type="button" data-row-note="FTP 账号 ${escapeHtml(item.user)} 已记录在本地配置中。需要真实 FileZilla 用户同步时，可继续接入 FileZilla Server 配置写入。">说明</button>
        </div></td>
      `
    }
  };

  const config = configsByKind[kind];
  const rows = config.data.filter((item) => JSON.stringify(item).toLowerCase().includes(q));
  $(config.target).innerHTML = rows.map((item, index) => `<tr>${config.cells(item, index)}</tr>`).join("");
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
    .map((item, index) => `
      <div class="software-row">
        <div class="software-icon">${softwareIcon(item.icon)}</div>
        <div class="software-name">${escapeHtml(item.name)}</div>
        <div class="software-desc" title="${escapeHtml(item.executable || item.installDir || item.desc)}">${escapeHtml(item.desc)}</div>
        <button class="install-button" type="button" data-software-index="${software.indexOf(item)}" ${item.installed ? "disabled" : ""}>${item.installed ? "已安装" : "安装"}</button>
        ${item.serviceId || item.installed ? `<button class="primary" type="button" data-open-settings="${software.indexOf(item)}">设置</button>` : "<span></span>"}
      </div>
    `)
    .join("");
}

function renderSettings() {
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

async function loadConfigFile(id = activeConfig) {
  if (!canUseBackend) return;
  try {
    const file = await api(`/api/config-files/${encodeURIComponent(id)}`);
    configContents[id] = file.content || "";
    const index = configFiles.findIndex((item) => item.id === file.id);
    if (index >= 0) configFiles[index] = { ...configFiles[index], ...file };
    renderSettings();
  } catch (error) {
    addLog(`读取配置失败：${error.message}`);
  }
}

function setView(view) {
  activeView = view;
  $$(".nav-item").forEach((item) => item.classList.toggle("is-active", item.dataset.view === view));
  $$(".view").forEach((item) => item.classList.toggle("is-visible", item.id === `view-${view}`));
  if (view === "settings") loadConfigFile(activeConfig);
}

function field(label, name, value = "", type = "text") {
  return `
    <div class="field">
      <label for="${name}">${label}</label>
      <input id="${name}" name="${name}" type="${type}" value="${value}" required />
    </div>
  `;
}

function openModal(type, title = "") {
  modalType = type;
  const modalTitle = $("#modalTitle");
  const fields = $("#modalFields");

  const templates = {
    site: {
      title: "创建网站",
      html: field("域名", "domain", "demo.local") + field("端口", "port", "80") + field("根目录", "path", "D:/phpstudy_pro/WWW/demo")
    },
    database: {
      title: "创建数据库",
      html: field("数据库", "db", "demo_app") + field("用户", "user", "demo_app") + field("密码", "pass", "123456", "password")
    },
    ftp: {
      title: "创建FTP",
      html: field("用户名", "user", "demo_ftp") + field("根目录", "path", "D:/phpstudy_pro/WWW/demo") + field("权限", "permission", "读写")
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
}

async function handleModalSubmit(event) {
  event.preventDefault();
  const data = Object.fromEntries(new FormData(event.currentTarget).entries());
  const submit = event.submitter;
  if (submit) submit.disabled = true;
  try {
    if (canUseBackend) {
      if (modalType === "site") await postApi("/api/sites", data);
      if (modalType === "database") await postApi("/api/databases", data);
      if (modalType === "ftp") await postApi("/api/ftp", data);
      if (modalType === "root") await postApi("/api/databases/root-password", data);
      closeModal();
      return;
    }

    if (modalType === "site") {
      websites.push({ domain: data.domain, port: data.port, path: data.path, status: "正常", expire: "2035-12-03" });
      renderRows("website");
      addLog(`网站 ${data.domain} 已创建`);
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

    const rowNote = event.target.closest("[data-row-note]");
    if (rowNote) {
      openModal("config", "操作说明");
      $("#modalFields").innerHTML = `<p class="modal-note">${escapeHtml(rowNote.dataset.rowNote)}</p>`;
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
      const anyRunning = services.some((item) => item.running);
      if (canUseBackend) {
        await runBackend(() => postApi(`/api/suite/${anyRunning ? "stop" : "start"}`));
      } else {
        services.forEach((item) => {
          item.running = !anyRunning;
        });
        quickAction.textContent = anyRunning ? "启动" : "停止";
        addLog(`WNMP 套件${anyRunning ? "已停止" : "已启动"}`);
        renderServices();
        renderLegend();
      }
    }
    if (quickAction?.dataset.action === "autostart-toggle") {
      quickAction.textContent = quickAction.textContent === "启用" ? "停用" : "启用";
      const dot = quickAction.parentElement.querySelector(".status-dot");
      dot.classList.toggle("running");
      dot.classList.toggle("stopped");
      addLog(`开机自启已${quickAction.textContent === "停用" ? "启用" : "关闭"}`);
    }
    if (quickAction?.dataset.action === "open-db-tool") {
      addLog("数据库工具已打开");
      window.open("http://127.0.0.1/phpmyadmin", "_blank");
    }
    if (quickAction?.dataset.action === "show-all") {
      activeSoftwareTab = "全部";
      activeCategory = "全部";
      renderSoftwareTabs();
      renderSoftware();
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

    if (quickAction?.dataset.action === "reload-config") {
      await loadConfigFile(activeConfig);
    }
    if (quickAction?.dataset.action === "save-config") {
      const editor = $("#configEditor");
      const content = editor ? editor.value : "";
      configContents[activeConfig] = content;
      await runBackend(() => postApi(`/api/config-files/${encodeURIComponent(activeConfig)}`, { content }));
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

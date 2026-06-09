const http = require("http");
const https = require("https");
const fs = require("fs");
const path = require("path");
const os = require("os");
const net = require("net");
const { spawn, execFile } = require("child_process");

const ROOT = __dirname;
const DATA_DIR = process.env.XPCN_DATA_DIR ? path.resolve(process.env.XPCN_DATA_DIR) : path.join(ROOT, "data");
const CONFIG_PATH = path.join(DATA_DIR, "config.json");
const RUNTIME_DIR = path.join(ROOT, "runtime");
const HOSTS_PATH = process.env.XPCN_HOSTS_PATH ? path.resolve(process.env.XPCN_HOSTS_PATH) : "C:/Windows/System32/drivers/etc/hosts";
const STARTUP_DIR = process.env.XPCN_STARTUP_DIR || (process.env.APPDATA ? path.join(process.env.APPDATA, "Microsoft", "Windows", "Start Menu", "Programs", "Startup") : "");
const STARTUP_COMMAND = STARTUP_DIR ? path.join(STARTUP_DIR, "XPCN Local Manager.cmd") : "";
const SERVICE_DRY_RUN = process.env.XPCN_SERVICE_DRY_RUN === "1";
const PORT_OVERRIDE = process.env.XPCN_PORT !== undefined && process.env.XPCN_PORT !== "";
const REQUESTED_PORT = Number(process.env.XPCN_PORT || 18113);
const DEFAULT_PORT = Number.isFinite(REQUESTED_PORT) && REQUESTED_PORT > 0 ? REQUESTED_PORT : 18113;

function toSlash(value) {
  return String(value || "").replace(/\\/g, "/");
}

function exists(filePath) {
  return !!filePath && fs.existsSync(filePath);
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function writeJson(filePath, value) {
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, JSON.stringify(value, null, 2), "utf8");
}

function readJsonBody(req) {
  return new Promise((resolve, reject) => {
    let body = "";
    req.on("data", (chunk) => {
      body += chunk;
      if (body.length > 10 * 1024 * 1024) {
        reject(new Error("请求体过大"));
        req.destroy();
      }
    });
    req.on("end", () => {
      if (!body) return resolve({});
      try {
        resolve(JSON.parse(body));
      } catch (error) {
        reject(new Error("JSON 格式不正确"));
      }
    });
  });
}

function execFileAsync(file, args = [], options = {}) {
  return new Promise((resolve, reject) => {
    execFile(file, args, { windowsHide: true, maxBuffer: 1024 * 1024 * 5, ...options }, (error, stdout, stderr) => {
      if (error) {
        error.stdout = stdout;
        error.stderr = stderr;
        reject(error);
        return;
      }
      resolve({ stdout, stderr });
    });
  });
}

function execFileWithInputFile(file, args = [], inputPath, options = {}) {
  return new Promise((resolve, reject) => {
    let settled = false;
    let stdout = "";
    let stderr = "";
    const finish = (error, result) => {
      if (settled) return;
      settled = true;
      if (error) reject(error);
      else resolve(result);
    };
    const child = spawn(file, args, { windowsHide: true, stdio: ["pipe", "pipe", "pipe"], ...options });
    const input = fs.createReadStream(inputPath);
    const append = (target, chunk) => {
      const next = target + chunk.toString();
      if (next.length > 5 * 1024 * 1024) {
        child.kill();
        const error = new Error("命令输出过大");
        error.stdout = stdout;
        error.stderr = stderr;
        finish(error);
        return target;
      }
      return next;
    };
    child.stdout.on("data", (chunk) => {
      stdout = append(stdout, chunk);
    });
    child.stderr.on("data", (chunk) => {
      stderr = append(stderr, chunk);
    });
    child.on("error", finish);
    child.on("close", (code) => {
      if (code === 0) {
        finish(null, { stdout, stderr });
        return;
      }
      const error = new Error(`${path.basename(file)} 执行失败，退出码 ${code}`);
      error.stdout = stdout;
      error.stderr = stderr;
      finish(error);
    });
    input.on("error", (error) => {
      child.kill();
      finish(error);
    });
    child.stdin.on("error", () => {});
    input.pipe(child.stdin);
  });
}

function detectPaths() {
  const phpStudyRoot = process.env.XPCN_PHPSTUDY || (exists("D:/phpstudy_pro") ? "D:/phpstudy_pro" : path.join(RUNTIME_DIR, "phpstudy_pro"));
  const ext = path.join(phpStudyRoot, "Extensions");
  const runtimeComponents = path.join(RUNTIME_DIR, "components");
  const redisRoot = process.env.XPCN_REDIS || (exists("D:/redis-windows-7.2.4") ? "D:/redis-windows-7.2.4" : path.join(runtimeComponents, "redis"));
  const minioRoot = process.env.XPCN_MINIO || (exists("D:/minio/minio.exe") ? "D:/minio" : path.join(runtimeComponents, "minio"));

  return {
    phpStudyRoot,
    wwwRoot: path.join(phpStudyRoot, "WWW"),
    apacheRoot: path.join(ext, "Apache2.4.39"),
    nginxRoot: path.join(ext, "Nginx1.15.11"),
    ftpRoot: path.join(ext, "FTP0.9.60"),
    mysql57Root: path.join(ext, "MySQL5.7.26"),
    mysql80Root: path.join(ext, "MySQL8.0.12"),
    mariadbRoot: path.join(ext, "MariaDB10.11"),
    phpRoot: path.join(ext, "php", "php7.3.4nts"),
    redisRoot,
    minioRoot,
    runtimeComponents
  };
}

const EDITABLE_PATH_LABELS = {
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

const PHPSTUDY_CHILD_PATHS = {
  wwwRoot: ["WWW"],
  apacheRoot: ["Extensions", "Apache2.4.39"],
  nginxRoot: ["Extensions", "Nginx1.15.11"],
  mysql57Root: ["Extensions", "MySQL5.7.26"],
  mysql80Root: ["Extensions", "MySQL8.0.12"],
  mariadbRoot: ["Extensions", "MariaDB10.11"],
  phpRoot: ["Extensions", "php", "php7.3.4nts"],
  ftpRoot: ["Extensions", "FTP0.9.60"]
};

function slashJoin(...parts) {
  return toSlash(path.join(...parts));
}

function phpStudyChildPaths(root) {
  return Object.fromEntries(Object.entries(PHPSTUDY_CHILD_PATHS).map(([key, segments]) => [key, slashJoin(root, ...segments)]));
}

function mergeArrayById(baseItems, savedItems) {
  if (!Array.isArray(savedItems)) return baseItems;
  const merged = [...savedItems];
  for (const item of baseItems) {
    if (item?.id && !merged.some((saved) => saved?.id === item.id)) merged.push(item);
  }
  return merged;
}

function defaultConfig() {
  const p = detectPaths();
  const minioData = exists("D:/minio") ? "D:/minio" : path.join(p.minioRoot, "data");

  return {
    port: DEFAULT_PORT,
    paths: p,
    mysqlRootPassword: "",
    minio: {
      rootUser: "minioadmin",
      rootPassword: "minioadmin",
      dataDir: minioData,
      apiPort: 9000,
      consolePort: 9001
    },
    php: {
      cgiPort: 9073
    },
    systemSettings: {
      autostart: false,
      startSuiteOnLaunch: false,
      phpMyAdminUrl: "http://127.0.0.1/phpmyadmin"
    },
    services: [
      {
        id: "apache",
        name: "Apache2.4.39",
        type: "square",
        processName: "httpd.exe",
        port: 80,
        cwd: toSlash(p.apacheRoot),
        exe: toSlash(path.join(p.apacheRoot, "bin", "httpd.exe")),
        args: ["-f", toSlash(path.join(p.apacheRoot, "conf", "httpd.conf"))],
        configFile: toSlash(path.join(p.apacheRoot, "conf", "httpd.conf")),
        auto: true
      },
      {
        id: "ftp",
        name: "FTP0.9.60",
        type: "square",
        processName: "FileZilla Server.exe",
        port: 21,
        cwd: toSlash(p.ftpRoot),
        exe: toSlash(path.join(p.ftpRoot, "FileZilla Server.exe")),
        args: [],
        configFile: toSlash(path.join(p.ftpRoot, "FileZilla Server.xml")),
        auto: true
      },
      {
        id: "mysql57",
        name: "MySQL5.7.26",
        type: "square",
        processName: "mysqld.exe",
        port: 3307,
        cwd: toSlash(p.mysql57Root),
        exe: toSlash(path.join(p.mysql57Root, "bin", "mysqld.exe")),
        args: [`--defaults-file=${toSlash(path.join(p.mysql57Root, "my.ini"))}`],
        clientExe: toSlash(path.join(p.mysql57Root, "bin", "mysql.exe")),
        configFile: toSlash(path.join(p.mysql57Root, "my.ini")),
        auto: true
      },
      {
        id: "mysql80",
        name: "MySQL8.0.12",
        type: "square",
        processName: "mysqld.exe",
        port: 3306,
        cwd: toSlash(p.mysql80Root),
        exe: toSlash(path.join(p.mysql80Root, "bin", "mysqld.exe")),
        args: [`--defaults-file=${toSlash(path.join(p.mysql80Root, "my.ini"))}`],
        clientExe: toSlash(path.join(p.mysql80Root, "bin", "mysql.exe")),
        configFile: toSlash(path.join(p.mysql80Root, "my.ini")),
        auto: true
      },
      {
        id: "mariadb",
        name: "MariaDB10.11",
        type: "square",
        processName: "mysqld.exe",
        port: 3308,
        cwd: toSlash(p.mariadbRoot),
        exe: toSlash(path.join(p.mariadbRoot, "bin", "mysqld.exe")),
        args: [`--defaults-file=${toSlash(path.join(p.mariadbRoot, "my.ini"))}`],
        clientExe: toSlash(path.join(p.mariadbRoot, "bin", "mysql.exe")),
        configFile: toSlash(path.join(p.mariadbRoot, "my.ini")),
        auto: false
      },
      {
        id: "nginx",
        name: "Nginx1.15.11",
        type: "square",
        processName: "nginx.exe",
        port: 80,
        cwd: toSlash(p.nginxRoot),
        exe: toSlash(path.join(p.nginxRoot, "nginx.exe")),
        args: ["-p", toSlash(p.nginxRoot), "-c", "conf/nginx.conf"],
        configFile: toSlash(path.join(p.nginxRoot, "conf", "nginx.conf")),
        auto: true
      },
      {
        id: "redis",
        name: "Redis7.2.4",
        type: "square",
        processName: "redis-server.exe",
        port: 6379,
        cwd: toSlash(p.redisRoot),
        exe: toSlash(path.join(p.redisRoot, "redis-server.exe")),
        args: ["redis.conf"],
        configFile: toSlash(path.join(p.redisRoot, "redis.conf")),
        auto: false
      },
      {
        id: "php73",
        name: "PHP7.3 CGI",
        type: "square",
        processName: "php-cgi.exe",
        port: 9073,
        cwd: toSlash(p.phpRoot),
        exe: toSlash(path.join(p.phpRoot, "php-cgi.exe")),
        args: ["-b", "127.0.0.1:9073", "-c", toSlash(path.join(p.phpRoot, "php.ini"))],
        configFile: toSlash(path.join(p.phpRoot, "php.ini")),
        auto: true
      },
      {
        id: "minio",
        name: "MinIO",
        type: "square",
        processName: "minio.exe",
        port: 9000,
        cwd: toSlash(p.minioRoot),
        exe: toSlash(path.join(p.minioRoot, "minio.exe")),
        args: ["server", toSlash(minioData), "--address", ":9000", "--console-address", ":9001"],
        env: {
          MINIO_ROOT_USER: "minioadmin",
          MINIO_ROOT_PASSWORD: "minioadmin"
        },
        configFile: toSlash(path.join(p.minioRoot, "minio.env")),
        auto: false
      }
    ],
    sites: [
      { domain: "localhost", port: "80", path: toSlash(path.join(p.wwwRoot, "dist")), status: "正常", expire: "2035-12-03" }
    ],
    databases: [
      { db: "root", user: "root", pass: "******", status: "正常" }
    ],
    ftpAccounts: [
      { user: "localhost", path: toSlash(p.wwwRoot), permission: "读写", status: "正常" }
    ],
    software: [
      {
        id: "apache",
        name: "Apache2.4.39",
        category: "Web Servers",
        group: "全部",
        desc: "web服务，发布包内置",
        icon: "monitor",
        serviceId: "apache",
        installDir: toSlash(p.apacheRoot),
        executable: toSlash(path.join(p.apacheRoot, "bin", "httpd.exe")),
        downloadUrl: "",
        installNote: "Apache Windows 二进制通常来自 Apache Lounge。可在 data/config.json 中配置 downloadUrl 后一键安装。"
      },
      {
        id: "nginx",
        name: "Nginx1.26.3",
        category: "Web Servers",
        group: "全部",
        desc: "官方 Windows nginx 服务",
        icon: "monitor",
        serviceId: "nginx",
        installDir: toSlash(p.nginxRoot),
        executable: toSlash(path.join(p.nginxRoot, "nginx.exe")),
        downloadUrl: "https://nginx.org/download/nginx-1.26.3.zip",
        archiveRoot: true
      },
      {
        id: "mysql80",
        name: "MySQL8.0",
        category: "数据库",
        group: "系统环境",
        desc: "数据库服务",
        icon: "db",
        serviceId: "mysql80",
        installDir: toSlash(p.mysql80Root),
        executable: toSlash(path.join(p.mysql80Root, "bin", "mysqld.exe")),
        downloadUrl: "https://dev.mysql.com/get/Downloads/MySQL-8.0/mysql-8.0.12-winx64.zip",
        archiveRoot: true
      },
      {
        id: "mysql57",
        name: "MySQL5.7",
        category: "数据库",
        group: "系统环境",
        desc: "数据库服务",
        icon: "db",
        serviceId: "mysql57",
        installDir: toSlash(p.mysql57Root),
        executable: toSlash(path.join(p.mysql57Root, "bin", "mysqld.exe")),
        downloadUrl: "https://dev.mysql.com/get/Downloads/MySQL-5.7/mysql-5.7.26-winx64.zip",
        archiveRoot: true
      },
      {
        id: "redis",
        name: "Redis7.2.4",
        category: "redis",
        group: "系统环境",
        desc: "缓存、Session、队列与分布式锁服务",
        icon: "redis",
        serviceId: "redis",
        installDir: toSlash(p.redisRoot),
        executable: toSlash(path.join(p.redisRoot, "redis-server.exe")),
        downloadUrl: "https://www.nuget.org/api/v2/package/redis.windows.redist.x64/7.2.4",
        archiveRoot: false,
        postInstall: "redis-nuget"
      },
      {
        id: "mariadb",
        name: "MariaDB10.11",
        category: "数据库",
        group: "系统环境",
        desc: "MariaDB 数据库服务",
        icon: "db",
        serviceId: "mariadb",
        installDir: toSlash(p.mariadbRoot),
        executable: toSlash(path.join(p.mariadbRoot, "bin", "mysqld.exe")),
        downloadUrl: "https://archive.mariadb.org/mariadb-10.11.8/winx64-packages/mariadb-10.11.8-winx64.zip",
        archiveRoot: true
      },
      {
        id: "minio",
        name: "MinIO",
        category: "对象存储",
        group: "工具",
        desc: "兼容 S3 API 的本地对象存储服务",
        icon: "minio",
        serviceId: "minio",
        installDir: toSlash(p.minioRoot),
        executable: toSlash(path.join(p.minioRoot, "minio.exe")),
        downloadUrl: "https://dl.min.io/server/minio/release/windows-amd64/minio.exe",
        rawDownload: true,
        rawFileName: "minio.exe"
      },
      {
        id: "mc",
        name: "MinIO Client",
        category: "对象存储",
        group: "工具",
        desc: "MinIO 命令行客户端",
        icon: "minio",
        installDir: toSlash(p.minioRoot),
        executable: toSlash(path.join(p.minioRoot, "mc.exe")),
        downloadUrl: "https://dl.min.io/client/mc/release/windows-amd64/mc.exe",
        rawDownload: true,
        rawFileName: "mc.exe"
      },
      {
        id: "php73",
        name: "php7.3.33nts",
        category: "php",
        group: "系统环境",
        desc: "PHP NTS 运行环境",
        icon: "monitor",
        installDir: toSlash(p.phpRoot),
        executable: toSlash(path.join(p.phpRoot, "php-cgi.exe")),
        downloadUrl: "https://windows.php.net/downloads/releases/archives/php-7.3.33-nts-Win32-VC15-x64.zip",
        archiveRoot: false
      },
      {
        id: "ftp",
        name: "FileZilla Server",
        category: "文件服务",
        group: "工具",
        desc: "FTP 文件服务",
        icon: "monitor",
        serviceId: "ftp",
        installDir: toSlash(p.ftpRoot),
        executable: toSlash(path.join(p.ftpRoot, "FileZilla Server.exe")),
        downloadUrl: "",
        installNote: "可配置 FileZilla Server 离线包或镜像 URL 后一键安装。"
      }
    ],
    configFiles: [
      { id: "php.ini", label: "php.ini", path: toSlash(path.join(p.phpRoot, "php.ini")) },
      { id: "httpd.conf", label: "httpd.conf", path: toSlash(path.join(p.apacheRoot, "conf", "httpd.conf")) },
      { id: "nginx.conf", label: "nginx.conf", path: toSlash(path.join(p.nginxRoot, "conf", "nginx.conf")) },
      { id: "vhosts.conf", label: "vhosts.conf", path: toSlash(path.join(p.apacheRoot, "conf", "vhosts", "0localhost_80.conf")) },
      { id: "mysql.ini", label: "mysql.ini", path: toSlash(path.join(p.mysql80Root, "my.ini")) },
      { id: "mariadb.ini", label: "mariadb.ini", path: toSlash(path.join(p.mariadbRoot, "my.ini")) },
      { id: "redis.conf", label: "redis.conf", path: toSlash(path.join(p.redisRoot, "redis.conf")) },
      { id: "minio.env", label: "minio.env", path: toSlash(path.join(p.minioRoot, "minio.env")) },
      { id: "hosts", label: "hosts", path: toSlash(HOSTS_PATH) }
    ],
    logs: []
  };
}

function mergeConfig(base, saved) {
  if (!saved || typeof saved !== "object") return base;
  const merged = {
    ...base,
    ...saved,
    paths: { ...base.paths, ...(saved.paths || {}) },
    minio: { ...base.minio, ...(saved.minio || {}) },
    php: { ...base.php, ...(saved.php || {}) },
    systemSettings: { ...base.systemSettings, ...(saved.systemSettings || {}) },
    services: mergeArrayById(base.services, saved.services),
    sites: Array.isArray(saved.sites) ? saved.sites : base.sites,
    databases: Array.isArray(saved.databases) ? saved.databases : base.databases,
    ftpAccounts: Array.isArray(saved.ftpAccounts) ? saved.ftpAccounts : base.ftpAccounts,
    software: mergeArrayById(base.software, saved.software),
    configFiles: mergeArrayById(base.configFiles, saved.configFiles),
    logs: Array.isArray(saved.logs) ? saved.logs : []
  };
  if (PORT_OVERRIDE) merged.port = DEFAULT_PORT;
  return merged;
}

function loadConfig() {
  const base = defaultConfig();
  if (!exists(CONFIG_PATH)) {
    writeJson(CONFIG_PATH, base);
    return base;
  }
  const saved = JSON.parse(fs.readFileSync(CONFIG_PATH, "utf8"));
  return mergeConfig(base, saved);
}

let config = loadConfig();

function serviceById(id) {
  return config.services.find((service) => service.id === id);
}

function softwareById(id) {
  return config.software.find((software) => software.id === id);
}

function updateKnownService(id, values) {
  const service = serviceById(id);
  if (service) Object.assign(service, values);
}

function updateKnownSoftware(id, values) {
  const item = softwareById(id);
  if (item) Object.assign(item, values);
}

function updateKnownConfigFile(id, filePath) {
  const item = config.configFiles.find((file) => file.id === id);
  if (item) item.path = toSlash(filePath);
}

function refreshDerivedPaths() {
  const p = config.paths;
  const minioData = config.minio.dataDir || slashJoin(p.minioRoot, "data");
  const phpCgiPort = Number(config.php?.cgiPort || 9073);
  const minioApiPort = Number(config.minio.apiPort || 9000);
  const minioConsolePort = Number(config.minio.consolePort || 9001);

  updateKnownService("apache", {
    cwd: toSlash(p.apacheRoot),
    exe: slashJoin(p.apacheRoot, "bin", "httpd.exe"),
    args: ["-f", slashJoin(p.apacheRoot, "conf", "httpd.conf")],
    configFile: slashJoin(p.apacheRoot, "conf", "httpd.conf")
  });
  updateKnownService("ftp", {
    cwd: toSlash(p.ftpRoot),
    exe: slashJoin(p.ftpRoot, "FileZilla Server.exe"),
    args: [],
    configFile: slashJoin(p.ftpRoot, "FileZilla Server.xml")
  });
  updateKnownService("mysql57", {
    cwd: toSlash(p.mysql57Root),
    exe: slashJoin(p.mysql57Root, "bin", "mysqld.exe"),
    args: [`--defaults-file=${slashJoin(p.mysql57Root, "my.ini")}`],
    clientExe: slashJoin(p.mysql57Root, "bin", "mysql.exe"),
    configFile: slashJoin(p.mysql57Root, "my.ini")
  });
  updateKnownService("mysql80", {
    cwd: toSlash(p.mysql80Root),
    exe: slashJoin(p.mysql80Root, "bin", "mysqld.exe"),
    args: [`--defaults-file=${slashJoin(p.mysql80Root, "my.ini")}`],
    clientExe: slashJoin(p.mysql80Root, "bin", "mysql.exe"),
    configFile: slashJoin(p.mysql80Root, "my.ini")
  });
  updateKnownService("mariadb", {
    cwd: toSlash(p.mariadbRoot),
    exe: slashJoin(p.mariadbRoot, "bin", "mysqld.exe"),
    args: [`--defaults-file=${slashJoin(p.mariadbRoot, "my.ini")}`],
    clientExe: slashJoin(p.mariadbRoot, "bin", "mysql.exe"),
    configFile: slashJoin(p.mariadbRoot, "my.ini")
  });
  updateKnownService("nginx", {
    cwd: toSlash(p.nginxRoot),
    exe: slashJoin(p.nginxRoot, "nginx.exe"),
    args: ["-p", toSlash(p.nginxRoot), "-c", "conf/nginx.conf"],
    configFile: slashJoin(p.nginxRoot, "conf", "nginx.conf")
  });
  updateKnownService("redis", {
    cwd: toSlash(p.redisRoot),
    exe: slashJoin(p.redisRoot, "redis-server.exe"),
    args: ["redis.conf"],
    configFile: slashJoin(p.redisRoot, "redis.conf")
  });
  updateKnownService("php73", {
    cwd: toSlash(p.phpRoot),
    exe: slashJoin(p.phpRoot, "php-cgi.exe"),
    port: phpCgiPort,
    args: ["-b", `127.0.0.1:${phpCgiPort}`, "-c", slashJoin(p.phpRoot, "php.ini")],
    configFile: slashJoin(p.phpRoot, "php.ini")
  });
  updateKnownService("minio", {
    cwd: toSlash(p.minioRoot),
    exe: slashJoin(p.minioRoot, "minio.exe"),
    port: minioApiPort,
    args: ["server", toSlash(minioData), "--address", `:${minioApiPort}`, "--console-address", `:${minioConsolePort}`],
    env: {
      MINIO_ROOT_USER: config.minio.rootUser,
      MINIO_ROOT_PASSWORD: config.minio.rootPassword
    },
    configFile: slashJoin(p.minioRoot, "minio.env")
  });

  updateKnownSoftware("apache", {
    installDir: toSlash(p.apacheRoot),
    executable: slashJoin(p.apacheRoot, "bin", "httpd.exe")
  });
  updateKnownSoftware("nginx", {
    installDir: toSlash(p.nginxRoot),
    executable: slashJoin(p.nginxRoot, "nginx.exe")
  });
  updateKnownSoftware("mysql80", {
    installDir: toSlash(p.mysql80Root),
    executable: slashJoin(p.mysql80Root, "bin", "mysqld.exe")
  });
  updateKnownSoftware("mysql57", {
    installDir: toSlash(p.mysql57Root),
    executable: slashJoin(p.mysql57Root, "bin", "mysqld.exe")
  });
  updateKnownSoftware("mariadb", {
    installDir: toSlash(p.mariadbRoot),
    executable: slashJoin(p.mariadbRoot, "bin", "mysqld.exe")
  });
  updateKnownSoftware("redis", {
    installDir: toSlash(p.redisRoot),
    executable: slashJoin(p.redisRoot, "redis-server.exe")
  });
  updateKnownSoftware("minio", {
    installDir: toSlash(p.minioRoot),
    executable: slashJoin(p.minioRoot, "minio.exe")
  });
  updateKnownSoftware("mc", {
    installDir: toSlash(p.minioRoot),
    executable: slashJoin(p.minioRoot, "mc.exe")
  });
  updateKnownSoftware("php73", {
    installDir: toSlash(p.phpRoot),
    executable: slashJoin(p.phpRoot, "php-cgi.exe")
  });
  updateKnownSoftware("ftp", {
    installDir: toSlash(p.ftpRoot),
    executable: slashJoin(p.ftpRoot, "FileZilla Server.exe")
  });

  updateKnownConfigFile("php.ini", slashJoin(p.phpRoot, "php.ini"));
  updateKnownConfigFile("httpd.conf", slashJoin(p.apacheRoot, "conf", "httpd.conf"));
  updateKnownConfigFile("nginx.conf", slashJoin(p.nginxRoot, "conf", "nginx.conf"));
  updateKnownConfigFile("vhosts.conf", slashJoin(p.apacheRoot, "conf", "vhosts", "0localhost_80.conf"));
  updateKnownConfigFile("mysql.ini", slashJoin(p.mysql80Root, "my.ini"));
  updateKnownConfigFile("mariadb.ini", slashJoin(p.mariadbRoot, "my.ini"));
  updateKnownConfigFile("redis.conf", slashJoin(p.redisRoot, "redis.conf"));
  updateKnownConfigFile("minio.env", slashJoin(p.minioRoot, "minio.env"));
  updateKnownConfigFile("hosts", toSlash(HOSTS_PATH));
}

refreshDerivedPaths();

function saveConfig() {
  writeJson(CONFIG_PATH, config);
}

function addLog(message) {
  const date = new Date();
  const stamp = localTimestamp(date);
  config.logs.unshift(`${stamp} ${message}`);
  config.logs = config.logs.slice(0, 300);
  saveConfig();
}

function localTimestamp(date = new Date()) {
  const pad = (value) => String(value).padStart(2, "0");
  return [
    date.getFullYear(),
    "-",
    pad(date.getMonth() + 1),
    "-",
    pad(date.getDate()),
    " ",
    pad(date.getHours()),
    ":",
    pad(date.getMinutes()),
    ":",
    pad(date.getSeconds())
  ].join("");
}

function compactTimestamp(date = new Date()) {
  return localTimestamp(date).replace(/[-: ]/g, "");
}

function publicService(service, running) {
  return {
    id: service.id,
    name: service.name,
    type: running ? "triangle" : "square",
    running,
    auto: !!service.auto,
    port: service.port,
    configFile: service.configFile,
    installed: exists(service.exe)
  };
}

function normalizePathText(value) {
  return toSlash(value).toLowerCase();
}

function publicSoftware(item) {
  return {
    id: item.id,
    name: item.name,
    category: item.category,
    group: item.group,
    desc: item.desc,
    icon: item.icon,
    serviceId: item.serviceId || "",
    installDir: item.installDir,
    executable: item.executable,
    installed: exists(item.executable),
    installable: !!item.downloadUrl,
    installNote: item.installNote || ""
  };
}

function portOpen(port, host = "127.0.0.1", timeout = 500) {
  return new Promise((resolve) => {
    if (!port) return resolve(false);
    const socket = new net.Socket();
    let settled = false;
    const done = (value) => {
      if (settled) return;
      settled = true;
      socket.destroy();
      resolve(value);
    };
    socket.setTimeout(timeout);
    socket.once("connect", () => done(true));
    socket.once("timeout", () => done(false));
    socket.once("error", () => done(false));
    socket.connect(Number(port), host);
  });
}

async function taskRunning(processName) {
  if (!processName) return false;
  try {
    const { stdout } = await execFileAsync("tasklist.exe", ["/FI", `IMAGENAME eq ${processName}`, "/FO", "CSV", "/NH"]);
    return stdout.toLowerCase().includes(processName.toLowerCase());
  } catch {
    return false;
  }
}

function parseWmicProcessRows(stdout) {
  return String(stdout || "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.toLowerCase().startsWith("node,"))
    .map((line) => {
      const columns = line.split(",");
      const pid = Number(columns[columns.length - 1]);
      const executablePath = columns.slice(1, -1).join(",").trim();
      return { pid, executablePath };
    })
    .filter((row) => Number.isInteger(row.pid) && row.pid > 0);
}

function parsePowerShellProcessRows(stdout) {
  const text = String(stdout || "").trim();
  if (!text) return [];
  const parsed = JSON.parse(text);
  return (Array.isArray(parsed) ? parsed : [parsed])
    .filter(Boolean)
    .map((item) => ({ pid: Number(item.ProcessId), executablePath: String(item.ExecutablePath || "").trim() }))
    .filter((row) => Number.isInteger(row.pid) && row.pid > 0);
}

function processRowMatchesExecutable(row, executablePath) {
  const expected = normalizePathText(executablePath);
  return !!expected && normalizePathText(row.executablePath) === expected;
}

async function queryProcessRows(processName) {
  if (!processName) return [];
  const wqlName = String(processName).replace(/'/g, "''");
  try {
    const { stdout } = await execFileAsync("wmic.exe", [
      "process",
      "where",
      `name='${wqlName}'`,
      "get",
      "ExecutablePath,ProcessId",
      "/format:csv"
    ]);
    const rows = parseWmicProcessRows(stdout);
    if (rows.some((row) => row.executablePath)) return rows;
  } catch {
    // Fall back to PowerShell below. WMIC is optional on newer Windows builds.
  }

  try {
    const command = `$ErrorActionPreference='Stop'; Get-CimInstance Win32_Process -Filter "Name='${wqlName}'" | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress`;
    const { stdout } = await execFileAsync("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command]);
    return parsePowerShellProcessRows(stdout);
  } catch {
    return [];
  }
}

async function matchingProcessIds(service) {
  if (!service.processName || !service.exe) return [];
  const rows = await queryProcessRows(service.processName);
  return rows
    .filter((row) => processRowMatchesExecutable(row, service.exe))
    .map((row) => row.pid);
}

function taskkillProcessAbsent(error) {
  const text = String(error?.stdout || "") + String(error?.stderr || "") + String(error?.message || "");
  return /not found|没有找到|找不到|不存在/i.test(text);
}

async function processMatchesExecutable(service) {
  return (await matchingProcessIds(service)).length > 0;
}

async function serviceRunning(service) {
  if (SERVICE_DRY_RUN) return false;
  if (await processMatchesExecutable(service)) return true;
  if (exists(service.exe)) return false;
  if (await taskRunning(service.processName)) return true;
  return portOpen(service.port);
}

function assertInstalled(service) {
  if (!exists(service.exe)) {
    throw new Error(`${service.name} 未安装或 exe 路径不存在：${service.exe}`);
  }
}

async function startService(service) {
  if (SERVICE_DRY_RUN) {
    addLog(`${service.name} 已启动`);
    return `${service.name} 已启动`;
  }
  assertInstalled(service);
  if (await serviceRunning(service)) return `${service.name} 已经在运行`;
  ensureDir(service.cwd || path.dirname(service.exe));
  const child = spawn(service.exe, service.args || [], {
    cwd: service.cwd || path.dirname(service.exe),
    env: { ...process.env, ...(service.env || {}) },
    detached: true,
    stdio: "ignore",
    windowsHide: true
  });
  child.unref();
  await new Promise((resolve) => setTimeout(resolve, 900));
  addLog(`${service.name} 已启动`);
  return `${service.name} 已启动`;
}

async function stopService(service) {
  if (SERVICE_DRY_RUN) {
    addLog(`${service.name} 已停止`);
    return `${service.name} 已停止`;
  }
  if (service.id === "nginx" && exists(service.exe)) {
    try {
      await execFileAsync(service.exe, ["-p", service.cwd, "-s", "quit"], { cwd: service.cwd });
      await new Promise((resolve) => setTimeout(resolve, 700));
    } catch {
      // Fall back to PID-based termination below.
    }
  }

  const pids = await matchingProcessIds(service);
  for (const pid of pids) {
    try {
      await execFileAsync("taskkill.exe", ["/F", "/T", "/PID", String(pid)]);
    } catch (error) {
      if (!taskkillProcessAbsent(error)) throw error;
    }
  }
  addLog(`${service.name} 已停止`);
  return `${service.name} 已停止`;
}

async function restartService(service) {
  await stopService(service);
  await new Promise((resolve) => setTimeout(resolve, 500));
  return startService(service);
}

async function runSuite(action) {
  const targets = config.services.filter((service) => service.auto);
  const results = [];
  for (const service of targets) {
    try {
      if (action === "start") results.push(await startService(service));
      else if (action === "stop") results.push(await stopService(service));
      else throw new Error("套件操作不存在");
    } catch (error) {
      results.push(`${service.name}: ${error.message}`);
    }
  }
  const message = action === "start" ? "自动套件启动完成" : "自动套件停止完成";
  addLog(message);
  return { message, results };
}

function updateServiceAuto(id, enabled) {
  const service = config.services.find((item) => item.id === id);
  if (!service) throw new Error("服务不存在");
  service.auto = !!enabled;
  saveConfig();
  const message = `${service.name} 已${service.auto ? "加入" : "移出"}一键套件`;
  addLog(message);
  return { message, service: { id: service.id, auto: service.auto } };
}

function safeName(value) {
  return String(value || "")
    .trim()
    .replace(/[^a-zA-Z0-9._-]/g, "_")
    .slice(0, 120);
}

function backupFile(filePath) {
  if (!exists(filePath)) return "";
  const backup = `${filePath}.${Date.now()}.bak`;
  fs.copyFileSync(filePath, backup);
  return backup;
}

function apacheVhost(site) {
  const doc = toSlash(site.path);
  const phpRoot = toSlash(config.paths.phpRoot);
  return `<VirtualHost _default_:${site.port}>
    ServerName ${site.domain}
    DocumentRoot "${doc}"
    FcgidInitialEnv PHPRC "${phpRoot}"
    AddHandler fcgid-script .php
    FcgidWrapper "${phpRoot}/php-cgi.exe" .php
  <Directory "${doc}">
      Options FollowSymLinks ExecCGI
      AllowOverride All
      Order allow,deny
      Allow from all
      Require all granted
      DirectoryIndex index.php index.html
  </Directory>
</VirtualHost>
`;
}

function nginxVhost(site) {
  const doc = toSlash(site.path);
  const phpCgiPort = Number(config.php?.cgiPort || 9073);
  return `server {
        listen        ${site.port};
        server_name  ${site.domain};
        root   "${doc}";
        location / {
            index index.php index.html;
            include ${doc}/nginx.htaccess;
            autoindex off;
        }
        location ~ \\.php(.*)$ {
            fastcgi_pass   127.0.0.1:${phpCgiPort};
            fastcgi_index  index.php;
            fastcgi_split_path_info  ^((?U).+\\.php)(/?.+)$;
            fastcgi_param  SCRIPT_FILENAME  $document_root$fastcgi_script_name;
            fastcgi_param  PATH_INFO  $fastcgi_path_info;
            fastcgi_param  PATH_TRANSLATED  $document_root$fastcgi_path_info;
            include        fastcgi_params;
        }
}
`;
}

function ensureListenPort(port) {
  const apacheListen = path.join(config.paths.apacheRoot, "conf", "vhosts", "Listen.conf");
  if (exists(apacheListen)) {
    const current = fs.readFileSync(apacheListen, "utf8");
    if (!new RegExp(`^\\s*Listen\\s+${port}\\s*$`, "m").test(current)) {
      backupFile(apacheListen);
      fs.appendFileSync(apacheListen, `\nListen ${port}\n`, "utf8");
    }
  }
}

function siteConfigName(site) {
  return `${safeName(site.domain)}_${site.port}.conf`;
}

function siteConfigId(site) {
  return `site:${siteConfigName(site)}`;
}

function siteApacheConfigPath(site) {
  return path.join(config.paths.apacheRoot, "conf", "vhosts", siteConfigName(site));
}

function removeSiteConfig(site) {
  if (!site) return;
  const fileBase = siteConfigName(site);
  const apacheFile = siteApacheConfigPath(site);
  const nginxFile = path.join(config.paths.nginxRoot, "conf", "vhosts", fileBase);
  for (const filePath of [apacheFile, nginxFile]) {
    if (exists(filePath)) backupFile(filePath);
    if (exists(filePath)) fs.unlinkSync(filePath);
  }
}

function writeSiteConfig(site) {
  const fileBase = siteConfigName(site);
  const apacheDir = path.join(config.paths.apacheRoot, "conf", "vhosts");
  const nginxDir = path.join(config.paths.nginxRoot, "conf", "vhosts");
  if (exists(apacheDir)) fs.writeFileSync(path.join(apacheDir, fileBase), apacheVhost(site), "utf8");
  if (exists(nginxDir)) fs.writeFileSync(path.join(nginxDir, fileBase), nginxVhost(site), "utf8");
}

function hostsMarker(domain) {
  return `# XP.CN ${domain}`;
}

function readHostsLines() {
  if (!exists(HOSTS_PATH)) return [];
  return fs.readFileSync(HOSTS_PATH, "utf8").split(/\r?\n/);
}

function writeHostsLines(lines) {
  ensureDir(path.dirname(HOSTS_PATH));
  if (exists(HOSTS_PATH)) backupFile(HOSTS_PATH);
  fs.writeFileSync(HOSTS_PATH, `${lines.filter((line, index) => line !== "" || index < lines.length - 1).join(os.EOL)}${os.EOL}`, "utf8");
}

function syncHosts(domain) {
  const cleanDomain = String(domain || "").trim();
  if (!cleanDomain || cleanDomain === "localhost" || /^\d+\.\d+\.\d+\.\d+$/.test(cleanDomain)) return;
  const marker = hostsMarker(cleanDomain);
  const lines = readHostsLines().filter((line) => !line.includes(marker));
  lines.push(`127.0.0.1 ${cleanDomain} ${marker}`);
  writeHostsLines(lines);
}

function removeHosts(domain) {
  const cleanDomain = String(domain || "").trim();
  if (!cleanDomain) return;
  const marker = hostsMarker(cleanDomain);
  const lines = readHostsLines();
  const next = lines.filter((line) => !line.includes(marker));
  if (next.length !== lines.length) writeHostsLines(next);
}

function trySyncHosts(domain) {
  try {
    syncHosts(domain);
    return true;
  } catch (error) {
    addLog(`hosts 同步失败：${error.message}`);
    return false;
  }
}

function tryRemoveHosts(domain) {
  try {
    removeHosts(domain);
  } catch (error) {
    addLog(`hosts 清理失败：${error.message}`);
  }
}

function findSiteConflict(domain, port, exceptIndex = -1) {
  const normalizedDomain = String(domain || "").trim().toLowerCase();
  const normalizedPort = String(port || "").trim();
  return config.sites.findIndex((item, index) => {
    if (index === exceptIndex) return false;
    return String(item.domain || "").trim().toLowerCase() === normalizedDomain && String(item.port || "").trim() === normalizedPort;
  });
}

function assertUniqueSite(site, exceptIndex = -1) {
  if (findSiteConflict(site.domain, site.port, exceptIndex) >= 0) {
    throw new Error(`网站 ${site.domain}:${site.port} 已存在`);
  }
}

function assertValidPort(port) {
  if (!/^\d+$/.test(String(port || ""))) throw new Error("端口必须是数字");
  const value = Number(port);
  if (!Number.isInteger(value) || value < 1 || value > 65535) throw new Error("端口必须在 1-65535 之间");
}

function normalizeServicePort(port, label) {
  try {
    assertValidPort(port);
  } catch (error) {
    throw new Error(`${label}${error.message}`);
  }
  return Number(port);
}

function isValidIpv4(domain) {
  const parts = domain.split(".");
  return parts.length === 4 && parts.every((part) => {
    if (!/^\d{1,3}$/.test(part)) return false;
    const value = Number(part);
    return value >= 0 && value <= 255 && part === String(value);
  });
}

function isValidDnsName(domain) {
  if (domain.length > 253) return false;
  return domain.split(".").every((label) => {
    if (!/^[a-zA-Z0-9-]{1,63}$/.test(label)) return false;
    return !label.startsWith("-") && !label.endsWith("-");
  });
}

function assertValidDomain(domain) {
  const value = String(domain || "").trim();
  if (!value) throw new Error("域名不能为空");
  if (value !== domain || /[\s\\/:#?%]/.test(value)) throw new Error("域名格式不正确");
  if (value.toLowerCase() === "localhost" || isValidIpv4(value) || isValidDnsName(value)) return;
  throw new Error("域名格式不正确");
}

function createSite(data) {
  const site = {
    domain: String(data.domain || "").trim(),
    port: String(data.port || "80").trim(),
    path: toSlash(data.path || path.join(config.paths.wwwRoot, safeName(data.domain || "site"))),
    status: "正常",
    expire: data.expire || "2035-12-03"
  };
  assertValidDomain(site.domain);
  assertValidPort(site.port);
  assertUniqueSite(site);

  ensureDir(site.path);
  ensureListenPort(site.port);

  writeSiteConfig(site);
  trySyncHosts(site.domain);

  config.sites.push(site);
  addLog(`网站 ${site.domain}:${site.port} 已创建`);
  saveConfig();
  return site;
}

function updateSite(indexValue, data) {
  const index = Number(indexValue);
  if (!Number.isInteger(index) || index < 0 || index >= config.sites.length) {
    throw new Error("网站记录不存在");
  }
  const site = {
    ...config.sites[index],
    domain: String(data.domain || "").trim(),
    port: String(data.port || "80").trim(),
    path: toSlash(data.path || config.sites[index].path),
    expire: data.expire || config.sites[index].expire || "2035-12-03",
    status: data.status || config.sites[index].status || "正常"
  };
  assertValidDomain(site.domain);
  assertValidPort(site.port);
  assertUniqueSite(site, index);
  ensureDir(site.path);
  ensureListenPort(site.port);
  removeSiteConfig(config.sites[index]);
  if (config.sites[index].domain !== site.domain) tryRemoveHosts(config.sites[index].domain);
  writeSiteConfig(site);
  trySyncHosts(site.domain);
  config.sites[index] = site;
  addLog(`网站 ${site.domain}:${site.port} 已更新`);
  saveConfig();
  return site;
}

function siteConfigFile(indexValue) {
  const index = Number(indexValue);
  if (!Number.isInteger(index) || index < 0 || index >= config.sites.length) {
    throw new Error("网站记录不存在");
  }
  const site = config.sites[index];
  const filePath = siteApacheConfigPath(site);
  return {
    id: siteConfigId(site),
    label: siteConfigName(site),
    path: toSlash(filePath),
    exists: exists(filePath),
    content: exists(filePath) ? fs.readFileSync(filePath, "utf8") : ""
  };
}

function mysqlArgs(service, sql, passwordOverride) {
  const args = ["-uroot"];
  const password = passwordOverride ?? config.mysqlRootPassword;
  if (password) args.push(`-p${password}`);
  args.push("-P", String(service.port || 3306), "-h", "127.0.0.1", "-e", sql);
  return args;
}

function mysqlImportArgs(service, db, passwordOverride) {
  const args = ["-uroot"];
  const password = passwordOverride ?? config.mysqlRootPassword;
  if (password) args.push(`-p${password}`);
  args.push("-P", String(service.port || 3306), "-h", "127.0.0.1", db);
  return args;
}

function mysqlService() {
  const preferred = ["mysql80", "mariadb", "mysql57"]
    .map((id) => config.services.find((item) => item.id === id))
    .find((item) => item?.clientExe && exists(item.clientExe));
  return preferred || config.services.find((item) => item.clientExe);
}

function sqlIdent(value) {
  return `\`${String(value).replace(/`/g, "``")}\``;
}

function sqlString(value) {
  return `'${String(value).replace(/\\/g, "\\\\").replace(/'/g, "\\'")}'`;
}

function findDatabaseConflict(db) {
  const normalizedDb = String(db || "").trim().toLowerCase();
  return config.databases.findIndex((item) => String(item.db || "").trim().toLowerCase() === normalizedDb);
}

function assertUniqueDatabase(db) {
  if (findDatabaseConflict(db) >= 0) {
    throw new Error(`数据库 ${db} 已存在`);
  }
}

function databaseRecord(indexValue) {
  const index = Number(indexValue);
  if (!Number.isInteger(index) || index < 0 || index >= config.databases.length) {
    throw new Error("数据库记录不存在");
  }
  return config.databases[index];
}

function resolveSqlFile(filePath) {
  const raw = String(filePath || "").trim();
  if (!raw) throw new Error("SQL 文件路径不能为空");
  const resolved = path.resolve(raw);
  if (!exists(resolved)) throw new Error(`SQL 文件不存在：${toSlash(resolved)}`);
  if (!fs.statSync(resolved).isFile()) throw new Error("SQL 文件路径不是文件");
  if (path.extname(resolved).toLowerCase() !== ".sql") throw new Error("只能导入 .sql 文件");
  return resolved;
}

async function createDatabase(data) {
  const db = String(data.db || "").trim();
  const user = String(data.user || db).trim();
  const pass = String(data.pass || "").trim();
  if (!db || !user || !pass) throw new Error("数据库名、用户和密码都不能为空");
  assertUniqueDatabase(db);
  const service = mysqlService();
  if (!service || !exists(service.clientExe)) throw new Error("未找到 mysql.exe，无法创建数据库");
  const sql = [
    `CREATE DATABASE IF NOT EXISTS ${sqlIdent(db)} DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci`,
    `CREATE USER IF NOT EXISTS ${sqlString(user)}@'localhost' IDENTIFIED BY ${sqlString(pass)}`,
    `GRANT ALL PRIVILEGES ON ${sqlIdent(db)}.* TO ${sqlString(user)}@'localhost'`,
    "FLUSH PRIVILEGES"
  ].join("; ");
  await execFileAsync(service.clientExe, mysqlArgs(service, sql));
  const record = { db, user, pass: "******", status: "正常" };
  config.databases.push(record);
  addLog(`数据库 ${db} 已创建`);
  saveConfig();
  return record;
}

async function changeRootPassword(data) {
  const service = mysqlService();
  if (!service || !exists(service.clientExe)) throw new Error("未找到 mysql.exe，无法修改 root 密码");
  const newPass = String(data.pass || "").trim();
  if (!newPass) throw new Error("新密码不能为空");
  const sql = `ALTER USER 'root'@'localhost' IDENTIFIED BY ${sqlString(newPass)}; FLUSH PRIVILEGES`;
  await execFileAsync(service.clientExe, mysqlArgs(service, sql));
  config.mysqlRootPassword = newPass;
  addLog("root 密码已修改");
  saveConfig();
  return { ok: true };
}

async function exportDatabase(indexValue) {
  const record = databaseRecord(indexValue);
  const db = String(record.db || "").trim();
  if (!db) throw new Error("数据库名不能为空");
  const backupsDir = databaseBackupsDir();
  ensureDir(backupsDir);
  const stamp = compactTimestamp();
  const filePath = path.join(backupsDir, `${safeName(db)}-${stamp}.sql`);

  if (SERVICE_DRY_RUN) {
    fs.writeFileSync(filePath, `-- XP.CN dry-run backup\n-- database: ${db}\n`, "utf8");
  } else {
    const service = mysqlService();
    const dumpExe = service?.dumpExe || (service?.clientExe ? path.join(path.dirname(service.clientExe), "mysqldump.exe") : "");
    if (!service || !exists(dumpExe)) throw new Error("未找到 mysqldump.exe，无法导出数据库");
    const args = ["-uroot"];
    if (config.mysqlRootPassword) args.push(`-p${config.mysqlRootPassword}`);
    args.push("-P", String(service.port || 3306), "-h", "127.0.0.1", db);
    const { stdout } = await execFileAsync(dumpExe, args);
    fs.writeFileSync(filePath, stdout, "utf8");
  }

  const message = `数据库 ${db} 已导出：${toSlash(filePath)}`;
  addLog(message);
  return { message, path: toSlash(filePath) };
}

function databaseBackupsDir() {
  return path.join(DATA_DIR, "backups");
}

function resolveDatabaseBackupPath(data) {
  const raw = String(data.path || data.name || "").trim();
  if (!raw) throw new Error("备份文件不能为空");
  const backupsDir = databaseBackupsDir();
  const resolved = path.resolve(path.isAbsolute(raw) ? raw : path.join(backupsDir, raw));
  const relative = path.relative(backupsDir, resolved);
  if (!relative || relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error("只能操作备份目录中的 SQL 文件");
  }
  if (path.extname(resolved).toLowerCase() !== ".sql") throw new Error("只能操作 .sql 备份文件");
  if (!exists(resolved)) throw new Error(`备份文件不存在：${toSlash(resolved)}`);
  if (!fs.statSync(resolved).isFile()) throw new Error("备份路径不是文件");
  return resolved;
}

async function importDatabase(indexValue, data) {
  const record = databaseRecord(indexValue);
  const db = String(record.db || "").trim();
  if (!db) throw new Error("数据库名不能为空");
  const filePath = resolveSqlFile(data.path || data.file || data.backupPath);

  if (!SERVICE_DRY_RUN) {
    const service = mysqlService();
    if (!service || !exists(service.clientExe)) throw new Error("未找到 mysql.exe，无法导入数据库");
    await execFileWithInputFile(service.clientExe, mysqlImportArgs(service, db), filePath);
  }

  const message = `数据库 ${db} 已导入：${toSlash(filePath)}`;
  addLog(message);
  return { message, path: toSlash(filePath) };
}

function deleteDatabaseBackup(data) {
  const filePath = resolveDatabaseBackupPath(data);
  fs.unlinkSync(filePath);
  const message = `数据库备份已删除：${toSlash(filePath)}`;
  addLog(message);
  return { message, path: toSlash(filePath) };
}

function listDatabaseBackups() {
  const backupsDir = databaseBackupsDir();
  if (!exists(backupsDir)) return [];
  return fs.readdirSync(backupsDir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith(".sql"))
    .map((entry) => {
      const filePath = path.join(backupsDir, entry.name);
      const stat = fs.statSync(filePath);
      return {
        name: entry.name,
        path: toSlash(filePath),
        size: stat.size,
        modifiedAt: stat.mtime.toISOString()
      };
    })
    .sort((a, b) => b.modifiedAt.localeCompare(a.modifiedAt))
    .slice(0, 50);
}

function findFtpConflict(user, exceptIndex = -1) {
  const normalizedUser = String(user || "").trim().toLowerCase();
  return config.ftpAccounts.findIndex((item, index) => {
    if (index === exceptIndex) return false;
    return String(item.user || "").trim().toLowerCase() === normalizedUser;
  });
}

function assertUniqueFtpAccount(account, exceptIndex = -1) {
  if (findFtpConflict(account.user, exceptIndex) >= 0) {
    throw new Error(`FTP账号 ${account.user} 已存在`);
  }
}

function assertValidFtpUser(user) {
  if (!user) throw new Error("FTP 用户名不能为空");
  if (!/^[a-zA-Z0-9._-]{1,64}$/.test(user)) throw new Error("FTP 用户名只能包含字母、数字、点、下划线和短横线");
}

function escapeXml(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

function ftpConfigPath() {
  const service = serviceById("ftp");
  return service?.configFile || slashJoin(config.paths.ftpRoot, "FileZilla Server.xml");
}

function ftpPermissionValue(permission) {
  const text = String(permission || "读写");
  const canWrite = text.includes("写");
  const canRead = !text.includes("只写");
  return {
    read: canRead ? "1" : "0",
    write: canWrite ? "1" : "0",
    delete: canWrite ? "1" : "0",
    append: canWrite ? "1" : "0",
    create: canWrite ? "1" : "0",
    list: "1",
    subdirs: "1"
  };
}

function ftpUserXml(account) {
  const permission = ftpPermissionValue(account.permission);
  const home = toSlash(account.path);
  return [
    `  <!-- XP.CN managed account: ${escapeXml(account.user)} -->`,
    `  <User Name="${escapeXml(account.user)}">`,
    "    <Option Name=\"Pass\"></Option>",
    "    <Option Name=\"Group\"></Option>",
    "    <Option Name=\"Bypass server userlimit\">0</Option>",
    "    <Option Name=\"User Limit\">0</Option>",
    "    <Option Name=\"IP Limit\">0</Option>",
    "    <Option Name=\"Enabled\">1</Option>",
    "    <Option Name=\"Comments\">Managed by XP.CN local manager</Option>",
    "    <Option Name=\"ForceSsl\">0</Option>",
    "    <IpFilter>",
    "      <Disallowed />",
    "      <Allowed />",
    "    </IpFilter>",
    "    <Permissions>",
    `      <Permission Dir="${escapeXml(home)}">`,
    `        <Option Name="FileRead">${permission.read}</Option>`,
    `        <Option Name="FileWrite">${permission.write}</Option>`,
    `        <Option Name="FileDelete">${permission.delete}</Option>`,
    `        <Option Name="FileAppend">${permission.append}</Option>`,
    `        <Option Name="DirCreate">${permission.create}</Option>`,
    `        <Option Name="DirDelete">${permission.delete}</Option>`,
    `        <Option Name="DirList">${permission.list}</Option>`,
    `        <Option Name="DirSubdirs">${permission.subdirs}</Option>`,
    "        <Option Name=\"IsHome\">1</Option>",
    "        <Option Name=\"AutoCreate\">1</Option>",
    "      </Permission>",
    "    </Permissions>",
    "  </User>"
  ].join(os.EOL);
}

function defaultFtpConfigXml() {
  return [
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>",
    "<FileZillaServer>",
    "  <Settings />",
    "  <Groups />",
    "  <Users>",
    "  </Users>",
    "</FileZillaServer>",
    ""
  ].join(os.EOL);
}

function removeManagedFtpUser(xml, user) {
  const escaped = String(user || "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`\\s*<!-- XP\\.CN managed account: ${escaped} -->\\s*<User\\b[^>]*>[\\s\\S]*?<\\/User>`, "g");
  return xml.replace(pattern, "");
}

function upsertFtpUserXml(xml, account) {
  let next = removeManagedFtpUser(xml, account.user);
  const userXml = ftpUserXml(account);
  if (/<Users\b[^>]*>[\s\S]*?<\/Users>/.test(next)) {
    return next.replace(/<\/Users>/, `${userXml}${os.EOL}  </Users>`);
  }
  if (/<FileZillaServer\b[^>]*>/.test(next)) {
    return next.replace(/<\/FileZillaServer>/, `  <Users>${os.EOL}${userXml}${os.EOL}  </Users>${os.EOL}</FileZillaServer>`);
  }
  return defaultFtpConfigXml().replace(/<\/Users>/, `${userXml}${os.EOL}  </Users>`);
}

function writeFtpConfigXml(xml) {
  const filePath = ftpConfigPath();
  ensureDir(path.dirname(filePath));
  if (exists(filePath)) backupFile(filePath);
  fs.writeFileSync(filePath, xml, "utf8");
}

function syncFtpConfig(account, previousUser = "") {
  const filePath = ftpConfigPath();
  const current = exists(filePath) ? fs.readFileSync(filePath, "utf8") : defaultFtpConfigXml();
  const withoutPrevious = previousUser && previousUser !== account.user ? removeManagedFtpUser(current, previousUser) : current;
  writeFtpConfigXml(upsertFtpUserXml(withoutPrevious, account));
}

function removeFtpConfigUser(account) {
  if (!account) return;
  const filePath = ftpConfigPath();
  if (!exists(filePath)) return;
  writeFtpConfigXml(removeManagedFtpUser(fs.readFileSync(filePath, "utf8"), account.user));
}

function createFtpAccount(data) {
  const account = {
    user: String(data.user || "").trim(),
    path: toSlash(data.path || config.paths.wwwRoot),
    permission: String(data.permission || "读写"),
    status: "正常"
  };
  assertValidFtpUser(account.user);
  assertUniqueFtpAccount(account);
  ensureDir(account.path);
  syncFtpConfig(account);
  config.ftpAccounts.push(account);
  addLog(`FTP账号 ${account.user} 已创建`);
  saveConfig();
  return account;
}

function updateFtpAccount(indexValue, data) {
  const index = Number(indexValue);
  if (!Number.isInteger(index) || index < 0 || index >= config.ftpAccounts.length) {
    throw new Error("FTP 记录不存在");
  }
  const account = {
    ...config.ftpAccounts[index],
    user: String(data.user || "").trim(),
    path: toSlash(data.path || config.ftpAccounts[index].path),
    permission: String(data.permission || config.ftpAccounts[index].permission || "读写"),
    status: data.status || config.ftpAccounts[index].status || "正常"
  };
  assertValidFtpUser(account.user);
  assertUniqueFtpAccount(account, index);
  ensureDir(account.path);
  syncFtpConfig(account, config.ftpAccounts[index].user);
  config.ftpAccounts[index] = account;
  addLog(`FTP账号 ${account.user} 已更新`);
  saveConfig();
  return account;
}

function setAutostart(enabled) {
  if (!STARTUP_COMMAND) throw new Error("未找到 Windows Startup 目录，无法设置开机自启");
  ensureDir(STARTUP_DIR);
  if (enabled) {
    const lines = [
      "@echo off",
      `cd /d "${ROOT}"`,
      `set "XPCN_PORT=${config.port || DEFAULT_PORT}"`,
      `set "XPCN_DATA_DIR=${DATA_DIR}"`,
      `start "" /min "${process.execPath}" "${path.join(ROOT, "server.js")}"`
    ];
    fs.writeFileSync(STARTUP_COMMAND, `${lines.join("\r\n")}\r\n`, "utf8");
    return;
  }
  if (exists(STARTUP_COMMAND)) fs.unlinkSync(STARTUP_COMMAND);
}

function publicSystemSettings() {
  return {
    ...config.systemSettings,
    port: config.port || DEFAULT_PORT,
    dataDir: DATA_DIR,
    configPath: CONFIG_PATH,
    autostartPath: STARTUP_COMMAND,
    autostartInstalled: STARTUP_COMMAND ? exists(STARTUP_COMMAND) : false,
    paths: config.paths,
    php: config.php,
    minio: {
      apiPort: config.minio.apiPort,
      consolePort: config.minio.consolePort,
      dataDir: config.minio.dataDir
    },
    editablePathLabels: EDITABLE_PATH_LABELS
  };
}

function updateSystemSettings(data) {
  if (!data || typeof data !== "object") throw new Error("系统设置格式不正确");
  const next = { ...config.systemSettings };

  if (Object.prototype.hasOwnProperty.call(data, "autostart")) {
    next.autostart = !!data.autostart;
    setAutostart(next.autostart);
  }
  if (Object.prototype.hasOwnProperty.call(data, "startSuiteOnLaunch")) {
    next.startSuiteOnLaunch = !!data.startSuiteOnLaunch;
  }
  if (Object.prototype.hasOwnProperty.call(data, "phpMyAdminUrl")) {
    const url = String(data.phpMyAdminUrl || "").trim();
    if (!/^https?:\/\//i.test(url)) throw new Error("phpMyAdmin 地址必须以 http:// 或 https:// 开头");
    next.phpMyAdminUrl = url;
  }

  const nextPhp = { ...config.php };
  const nextMinio = { ...config.minio };
  if (Object.prototype.hasOwnProperty.call(data, "phpCgiPort")) {
    nextPhp.cgiPort = normalizeServicePort(data.phpCgiPort, "PHP-CGI ");
  }
  if (Object.prototype.hasOwnProperty.call(data, "minioApiPort")) {
    nextMinio.apiPort = normalizeServicePort(data.minioApiPort, "MinIO API ");
  }
  if (Object.prototype.hasOwnProperty.call(data, "minioConsolePort")) {
    nextMinio.consolePort = normalizeServicePort(data.minioConsolePort, "MinIO 控制台 ");
  }
  if (nextPhp.cgiPort === nextMinio.apiPort || nextPhp.cgiPort === nextMinio.consolePort || nextMinio.apiPort === nextMinio.consolePort) {
    throw new Error("PHP-CGI、MinIO API 和 MinIO 控制台端口不能重复");
  }

  config.systemSettings = next;
  config.php = nextPhp;
  config.minio = nextMinio;
  refreshDerivedPaths();
  saveConfig();
  addLog("系统设置已保存");
  return publicSystemSettings();
}

function normalizeEditablePath(value, label) {
  const clean = String(value || "").trim();
  if (!clean) throw new Error(`${label} 不能为空`);
  if (!path.isAbsolute(clean)) throw new Error(`${label} 必须是绝对路径`);
  return toSlash(path.normalize(clean));
}

function updatePaths(data) {
  if (!data || typeof data !== "object") throw new Error("路径设置格式不正确");
  const next = { ...config.paths };
  const previous = { ...config.paths };
  const keys = Object.keys(EDITABLE_PATH_LABELS);
  const hasEditableKey = keys.some((key) => Object.prototype.hasOwnProperty.call(data, key));
  if (!hasEditableKey) throw new Error("没有可保存的路径");

  if (Object.prototype.hasOwnProperty.call(data, "phpStudyRoot")) {
    next.phpStudyRoot = normalizeEditablePath(data.phpStudyRoot, EDITABLE_PATH_LABELS.phpStudyRoot);
  }

  const phpStudyRootChanged = normalizePathText(previous.phpStudyRoot) !== normalizePathText(next.phpStudyRoot);
  const previousChildDefaults = phpStudyChildPaths(previous.phpStudyRoot);
  const nextChildDefaults = phpStudyChildPaths(next.phpStudyRoot);
  for (const key of keys) {
    if (key === "phpStudyRoot") continue;
    if (Object.prototype.hasOwnProperty.call(data, key)) {
      const clean = normalizeEditablePath(data[key], EDITABLE_PATH_LABELS[key]);
      const shouldFollowNewRoot = phpStudyRootChanged
        && Object.prototype.hasOwnProperty.call(PHPSTUDY_CHILD_PATHS, key)
        && normalizePathText(clean) === normalizePathText(previousChildDefaults[key]);
      next[key] = shouldFollowNewRoot ? nextChildDefaults[key] : clean;
    } else if (phpStudyRootChanged && Object.prototype.hasOwnProperty.call(PHPSTUDY_CHILD_PATHS, key)) {
      next[key] = nextChildDefaults[key];
    }
  }

  const minioRootChanged = normalizePathText(previous.minioRoot) !== normalizePathText(next.minioRoot);
  const currentMinioData = normalizePathText(config.minio.dataDir);
  const oldMinioRoot = normalizePathText(previous.minioRoot);
  const oldMinioData = normalizePathText(slashJoin(previous.minioRoot, "data"));
  if (minioRootChanged && (!currentMinioData || currentMinioData === oldMinioRoot || currentMinioData === oldMinioData)) {
    config.minio.dataDir = slashJoin(next.minioRoot, "data");
  }

  config.paths = next;
  refreshDerivedPaths();
  addLog("本机路径已保存");
  saveConfig();
  return publicSystemSettings();
}

function removeRecord(kind, indexValue) {
  const collections = {
    sites: {
      label: "网站",
      items: config.sites,
      describe: (item) => `${item.domain}:${item.port}`,
      afterRemove: (item) => {
        removeSiteConfig(item);
        tryRemoveHosts(item.domain);
      }
    },
    databases: {
      label: "数据库",
      items: config.databases,
      describe: (item) => item.db,
      guard: (item) => item.db === "root" ? "root 数据库记录不能移除" : ""
    },
    ftp: {
      label: "FTP账号",
      items: config.ftpAccounts,
      describe: (item) => item.user,
      afterRemove: (item) => {
        removeFtpConfigUser(item);
      }
    }
  };
  const collection = collections[kind];
  if (!collection) throw new Error("记录类型不存在");
  const index = Number(indexValue);
  if (!Number.isInteger(index) || index < 0 || index >= collection.items.length) {
    throw new Error("记录不存在");
  }
  const record = collection.items[index];
  const guarded = collection.guard ? collection.guard(record) : "";
  if (guarded) throw new Error(guarded);
  const [removed] = collection.items.splice(index, 1);
  if (collection.afterRemove) collection.afterRemove(removed);
  const message = `${collection.label} ${collection.describe(removed)} 已从管理台移除`;
  addLog(message);
  saveConfig();
  return { message, removed };
}

function getConfigFile(id) {
  if (String(id || "").startsWith("site:")) {
    const site = config.sites.find((item) => siteConfigId(item) === id);
    if (!site) throw new Error("网站配置文件不存在");
    return siteConfigFile(config.sites.indexOf(site));
  }
  const item = config.configFiles.find((file) => file.id === id);
  if (!item) throw new Error("配置文件不存在");
  return {
    ...item,
    exists: exists(item.path),
    content: exists(item.path) ? fs.readFileSync(item.path, "utf8") : ""
  };
}

function saveConfigFile(id, content) {
  if (String(id || "").startsWith("site:")) {
    const site = config.sites.find((item) => siteConfigId(item) === id);
    if (!site) throw new Error("网站配置文件不存在");
    const filePath = siteApacheConfigPath(site);
    ensureDir(path.dirname(filePath));
    backupFile(filePath);
    fs.writeFileSync(filePath, String(content || ""), "utf8");
    addLog(`${siteConfigName(site)} 已保存`);
    return siteConfigFile(config.sites.indexOf(site));
  }
  const item = config.configFiles.find((file) => file.id === id);
  if (!item) throw new Error("配置文件不存在");
  ensureDir(path.dirname(item.path));
  backupFile(item.path);
  fs.writeFileSync(item.path, String(content || ""), "utf8");
  addLog(`${item.label} 已保存`);
  return getConfigFile(id);
}

function openFolder(folderPath) {
  if (!folderPath) throw new Error("目录路径不能为空");
  if (!exists(folderPath) || !fs.statSync(folderPath).isDirectory()) throw new Error(`目录不存在：${folderPath}`);
  if (SERVICE_DRY_RUN) return `已验证目录：${folderPath}`;
  spawn("explorer.exe", [folderPath], { detached: true, stdio: "ignore", windowsHide: true }).unref();
  return `已打开目录：${folderPath}`;
}

function openFile(filePath) {
  if (!filePath) throw new Error("文件路径不能为空");
  if (!exists(filePath) || !fs.statSync(filePath).isFile()) throw new Error(`文件不存在：${filePath}`);
  if (SERVICE_DRY_RUN) return `已验证文件：${filePath}`;
  spawn("notepad.exe", [filePath], { detached: true, stdio: "ignore", windowsHide: true }).unref();
  return `已打开文件：${filePath}`;
}

function openUrl(url) {
  if (!/^https?:\/\//i.test(String(url || ""))) throw new Error("URL 不合法");
  spawn("cmd.exe", ["/c", "start", "", url], { detached: true, stdio: "ignore", windowsHide: true }).unref();
  return `已打开：${url}`;
}

function download(url, destination) {
  return new Promise((resolve, reject) => {
    ensureDir(path.dirname(destination));
    const client = url.startsWith("https:") ? https : http;
    const request = client.get(url, (response) => {
      if ([301, 302, 303, 307, 308].includes(response.statusCode) && response.headers.location) {
        response.resume();
        download(new URL(response.headers.location, url).toString(), destination).then(resolve, reject);
        return;
      }
      if (response.statusCode < 200 || response.statusCode >= 300) {
        reject(new Error(`下载失败：HTTP ${response.statusCode}`));
        response.resume();
        return;
      }
      const file = fs.createWriteStream(destination);
      response.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", reject);
    });
    request.on("error", reject);
  });
}

async function expandArchive(archivePath, destination) {
  ensureDir(destination);
  const command = `Expand-Archive -LiteralPath '${archivePath.replace(/'/g, "''")}' -DestinationPath '${destination.replace(/'/g, "''")}' -Force`;
  await execFileAsync("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command]);
}

function copyExtracted(source, destination, stripRoot) {
  let actualSource = source;
  const entries = fs.readdirSync(source, { withFileTypes: true }).filter((entry) => entry.name !== "__MACOSX");
  if (stripRoot && entries.length === 1 && entries[0].isDirectory()) {
    actualSource = path.join(source, entries[0].name);
  }
  ensureDir(destination);
  fs.cpSync(actualSource, destination, { recursive: true, force: true });
}

function normalizeRedisNuget(installDir) {
  const candidates = [
    path.join(installDir, "tools"),
    path.join(installDir, "content"),
    installDir
  ];
  for (const candidate of candidates) {
    if (exists(path.join(candidate, "redis-server.exe"))) {
      if (candidate !== installDir) {
        fs.cpSync(candidate, installDir, { recursive: true, force: true });
      }
      if (!exists(path.join(installDir, "redis.conf"))) {
        fs.writeFileSync(path.join(installDir, "redis.conf"), "port 6379\nbind 127.0.0.1\nappendonly yes\n", "utf8");
      }
      return;
    }
  }
}

function applyServiceInstallPath(software) {
  if (!software.serviceId) return;
  const service = config.services.find((item) => item.id === software.serviceId);
  if (!service) return;
  service.exe = software.executable;
  service.cwd = software.installDir;
  if (service.id === "nginx") {
    service.args = ["-p", software.installDir, "-c", "conf/nginx.conf"];
    service.configFile = toSlash(path.join(software.installDir, "conf", "nginx.conf"));
  }
  if (service.id === "redis") {
    service.args = ["redis.conf"];
    service.configFile = toSlash(path.join(software.installDir, "redis.conf"));
  }
  if (service.id === "minio") {
    service.port = Number(config.minio.apiPort || 9000);
    service.args = ["server", toSlash(config.minio.dataDir), "--address", `:${config.minio.apiPort || 9000}`, "--console-address", `:${config.minio.consolePort || 9001}`];
    service.configFile = toSlash(path.join(software.installDir, "minio.env"));
  }
}

async function installSoftware(id) {
  const item = config.software.find((software) => software.id === id);
  if (!item) throw new Error("软件不存在");
  if (exists(item.executable)) return `${item.name} 已安装`;
  if (!item.downloadUrl) {
    throw new Error(item.installNote || `${item.name} 未配置下载地址，请在 data/config.json 中补充 downloadUrl`);
  }

  ensureDir(item.installDir);
  addLog(`${item.name} 开始下载`);
  if (item.rawDownload) {
    const target = path.join(item.installDir, item.rawFileName || path.basename(item.executable));
    await download(item.downloadUrl, target);
  } else {
    const tempDir = path.join(os.tmpdir(), `xpcn-${id}-${Date.now()}`);
    const archive = path.join(tempDir, `${id}.zip`);
    const extracted = path.join(tempDir, "extracted");
    ensureDir(tempDir);
    await download(item.downloadUrl, archive);
    await expandArchive(archive, extracted);
    copyExtracted(extracted, item.installDir, !!item.archiveRoot);
    if (item.postInstall === "redis-nuget") normalizeRedisNuget(item.installDir);
  }

  applyServiceInstallPath(item);
  saveConfig();
  addLog(`${item.name} 已安装`);
  return `${item.name} 已安装`;
}

async function uninstallSoftware(id) {
  const item = config.software.find((software) => software.id === id);
  if (!item) throw new Error("软件不存在");
  throw new Error(`${item.name} 暂未启用自动卸载。为避免误删外部目录，请先停止服务后手动确认删除路径：${item.installDir}`);
}

async function state() {
  const services = [];
  for (const service of config.services) {
    services.push(publicService(service, await serviceRunning(service)));
  }
  return {
    services,
    websites: config.sites,
    databases: config.databases,
    databaseBackups: listDatabaseBackups(),
    ftpAccounts: config.ftpAccounts,
    software: config.software.map(publicSoftware),
    configFiles: config.configFiles.map((item) => ({ id: item.id, label: item.label, path: item.path, exists: exists(item.path) })),
    systemSettings: publicSystemSettings(),
    php: config.php,
    logs: config.logs,
    version: "8.1.1.3-local",
    configPath: CONFIG_PATH
  };
}

function send(res, statusCode, data, headers = {}) {
  const body = typeof data === "string" ? data : JSON.stringify(data);
  res.writeHead(statusCode, {
    "Content-Type": typeof data === "string" ? "text/plain; charset=utf-8" : "application/json; charset=utf-8",
    "Cache-Control": "no-store",
    ...headers
  });
  res.end(body);
}

function isInsideRoot(filePath) {
  const relative = path.relative(ROOT, filePath);
  return relative === "" || (!!relative && !relative.startsWith("..") && !path.isAbsolute(relative));
}

function serveStatic(req, res) {
  const requestUrl = new URL(req.url, `http://127.0.0.1:${config.port || DEFAULT_PORT}`);
  let pathname = "";
  try {
    pathname = decodeURIComponent(requestUrl.pathname === "/" ? "/index.html" : requestUrl.pathname);
  } catch {
    send(res, 400, "Bad request");
    return;
  }
  const filePath = path.normalize(path.join(ROOT, pathname));
  if (!isInsideRoot(filePath)) {
    send(res, 403, "Forbidden");
    return;
  }
  if (!exists(filePath) || fs.statSync(filePath).isDirectory()) {
    send(res, 404, "Not found");
    return;
  }
  const ext = path.extname(filePath).toLowerCase();
  const types = {
    ".html": "text/html; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".png": "image/png",
    ".svg": "image/svg+xml"
  };
  res.writeHead(200, { "Content-Type": types[ext] || "application/octet-stream", "Cache-Control": "no-store" });
  fs.createReadStream(filePath).pipe(res);
}

async function handleApi(req, res) {
  const requestUrl = new URL(req.url, `http://127.0.0.1:${config.port || DEFAULT_PORT}`);
  const parts = requestUrl.pathname.split("/").filter(Boolean);
  try {
    if (req.method === "GET" && requestUrl.pathname === "/api/state") {
      send(res, 200, await state());
      return;
    }

    if (req.method === "POST" && parts[1] === "suite" && parts.length === 3) {
      const result = await runSuite(parts[2]);
      send(res, 200, { ok: true, ...result, state: await state() });
      return;
    }

    if (req.method === "POST" && requestUrl.pathname === "/api/logs/clear") {
      config.logs = [];
      saveConfig();
      send(res, 200, { ok: true, state: await state() });
      return;
    }

    if (req.method === "GET" && requestUrl.pathname === "/api/settings/system") {
      send(res, 200, publicSystemSettings());
      return;
    }

    if (req.method === "POST" && requestUrl.pathname === "/api/settings/system") {
      const body = await readJsonBody(req);
      send(res, 200, { ok: true, message: "系统设置已保存", systemSettings: updateSystemSettings(body) });
      return;
    }

    if (req.method === "POST" && requestUrl.pathname === "/api/settings/paths") {
      const body = await readJsonBody(req);
      send(res, 200, { ok: true, message: "本机路径已保存", systemSettings: updatePaths(body), state: await state() });
      return;
    }

    if (req.method === "POST" && parts[1] === "services" && parts[3] === "auto" && parts.length === 4) {
      const body = await readJsonBody(req);
      const result = updateServiceAuto(parts[2], body.auto);
      send(res, 200, { ok: true, ...result });
      return;
    }

    if (req.method === "POST" && parts[1] === "services" && parts.length === 4) {
      const service = config.services.find((item) => item.id === parts[2]);
      if (!service) throw new Error("服务不存在");
      const action = parts[3];
      let message = "";
      if (action === "start") message = await startService(service);
      else if (action === "stop") message = await stopService(service);
      else if (action === "restart") message = await restartService(service);
      else throw new Error("服务操作不存在");
      send(res, 200, { ok: true, message, state: await state() });
      return;
    }

    if (req.method === "POST" && requestUrl.pathname === "/api/sites") {
      const body = await readJsonBody(req);
      send(res, 200, { ok: true, site: createSite(body), state: await state() });
      return;
    }

    if (req.method === "PUT" && parts[1] === "sites" && parts.length === 3) {
      const body = await readJsonBody(req);
      send(res, 200, { ok: true, site: updateSite(parts[2], body), state: await state() });
      return;
    }

    if (req.method === "GET" && parts[1] === "sites" && parts[3] === "config" && parts.length === 4) {
      send(res, 200, siteConfigFile(parts[2]));
      return;
    }

    if (req.method === "DELETE" && parts[1] === "sites" && parts.length === 3) {
      const result = removeRecord("sites", parts[2]);
      send(res, 200, { ok: true, ...result, state: await state() });
      return;
    }

    if (req.method === "POST" && requestUrl.pathname === "/api/databases") {
      const body = await readJsonBody(req);
      send(res, 200, { ok: true, database: await createDatabase(body), state: await state() });
      return;
    }

    if (req.method === "GET" && requestUrl.pathname === "/api/databases/backups") {
      send(res, 200, { backups: listDatabaseBackups() });
      return;
    }

    if (req.method === "POST" && requestUrl.pathname === "/api/databases/backups/delete") {
      const body = await readJsonBody(req);
      const result = deleteDatabaseBackup(body);
      send(res, 200, { ok: true, ...result, state: await state() });
      return;
    }

    if (req.method === "POST" && parts[1] === "databases" && parts[3] === "export" && parts.length === 4) {
      const result = await exportDatabase(parts[2]);
      send(res, 200, { ok: true, ...result, state: await state() });
      return;
    }

    if (req.method === "POST" && parts[1] === "databases" && parts[3] === "import" && parts.length === 4) {
      const body = await readJsonBody(req);
      const result = await importDatabase(parts[2], body);
      send(res, 200, { ok: true, ...result, state: await state() });
      return;
    }

    if (req.method === "DELETE" && parts[1] === "databases" && parts.length === 3) {
      const result = removeRecord("databases", parts[2]);
      send(res, 200, { ok: true, ...result, state: await state() });
      return;
    }

    if (req.method === "POST" && requestUrl.pathname === "/api/databases/root-password") {
      const body = await readJsonBody(req);
      send(res, 200, { ok: true, result: await changeRootPassword(body), state: await state() });
      return;
    }

    if (req.method === "POST" && requestUrl.pathname === "/api/ftp") {
      const body = await readJsonBody(req);
      send(res, 200, { ok: true, account: createFtpAccount(body), state: await state() });
      return;
    }

    if (req.method === "PUT" && parts[1] === "ftp" && parts.length === 3) {
      const body = await readJsonBody(req);
      send(res, 200, { ok: true, account: updateFtpAccount(parts[2], body), state: await state() });
      return;
    }

    if (req.method === "DELETE" && parts[1] === "ftp" && parts.length === 3) {
      const result = removeRecord("ftp", parts[2]);
      send(res, 200, { ok: true, ...result, state: await state() });
      return;
    }

    if (req.method === "GET" && parts[1] === "config-files" && parts[2]) {
      send(res, 200, getConfigFile(decodeURIComponent(parts[2])));
      return;
    }

    if (req.method === "POST" && parts[1] === "config-files" && parts[2]) {
      const body = await readJsonBody(req);
      send(res, 200, { ok: true, file: saveConfigFile(decodeURIComponent(parts[2]), body.content), state: await state() });
      return;
    }

    if (req.method === "POST" && parts[1] === "open" && parts[2]) {
      const body = await readJsonBody(req);
      let message = "";
      if (parts[2] === "folder") message = openFolder(body.path);
      else if (parts[2] === "file") message = openFile(body.path);
      else if (parts[2] === "url") message = openUrl(body.url);
      else throw new Error("打开操作不存在");
      addLog(message);
      send(res, 200, { ok: true, message, state: await state() });
      return;
    }

    if (req.method === "POST" && parts[1] === "software" && parts.length === 4) {
      const action = parts[3];
      let message = "";
      if (action === "install") message = await installSoftware(parts[2]);
      else if (action === "uninstall") message = await uninstallSoftware(parts[2]);
      else {
        send(res, 404, { error: "软件操作不存在" });
        return;
      }
      send(res, 200, { ok: true, message, state: await state() });
      return;
    }

    send(res, 404, { error: "API not found" });
  } catch (error) {
    addLog(`错误：${error.message}`);
    send(res, 500, { error: error.message, stdout: error.stdout, stderr: error.stderr });
  }
}

const server = http.createServer((req, res) => {
  if (req.url.startsWith("/api/")) {
    handleApi(req, res);
    return;
  }
  serveStatic(req, res);
});

server.on("error", (error) => {
  if (error.code === "EADDRINUSE") {
    console.error(`端口 ${config.port || DEFAULT_PORT} 已被占用。管理台可能已经在运行： http://127.0.0.1:${config.port || DEFAULT_PORT}`);
    process.exit(1);
  }
  throw error;
});

server.listen(config.port || DEFAULT_PORT, "127.0.0.1", () => {
  console.log(`XP.CN local manager: http://127.0.0.1:${config.port || DEFAULT_PORT}`);
  console.log(`Config: ${CONFIG_PATH}`);
  if (config.systemSettings.startSuiteOnLaunch) {
    runSuite("start")
      .then((result) => console.log(result.message))
      .catch((error) => console.error(`Auto start failed: ${error.message}`));
  }
});

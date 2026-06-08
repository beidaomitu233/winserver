# XP.CN 小皮本地环境管理台

当前分支：`codex/simple-web-manager`

本分支不再使用 WinUI，改为最小依赖的 Web 技术栈，便于快速完成和迭代：

- 后端：Node.js 内置 `http` 服务，无 Express 依赖。
- 前端：原生 HTML / CSS / JavaScript。
- 入口：[server.js](server.js) + [index.html](index.html)。

## 启动

```powershell
.\run-web.ps1
```

或：

```powershell
npm start
```

默认地址：

```text
http://127.0.0.1:18113
```

可选环境变量：

- `XPCN_PORT`：覆盖管理台端口。
- `XPCN_DATA_DIR`：覆盖运行数据目录，便于测试或多实例隔离。
- `XPCN_STARTUP_DIR`：覆盖开机自启脚本目录，主要用于测试。
- `XPCN_HOSTS_PATH`：覆盖 hosts 文件路径，主要用于测试。

## 检查

```powershell
npm run check
```

运行轻量冒烟测试（会自动选择一个临时端口启动管理台，验证 `/api/state` 和首页）：

```powershell
npm test
```

## UI 分支切换

本项目按 UI 形态使用 git 分支隔离，当前可交付分支是 Web 版：

```text
codex/simple-web-manager
```

切换脚本：

```powershell
.\switch-ui.ps1 web
.\switch-ui.ps1 winforms
.\switch-ui.ps1 wpf
.\switch-ui.ps1 winui
```

第一次创建对应 UI 分支时：

```powershell
.\switch-ui.ps1 web -Create
```

注意：切换前工作区必须干净。当前仓库尚无初始提交时，需要先完成一次提交，git 才能稳定保存和切换各 UI 分支。

## 已接入能力

- 首页：读取 Apache、Nginx、PHP-CGI、MySQL/MariaDB、Redis、MinIO、FTP 等服务运行状态。
- 一键套件：启动或停止自动服务，并可在服务列表里调整每个服务是否加入套件。
- 网站：创建和编辑本地站点目录，并尽量写入 Apache/Nginx vhost 与 hosts 映射，支持移除管理台记录。
- 数据库：通过本机 MySQL/MariaDB `mysql.exe` 创建数据库/账号，支持修改 root 密码、导出 SQL 备份、查看最近备份和移除管理台记录。
- FTP：创建和编辑本地 FTP 账号目录，并同步写入 FileZilla Server.xml；移除记录时清理 XP.CN 托管的账号配置。
- 软件管理：识别本机组件安装状态，支持配置了下载地址的组件安装。
- 设置：读取和保存 `php.ini`、`httpd.conf`、`nginx.conf`、`my.ini`、`redis.conf`、`hosts` 等配置文件，支持系统设置、开机自启脚本，以及在界面中编辑本机组件路径。

## 本机数据

首次启动会生成 `data/config.json`，其中包含本机路径、服务列表、站点、数据库和日志等运行态数据。该文件已加入 `.gitignore`，避免不同 UI 分支之间互相污染本机配置。

默认 PHP-CGI 监听 `127.0.0.1:9073`，避免与 MinIO API 默认端口 `9000` 冲突；创建站点时生成的 Nginx vhost 会自动使用该端口。PHP-CGI、MinIO API 和 MinIO 控制台端口可在“设置 / 系统设置”中调整。

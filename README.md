# WinServer Cloud Stack Desktop

本地 Windows Web 服务管理面板（类似 phpStudy），基于 Tauri + Vue 3 + Rust。

## 技术栈

- 后端：Rust Agent（SQLite + Named Pipe + 服务管理）
- 前端：Vue 3 + TypeScript + Tauri v2
- 通信：Tauri IPC（JSON-RPC）

## 分支与协作（GitFlow · 强制）

本仓库自 **2026-07-11** 起按 GitFlow 管理，保证 **`main` 始终可发布**、日常开发不破坏可用性。

| 分支 | 含义 |
| --- | --- |
| `main` | 生产稳定线，仅接受 `release/*` / `hotfix/*` |
| `develop` | 集成线，功能默认合入此处 |
| `feature/*` · `bugfix/*` | 从 `develop` 拉出 |
| `release/*` · `hotfix/*` | 发版与热修 |

**必读规范：**

- [`docs/GITFLOW.md`](docs/GITFLOW.md) — 完整分支、合并、tag 与红线
- [`docs/RELEASE_CHECKLIST.md`](docs/RELEASE_CHECKLIST.md) — 合并 / 发版检查清单
- [`AGENTS.md`](AGENTS.md) — 人与 AI 协作者速查

```powershell
# 日常开发
git checkout develop
git pull
git checkout -b feature/your-topic
```

## 开发

```powershell
cd frontend
npm install
npm run tauri dev
```

## 构建

```powershell
cd frontend
npm run tauri build
```

## 已接入能力

- 首页：读取 Apache、Nginx、PHP-CGI、MySQL/MariaDB、Redis、MinIO 等服务运行状态
- 一键套件：启动或停止自动服务，并可在服务列表里调整每个服务是否加入套件
- 网站：创建和编辑本地站点目录，支持 Nginx/Apache vhost 与 hosts 映射
- 数据库：MySQL/PostgreSQL 数据库创建、账号管理、SQL 导出/导入/还原
- 软件管理：在线下载安装或导入本地 Nginx/PHP 运行环境
- 配置编辑：php.ini、nginx.conf、hosts 等配置文件在线编辑
- 设置：系统设置、路径配置、开机自启

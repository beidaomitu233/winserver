# WinServer Cloud Stack Desktop

本地 Windows Web 服务管理面板（类似 phpStudy），基于 Tauri + Vue 3 + Rust。

## 技术栈

- 后端：Rust Agent（SQLite + Named Pipe + 服务管理）
- 前端：Vue 3 + TypeScript + Tauri v2
- 通信：Tauri IPC（JSON-RPC）

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

# WinServer 介绍文档

## 1. 软件定位

WinServer 是一个面向 Windows 本地开发环境的服务面板，目标是提供类似 phpStudy 的一站式体验：用户打开软件后，不需要理解 Nginx、PHP、MySQL、Redis、hosts、端口、配置文件和进程管理的细节，就能通过清晰的按钮完成本地开发环境搭建。

核心体验要求：

- 打开即可看到服务启动区，首页简洁、少文字、少标签。
- 新手可以一键启动常用服务，不需要手动执行命令。
- 每个操作都必须形成真实闭环：页面操作、后端处理、配置落地、服务实际运行、检测通过、页面更新。
- 失败时不只提示“失败”，必须说明原因、位置、影响和可执行处理建议。

## 2. 软件设计目的

WinServer 要解决三类问题：

| 问题 | 用户痛点 | WinServer 设计 |
| --- | --- | --- |
| 本地环境安装复杂 | 新手不知道下载哪个 Nginx/PHP/MySQL 版本，不会配置路径 | 提供内置安装、离线导入、本地检测和完整性校验 |
| 配置链路容易断 | 手写 Nginx vhost、PHP FastCGI、hosts 容易出错 | 使用强类型输入生成配置，并在应用前验证 |
| 服务状态不可信 | 很多面板只记录“点过启动”，实际端口和进程可能失败 | 状态必须来自进程、端口、配置和健康检查 |

第一阶段以 Nginx + PHP + 站点 + hosts + 日志 + 软件管理为主；数据库、Redis、PostgreSQL、MinIO 等页面可以保留入口，但必须标明真实支持范围，不能用模拟成功替代真实能力。

## 3. 包含功能

### 3.1 首页启动面板

首页只承载最常用的启动和状态判断。

- 大尺寸服务卡片：Nginx、PHP、MySQL、Redis 可按安装情况显示。
- 每张卡片显示服务名、真实状态、端口、主按钮、次按钮。
- 主按钮根据状态变化：未安装、启动、停止、重启、查看问题。
- 移除非必要 tag、说明文字、堆叠指标和营销式文案。
- 首页允许保留小型快捷入口：创建站点、软件管理、日志、端口检查。

### 3.2 服务管理

- 启动、停止、重启服务。
- 查看 PID、端口、安装目录、配置文件。
- 自动检测启动失败原因。
- 区分期望状态和真实状态。
- 服务异常退出后能在后台刷新为 failed/degraded。

### 3.3 软件/运行环境管理

- 检测本机已安装服务。
- 导入本地 Nginx、PHP。
- 安装内置 bundled runtime。
- 下载并安装 runtime，具备进度、校验、解压、探测、登记闭环。
- 删除未被站点引用的 runtime，不能误删用户目录。

### 3.4 网站管理

- 创建、编辑、启用、停用、删除站点。
- 每个站点可以绑定域名、端口、根目录、服务器类型、PHP 版本。
- 自动生成 vhost 配置。
- 自动同步 hosts 中由 WinServer 管理的记录。
- 创建/编辑/切换 PHP 失败时必须回滚。

### 3.5 PHP 管理

- 支持多个 PHP runtime 共存。
- 每个 PHP runtime 独立 FastCGI 端口。
- 网站切换 PHP 后要验证实际 PHP 版本。
- php.ini 可查看、编辑、备份。

### 3.6 数据库与工具

- MySQL 作为第二优先级能力，支持连接检查、库列表、本地管理记录。
- phpMyAdmin 地址来自设置，不允许硬编码。
- PostgreSQL、Redis、MinIO 可作为后续模块，但未完成前不得伪装为已完成。

### 3.7 日志与诊断

- 日志中心展示操作日志、Nginx access/error、PHP error、Agent 日志。
- 端口检查展示端口是否被占用、PID、进程名、建议动作。
- 配置编辑必须先备份，再保存，再校验相关服务配置。
- 诊断结果统一包含错误码、标题、原因、影响对象、建议操作。

## 4. 当前仓库目录结构

| 路径 | 说明 | 规划要求 |
| --- | --- | --- |
| `frontend/` | Vue 3 + TypeScript 前端界面 | 负责页面、组件、Pinia 状态和 Tauri invoke |
| `frontend/src-tauri/` | Tauri 2 桥接层 | 负责命令注册、权限能力、调用 Agent handler |
| `agent/` | Rust Core Agent | 负责进程、配置、站点、runtime、日志、数据库和系统操作 |
| `shared/` | 前后端共享 Rust 类型和 JSON-RPC 协议 | 维护稳定 API 字段、错误码和状态枚举 |
| `tests/e2e/` | Playwright 端到端测试 | 覆盖页面操作、关键闭环和回归 |
| `agent/tests/` | Rust 集成测试 | 覆盖 API、runtime 安装、PRD 验收链路 |
| `docs/` | 产品、架构、计划和验收文档 | 本轮文档体系的唯一修改范围 |

建议发布态目录：

```text
WinServer/
  app/
    WinServer.exe
    WinServerAgent.exe
  runtimes/
    nginx/
    php/
    mysql/
    redis/
  data/
    database/winserver.db
    configs/
    logs/
    backups/
  downloads/
  staging/
  temp/
```

目录原则：

- 应用程序、运行时、用户数据必须分离。
- 应用升级不得覆盖 `data/`。
- runtime 更新不得覆盖旧版本。
- 临时下载、解压和正式目录必须分离，失败时可以清理 staging。

## 5. 接口设计

WinServer 使用 Tauri command 调用本地 Rust handler，handler 内部采用 JSON-RPC 风格方法名。UI 不允许直接执行系统命令。

### 5.1 请求结构

```json
{
  "id": "uuid",
  "method": "service.start",
  "params": {
    "serviceId": "nginx"
  }
}
```

### 5.2 响应结构

```json
{
  "id": "uuid",
  "result": {
    "state": {},
    "message": "nginx 已启动"
  }
}
```

失败响应：

```json
{
  "id": "uuid",
  "error": {
    "code": -32000,
    "message": "80 端口已被占用",
    "data": {
      "errorCode": "PORT_OCCUPIED",
      "port": 80,
      "pid": 5328
    }
  }
}
```

### 5.3 核心方法族

| 方法族 | 代表方法 | 页面 | 说明 |
| --- | --- | --- | --- |
| 状态 | `state.get`, `state.getFast` | 全局 | 获取服务、站点、软件、设置、日志摘要 |
| 服务 | `service.start`, `service.stop`, `service.restart`, `service.toggleAuto` | 首页/服务卡片 | 服务生命周期控制 |
| 套件 | `suite.start`, `suite.stop` | 首页 | 按 auto 配置启动/停止一组服务 |
| 站点 | `site.create`, `site.update`, `site.delete`, `site.enable`, `site.disable`, `site.switchPhp`, `site.config` | 网站页 | 站点生命周期和 PHP 切换 |
| runtime | `runtime.import`, `runtime.list` | 软件页 | 导入与查询运行环境 |
| 软件 | `software.detectLocal`, `software.install`, `software.downloadInstall`, `software.installBundled`, `software.uninstall` | 软件页 | 检测、安装、删除 runtime |
| 配置 | `config.get`, `config.save` | 设置/网站/首页 | 读取和保存配置文件 |
| 日志 | `log.list`, `log.clear` | 日志页 | 读取、搜索、清空日志 |
| 端口 | `port.check` | 首页/设置 | 端口检测 |
| hosts | `hosts.sync`, `hosts.remove` | 网站闭环 | 只维护 WinServer 管理的域名 |
| 数据库 | `database.create`, `database.delete`, `database.export`, `database.import`, `database.backups` | 数据库页 | MySQL/PostgreSQL 管理能力 |
| 设置 | `settings.get`, `settings.update`, `settings.paths` | 设置页 | 全局配置和路径 |
| 文件 | `files.list` | 文件页 | 浏览本地目录 |

## 6. 状态模型

服务必须同时记录两类状态：

| 字段 | 说明 | 示例 |
| --- | --- | --- |
| `desired_state` | 用户希望服务处于的状态 | `running`, `stopped` |
| `actual_state`/`state` | 真实检测结果 | `running`, `failed`, `degraded` |

状态枚举：

- `installed`：已登记但未启动。
- `starting`：启动命令已发出，等待检测。
- `running`：进程、端口、配置、健康检查均通过。
- `degraded`：部分检测失败但仍有进程或端口响应。
- `stopping`：停止中。
- `stopped`：进程和端口均释放。
- `failed`：启动或运行失败，有错误原因。
- `unknown`：无法检测或尚未探测。

UI 规则：

- 不能因为接口返回 200 就显示运行中。
- `running` 必须来自 Agent 的真实检测。
- `failed` 卡片必须提供“查看问题”或“诊断”入口。
- `desired_state=running` 且 `state=failed` 时显示为“启动失败”，不是“已停止”。

## 7. 链路闭环

### 7.1 启动服务闭环

```text
用户点击启动
  -> 前端设置按钮 loading
  -> 调用 service.start
  -> Agent 校验 runtime 是否安装
  -> 检查可执行文件、cwd、配置文件
  -> 检查端口占用
  -> 启动进程并记录 PID
  -> 检测端口是否监听
  -> 执行服务健康检查
  -> 更新 service_instances
  -> 写 operation_logs
  -> 返回 state
  -> 前端刷新卡片状态
```

任一步失败：

- 服务状态写为 `failed`。
- 返回错误码和可读提示。
- 不显示虚假成功。
- 不误杀非 WinServer 管理的同名进程。

### 7.2 安装/导入 runtime 闭环

```text
用户选择安装/导入
  -> 前端提交 runtime 类型、路径或软件 ID
  -> Agent 下载或读取本地目录
  -> 校验目录/压缩包/哈希/关键文件
  -> 执行版本探测命令
  -> 分配端口和服务 ID
  -> 写 runtimes、software、service_instances、config_files
  -> 刷新 state
  -> 软件页可选择该 runtime
```

失败时：

- 删除未完成的 staging。
- 不修改正式 runtime 指向。
- 写入安装失败日志。
- 展示缺失文件或版本探测失败原因。

### 7.3 创建站点闭环

```text
用户填写域名、端口、目录、PHP 版本
  -> 前端表单校验
  -> 调用 site.create
  -> Agent 校验域名、端口、目录、runtime
  -> 确保 PHP FastCGI 可用
  -> 生成临时 vhost
  -> 执行 Nginx 配置检查
  -> 备份旧配置和 hosts
  -> 写正式 vhost
  -> 写 hosts 管理区块
  -> reload Nginx
  -> HTTP/PHP 健康检查
  -> 写 sites
  -> 返回新 state
```

失败时：

- 恢复旧 vhost。
- 恢复 hosts。
- 不插入半成品站点。
- Nginx 继续使用旧配置。
- 页面显示失败步骤和处理建议。

### 7.4 切换 PHP 闭环

```text
用户选择新的 PHP runtime
  -> 调用 site.switchPhp
  -> 校验目标 PHP 完整性
  -> 启动目标 php-cgi
  -> 生成新 vhost
  -> Nginx 配置检查
  -> reload Nginx
  -> 访问测试 PHP 页面验证实际版本
  -> 更新 sites.php_runtime_id
```

失败时：

- 恢复旧 vhost。
- 保留旧 PHP runtime。
- 不更新数据库中的 PHP 指向。

## 8. 关联文档

| 文档 | 作用 |
| --- | --- |
| `docs/prd.md` | 产品功能、交互、状态、业务闭环的完整需求 |
| `docs/验收文档.md` | 单元、集成、系统、人工验收的完成标准 |
| `docs/PROJECT_DOCUMENT.md` | 面向执行模型的项目总说明 |
| `docs/FRONTEND_PLAN.md` | 前端逐任务实现计划 |
| `docs/BACKEND_PLAN.md` | 后端逐任务实现计划 |
| `docs/DATABASE_PLAN.md` | SQLite 表、字段、索引、迁移和依赖 |
| `docs/COMMUNICATION.md` | 前端、后端、数据库、审查模型的问题登记表 |

## 9. 文档维护任务

- [ ] 任务编号：DOC-INTRO-001
  模块：介绍文档
  目标：每次 PRD 范围变化后同步软件定位、功能范围和闭环说明。
  实现说明：由文档负责人比对 `prd.md`、`验收文档.md` 和计划文件，只更新事实变化，不加入未确认承诺。
  依赖文档：`docs/prd.md`、`docs/验收文档.md`
  验收标准：介绍文档中的功能范围、目录结构和接口族与执行计划一致。
  测试要求：执行文档审查，确认无废弃接口名、无伪完成描述。

- [ ] 任务编号：DOC-INTRO-002
  模块：链路闭环
  目标：任何新增功能都必须补充用户操作到真实检测的闭环。
  实现说明：新增流程必须包含前端入口、后端方法、数据表、文件系统副作用、失败回滚和验收方式。
  依赖文档：`docs/PROJECT_DOCUMENT.md`、`docs/COMMUNICATION.md`
  验收标准：执行模型可以根据闭环描述拆出前端、后端、数据库任务。
  测试要求：审查新增功能是否能映射到至少一个自动测试和一个人工验收步骤。


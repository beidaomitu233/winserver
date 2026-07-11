# WinServer 项目总文档

## 1. 项目背景与目标

WinServer 是 Windows 本地 Web 服务面板，目标是提供类似 phpStudy 的本地环境管理体验，并在稳定性、错误诊断、状态可信度和任务闭环上做优化。

核心目标：

- 新手打开软件即可启动本地开发环境。
- Nginx、PHP、站点、hosts、日志、配置形成真实闭环。
- 首页简洁，启动服务卡片突出，去掉多余说明和标签。
- 后端只暴露受控接口，不允许前端直接执行系统命令。
- 所有功能必须经过单元测试、集成测试、系统测试和人工验收。

关联文档：

- `docs/GITFLOW.md`（**强制**：分支与发版，保证可用性）
- `docs/RELEASE_CHECKLIST.md`（合入 develop / main 检查清单）
- `AGENTS.md`（协作者 / Agent 速查）
- `docs/WINSERVER_INTRODUCTION.md`
- `docs/prd.md`
- `docs/验收文档.md`
- `docs/FRONTEND_PLAN.md`
- `docs/BACKEND_PLAN.md`
- `docs/DATABASE_PLAN.md`
- `docs/COMMUNICATION.md`

## 2. 用户角色与权限

| 用户角色 | 主要目标 | 权限边界 |
| --- | --- | --- |
| 新手开发者 | 一键搭建 Nginx/PHP 本地站点 | 可以安装/导入 runtime、启动服务、建站 |
| PHP 开发者 | 多 PHP 版本和多站点管理 | 可以切换 PHP、编辑配置、查看日志 |
| 前端开发者 | 本地静态服务、反向代理调试 | 可以创建静态站点、启动 Nginx |
| 测试人员 | 验证安装、故障和回归链路 | 可以执行全量验收、查看日志 |

第一阶段权限模型：

- 本地单用户。
- 不做登录。
- UI 普通权限运行。
- 写 hosts、安装服务、修改受保护目录必须由受控 Agent 或提权流程完成。

## 3. 核心业务流程

### 3.1 首次使用流程

```text
启动应用
  -> Agent 初始化 SQLite 和数据目录
  -> 检测本机 runtime
  -> 首页显示服务卡片
  -> 用户安装/导入 Nginx
  -> 用户安装/导入 PHP
  -> 用户启动服务
  -> 用户创建站点
  -> 浏览器访问本地域名
```

验收重点：

- 首屏不黑屏。
- 未安装状态清晰。
- 安装/导入后服务可启动。
- 建站后域名可访问。

### 3.2 服务启动流程

```text
前端 service_start
  -> Tauri command
  -> handler service.start
  -> ProcessManager 校验和启动
  -> PortManager 检测端口
  -> 健康检查
  -> service_instances 更新
  -> operation_logs 记录
  -> 返回 AppState
```

### 3.3 网站创建流程

```text
前端 site_create
  -> 表单校验
  -> handler site.create
  -> SiteManager 校验站点
  -> RuntimeManager 确认 PHP
  -> ConfigManager 生成 vhost
  -> HostsManager 写 managed 区块
  -> Nginx reload
  -> HTTP/PHP health check
  -> sites 写入
  -> 返回 AppState
```

### 3.4 配置编辑流程

```text
读取 config.get
  -> 展示文件内容
  -> 用户保存 config.save
  -> 后端校验 fileId 和路径
  -> 备份旧文件
  -> 写入新文件
  -> 对关联服务执行配置检查
  -> 失败时提示恢复
  -> 写操作日志
```

## 4. 系统模块划分

| 模块 | 前端页面/组件 | 后端模块 | 数据表 | 优先级 |
| --- | --- | --- | --- | --- |
| 首页启动 | `DashboardPage.vue`、服务卡片 | `ProcessManager`、`PortManager` | `service_instances`、`operation_logs` | P0 |
| 软件管理 | `SoftwarePage.vue` | `RuntimeManager` | `software`、`runtimes`、`service_instances` | P0 |
| 网站管理 | `SitesPage.vue`、站点表单 | `SiteManager`、`HostsManager`、`ConfigManager` | `sites`、`config_files` | P0 |
| PHP 切换 | `SitesPage.vue` | `SiteManager`、`RuntimeManager` | `sites`、`runtimes` | P0 |
| 配置编辑 | `ConfigEditorModal.vue` | `config.get`、`config.save` | `config_files`、`operation_logs` | P0 |
| 日志中心 | `LogsPage.vue` | `log.list`、`log.clear` | `operation_logs` | P0 |
| 端口诊断 | `PortCheckModal.vue` | `PortManager` | 无直接表，写日志可选 | P0 |
| 设置 | `SettingsPage.vue` | `settings.get/update/paths` | `settings` | P1 |
| 数据库 | `DatabasePage.vue` | database handlers | `databases`、`settings` | P1 |
| 文件浏览 | `FilesPage.vue` | `files.list` | 无 | P2 |

## 5. 前端页面结构

当前页面结构采用单页应用内部状态切换：

| 页面 ID | 文件 | 主要职责 |
| --- | --- | --- |
| `dashboard` | `frontend/src/pages/DashboardPage.vue` | 首页启动卡片、快捷入口、最近错误 |
| `sites` | `frontend/src/pages/SitesPage.vue` | 站点列表、新建、编辑、启停、删除、打开 |
| `database` | `frontend/src/pages/DatabasePage.vue` | 数据库管理和 phpMyAdmin |
| `software` | `frontend/src/pages/SoftwarePage.vue` | runtime 检测、安装、导入、删除 |
| `files` | `frontend/src/pages/FilesPage.vue` | 本地目录浏览 |
| `logs` | `frontend/src/pages/LogsPage.vue` | 日志查看、搜索、清空 |
| `settings` | `frontend/src/pages/SettingsPage.vue` | 设置、路径、配置、备份、安全 |

全局组件：

- `AppSidebar.vue`：导航和小型状态摘要。
- `AppTopbar.vue`：页面标题和刷新入口。
- `AppStatusbar.vue`：底部状态。
- `ConfigEditorModal.vue`：配置文件编辑器。
- `PortCheckModal.vue`：端口检查。

## 6. 后端服务结构

| 后端模块 | 责任 |
| --- | --- |
| `RequestHandler` | JSON-RPC 方法路由、参数解析、返回 AppState |
| `ProcessManager` | 服务启动、停止、重启、健康监控、系统资源 |
| `RuntimeManager` | runtime 导入、安装、下载、探测、完整性校验 |
| `SiteManager` | 站点创建、编辑、启用、停用、删除、PHP 切换 |
| `PortManager` | 端口检测和占用诊断 |
| `HostsManager` | hosts managed 区块写入、删除、回滚 |
| `ConfigManager` | 配置模板生成、备份、验证、保存 |
| `Database` | SQLite 迁移、仓储、设置、日志 |

## 7. 数据库核心实体

| 实体 | 表 | 用途 |
| --- | --- | --- |
| 服务实例 | `service_instances` | 保存服务配置、安装状态、期望状态和最近检测信息 |
| 站点 | `sites` | 保存域名、端口、目录、PHP runtime 和状态 |
| runtime | `runtimes` | 保存导入/安装的 Nginx/PHP/MySQL 等运行环境 |
| 软件目录 | `software` | 保存软件卡片、安装方式、下载地址和安装状态 |
| 配置文件 | `config_files` | 保存可编辑配置文件 ID、标签、路径 |
| 设置 | `settings` | 保存 key/value 系统设置 |
| 操作日志 | `operation_logs` | 保存用户操作、结果和错误 |
| 数据库记录 | `databases` | 保存本地管理的数据库信息 |

详见 `docs/DATABASE_PLAN.md`。

## 8. 接口协作原则

1. 所有前端调用必须集中封装，不在组件中散落复杂参数拼装。
2. 接口命名使用 `domain.action`，如 `site.create`。
3. 前端字段使用 camelCase，后端内部可映射 snake_case，但返回结构要稳定。
4. 每个修改接口成功后尽量返回最新 `state`，前端统一刷新 store。
5. 错误响应必须包含可读 message；重要错误应包含结构化 data。
6. 前端不得自行推断服务运行成功，只能展示后端 state。
7. 新增接口必须同步更新 `FRONTEND_PLAN.md`、`BACKEND_PLAN.md`、`DATABASE_PLAN.md` 和 `COMMUNICATION.md`。

## 9. 状态流转说明

### 9.1 服务状态

```text
unknown -> installed -> starting -> running
running -> stopping -> stopped
starting -> failed
running -> degraded
degraded -> failed/running
failed -> starting/stopped
```

流转要求：

- `starting` 超时必须变成 `failed`。
- `running` 的服务异常退出必须变成 `failed` 或 `stopped`。
- `degraded` 必须说明哪一层检测失败。
- `desired_state` 和 `actual_state` 不一致时，页面必须提示差异。

### 9.2 站点状态

| 状态 | 说明 |
| --- | --- |
| `active` | vhost、hosts、健康检查通过 |
| `disabled` | 用户停用，vhost/hosts 不生效 |
| `error` | 配置或健康检查异常 |
| `deleting` | 删除流程进行中，失败后恢复 |

### 9.3 runtime 状态

| 状态 | 说明 |
| --- | --- |
| `available` | 可安装或可导入 |
| `installing` | 正在安装/解压/下载 |
| `installed` | 已登记且完整性通过 |
| `corrupted` | 已登记但关键文件缺失 |
| `removing` | 删除/移除中 |

## 10. 异常场景说明

| 场景 | 处理要求 |
| --- | --- |
| 端口被占用 | 显示端口、PID、进程名、建议动作；不得自动结束外部进程 |
| 配置语法错误 | 展示 stderr；恢复旧配置；服务保持旧状态 |
| runtime 不完整 | 标记 corrupted 或未安装；提示缺失文件 |
| hosts 无权限 | 提示权限问题；不写半条记录 |
| 网站目录不存在 | 让用户创建或重新选择；默认不自动创建深层路径 |
| 服务启动后退出 | 捕获退出码和 stderr；状态 failed |
| 数据库写入失败 | 回滚文件/hosts/配置副作用 |
| 日志文件不存在 | 显示空状态，不崩溃 |
| 下载失败 | 保留失败状态和重试入口，清理 staging |

## 11. 安全与权限要求

- 前端不能直接调用 shell。
- 后端只允许执行受控的 runtime 可执行文件。
- 命令参数必须数组化，不允许拼接完整命令字符串。
- hosts 和配置文件写入路径必须来自受信表或 runtime/site 关系。
- 密码字段不能记录明文日志。
- 下载包必须校验哈希或签名。
- 删除操作必须确认，且不删除用户项目源代码。

## 12. 性能与可扩展要求

- 首屏可交互不超过 3 秒。
- 页面切换不超过 500ms。
- 状态刷新不超过 2 秒。
- 大日志读取尾部并限制行数。
- Runtime 安装、下载、解压在后台执行。
- 后续支持 Apache/MySQL/Redis 时通过 adapter 扩展，避免在业务层堆叠 if/else。

## 13. Git 协作规范

分支：

- 主分支：`main`
- 开发分支：`dev`
- 前端分支：`feature/frontend-模块名`
- 后端分支：`feature/backend-模块名`
- 数据库分支：`feature/database-模块名`
- 文档分支：`docs/architecture-plan`

提交格式：

- `docs: update architecture plan`
- `feat(frontend): complete dashboard service cards`
- `feat(backend): complete service start api`
- `feat(database): add runtime package fields`
- `test: add service lifecycle tests`
- `fix: resolve review issue`

执行规则：

1. 每次只领取一个任务包。
2. 每完成一个任务包必须运行测试。
3. 对照验收标准自检。
4. 勾选对应 Plan。
5. 每完成一个任务包必须提交一次 Git，不允许多个未验收任务包混在同一次提交中。
6. 遇到接口、字段、范围冲突，写入 `docs/COMMUNICATION.md`，不得私自扩大实现。
7. 合并到 `dev` 前必须通过代码审查和测试记录。
8. `main` 只接收可人工验收的稳定版本。

## 14. 跨端任务包

- [ ] 任务编号：PROJ-001
  模块：启动闭环
  目标：首页到 Agent 的服务启动/停止/重启全链路。
  前端任务：`FE-001`、`FE-002`、`FE-003`
  后端任务：`BE-001`、`BE-002`、`BE-003`
  数据库任务：`DB-001`、`DB-007`
  验收标准：`AC-HOME-002`、`AC-SERVICE-001`、`AC-SERVICE-002`

- [ ] 任务编号：PROJ-002
  模块：runtime 安装导入
  目标：软件页安装/导入 Nginx/PHP 并可启动。
  前端任务：`FE-004`、`FE-005`
  后端任务：`BE-004`、`BE-005`
  数据库任务：`DB-002`、`DB-004`
  验收标准：`AC-RUNTIME-001`、`AC-RUNTIME-002`、`AC-RUNTIME-003`

- [ ] 任务编号：PROJ-003
  模块：站点生命周期
  目标：创建、编辑、启用、停用、删除站点闭环。
  前端任务：`FE-006`、`FE-007`、`FE-008`
  后端任务：`BE-006`、`BE-007`、`BE-008`
  数据库任务：`DB-003`、`DB-007`
  验收标准：`AC-SITE-001`、`AC-SITE-002`、`AC-SITE-003`、`AC-SITE-004`

- [ ] 任务编号：PROJ-004
  模块：PHP 切换
  目标：多 PHP runtime 可被站点切换并真实验证。
  前端任务：`FE-009`
  后端任务：`BE-009`
  数据库任务：`DB-002`、`DB-003`
  验收标准：`AC-PHP-001`

- [ ] 任务编号：PROJ-005
  模块：配置日志诊断
  目标：配置编辑、日志查看、端口检查可用。
  前端任务：`FE-010`、`FE-011`、`FE-012`
  后端任务：`BE-010`、`BE-011`、`BE-012`
  数据库任务：`DB-005`、`DB-007`
  验收标准：`AC-CONFIG-001`、`AC-CONFIG-002`、`AC-LOG-001`、`AC-DIAG-001`

## 15. 待确认问题

| 编号 | 问题 | 默认假设 | 影响范围 | 状态 |
| --- | --- | --- | --- | --- |
| Q-001 | 是否必须第一阶段完成 MySQL 下载/安装 | 先完成检测和已安装 MySQL 管理，安装作为 P1 | 数据库页、软件页 | 待确认 |
| Q-002 | 是否需要内置 Nginx/PHP 二进制包 | 支持 bundled 安装路径，但文档不承诺具体版本 | 安装验收 | 待确认 |
| Q-003 | Agent 是否必须注册 Windows Service | 开发期控制台，发布期服务化 | 启动、权限 | 待确认 |
| Q-004 | hosts 写入采用自动提权还是提示用户 | 优先 Agent 受控高权限；失败可提示手动授权 | 站点闭环 | 待确认 |
| Q-005 | 首页服务卡片展示数量上限 | P0 服务优先，其他折叠到更多 | 首页 UI | 待确认 |

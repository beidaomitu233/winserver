# WinServer 后端执行计划

## 1. 文档目的

本文档面向后端执行模型，用于把 `docs/prd.md` 中的真实系统能力拆成可执行、可测试、可审查的 Rust Agent 任务。后端任务必须围绕“真实检测、配置落地、进程控制、失败回滚、日志记录、稳定响应结构”完成。

执行边界：

- 后端负责 Tauri command 对应的 JSON-RPC handler、manager、SQLite 仓储、文件系统、进程、端口、hosts、配置和日志。
- 后端不得依赖前端传入任意命令。
- 后端不得只修改数据库而不执行对应系统副作用。
- 后端不得用临时成功、空方法、mock 结果通过验收。

## 2. 后端技术栈与推荐代码库

| 类别 | 技术/库 | 说明 |
| --- | --- | --- |
| 语言 | Rust | Core Agent 和 Tauri bridge |
| 桌面桥接 | Tauri 2 command | `frontend/src-tauri/src/commands.rs` |
| 异步运行 | Tokio | 后台任务、超时、健康检查 |
| 数据库 | SQLite + rusqlite | 本地状态、配置、日志 |
| 序列化 | serde / serde_json | JSON-RPC 请求响应 |
| 错误处理 | anyhow + thiserror | 业务错误与上下文 |
| 日志 | tracing | Agent、操作、诊断日志 |
| 文件模板 | Handlebars/MiniJinja/Tera | 生成 Nginx/PHP 配置 |
| 系统能力 | std::process + Windows API | 进程、端口、服务、权限 |

当前代码相关路径：

| 路径 | 说明 |
| --- | --- |
| `agent/src/server/handler.rs` | JSON-RPC 方法路由和 handler |
| `agent/src/managers/process_manager.rs` | 进程和服务生命周期 |
| `agent/src/managers/runtime_manager.rs` | runtime 导入/安装/探测 |
| `agent/src/managers/site_manager.rs` | 站点生命周期 |
| `agent/src/managers/hosts_manager.rs` | hosts 写入 |
| `agent/src/managers/config_manager.rs` | 配置生成与保存 |
| `agent/src/managers/port_manager.rs` | 端口检测 |
| `agent/src/database/connection.rs` | SQLite 仓储 |
| `agent/src/database/schema.rs` | 迁移和 seed |
| `shared/src/types.rs` | 前后端共享类型 |
| `shared/src/protocol.rs` | JSON-RPC 协议和错误码 |

## 3. 服务模块划分

| 模块 | 职责 | 禁止事项 |
| --- | --- | --- |
| RequestHandler | 解析 method/params，调用业务 manager，返回稳定响应 | 不写复杂业务流程，不吞错误 |
| ProcessManager | 启停进程、检测 PID/端口、后台健康监控 | 不按进程名全局强杀 |
| RuntimeManager | 导入、安装、下载、探测 runtime | 不登记未校验通过的 runtime |
| SiteManager | 创建/编辑/启停/删除站点、切 PHP | 不在任一步失败后留下半成品 |
| ConfigManager | 模板生成、配置备份、保存、验证 | 不拼接未校验用户输入 |
| HostsManager | managed hosts 区块增删、备份、恢复 | 不覆盖用户手写 hosts |
| PortManager | 检查端口、识别占用进程 | 不只返回 true/false |
| Database | 仓储、迁移、事务、日志 | 不由 UI 直接访问 |
| Diagnostics | 错误码、建议动作、日志定位 | 不只返回“失败” |

## 4. API 路由清单

### 4.1 状态与资源

| 方法 | 请求参数 | 响应字段 | 错误码 |
| --- | --- | --- | --- |
| `state.get` | 无 | `AppState` | `DATABASE_ERROR`、`STATE_BUILD_FAILED` |
| `state.getFast` | 无 | `AppState` | 同上 |
| `resource.get` | 无 | `SystemResource` | `RESOURCE_READ_FAILED` |

### 4.2 服务

| 方法 | 请求参数 | 响应字段 | 错误码 |
| --- | --- | --- | --- |
| `service.start` | `serviceId` | `state`、`message`、`diagnostics?` | `SERVICE_NOT_FOUND`、`BINARY_NOT_FOUND`、`PORT_OCCUPIED`、`CONFIG_INVALID`、`PROCESS_EXITED`、`HEALTH_CHECK_FAILED` |
| `service.stop` | `serviceId` | `state`、`message` | `SERVICE_NOT_FOUND`、`PROCESS_STOP_FAILED` |
| `service.restart` | `serviceId` | `state`、`message` | 同 start/stop |
| `service.toggleAuto` | `serviceId`、`auto` | `state` | `SERVICE_NOT_FOUND`、`DATABASE_ERROR` |
| `suite.start` | 无 | `state`、`summary` | 单项错误进入 summary |
| `suite.stop` | 无 | `state`、`summary` | 单项错误进入 summary |

### 4.3 runtime 与软件

| 方法 | 请求参数 | 响应字段 | 错误码 |
| --- | --- | --- | --- |
| `runtime.list` | 无 | `RuntimeManifest[]` | `DATABASE_ERROR` |
| `runtime.import` | `runtimeType`、`installPath`、`port?` | `runtime`、`state` | `PATH_INVALID`、`RUNTIME_CORRUPTED`、`PORT_OCCUPIED` |
| `software.detectLocal` | 无 | `localServices` | `DETECT_FAILED` |
| `software.installBundled` | `softwareId` | `runtime`、`state`、`message` | `BUNDLE_NOT_FOUND`、`EXTRACT_FAILED`、`RUNTIME_CORRUPTED` |
| `software.downloadInstall` | `softwareId` | `runtime`、`state`、`message` | `DOWNLOAD_FAILED`、`CHECKSUM_FAILED`、`EXTRACT_FAILED` |
| `software.downloadProgress` | `softwareId` | `total`、`downloaded`、`percent`、`phase` | 无则返回 idle |
| `software.uninstall` | `softwareId` | `state`、`message` | `SERVICE_RUNNING`、`RUNTIME_IN_USE` |

### 4.4 站点

| 方法 | 请求参数 | 响应字段 | 错误码 |
| --- | --- | --- | --- |
| `site.list` | 无 | `SiteInfo[]` | `DATABASE_ERROR` |
| `site.create` | `domain`、`port`、`path`、`server`、`phpRuntimeId?` | `state`、`site?` | `DOMAIN_CONFLICT`、`PATH_INVALID`、`PORT_OCCUPIED`、`CONFIG_INVALID`、`HOSTS_PERMISSION_DENIED`、`HEALTH_CHECK_FAILED` |
| `site.update` | `siteId`、`domain`、`port`、`path`、`server`、`phpRuntimeId?` | `state`、`site?` | 同 create |
| `site.enable` | `siteId` | `state` | `SITE_NOT_FOUND`、`CONFIG_INVALID`、`HEALTH_CHECK_FAILED` |
| `site.disable` | `siteId` | `state` | `SITE_NOT_FOUND`、`CONFIG_INVALID` |
| `site.delete` | `siteId` | `state` | `SITE_NOT_FOUND`、`CONFIG_INVALID` |
| `site.switchPhp` | `siteId`、`phpRuntimeId` | `state` | `RUNTIME_CORRUPTED`、`CONFIG_INVALID`、`HEALTH_CHECK_FAILED` |
| `site.config` | `siteId` | `id`、`label`、`path`、`exists`、`content` | `SITE_NOT_FOUND` |

### 4.5 配置、日志、端口、hosts

| 方法 | 请求参数 | 响应字段 | 错误码 |
| --- | --- | --- | --- |
| `config.get` | `fileId` | `id`、`label`、`path`、`exists`、`content` | `CONFIG_NOT_FOUND`、`READ_FAILED` |
| `config.save` | `fileId`、`content` | `state`、`message`、`backupPath?` | `CONFIG_NOT_FOUND`、`WRITE_DENIED`、`CONFIG_INVALID` |
| `log.list` | `source?`、`search?` | `source`、`path?`、`logs` | `LOG_SOURCE_INVALID` |
| `log.clear` | `source?` | `message`、`path?` | `WRITE_DENIED` |
| `port.check` | `port`、`address?`、`protocol?` | `port`、`isOpen`、`pid`、`processName`、`path?` | `PORT_CHECK_FAILED` |
| `hosts.sync` | `domain` | `message` | `HOSTS_PERMISSION_DENIED`、`DOMAIN_CONFLICT` |
| `hosts.remove` | `domain` | `message` | `HOSTS_PERMISSION_DENIED` |

### 4.6 设置、数据库、文件

| 方法 | 请求参数 | 响应字段 | 错误码 |
| --- | --- | --- | --- |
| `settings.get` | 无 | `SystemSettings` | `DATABASE_ERROR` |
| `settings.update` | 设置 key/value | `message`、`state?` | `VALIDATION_FAILED` |
| `settings.paths` | 路径 key/value | `state` | `PATH_INVALID` |
| `database.create` | `db`、`user`、`pass` | `state`、`message` | `MYSQL_NOT_INSTALLED`、`MYSQL_AUTH_FAILED`、`SQL_EXEC_FAILED` |
| `database.delete` | `db` | `state`、`message` | `DATABASE_NOT_FOUND` |
| `database.changePassword` | `db`、`user`、`pass` | `state`、`message` | `SQL_EXEC_FAILED` |
| `database.rootPassword` | `currentPass`、`pass` | `message` | `MYSQL_AUTH_FAILED` |
| `database.export` | `db` | `path`、`message` | `EXPORT_FAILED` |
| `database.import` | `db`、`path` | `message` | `PATH_INVALID`、`IMPORT_FAILED` |
| `database.backups` | 无 | `backups[]` | `PATH_INVALID` |
| `database.deleteBackup` | `path` | `message` | `PATH_INVALID`、`WRITE_DENIED` |
| `files.list` | `path` | `entries[]` | `PATH_INVALID`、`READ_DENIED` |

## 5. 响应结构与错误码

标准成功响应：

```json
{
  "id": "request-id",
  "result": {
    "state": {},
    "message": "操作成功"
  }
}
```

标准失败响应：

```json
{
  "id": "request-id",
  "error": {
    "code": -32000,
    "message": "80 端口已被占用",
    "data": {
      "errorCode": "PORT_OCCUPIED",
      "port": 80,
      "pid": 5328,
      "processName": "iisexpress.exe",
      "suggestions": ["停止占用程序", "修改 Nginx 端口"]
    }
  }
}
```

错误返回要求：

- `message` 必须给用户可读文本。
- `data.errorCode` 必须给机器可判定错误码。
- 涉及文件必须返回路径。
- 涉及端口必须返回端口和占用信息。
- 涉及回滚必须返回是否已回滚。

## 6. 权限鉴权逻辑

第一阶段无用户登录，但存在系统权限边界：

| 操作 | 权限检查 | 失败处理 |
| --- | --- | --- |
| 写 hosts | 检查 hosts 可写或 Agent 权限 | 返回 `HOSTS_PERMISSION_DENIED` |
| 写 runtime/data 目录 | 检查目录存在和可写 | 返回 `PATH_INVALID`/`WRITE_DENIED` |
| 打开 URL/文件/目录 | 限制为明确用户动作 | Tauri command 处理 |
| 执行 runtime | exe 路径必须来自数据库登记或可信导入 | 路径不可信则拒绝 |
| 数据库密码 | 不记录明文日志 | 日志脱敏 |

## 7. 业务服务层逻辑

### 7.1 服务启动事务

```text
validate service
  -> validate runtime files
  -> validate config
  -> check port owner
  -> spawn process
  -> persist pid
  -> wait port
  -> health check
  -> update service state
  -> write operation log
```

失败要求：

- 未 spawn 前失败：只写 failed 状态和日志。
- spawn 后失败：尝试停止该 PID/进程组。
- 端口被同一服务占用：识别为已运行或 degraded，不直接失败。

### 7.2 站点创建事务

```text
validate input
  -> prepare draft site
  -> ensure server runtime
  -> ensure php runtime when needed
  -> render temporary vhost
  -> validate nginx config
  -> backup vhost and hosts
  -> write vhost
  -> write hosts managed block
  -> reload nginx
  -> health check site
  -> insert site
  -> commit
```

回滚要求：

- 恢复 vhost。
- 恢复 hosts。
- 删除临时文件。
- 不插入或删除半成品 site。
- 尽量 reload 回旧配置。

### 7.3 PHP 切换事务

```text
load site
  -> load target php runtime
  -> ensure php-cgi running
  -> render new vhost
  -> validate config
  -> backup old vhost
  -> write new vhost
  -> reload nginx
  -> verify PHP_VERSION
  -> update sites.php_runtime_id
```

失败要求：

- 恢复旧 vhost。
- 保持旧 PHP 数据库值。
- 写明失败阶段。

### 7.4 runtime 安装事务

```text
prepare staging
  -> download/copy/extract
  -> verify files
  -> probe version
  -> allocate port/service id
  -> move to final dir
  -> insert runtime/software/service/config_files
  -> cleanup staging
```

失败要求：

- 清理 staging。
- 不写 installed=true。
- 如果数据库已写入，事务回滚。

## 8. 数据校验规则

| 数据 | 规则 |
| --- | --- |
| serviceId | 必须存在于 `service_instances` |
| runtimeType | 枚举：nginx/php/mysql/redis/apache/postgresql/minio |
| installPath | 绝对路径，目录存在 |
| domain | 非空、合法本地域名、不能重复 |
| port | 1-65535 |
| document_root | 绝对路径，存在或明确允许创建 |
| phpRuntimeId | 必须存在且 installed=true |
| fileId | 必须存在于 `config_files` 或可信 site config |
| settings URL | http/https URL |
| database name | 字母、数字、下划线，不能空 |

## 9. 日志与异常处理

### 9.1 操作日志字段

每个用户触发的修改操作必须写：

- action。
- target_id。
- success。
- error_code。
- message。
- created_at。

### 9.2 日志规则

- 密码脱敏。
- 路径可记录。
- stderr 可记录但注意截断。
- 大文本不要完整写入 SQLite。
- 成功和失败都要记录。

### 9.3 异常处理

- handler 不直接 `unwrap` 用户输入。
- 所有参数缺失返回明确错误。
- 文件写入失败携带 path。
- 进程失败携带 stderr/exitCode。
- 配置失败携带 config path 和校验输出。

## 10. 安全要求

- 不允许从前端传入任意 exe 路径直接执行，除非先登记为可信 runtime。
- 不允许字符串拼接 shell 命令。
- 不允许全局 `taskkill /IM nginx.exe /F`。
- 不允许修改 hosts 非 managed 区块。
- 不允许删除用户项目源码目录。
- 不允许日志记录数据库明文密码。

## 11. 性能要求

| 场景 | 要求 |
| --- | --- |
| `state.getFast` | 2 秒内缓存可返回 |
| `state.get` | 不阻塞 UI，重检测应有超时 |
| 服务启动 | 默认 10-15 秒超时 |
| 日志读取 | 尾部读取，最多 500 行 |
| 下载安装 | 后台执行，支持进度 |
| 健康监控 | 5 秒左右轮询，不高 CPU |

## 12. 后端任务包

### BE-PKG-01：协议、状态和错误基础

- [x] 任务编号：BE-001
  模块：JSON-RPC 响应与错误结构
  目标：统一成功/失败响应，支持结构化错误 data。
  接口：所有 handler。
  请求参数：按各接口定义。
  响应字段：`result` 或 `error.code/message/data`。
  业务流程：handler 捕获业务错误 -> 转换错误码 -> 补充 data -> 返回前端。
  异常处理：参数缺失、未知方法、业务失败、系统异常。
  数据表：`operation_logs` 可记录失败。
  依赖前端：`ErrorDetailPanel` 读取 message/data。
  验收标准：关键错误不只返回字符串；前端能识别 `errorCode`。
  测试要求：覆盖未知方法、缺少参数、结构化业务错误。

- [ ] 任务编号：BE-002
  模块：AppState 构建
  目标：稳定返回 services、sites、software、databases、config_files、settings、logs。
  接口：`state.get`、`state.getFast`。
  请求参数：无。
  响应字段：`AppState`。
  业务流程：读取数据库 -> 对服务执行轻量真实检测 -> 组装 state -> 更新缓存。
  异常处理：单个日志文件不存在不影响 state；数据库异常返回错误。
  数据表：`service_instances`、`sites`、`software`、`runtimes`、`config_files`、`settings`、`operation_logs`。
  依赖前端：首页、所有列表页面。
  验收标准：字段稳定；空表返回空数组；服务状态不是纯数据库缓存。
  测试要求：覆盖空库、seed 数据、服务运行/停止状态。

- [ ] 任务编号：BE-003
  模块：操作日志基础
  目标：所有修改操作写入统一操作日志。
  接口：所有 create/update/delete/start/stop/save/install。
  请求参数：action、target、success、message。
  响应字段：无直接响应，体现在日志页。
  业务流程：业务开始 -> 成功/失败 -> add_log -> 返回。
  异常处理：日志写入失败不得掩盖主操作成功，但必须 tracing 记录。
  数据表：`operation_logs`。
  依赖前端：日志页。
  验收标准：服务启动失败、站点创建失败、配置保存失败均可在操作日志看到。
  测试要求：覆盖成功日志、失败日志、日志列表排序。

### BE-PKG-02：服务生命周期

- [ ] 任务编号：BE-004
  模块：服务启动
  目标：实现服务启动真实闭环。
  接口：`service.start`。
  请求参数：`serviceId`。
  响应字段：`state`、`message`、`diagnostics?`。
  业务流程：校验 service -> 校验 installed/exe/cwd/config -> 检查端口 -> 启动进程 -> 记录 PID -> 检测端口 -> 健康检查 -> 更新状态。
  异常处理：服务不存在、未安装、exe 缺失、端口占用、配置错误、进程退出、健康检查失败。
  数据表：`service_instances`、`operation_logs`。
  依赖前端：`FE-002`。
  验收标准：不能只 spawn 后返回成功；端口/健康失败不能显示 running。
  测试要求：覆盖正常启动、未安装、端口占用、exe 缺失。

- [ ] 任务编号：BE-005
  模块：服务停止
  目标：安全停止 WinServer 管理的服务。
  接口：`service.stop`。
  请求参数：`serviceId`。
  响应字段：`state`、`message`。
  业务流程：读取 PID -> 验证 PID 归属 -> graceful stop -> 等待退出 -> 端口释放 -> 更新状态。
  异常处理：PID 不存在、停止超时、端口未释放。
  数据表：`service_instances`、`operation_logs`。
  依赖前端：`FE-002`。
  验收标准：不误杀外部同名进程；停止后端口释放。
  测试要求：覆盖 PID 已不存在、正常停止、外部同名进程保护。

- [x] 任务编号：BE-006
  模块：服务重启与套件操作
  目标：实现 `service.restart`、`suite.start`、`suite.stop`。
  接口：`service.restart`、`suite.start`、`suite.stop`。
  请求参数：`serviceId?`。
  响应字段：`state`、`summary`。
  业务流程：restart = stop + start；suite 按 auto=true 且 installed=true 遍历；单项错误进入 summary。
  异常处理：部分服务失败不阻断全部。
  数据表：`service_instances`、`operation_logs`。
  依赖前端：`FE-003`。
  验收标准：批量操作返回成功/失败/跳过汇总。
  测试要求：覆盖全成功、部分失败、全部跳过。

- [ ] 任务编号：BE-007
  模块：后台健康监控
  目标：检测运行中服务异常退出并更新状态。
  接口：内部任务，影响 `state.get`。
  请求参数：无。
  响应字段：无。
  业务流程：周期读取 running/degraded 服务 -> 检查 PID/端口 -> 状态变化写 DB 和日志。
  异常处理：检测失败记录 tracing，不崩溃。
  数据表：`service_instances`、`operation_logs`。
  依赖前端：首页状态刷新。
  验收标准：外部杀掉服务后，状态变为 failed/stopped。
  测试要求：集成测试模拟 PID 消失。

### BE-PKG-03：runtime 与软件安装

- [ ] 任务编号：BE-008
  模块：runtime 导入
  目标：导入本地 Nginx/PHP 并登记。
  接口：`runtime.import`。
  请求参数：`runtimeType`、`installPath`、`port?`。
  响应字段：`runtime`、`state`。
  业务流程：校验路径 -> 检查关键文件 -> 版本探测 -> 分配 serviceId/端口 -> 写 runtimes/software/service_instances/config_files。
  异常处理：路径不存在、关键文件缺失、版本探测失败、端口占用、重复导入。
  数据表：`runtimes`、`software`、`service_instances`、`config_files`、`operation_logs`。
  依赖前端：`FE-008`。
  验收标准：导入失败不写半成品；导入成功服务可启动。
  测试要求：覆盖 Nginx 成功、PHP 成功、缺文件、重复导入。

- [ ] 任务编号：BE-009
  模块：bundled 安装
  目标：从随包资源安装 runtime。
  接口：`software.installBundled`。
  请求参数：`softwareId`。
  响应字段：`runtime`、`state`、`message`。
  业务流程：定位 bundle -> 复制/解压到 staging -> 校验文件 -> 探测版本 -> 移动到正式目录 -> 登记数据库 -> 清理 staging。
  异常处理：bundle 不存在、解压失败、校验失败、正式目录不可写。
  数据表：`software`、`runtimes`、`service_instances`、`config_files`。
  依赖前端：`FE-009`。
  验收标准：失败清理 staging；成功后 installed=true。
  测试要求：覆盖 bundle 缺失、成功安装、重复安装。

- [ ] 任务编号：BE-010
  模块：在线下载与进度
  目标：实现下载、校验、解压、进度查询。
  接口：`software.downloadInstall`、`software.downloadProgress`。
  请求参数：`softwareId`。
  响应字段：progress、runtime、state。
  业务流程：创建 progress -> 下载 temp -> 校验大小/哈希 -> 解压 staging -> 探测 -> 登记 -> 清理。
  异常处理：网络失败、校验失败、解压失败、重复下载。
  数据表：`software`、`runtimes`、`service_instances`。
  依赖前端：`FE-010`。
  验收标准：进度真实；失败不进入正式目录。
  测试要求：可用 mock downloader 覆盖成功/失败/校验失败。

- [ ] 任务编号：BE-011
  模块：runtime 移除
  目标：安全移除未被使用的 runtime 管理记录。
  接口：`software.uninstall`。
  请求参数：`softwareId`。
  响应字段：`state`、`message`。
  业务流程：检查服务是否运行 -> 检查站点引用 -> 更新 installed=false -> 写日志。
  异常处理：服务运行中、站点引用、runtime 不存在。
  数据表：`software`、`runtimes`、`service_instances`、`sites`。
  依赖前端：`FE-011`。
  验收标准：被引用 runtime 不可移除；不删除源码目录。
  测试要求：覆盖运行中、被引用、成功移除。

### BE-PKG-04：站点与 PHP

- [ ] 任务编号：BE-012
  模块：创建站点
  目标：完成站点创建完整事务。
  接口：`site.create`。
  请求参数：`domain`、`port`、`path`、`server`、`phpRuntimeId?`。
  响应字段：`state`、`site?`。
  业务流程：校验输入 -> 确认 runtime -> 生成临时 vhost -> nginx -t -> 备份 -> 写 vhost/hosts -> reload -> health check -> 插入 sites。
  异常处理：域名重复、端口占用、目录无效、PHP 不完整、配置错误、hosts 无权限、健康检查失败。
  数据表：`sites`、`runtimes`、`service_instances`、`operation_logs`。
  依赖前端：`FE-013`。
  验收标准：失败回滚文件/hosts/数据库；成功后域名可访问。
  测试要求：覆盖成功、端口冲突、配置错误回滚、hosts 失败回滚。

- [ ] 任务编号：BE-013
  模块：编辑站点
  目标：安全修改站点域名、端口、目录、服务器。
  接口：`site.update`。
  请求参数：`siteId`、`domain`、`port`、`path`、`server`、`phpRuntimeId?`。
  响应字段：`state`、`site?`。
  业务流程：加载旧站点 -> 构建新配置 -> 验证 -> 备份旧配置/hosts -> 应用新配置 -> reload -> health check -> 更新 sites。
  异常处理：站点不存在、域名冲突、端口占用、reload 失败。
  数据表：`sites`、`operation_logs`。
  依赖前端：`FE-014`。
  验收标准：失败时旧站点仍可访问。
  测试要求：覆盖修改域名、端口、目录、失败回滚。

- [ ] 任务编号：BE-014
  模块：启用/停用站点
  目标：实现站点临时下线和恢复。
  接口：`site.enable`、`site.disable`。
  请求参数：`siteId`。
  响应字段：`state`。
  业务流程：disable 删除/禁用 vhost 和 hosts -> nginx -t -> reload -> status=disabled；enable 重新校验并生成配置 -> health check -> status=active。
  异常处理：站点不存在、目录缺失、端口占用、hosts 无权限、reload 失败。
  数据表：`sites`、`operation_logs`。
  依赖前端：`FE-015`。
  验收标准：失败不虚假改变 status；其他站点不受影响。
  测试要求：覆盖启用成功、停用成功、失败回滚。

- [ ] 任务编号：BE-015
  模块：删除站点
  目标：删除管理记录和配置，不删除源码。
  接口：`site.delete`。
  请求参数：`siteId`。
  响应字段：`state`。
  业务流程：加载站点 -> 备份 vhost/hosts -> 删除 vhost/hosts -> nginx -t -> reload -> 删除 sites 记录。
  异常处理：站点不存在、reload 失败、hosts 无权限。
  数据表：`sites`、`operation_logs`。
  依赖前端：`FE-016`。
  验收标准：源代码目录不删除；删除失败保留站点记录。
  测试要求：覆盖成功删除、reload 失败回滚、删除其中一个站点不影响其他站点。

- [ ] 任务编号：BE-016
  模块：PHP 切换
  目标：站点切换 PHP 后验证真实 PHP 版本。
  接口：`site.switchPhp`。
  请求参数：`siteId`、`phpRuntimeId`。
  响应字段：`state`。
  业务流程：校验站点和 PHP -> 确保 php-cgi running -> 生成 vhost -> nginx -t -> reload -> PHP_VERSION 检查 -> 更新 sites。
  异常处理：PHP 不完整、FastCGI 启动失败、配置错误、版本验证失败。
  数据表：`sites`、`runtimes`、`service_instances`。
  依赖前端：`FE-018`。
  验收标准：失败恢复旧 PHP；成功后实际版本一致。
  测试要求：覆盖成功、目标 PHP 缺失、reload 失败、版本不一致。

### BE-PKG-05：配置、hosts、端口、日志

- [ ] 任务编号：BE-017
  模块：hosts managed 区块
  目标：只维护 WinServer managed hosts 记录。
  接口：`hosts.sync`、`hosts.remove`，以及站点流程内部调用。
  请求参数：`domain`。
  响应字段：`message`。
  业务流程：读取 hosts -> 解析 managed 区块 -> 检查用户区冲突 -> 备份 -> 修改 -> 写回 -> 验证。
  异常处理：权限不足、域名冲突、文件不存在。
  数据表：无直接表，操作写 `operation_logs`。
  依赖前端：站点创建/删除/启停。
  验收标准：用户手写 hosts 不被修改。
  测试要求：覆盖无区块、有区块、用户区冲突、权限失败。

- [ ] 任务编号：BE-018
  模块：配置读取与保存
  目标：可信路径配置文件可读取、备份、保存和校验。
  接口：`config.get`、`config.save`、`site.config`。
  请求参数：`fileId`、`content` 或 `siteId`。
  响应字段：`content`、`path`、`state`、`backupPath?`。
  业务流程：校验 fileId/path -> 读取内容 -> 保存前备份 -> 写入 -> 关联服务配置检查 -> 返回。
  异常处理：文件不存在、无权限、配置校验失败、文件过大。
  数据表：`config_files`、`operation_logs`。
  依赖前端：`FE-019`。
  验收标准：保存前备份；坏配置不破坏运行服务。
  测试要求：覆盖读取、保存、无权限、配置错误。

- [x] 任务编号：BE-019
  模块：端口检查增强
  目标：端口检查返回 PID、进程名和占用归属。
  接口：`port.check`。
  请求参数：`port`、`address?`、`protocol?`。
  响应字段：`port`、`isOpen`、`pid`、`processName`、`ownerType?`、`ownerId?`。
  业务流程：校验端口 -> 查询监听 -> 获取 PID/进程名 -> 判断是否 WinServer 管理 -> 返回。
  异常处理：权限不足、系统命令失败。
  数据表：`service_instances` 可用于归属判断。
  依赖前端：`FE-020`。
  验收标准：不能只返回 boolean；占用信息可定位。
  测试要求：覆盖空闲、外部占用、自身占用。

- [ ] 任务编号：BE-020
  模块：日志读取与清空
  目标：多源日志尾部读取、搜索、清空。
  接口：`log.list`、`log.clear`。
  请求参数：`source?`、`search?`。
  响应字段：`source`、`path?`、`logs`。
  业务流程：解析 source -> operation 从 DB 读；文件日志按路径尾部读取 -> 搜索过滤 -> 限制 500 行。
  异常处理：文件不存在返回提示行或空状态；无权限返回错误。
  数据表：`operation_logs`。
  依赖前端：`FE-021`。
  验收标准：10MB+ 日志不卡顿；清空只作用当前来源。
  测试要求：覆盖大日志、搜索、文件不存在、清空。

### BE-PKG-06：设置、数据库、文件

- [ ] 任务编号：BE-021
  模块：设置读写
  目标：稳定读写 SystemSettings 和路径设置。
  接口：`settings.get`、`settings.update`、`settings.paths`。
  请求参数：设置 key/value。
  响应字段：`SystemSettings` 或 `state`。
  业务流程：校验字段 -> upsert settings -> 如路径设置则校验绝对路径/可写 -> 返回最新 state。
  异常处理：非法端口、非法 URL、相对路径、不可写目录。
  数据表：`settings`。
  依赖前端：`FE-022`、`FE-023`。
  验收标准：非法设置不能保存；重启后持久。
  测试要求：覆盖合法/非法 URL、端口、路径。

- [ ] 任务编号：BE-022
  模块：MySQL 数据库操作
  目标：已安装 MySQL 的创建库、改密、导入、导出。
  接口：`database.create`、`database.changePassword`、`database.rootPassword`、`database.export`、`database.import`。
  请求参数：按接口表。
  响应字段：`state`、`message`、`path?`。
  业务流程：确认 MySQL installed/running -> 读取 root 密码 -> 执行 mysql/mysqldump -> 成功后更新本地记录。
  异常处理：MySQL 未安装、未启动、root 密码错误、SQL 执行失败、导入文件不存在。
  数据表：`databases`、`settings`、`operation_logs`。
  依赖前端：`FE-024`。
  验收标准：SQL 执行失败不写错误记录；密码脱敏。
  测试要求：可用 fake mysql 命令覆盖成功/失败；真实集成测试单独执行。

- [x] 任务编号：BE-023
  模块：备份列表和删除
  目标：返回数据库备份列表并安全删除备份文件。
  接口：`database.backups`、`database.deleteBackup`。
  请求参数：`path?`。
  响应字段：`backups[]`、`message`。
  业务流程：读取备份目录 -> 过滤允许扩展名 -> 返回文件名/大小/时间；删除时校验路径在备份目录内。
  异常处理：目录不存在、路径逃逸、无权限。
  数据表：`settings`。
  依赖前端：`FE-023`、`FE-024`。
  验收标准：不能删除备份目录外文件。
  测试要求：覆盖空目录、正常列表、路径逃逸删除被拒。

- [x] 任务编号：BE-024
  模块：文件只读浏览
  目标：实现目录只读列表。
  接口：`files.list`。
  请求参数：`path`。
  响应字段：`entries[]`，包含 `name`、`isDir`、`size`、`modifiedAt`。
  业务流程：校验路径 -> 确认目录 -> read_dir -> 读取 metadata -> 排序返回。
  异常处理：路径不存在、不是目录、无权限。
  数据表：无。
  依赖前端：`FE-025`。
  验收标准：不提供删除/移动/写入能力。
  测试要求：覆盖有效目录、空目录、无效路径。

- [ ] 任务编号：BE-025
  模块：后端全量验收测试
  目标：建立后端 PRD 验收集成测试。
  接口：所有 P0 handler。
  请求参数：测试夹具。
  响应字段：测试断言。
  业务流程：准备临时数据目录 -> seed -> 导入 fake runtime -> 启动/建站/切 PHP/日志/设置 -> 清理。
  异常处理：测试失败输出日志和临时目录。
  数据表：全部核心表。
  依赖前端：无直接依赖。
  验收标准：`cargo test`、`cargo test --test api_integration`、`cargo test --test prd_acceptance` 通过。
  测试要求：本任务本身必须补齐测试覆盖矩阵。

## 13. 后端交付要求

每完成一个后端任务包必须：

- [ ] 运行 `cargo test`。
- [ ] 运行相关集成测试。
- [ ] 对照 `docs/验收文档.md` 填写验收结果。
- [ ] 更新对应 Plan 勾选状态。
- [ ] 若前端字段/接口需要变更，写入 `docs/COMMUNICATION.md`。
- [ ] 按 Git 规范提交，例如 `feat(backend): complete service lifecycle api`。

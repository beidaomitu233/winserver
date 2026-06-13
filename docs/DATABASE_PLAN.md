# WinServer 数据库计划

## 1. 文档目的

本文档面向数据库和后端执行模型，用于定义 WinServer 本地 SQLite 数据模型、字段契约、状态枚举、索引、约束、迁移和页面/接口依赖。执行模型必须先对照本文确认表结构，再进行后端仓储和前端字段接入。

## 2. 数据库原则

- SQLite 仅由 Agent 直接访问，前端不得直接读写 SQLite。
- 所有 UI 状态通过 `state.get` 或业务接口返回。
- 数据库保存期望状态和配置关系，不直接等同真实运行状态。
- 服务真实状态必须由 Agent 检测后更新或覆盖。
- 修改站点、hosts、配置、runtime 安装等操作必须使用事务或补偿回滚。
- 所有核心表必须有 `created_at`、`updated_at`，除日志/字典类可按需简化。
- 第一阶段默认不做物理级复杂多租户；本地单用户。

## 3. SQLite 全局设置

建议初始化时执行：

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA busy_timeout = 5000;
PRAGMA synchronous = NORMAL;
```

注意事项：

- 写入由 Agent 串行化或通过短事务控制。
- 不允许 UI 和 Agent 同时写库。
- 迁移必须可重复执行，使用 `CREATE TABLE IF NOT EXISTS` 和 `add_column_if_missing`。
- 任何字段重命名必须提供兼容迁移或数据复制。

## 4. 数据表清单

| 表名 | 用途 | 优先级 | 当前/规划 |
| --- | --- | --- | --- |
| `service_instances` | 服务实例、端口、进程、状态 | P0 | 当前已有，需增强 |
| `runtimes` | Nginx/PHP/MySQL 等 runtime 清单 | P0 | 当前已有，需增强 |
| `software` | 软件页卡片、安装方式、下载信息 | P0 | 当前已有，需增强 |
| `sites` | 本地站点基础信息 | P0 | 当前已有，需增强 |
| `site_domains` | 多域名绑定 | P1 | 规划新增 |
| `config_files` | 可编辑配置文件路径 | P0 | 当前已有，需增强 |
| `settings` | 系统设置 key/value | P0 | 当前已有 |
| `operation_logs` | 操作日志 | P0 | 当前已有，需增强 |
| `databases` | 本地数据库管理记录 | P1 | 当前已有，需增强 |
| `runtime_install_tasks` | runtime 下载/安装任务记录 | P1 | 规划新增 |
| `diagnostic_events` | 诊断事件和错误详情 | P1 | 规划新增 |
| `backups` | 配置/数据库备份索引 | P1 | 规划新增 |

## 5. 状态字段枚举

### 5.1 服务状态

适用字段：`service_instances.desired_state`、`service_instances.actual_state`

| 值 | 说明 |
| --- | --- |
| `installed` | 已安装/已登记，但未启动 |
| `starting` | 启动中 |
| `running` | 进程、端口、健康检查通过 |
| `degraded` | 部分异常 |
| `stopping` | 停止中 |
| `stopped` | 已停止 |
| `failed` | 启动或运行失败 |
| `unknown` | 未检测或未知 |

### 5.2 站点状态

适用字段：`sites.status`

| 值 | 说明 |
| --- | --- |
| `active` | 启用且配置生效 |
| `disabled` | 用户停用 |
| `error` | 站点配置或健康检查异常 |
| `deleting` | 删除事务中，临时状态 |

### 5.3 软件/runtime 状态

适用字段：`software.status`、`runtimes.status`

| 值 | 说明 |
| --- | --- |
| `available` | 可安装或可导入 |
| `installing` | 安装中 |
| `installed` | 已安装且完整 |
| `corrupted` | 已登记但关键文件缺失 |
| `removing` | 移除中 |
| `failed` | 安装或检测失败 |

### 5.4 安装任务状态

适用字段：`runtime_install_tasks.status`

| 值 | 说明 |
| --- | --- |
| `queued` | 已创建未开始 |
| `downloading` | 下载中 |
| `extracting` | 解压中 |
| `verifying` | 校验中 |
| `registering` | 登记中 |
| `done` | 完成 |
| `failed` | 失败 |
| `cancelled` | 已取消 |

## 6. 表结构设计

### 6.1 `service_instances`

**用途**

保存 WinServer 管理的服务实例配置、安装状态、期望状态、最近检测状态、PID、端口、配置文件和错误信息。

**被哪些页面使用**

- 首页服务卡片。
- 软件页安装/导入结果。
- 设置页配置文件和路径。
- 数据库页 MySQL 状态。

**被哪些接口使用**

- `state.get`
- `service.start`
- `service.stop`
- `service.restart`
- `service.toggleAuto`
- `suite.start`
- `suite.stop`
- `runtime.import`
- `software.uninstall`
- `port.check`

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | TEXT | 是 | 无 | 主键，如 `nginx`、`php83` |
| `name` | TEXT | 是 | 无 | 展示名 |
| `service_type` | TEXT | 是 | 无 | nginx/php/mysql/redis/apache |
| `runtime_id` | TEXT | 否 | NULL | 关联 `runtimes.id`，规划增强 |
| `process_name` | TEXT | 否 | NULL | 进程名 |
| `port` | INTEGER | 否 | NULL | 主端口 |
| `host` | TEXT | 否 | `127.0.0.1` | 监听地址，规划增强 |
| `exe` | TEXT | 否 | NULL | 可执行文件路径 |
| `args` | TEXT | 否 | NULL | JSON 字符串参数数组，禁止 shell 拼接 |
| `cwd` | TEXT | 否 | NULL | 工作目录 |
| `config_file` | TEXT | 否 | NULL | 主配置文件 |
| `auto` | INTEGER | 是 | 0 | 是否参与套件启动 |
| `installed` | INTEGER | 是 | 0 | 是否已安装/导入 |
| `desired_state` | TEXT | 是 | `installed` | 期望状态 |
| `actual_state` | TEXT | 是 | `unknown` | 最近真实状态 |
| `pid` | INTEGER | 否 | NULL | 主 PID |
| `job_id` | TEXT | 否 | NULL | Windows Job Object 逻辑 ID，规划增强 |
| `started_at` | TEXT | 否 | NULL | 最近启动时间 |
| `stopped_at` | TEXT | 否 | NULL | 最近停止时间 |
| `last_health_at` | TEXT | 否 | NULL | 最近健康检查时间 |
| `last_error_code` | TEXT | 否 | NULL | 最近错误码，规划增强 |
| `error_message` | TEXT | 否 | NULL | 最近错误消息 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |
| `updated_at` | TEXT | 是 | 当前时间 | 更新时间 |

**索引设计**

| 索引 | 字段 | 说明 |
| --- | --- | --- |
| `idx_service_instances_type` | `service_type` | 按类型筛选 |
| `idx_service_instances_runtime` | `runtime_id` | runtime 反查服务 |
| `idx_service_instances_state` | `actual_state` | 健康监控筛选 |
| `idx_service_instances_port` | `port` | 端口归属判断 |

**唯一约束**

- `id` 主键。
- 建议新增逻辑唯一：同一 `service_type + port + installed=1` 不应重复。

**外键或逻辑关联**

- `runtime_id` 逻辑关联 `runtimes.id`。
- 当前可先不强制外键，避免迁移破坏旧数据；仓储层必须校验。

**软删除策略**

- 不物理删除默认服务。
- runtime 移除时设置 `installed=0`，清空 `pid/error`，保留配置记录。

**迁移注意事项**

- 当前表已有 `error_message`，规划新增 `runtime_id`、`host`、`job_id`、`last_health_at`、`last_error_code` 时必须用 `add_column_if_missing`。

### 6.2 `runtimes`

**用途**

记录导入或安装的运行环境版本，如 Nginx、PHP、MySQL、Redis。

**被哪些页面使用**

- 软件页。
- 网站新建/编辑 PHP 下拉。
- 首页服务卡片间接使用。

**被哪些接口使用**

- `runtime.list`
- `runtime.import`
- `software.installBundled`
- `software.downloadInstall`
- `site.create`
- `site.switchPhp`

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | TEXT | 是 | 无 | 主键，如 `php-8.3.20-nts-x64` |
| `runtime_type` | TEXT | 是 | 无 | nginx/php/mysql/redis |
| `version` | TEXT | 是 | 无 | 探测版本 |
| `architecture` | TEXT | 否 | `x64` | x64/x86/arm64 |
| `install_path` | TEXT | 是 | 无 | 安装目录 |
| `entrypoint` | TEXT | 否 | NULL | 主入口 exe |
| `php_exe` | TEXT | 否 | NULL | PHP CLI，PHP 专用 |
| `php_cgi_exe` | TEXT | 否 | NULL | PHP FastCGI，PHP 专用 |
| `config_template` | TEXT | 否 | NULL | 配置模板 |
| `config_file` | TEXT | 否 | NULL | 主配置文件 |
| `fastcgi_port` | INTEGER | 否 | NULL | PHP FastCGI 端口 |
| `installed` | INTEGER | 是 | 0 | 是否可用 |
| `status` | TEXT | 是 | `available` | installed/corrupted/failed |
| `source` | TEXT | 否 | `local` | local/bundled/download |
| `checksum` | TEXT | 否 | NULL | 下载包或目录校验摘要 |
| `last_probe_at` | TEXT | 否 | NULL | 最近探测时间 |
| `last_error_code` | TEXT | 否 | NULL | 最近错误码 |
| `last_error_message` | TEXT | 否 | NULL | 最近错误 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |
| `updated_at` | TEXT | 是 | 当前时间 | 更新时间 |

**索引设计**

| 索引 | 字段 | 说明 |
| --- | --- | --- |
| `idx_runtimes_type` | `runtime_type` | 按类型查 |
| `idx_runtimes_status` | `status` | 查可用 runtime |
| `idx_runtimes_install_path` | `install_path` | 防重复导入 |

**唯一约束**

- `id` 主键。
- 建议 `runtime_type + install_path` 唯一。

**软删除策略**

- 不删除用户目录。
- 移除时 `installed=0`、`status=available` 或 `removed`（如新增枚举）。

**迁移注意事项**

- 当前表较简化，新增字段必须兼容旧行。
- 如果旧 `entrypoint` 已存 PHP CLI，则迁移时不要覆盖。

### 6.3 `software`

**用途**

软件页展示卡片和安装状态，关联 service/runtime。

**被哪些页面使用**

- 软件页。
- 首页未安装卡片跳转。

**被哪些接口使用**

- `state.get`
- `software.detectLocal`
- `software.install`
- `software.installBundled`
- `software.downloadInstall`
- `software.uninstall`

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | TEXT | 是 | 无 | 软件 ID |
| `name` | TEXT | 是 | 无 | 显示名 |
| `category` | TEXT | 是 | 无 | Web Servers/运行环境/数据库 |
| `service_id` | TEXT | 是 | 无 | 关联 service_instances.id |
| `runtime_type` | TEXT | 否 | NULL | nginx/php/mysql |
| `download_url` | TEXT | 否 | NULL | 下载地址 |
| `download_sha256` | TEXT | 否 | NULL | 哈希 |
| `install_dir` | TEXT | 否 | NULL | 默认安装目录 |
| `installed` | INTEGER | 是 | 0 | 是否已安装 |
| `installable` | INTEGER | 是 | 1 | 是否可安装 |
| `install_note` | TEXT | 否 | NULL | 安装说明 |
| `install_type` | TEXT | 是 | `local` | local/bundled/download |
| `install_path` | TEXT | 否 | NULL | 实际路径 |
| `status` | TEXT | 是 | `available` | available/installing/installed/failed |
| `has_local` | INTEGER | 是 | 0 | 检测到本机 |
| `has_bundled` | INTEGER | 是 | 0 | 随包可用 |
| `sort_order` | INTEGER | 是 | 0 | 软件页排序 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |
| `updated_at` | TEXT | 是 | 当前时间 | 更新时间 |

**索引设计**

| 索引 | 字段 | 说明 |
| --- | --- | --- |
| `idx_software_category` | `category` | 分类筛选 |
| `idx_software_service` | `service_id` | 服务关联 |
| `idx_software_status` | `status` | 状态筛选 |

**唯一约束**

- `id` 主键。

**软删除策略**

- 不删除软件卡片，只改变 `installed/status/install_path`。

### 6.4 `sites`

**用途**

保存本地站点基础信息、端口、目录、服务器类型、PHP runtime 和状态。

**被哪些页面使用**

- 网站页。
- 首页站点数量/快捷入口。
- PHP 切换。

**被哪些接口使用**

- `site.list`
- `site.create`
- `site.update`
- `site.enable`
- `site.disable`
- `site.delete`
- `site.switchPhp`
- `software.uninstall`

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | TEXT | 是 | 无 | 主键 |
| `name` | TEXT | 是 | `''` | 站点名称；本轮迁移已补字段，旧数据由 domain 回填 |
| `domain` | TEXT | 是 | 无 | 主域名 |
| `port` | INTEGER | 是 | 80 | HTTP 端口 |
| `document_root` | TEXT | 是 | 无 | 根目录 |
| `server_type` | TEXT | 是 | `nginx` | nginx/apache |
| `php_runtime_id` | TEXT | 否 | NULL | PHP runtime |
| `ssl` | INTEGER | 是 | 0 | 是否启用 SSL |
| `https_port` | INTEGER | 否 | NULL | HTTPS 端口，规划 |
| `status` | TEXT | 是 | `active` | active/disabled/error/deleting |
| `vhost_path` | TEXT | 否 | NULL | vhost 文件路径，规划增强 |
| `hosts_managed` | INTEGER | 是 | 1 | 是否写 hosts，规划增强 |
| `rewrite_template` | TEXT | 否 | NULL | 伪静态模板 |
| `last_health_at` | TEXT | 否 | NULL | 最近健康检查 |
| `last_error_code` | TEXT | 否 | NULL | 最近错误码 |
| `last_error_message` | TEXT | 否 | NULL | 最近错误 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |
| `updated_at` | TEXT | 是 | 当前时间 | 更新时间 |

**索引设计**

| 索引 | 字段 | 说明 |
| --- | --- | --- |
| `idx_sites_domain` | `domain` | 域名查询 |
| `idx_sites_name` | `name` | 站点名称展示/搜索 |
| `idx_sites_status` | `status` | 状态筛选 |
| `idx_sites_php_runtime` | `php_runtime_id` | runtime 引用检查 |
| `idx_sites_port` | `port` | 端口冲突检查 |

**唯一约束**

- `id` 主键。
- `domain` 必须唯一。
- 建议 `server_type + port + domain` 逻辑唯一。

**外键或逻辑关联**

- `php_runtime_id` 逻辑关联 `runtimes.id`。
- 当前可不强制外键，删除 runtime 前必须查 sites 引用。

**软删除策略**

- 第一阶段站点删除可物理删除记录，但必须先清理 vhost/hosts。
- 若后续需要审计，新增 `deleted_at` 和 `is_deleted`。

### 6.5 `site_domains`

**用途**

规划支持一个站点绑定多个域名。第一阶段可暂不实现，但 PRD 中 hosts 和 vhost 应预留多域名能力。

**被哪些页面使用**

- 网站新建/编辑高级配置。

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | TEXT | 是 | 无 | 主键 |
| `site_id` | TEXT | 是 | 无 | 关联 sites.id |
| `domain` | TEXT | 是 | 无 | 域名 |
| `is_primary` | INTEGER | 是 | 0 | 是否主域名 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |
| `updated_at` | TEXT | 是 | 当前时间 | 更新时间 |

**索引与约束**

- `idx_site_domains_site_id(site_id)`
- `UNIQUE(domain)`
- 每个 site 只能有一个 `is_primary=1`，SQLite 可用部分索引规划。

**迁移注意事项**

- 从 `sites.domain` 迁移到 `site_domains` 时，先保留旧字段兼容前端。

### 6.6 `config_files`

**用途**

记录可被 UI 打开的可信配置文件，防止前端任意路径写入。

**被哪些页面使用**

- 设置页配置文件列表。
- 首页服务卡片配置按钮。
- 网站页配置按钮。

**被哪些接口使用**

- `config.get`
- `config.save`
- `site.config`
- `state.get`

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | TEXT | 是 | 无 | 主键，如 `nginx.conf` |
| `label` | TEXT | 是 | 无 | 展示名 |
| `path` | TEXT | 是 | 无 | 绝对路径 |
| `owner_type` | TEXT | 否 | NULL | service/site/runtime/system |
| `owner_id` | TEXT | 否 | NULL | 关联对象 ID |
| `config_type` | TEXT | 否 | NULL | nginx/php/hosts/mysql |
| `editable` | INTEGER | 是 | 1 | 是否允许编辑 |
| `validate_command` | TEXT | 否 | NULL | 关联校验策略 key，不存任意命令 |
| `last_backup_path` | TEXT | 否 | NULL | 最近备份 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |
| `updated_at` | TEXT | 是 | 当前时间 | 更新时间 |

**索引设计**

- `idx_config_files_owner(owner_type, owner_id)`
- `idx_config_files_type(config_type)`
- `idx_config_files_path(path)`

**唯一约束**

- `id` 主键。
- `path` 建议唯一。

**安全要求**

- `path` 必须由后端生成或导入登记。
- `validate_command` 只能是策略 key，如 `nginx_test`，不能存完整 shell。

### 6.7 `settings`

**用途**

保存本地系统设置，key/value 形式。

**被哪些页面使用**

- 设置页。
- 数据库页 phpMyAdmin URL。
- Agent 数据目录和备份目录。

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `key` | TEXT | 是 | 无 | 主键 |
| `value` | TEXT | 否 | NULL | 字符串值 |
| `value_type` | TEXT | 否 | `string` | string/bool/int/json，规划 |
| `updated_at` | TEXT | 否 | 当前时间 | 更新时间，规划 |

**推荐 key**

| key | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `autostart` | bool | false | 开机启动 |
| `start_suite_on_launch` | bool | false | 启动应用自动启动套件 |
| `php_my_admin_url` | string | `http://127.0.0.1/phpmyadmin` | phpMyAdmin URL |
| `port` | int | 18113 | Agent/UI 默认端口或内部端口 |
| `data_dir` | string | `data` | 数据目录 |
| `runtime_dir` | string | `runtimes` | runtime 目录 |
| `backup_dir` | string | `data/backups` | 备份目录 |
| `mysql_root_password` | secret | 空 | MySQL root 密码，需考虑加密 |

**索引与约束**

- `key` 主键。

**安全注意**

- 密码类 key 不得写操作日志明文。
- 后续应接入系统凭据存储或加密。

### 6.8 `operation_logs`

**用途**

记录用户触发的关键操作、成功/失败和错误摘要，用于日志页和验收追踪。

**被哪些页面使用**

- 日志页。
- 首页最近错误。
- 诊断详情。

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | INTEGER | 是 | 自增 | 主键 |
| `action` | TEXT | 是 | 无 | 如 `service.start` |
| `target_type` | TEXT | 否 | NULL | service/site/runtime/config |
| `target_id` | TEXT | 否 | NULL | 目标 ID |
| `success` | INTEGER | 是 | 无 | 1/0 |
| `error_code` | TEXT | 否 | NULL | 业务错误码 |
| `message` | TEXT | 否 | NULL | 用户可读摘要 |
| `details_json` | TEXT | 否 | NULL | 截断后的结构化详情 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |

**索引设计**

- `idx_operation_logs_created(created_at)`
- `idx_operation_logs_action(action)`
- `idx_operation_logs_success(success)`
- `idx_operation_logs_target(target_type, target_id)`

**保留策略**

- 默认保留最近 5000 条或 30 天，后续可配置。
- 清空操作日志只清空本表，不删除服务文件日志。

**安全要求**

- `details_json` 必须脱敏密码、token、密钥。

### 6.9 `databases`

**用途**

保存 WinServer 管理的数据库记录，不等同 MySQL 真实库的唯一事实来源。同步时可从 MySQL 重新拉取。

**被哪些页面使用**

- 数据库页。

**被哪些接口使用**

- `database.create`
- `database.delete`
- `database.changePassword`
- `database.export`
- `database.import`
- `database.sync`

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `name` | TEXT | 是 | 无 | 主键，数据库名 |
| `user` | TEXT | 是 | 无 | 用户名 |
| `password` | TEXT | 是 | 无 | 当前已有明文，规划需加密或移除 |
| `engine` | TEXT | 是 | `mysql` | mysql/postgresql |
| `host` | TEXT | 否 | `127.0.0.1` | 主机 |
| `port` | INTEGER | 否 | NULL | 数据库端口 |
| `size` | TEXT | 否 | 空 | 展示大小 |
| `status` | TEXT | 是 | `active` | active/error/deleted |
| `last_sync_at` | TEXT | 否 | NULL | 最近同步时间 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |
| `updated_at` | TEXT | 是 | 当前时间 | 更新时间 |

**索引设计**

- `idx_databases_name(name)`
- `idx_databases_engine(engine)`
- `idx_databases_status(status)`

**唯一约束**

- `name` 当前为主键。
- 后续多引擎建议 `engine + name` 唯一。

**软删除策略**

- 删除按钮默认删除管理记录，可设置 `status=deleted` 或物理删除。
- 不默认执行 `DROP DATABASE`，除非 PRD 后续明确。

**安全注意**

- 当前 `password` 明文存储风险高；计划迁移为系统凭据或加密字段。

### 6.10 `runtime_install_tasks`

**用途**

规划记录 runtime 下载/安装任务，便于应用重启后查看失败状态和进度历史。

**被哪些页面使用**

- 软件页安装进度。

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | TEXT | 是 | 无 | 任务 ID |
| `software_id` | TEXT | 是 | 无 | 软件 ID |
| `runtime_type` | TEXT | 是 | 无 | 类型 |
| `source` | TEXT | 是 | 无 | bundled/download/local |
| `url` | TEXT | 否 | NULL | 下载地址 |
| `target_dir` | TEXT | 是 | 无 | 目标目录 |
| `staging_dir` | TEXT | 是 | 无 | 临时目录 |
| `status` | TEXT | 是 | `queued` | 状态 |
| `total_bytes` | INTEGER | 否 | 0 | 总大小 |
| `downloaded_bytes` | INTEGER | 否 | 0 | 已下载 |
| `phase` | TEXT | 否 | `queued` | 阶段 |
| `error_code` | TEXT | 否 | NULL | 错误码 |
| `error_message` | TEXT | 否 | NULL | 错误 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |
| `updated_at` | TEXT | 是 | 当前时间 | 更新时间 |
| `finished_at` | TEXT | 否 | NULL | 完成时间 |

**索引设计**

- `idx_runtime_install_tasks_software(software_id)`
- `idx_runtime_install_tasks_status(status)`
- `idx_runtime_install_tasks_created(created_at)`

### 6.11 `diagnostic_events`

**用途**

规划记录结构化诊断事件，增强首页“最近问题”和错误详情。

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | TEXT | 是 | 无 | 主键 |
| `source_type` | TEXT | 是 | 无 | service/site/runtime/config |
| `source_id` | TEXT | 是 | 无 | 来源 ID |
| `severity` | TEXT | 是 | `error` | info/warn/error |
| `error_code` | TEXT | 是 | 无 | 错误码 |
| `title` | TEXT | 是 | 无 | 标题 |
| `message` | TEXT | 是 | 无 | 文案 |
| `details_json` | TEXT | 否 | NULL | 详情 |
| `resolved` | INTEGER | 是 | 0 | 是否已解决 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |
| `resolved_at` | TEXT | 否 | NULL | 解决时间 |

**索引设计**

- `idx_diagnostic_events_source(source_type, source_id)`
- `idx_diagnostic_events_created(created_at)`
- `idx_diagnostic_events_resolved(resolved)`

### 6.12 `backups`

**用途**

规划记录配置文件、hosts、数据库导出的备份文件，便于恢复。

**字段设计**

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | TEXT | 是 | 无 | 主键 |
| `backup_type` | TEXT | 是 | 无 | config/hosts/database/settings |
| `source_path` | TEXT | 否 | NULL | 原文件 |
| `backup_path` | TEXT | 是 | 无 | 备份文件 |
| `target_type` | TEXT | 否 | NULL | service/site/database |
| `target_id` | TEXT | 否 | NULL | 目标 ID |
| `size_bytes` | INTEGER | 否 | 0 | 文件大小 |
| `created_by_action` | TEXT | 否 | NULL | 触发操作 |
| `created_at` | TEXT | 是 | 当前时间 | 创建时间 |

**索引设计**

- `idx_backups_type(backup_type)`
- `idx_backups_target(target_type, target_id)`
- `idx_backups_created(created_at)`

## 7. 字段命名和类型规则

- 表字段使用 snake_case。
- API 返回给前端可使用 camelCase，但必须在 `shared/src/types.rs` 和 `frontend/src/types/index.ts` 同步。
- 布尔值 SQLite 使用 INTEGER 0/1。
- 时间使用 ISO 8601 UTC 字符串。
- JSON 字段使用 TEXT 存 JSON 字符串，并在仓储层解析校验。
- 密码/密钥不得直接进入日志。

## 8. 创建时间/更新时间规则

| 操作 | 规则 |
| --- | --- |
| INSERT | 设置 `created_at` 和 `updated_at` 为当前 UTC 时间 |
| UPDATE | 更新 `updated_at` |
| 状态检测 | 只更新状态相关字段和 `updated_at/last_health_at` |
| 操作日志 | 只写 `created_at` |
| 安装任务 | 每阶段更新 `updated_at`，完成写 `finished_at` |

## 9. 数据迁移注意事项

1. 迁移必须幂等。
2. 新增字段使用 `add_column_if_missing`。
3. 新增非空字段必须有默认值，或分两步迁移。
4. 新增唯一约束前必须清理重复数据。
5. 当前 `databases.password` 明文风险需登记到 `COMMUNICATION.md`，后续迁移到加密/凭据存储。
6. `sites.name` 已在 2026-06-13 优化修复中通过 `add_column_if_missing` 幂等迁移补齐，并对旧数据执行 `name = domain` 回填；查询仍使用 `COALESCE(NULLIF(name,''), domain)` 兼容旧库。
7. `runtime_id` 相关字段应逐步补齐，先逻辑关联，稳定后再考虑外键。
8. 数据迁移失败必须阻止应用进入可操作状态，并显示错误。

## 10. 数据库任务包

- [ ] 任务编号：DB-001
  模块：迁移基础
  目标：统一 SQLite PRAGMA、幂等迁移和字段补齐工具。
  使用者：后端执行模型。
  使用位置：Agent 初始化。
  输入：当前数据库文件。
  输出：最新 schema。
  依赖接口：`state.get`。
  依赖表：全部核心表。
  异常情况：数据库不可写、迁移中断、旧字段缺失。
  验收标准：新库可创建，旧库可升级；重复启动不重复插入字段。
  测试要求：覆盖空库、旧库、重复迁移。

- [ ] 任务编号：DB-002
  模块：服务实例表增强
  目标：补齐 `service_instances` runtime、host、health、error 字段和索引。
  使用者：首页、服务管理、端口诊断。
  使用位置：服务启动和状态刷新。
  输入：服务配置和检测结果。
  输出：服务状态记录。
  依赖接口：`service.start`、`service.stop`、`state.get`、`port.check`。
  依赖表：`service_instances`、`runtimes`。
  异常情况：旧服务记录缺少 exe/cwd/config。
  验收标准：首页能准确展示 installed/state/pid/error。
  测试要求：覆盖状态更新、端口索引查询、runtime 关联。

- [ ] 任务编号：DB-003
  模块：runtime 表增强
  目标：补齐 PHP/Nginx runtime 所需字段。
  使用者：软件页、站点 PHP 选择。
  使用位置：导入、安装、切 PHP。
  输入：runtime 探测结果。
  输出：runtime 清单。
  依赖接口：`runtime.import`、`runtime.list`、`site.switchPhp`。
  依赖表：`runtimes`、`service_instances`。
  异常情况：重复路径、关键文件缺失。
  验收标准：PHP 下拉只展示 installed 且完整的 runtime。
  测试要求：覆盖 Nginx/PHP 插入、重复导入、状态 corrupted。

- [ ] 任务编号：DB-004
  模块：software 表增强
  目标：补齐软件页所需安装方式、下载校验和排序字段。
  使用者：软件页。
  使用位置：安装/导入/删除。
  输入：软件 seed 和安装结果。
  输出：软件卡片状态。
  依赖接口：`software.detectLocal`、`software.installBundled`、`software.downloadInstall`。
  依赖表：`software`、`runtimes`。
  异常情况：下载地址缺失、bundle 缺失。
  验收标准：软件页可按状态和分类筛选。
  测试要求：覆盖 seed、状态切换、安装失败记录。

- [ ] 任务编号：DB-005
  模块：sites 表增强
  目标：补齐站点名称、vhost、hosts、health、error 字段。
  使用者：网站页、首页站点统计。
  使用位置：创建、编辑、启停、删除、切 PHP。
  输入：站点表单。
  输出：站点记录。
  依赖接口：`site.create`、`site.update`、`site.enable`、`site.disable`、`site.delete`、`site.switchPhp`。
  依赖表：`sites`、`runtimes`。
  异常情况：域名重复、端口冲突、PHP 引用失效。
  验收标准：站点列表字段完整；runtime 被引用时不可删除。
  测试要求：覆盖唯一域名、状态切换、PHP 关联查询。

- [ ] 任务编号：DB-006
  模块：config_files 表增强
  目标：建立可信配置文件路径和 owner 关系。
  使用者：设置页、配置编辑器。
  使用位置：配置读取和保存。
  输入：runtime/站点生成的配置路径。
  输出：配置文件列表。
  依赖接口：`config.get`、`config.save`、`site.config`。
  依赖表：`config_files`、`sites`、`service_instances`。
  异常情况：路径不存在、owner 缺失。
  验收标准：前端只能编辑 config_files 或 site.config 返回的可信路径。
  测试要求：覆盖路径唯一、owner 查询、不可编辑配置。

- [ ] 任务编号：DB-007
  模块：operation_logs 增强
  目标：补齐 target、details_json 和日志索引。
  使用者：日志页、诊断。
  使用位置：所有修改接口。
  输入：操作结果。
  输出：操作日志。
  依赖接口：`log.list`、所有修改接口。
  依赖表：`operation_logs`。
  异常情况：详情过大、敏感字段。
  验收标准：日志可按时间倒序读取；密码脱敏。
  测试要求：覆盖成功/失败日志、搜索、脱敏。

- [ ] 任务编号：DB-008
  模块：settings 规范化
  目标：补齐设置 key、类型、默认值和校验。
  使用者：设置页、数据库页。
  使用位置：应用启动和设置保存。
  输入：设置 key/value。
  输出：SystemSettings。
  依赖接口：`settings.get`、`settings.update`、`settings.paths`。
  依赖表：`settings`。
  异常情况：非法端口、非法 URL、路径不可写。
  验收标准：新库默认设置完整；重启后设置不丢。
  测试要求：覆盖默认 seed、update、非法值拒绝。

- [ ] 任务编号：DB-009
  模块：databases 表安全改造
  目标：明确数据库管理记录和密码存储风险。
  使用者：数据库页。
  使用位置：创建库、改密、导入导出。
  输入：数据库名、用户、密码。
  输出：数据库记录。
  依赖接口：`database.create`、`database.sync`。
  依赖表：`databases`、`settings`。
  异常情况：MySQL 不可用、密码明文风险。
  验收标准：SQL 失败不写记录；日志不记录密码。
  测试要求：覆盖创建成功、创建失败、删除记录。

- [ ] 任务编号：DB-010
  模块：安装任务表
  目标：新增 `runtime_install_tasks` 支撑下载/安装进度持久化。
  使用者：软件页。
  使用位置：bundled 安装和在线下载。
  输入：softwareId、阶段、进度。
  输出：安装任务状态。
  依赖接口：`software.downloadInstall`、`software.downloadProgress`。
  依赖表：`runtime_install_tasks`、`software`。
  异常情况：应用重启、下载失败、取消。
  验收标准：安装失败可看到失败阶段；重启后不会误显示 installed。
  测试要求：覆盖阶段更新、失败、完成。

- [ ] 任务编号：DB-011
  模块：诊断与备份表
  目标：新增 `diagnostic_events` 和 `backups` 规划或实现。
  使用者：首页最近问题、配置恢复、日志页。
  使用位置：服务失败、配置保存、hosts 修改、数据库导出。
  输入：错误事件、备份路径。
  输出：诊断事件和备份索引。
  依赖接口：`config.save`、`service.start`、`site.create`、`database.export`。
  依赖表：`diagnostic_events`、`backups`。
  异常情况：备份文件不存在、事件重复。
  验收标准：配置保存失败能找到最近备份；首页可展示最近问题。
  测试要求：覆盖插入事件、标记 resolved、备份查询。

## 11. 数据库交付要求

每完成一个数据库任务包必须：

- [ ] 更新 schema 文档和迁移代码。
- [ ] 运行迁移单元测试。
- [ ] 运行后端依赖该表的集成测试。
- [ ] 检查前端类型是否需要同步。
- [ ] 若字段与 PRD/Plan 冲突，写入 `docs/COMMUNICATION.md`。
- [ ] 按 Git 规范提交，例如 `feat(database): add runtime install tasks table`。

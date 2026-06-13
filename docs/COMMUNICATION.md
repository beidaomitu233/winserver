# WinServer 协作沟通文件

## 1. 文件目的

本文档用于前端、后端、数据库、测试、审查模型之间登记需求不一致、接口冲突、字段缺失、验收疑问和优化建议。任何执行模型在实现过程中遇到文档冲突、接口缺失、字段不一致或范围不明确时，必须先登记到本文档，不得私自扩大实现范围或用临时逻辑绕过。

## 2. 状态定义

| 状态 | 含义 |
| --- | --- |
| 待确认 | 已发现问题，等待产品/架构确认 |
| 已确认 | 结论已明确，可进入执行 |
| 执行中 | 已有执行模型领取并处理中 |
| 已完成 | 问题已解决，文档/代码/测试已同步 |
| 驳回 | 不采纳该问题或优化建议，需写明原因 |

## 3. 问题类型

| 类型 | 说明 |
| --- | --- |
| 需求问题 | PRD 描述不完整、冲突或范围不清 |
| 接口问题 | 前后端接口名、参数、响应、错误码不一致 |
| 字段问题 | 前端类型、后端类型、数据库字段不一致 |
| 流程问题 | 功能链路缺少步骤、回滚或验收 |
| 测试问题 | 缺少测试数据、测试命令或验收标准不明确 |
| 安全问题 | 权限、路径、命令、密码、删除等安全边界 |
| 性能问题 | 启动、刷新、日志、下载等性能风险 |
| 优化建议 | 不阻塞执行，但建议改进 |

## 4. 协作规则

1. 每个任务包开始前，执行模型必须阅读相关 PRD、Plan、验收文档和本文档未关闭问题。
2. 遇到接口或字段冲突时，不得自行猜测；先登记问题，状态设为“待确认”。
3. 若问题已有“已确认”结论，执行模型必须按结论执行。
4. 若问题阻塞当前任务，但不阻塞其他任务，可先完成无冲突部分，并在提交说明中引用问题编号。
5. 若执行模型发现文档和代码现状不一致，必须同时写明“文档口径”和“当前代码口径”。
6. 每完成一个任务包，必须更新涉及的问题状态。
7. 状态改为“已完成”前，必须说明对应提交、测试或文档位置。
8. 不允许删除历史问题；若误报，状态设为“驳回”并说明原因。

## 5. 沟通登记表

| 序号 | 提出方 | 问题类型 | 功能问题描述 | 优化说明 | 涉及前端文件/模块 | 涉及后端文件/模块 | 涉及数据库表 | 状态 | 处理结论 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| COM-001 | 架构规划 | 需求问题 | 第一阶段 MySQL 是否必须达到完整安装、启动、建库、导入导出闭环尚未最终确认。 | 默认按 P1：未安装时页面明确不可用；已安装时支持基础命令闭环。 | `DatabasePage.vue`、`SoftwarePage.vue` | database handlers、`RuntimeManager`、`ProcessManager` | `databases`、`settings`、`service_instances` | 待确认 | 等产品确认 MySQL 是否升级为 P0。 |
| COM-002 | 架构规划 | 需求问题 | 是否随安装包内置 Nginx/PHP 二进制版本未确认。 | 默认保留 bundled 安装能力，同时支持本地导入和在线下载。 | `SoftwarePage.vue`、`RuntimeImportModal.vue` | `RuntimeManager` | `software`、`runtimes`、`runtime_install_tasks` | 待确认 | 需确认内置版本、目录和校验哈希。 |
| COM-003 | 架构规划 | 安全问题 | hosts 写入需要管理员权限，当前发布形态是否使用 Windows Service Agent 或即时提权未确认。 | 默认后端返回权限错误，前端展示授权/手动处理指引；发布期建议 Agent 服务化。 | `SitesPage.vue`、`SettingsPage.vue`、错误详情 | `HostsManager`、Agent 启动流程 | 无直接表，关联 `operation_logs` | 待确认 | 需确认提权方案和用户提示文案。 |
| COM-004 | 架构规划 | 字段问题 | `SiteInfo` 类型含 `name` 字段，但当前 `sites` 表可能没有 `name` 字段。 | 迁移补 `sites.name`，旧数据可由 `domain` 派生。 | `SitesPage.vue`、`SiteFormModal.vue` | `Database::list_sites`、`SiteManager` | `sites` | 已完成 | 2026-06-13 已通过幂等迁移补 `sites.name`、旧数据回填 `domain`，查询使用 `COALESCE` 兼容旧库；`cargo test` 通过。 |
| COM-005 | 架构规划 | 字段问题 | `PortCheckResult.is_open` 命名容易误解为“端口开放”，前端需要知道可用/占用和 owner。 | 后端增强返回 `available` 或 owner 信息；短期前端按文案解释 `is_open=true` 为端口已有监听。 | `PortCheckModal.vue` | `PortManager`、`port.check` | `service_instances` | 已完成 | 2026-06-13 已新增 `available`、`owner_type`、`owner_id` 字段，并在端口弹窗展示占用归属；`api_integration` 覆盖结构化返回。 |
| COM-006 | 架构规划 | 安全问题 | `databases.password` 当前可能明文存储。 | P1 前不得在日志显示密码；后续迁移到系统凭据或加密字段。 | `DatabasePage.vue`、`SettingsPage.vue` | database handlers | `databases`、`settings` | 待确认 | 2026-06-13 已完成短期输入校验和错误脱敏；本地加密/系统凭据方案仍待确认，`databases.password` 字段暂未迁移。 |
| COM-007 | 架构规划 | 流程问题 | 配置保存后是否必须自动 reload 服务，还是只保存并提示用户重启。 | P0：Nginx vhost/主配置保存必须 `nginx -t`；是否 reload 由保存入口决定，默认主配置保存后提示重启，站点配置保存后 reload。 | `ConfigEditorModal.vue` | `config.save`、`ConfigManager`、`ProcessManager` | `config_files`、`operation_logs` | 待确认 | 需产品确认配置保存后的自动应用策略。 |
| COM-008 | 架构规划 | 需求问题 | 首页是否展示 Apache/PostgreSQL/MinIO 等非 P0 服务。 | 默认首页只突出 Nginx/PHP/MySQL/Redis，其他服务进入软件页或折叠。 | `DashboardPage.vue`、`AppSidebar.vue` | `state.get` | `service_instances`、`software` | 已完成 | 2026-06-13 首页服务列表已按 Nginx/PHP/MySQL/Redis 过滤，非 P0 服务保留在软件页。 |
| COM-009 | 架构规划 | 测试问题 | 全量系统测试需要真实 Windows runtime，CI 环境可能缺少 Nginx/PHP/MySQL。 | 单元/集成测试用 fake runtime；系统测试在手动验收环境执行并记录。 | Playwright e2e | agent integration tests | 临时测试库 | 已完成 | 2026-06-13 `prd_acceptance` 已改为需 `WINSERVER_RUN_SYSTEM_ACCEPTANCE=1` 显式开启真实系统验收；默认自动测试不误触真实 runtime，`npm test`、`cargo test` 均已通过。 |
| COM-010 | 架构规划 | 接口问题 | `files.list` 当前 handler 存在，但 Tauri command 是否封装和前端是否接入需要确认。 | 前端文件页必须通过 Tauri command，不直接读本地文件。 | `FilesPage.vue` | `commands.rs`、`handler.rs` | 无 | 已完成 | 2026-06-13 已新增 `files_list` Tauri command 并接入 `FilesPage.vue` 只读目录浏览；E2E 文件页和后端 `files.list` 集成测试通过。 |

## 6. 决策记录

| 决策编号 | 日期 | 决策内容 | 影响范围 | 状态 |
| --- | --- | --- | --- | --- |
| DEC-001 | 2026-06-13 | 首页定位为“服务启动页”，移除非必要标签、长说明和拥挤指标，服务卡片放大。 | PRD、前端首页、验收 | 已确认 |
| DEC-002 | 2026-06-13 | 第一阶段无登录和云同步，本地单用户，系统权限通过 Agent/提权处理。 | 前后端、安全、验收 | 已确认 |
| DEC-003 | 2026-06-13 | 前端不得直接执行系统命令，所有系统能力通过 Tauri command/Agent。 | 前端、后端、安全 | 已确认 |
| DEC-004 | 2026-06-13 | 站点删除不删除用户项目源码目录，只删除 WinServer 管理记录、vhost 和 hosts 记录。 | 网站管理、验收 | 已确认 |
| DEC-005 | 2026-06-13 | 服务 running 状态必须来自进程、端口、配置和健康检查，不允许前端乐观设置。 | 首页、服务管理、测试 | 已确认 |

## 7. 执行模型填写模板

新增问题时复制以下模板追加到“沟通登记表”：

| 序号 | 提出方 | 问题类型 | 功能问题描述 | 优化说明 | 涉及前端文件/模块 | 涉及后端文件/模块 | 涉及数据库表 | 状态 | 处理结论 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| COM-XXX | 前端/后端/数据库/测试/审查 | 接口问题 | 描述冲突或缺失事实，写清当前任务编号。 | 提出建议，不直接扩大实现。 | 文件或模块 | 文件或模块 | 表名 | 待确认 | 等待确认。 |

## 8. 每个任务包完成后的沟通检查

- [ ] 是否遇到 PRD 与 Plan 不一致。
- [ ] 是否遇到前后端字段不一致。
- [ ] 是否遇到数据库字段缺失。
- [ ] 是否新增或修改接口契约。
- [ ] 是否新增错误码。
- [ ] 是否影响验收文档。
- [ ] 是否有未解决问题已登记到本文档。
- [ ] 是否需要把“待确认”改为“已确认/执行中/已完成/驳回”。

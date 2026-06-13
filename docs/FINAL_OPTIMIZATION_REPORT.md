# WinServer 最终全栈优化报告

生成日期：2026-06-13  
分支：`fix/fullstack-review-optimization`  
主修复提交：`6e9fe50 fix(fullstack): resolve review issues`

## 1. 修复范围

本轮围绕 `COMMUNICATION.md`、前后端计划、数据库计划和验收文档完成最终修复。仓库中未找到 `REVIEW_REPORT.md`，因此审查问题来源以 `docs/COMMUNICATION.md` 和各 Plan 未完成/冲突项为准。

重点修复范围：

- 前端：服务操作状态、端口检查、数据库弹窗、文件页、E2E 弹窗层级和测试 runner。
- 后端：JSON-RPC 结构化错误、批量服务摘要、端口归属、数据库 SQL 输入校验和脱敏、文件列表 command 桥接。
- 数据库：`sites.name` 幂等迁移、旧数据回填、索引同步。
- 测试：Rust 单元/集成/验收、Playwright 页面流、统一 `npm test` 汇总链路。
- 文档：前端/后端/数据库计划、沟通状态和本报告。

## 2. 已修复问题

| 问题编号 | 优先级 | 根因 | 修复方案 | 验收方式 |
| --- | --- | --- | --- | --- |
| COM-004 | P1 | `SiteInfo.name` 与 `sites` 表结构不一致 | 新增 `sites.name` 幂等迁移，旧数据按 `domain` 回填，查询使用兼容表达式 | `cargo test` |
| COM-005 | P1 | 端口检查仅返回 `is_open`，前端无法判断可用性和归属 | 增加 `available`、`owner_type`、`owner_id`，前端展示占用归属 | `api_integration`、前端构建 |
| COM-008 | P2 | 首页展示范围与阶段重点不一致 | 首页只突出 Nginx/PHP/MySQL/Redis，其他服务保留在软件页 | Playwright Dashboard 流程 |
| COM-009 | P1 | 系统验收依赖真实 runtime，自动测试环境不稳定 | `prd_acceptance` 默认跳过真实系统副作用，需 `WINSERVER_RUN_SYSTEM_ACCEPTANCE=1` 显式开启；`npm test` 汇总链路修复 | `cargo test`、`npm test` |
| COM-010 | P2 | `files.list` 有后端 handler，但缺 Tauri command 和前端接入 | 新增 `files_list` command，文件页改为只读目录浏览 | `api_integration`、Playwright Files 流程 |
| FE-002/FE-003 | P1 | 服务操作前端存在乐观状态和批量操作缺少摘要 | 移除前端乐观 running/failed 写入，增加单服务/套件 loading 与 summary 展示 | `npm test` |
| FE-020 | P2 | 端口弹窗结果语义不清 | 改用 `available` 和 owner 文案展示占用/可用/错误 | `npm run build` |
| FE-025/FE-026 | P2 | 文件页占位、E2E runner 路径和进程清理不稳定 | 文件页真实接入，只读浏览；修复 runner 根路径、固定 1420 端口、Windows 进程树清理 | `npm test` |
| BE-001 | P1 | handler 错误只返回字符串，前端无法识别错误码 | JSON-RPC error 增加 `data.errorCode/method/details` | `api_integration` |
| BE-006 | P1 | `suite.start/stop` 单项失败会影响汇总表达 | 返回 `summary`，区分 success/failed/skipped | `api_integration` |
| BE-019 | P2 | 端口检查缺少归属判断 | 结合监听信息和服务表识别 WinServer/外部占用 | `api_integration` |
| BE-023 | P1 | 备份删除存在路径逃逸风险 | 删除前校验文件必须位于备份目录内 | Rust 单元测试 |
| BE-024 | P2 | 文件浏览未通过 Tauri command 暴露 | 新增 `files_list` command 调用 `files.list` handler | `cargo test`、Playwright |

## 3. 未修复问题及原因

| 问题编号 | 状态 | 原因 | 建议 |
| --- | --- | --- | --- |
| COM-001 | 待确认 | MySQL 是否升级为 P0、以及第一阶段完整闭环范围仍需产品确认 | 保持当前基础闭环，最终验收前确认范围 |
| COM-002 | 待确认 | 内置 Nginx/PHP 二进制版本、目录和哈希需产品/发布确认 | 发布打包前补版本和校验哈希 |
| COM-003 | 待确认 | hosts 写入提权形态需产品/架构确认 | 建议 Windows Service Agent 或明确 UAC 提权方案 |
| COM-006 | 待确认 | 已完成日志/错误脱敏和 SQL 输入校验，但 `databases.password` 迁移到系统凭据或加密字段需架构决策 | 上线前完成凭据迁移方案 |
| COM-007 | 待确认 | 配置保存后自动 reload 还是提示重启仍需产品确认 | 产品确认后再调整 `config.save` 策略 |

## 4. 前端修改

- `frontend/src/stores/useServiceStore.ts`：移除服务状态乐观写入，增加单服务和套件 loading，批量操作展示摘要。
- `frontend/src/pages/DashboardPage.vue`：首页服务范围对齐阶段重点，按钮禁用态和操作中态对齐后端真实状态。
- `frontend/src/components/modals/PortCheckModal.vue`、`frontend/src/types/index.ts`：端口检查字段和展示同步。
- `frontend/src/pages/DatabasePage.vue`：phpMyAdmin 仅走 Tauri `open_url`，数据库弹窗使用 `Teleport` 避免顶栏遮挡。
- `frontend/src/pages/SitesPage.vue`、`frontend/src/pages/SoftwarePage.vue`：页面级弹窗挂载到 `body`，修复关闭按钮命中问题。
- `frontend/src/pages/FilesPage.vue`：从占位页改为只读目录浏览，接入 `files_list`。
- `frontend/src/styles/main.css`：提高全局 overlay 层级。
- `tests/e2e/*`：弹窗选择器改为只匹配可见弹窗，runner 修复根路径、固定端口和 Windows 进程清理。

## 5. 后端修改

- `shared/src/protocol.rs`：支持 `JsonRpcResponse::error_with_data`。
- `shared/src/types.rs`：`PortCheckResult` 增加 `available/owner_type/owner_id`。
- `agent/src/server/handler.rs`：结构化错误、错误分类、批量 summary、数据库输入校验/脱敏、备份路径逃逸防护、文件列表 handler 测试。
- `agent/src/managers/port_manager.rs`：端口检查返回可用性和占用归属基础字段。
- `frontend/src-tauri/src/commands.rs`、`frontend/src-tauri/src/lib.rs`：新增 `files_list` command，统一 handler 错误码透出。
- `frontend/src-tauri/src/app_state.rs`：移除无用状态字段。

## 6. 数据库修改

- `agent/src/database/schema.rs`：为 `sites` 增加 `name TEXT NOT NULL DEFAULT ''`，执行旧数据回填和 `idx_sites_name`。
- `agent/src/database/connection.rs`：站点查询和写入兼容 `name` 字段，旧导入路径同步。
- `docs/DATABASE_PLAN.md`：同步 `sites.name` 字段、索引和迁移说明。

回滚说明：

- `sites.name` 为非破坏性新增字段；如需回滚代码，可保留字段不影响旧查询。
- `idx_sites_name` 可通过 `DROP INDEX IF EXISTS idx_sites_name` 回滚。
- 旧数据回填仅从 `domain` 派生展示名，不删除原数据。

## 7. 安全优化

- 数据库名、用户名等 MySQL identifier 增加白名单校验。
- MySQL 命令错误输出对密码/root 密码脱敏。
- `database.import` 校验 SQL 文件存在。
- `database.deleteBackup` 限定只能删除备份目录下文件，阻断路径逃逸。
- `files.list` 只读浏览，不提供删除、移动、上传、新建等写操作。
- 前端不再用 `window.open` 绕过 Tauri 打开 phpMyAdmin。

## 8. 性能和稳定性优化

- 日志读取单元测试覆盖 tail/limit，避免大日志全量读取风险。
- 批量启动/停止改为 summary，单项失败不吞掉其他结果。
- `npm test` runner 修复 Windows dev server 残留进程问题。
- E2E 弹窗改为可见节点选择，避免隐藏全局弹窗污染测试。

## 9. 测试命令与结果

| 命令 | 结果 |
| --- | --- |
| `npm run build` | 通过，`vue-tsc -b && vite build` 成功 |
| `npx playwright test` | 通过，63 passed |
| `npm test` | 通过，构建、`cargo build`、API 集成、Rust lib、Playwright 和统一报告均成功 |
| `cargo test` | 通过，Rust 单元、API 集成、bundled runtime、PRD acceptance、doc tests 均成功 |

说明：

- `prd_acceptance` 默认不执行真实 Nginx/PHP/MySQL 系统副作用；真实系统验收需设置 `WINSERVER_RUN_SYSTEM_ACCEPTANCE=1`。
- Codex in-app Browser 自动化曾因本机沙箱凭据错误 `CreateProcessWithLogonW failed: 1326` 无法启动，已使用终端 Playwright 完成等价页面流程验证。

## 10. 文档更新

- `docs/FRONTEND_PLAN.md`：勾选 FE-002、FE-003、FE-020、FE-025、FE-026。
- `docs/BACKEND_PLAN.md`：勾选 BE-001、BE-006、BE-019、BE-023、BE-024。
- `docs/DATABASE_PLAN.md`：同步 `sites.name` 字段、索引和迁移说明。
- `docs/COMMUNICATION.md`：COM-004、COM-005、COM-008、COM-009、COM-010 改为已完成；COM-006 保留待确认并记录短期修复结论。

## 11. Git 状态

- 当前分支：`fix/fullstack-review-optimization`
- 主修复提交：`6e9fe50 fix(fullstack): resolve review issues`
- 文档补充提交：`docs(fullstack): update optimization report`
- 注意：本仓库当前包含大量既有未跟踪 Rust/Vue 工程文件和旧桌面文件删除。为了提交可构建状态，需要整体纳入当前工程结构；`.claude/`、`test-results/`、`playwright-report/` 等本地产物已忽略。

## 12. 是否建议最终验收

建议进入自动化验收和人工功能验收；不建议在 COM-001、COM-002、COM-003、COM-006、COM-007 完成产品/架构确认前直接宣布生产上线冻结。

最终验收建议：

- 先运行 `npm test` 和 `cargo test` 作为自动化门禁。
- 在真实 Windows runtime 环境设置 `WINSERVER_RUN_SYSTEM_ACCEPTANCE=1`，执行系统验收。
- 确认 MySQL 密码存储方案和 hosts 提权方案后再进入发布签核。

# WinServer 前端执行计划

## 1. 文档目的

本文档面向前端执行模型，用于把 `docs/prd.md` 中的页面级需求拆成可执行、可测试、可勾选的前端任务。执行模型必须按任务编号逐项完成，不得跳过接口接入、状态处理、异常提示和测试。

执行边界：

- 前端只负责 Vue/Tauri UI、组件、状态、表单、交互和接口调用。
- 前端不得直接执行系统命令，不得拼接 shell。
- 所有真实系统能力必须通过 Tauri command 调用后端。
- 当前计划不要求改后端代码；若发现接口缺失或字段不一致，写入 `docs/COMMUNICATION.md`。

## 2. 前端技术栈与推荐代码库

| 类别 | 技术/库 | 说明 |
| --- | --- | --- |
| 框架 | Vue 3 | 使用 Composition API |
| 语言 | TypeScript | 页面、store、接口模型必须有类型 |
| 构建 | Vite | 保持现有工程 |
| 桌面桥接 | Tauri 2 | 使用 `@tauri-apps/api/core` 的 `invoke` |
| 状态管理 | Pinia | 服务、站点、设置、数据库、UI 状态分 store |
| 样式 | CSS Variables + 现有 CSS | 不引入新 UI 框架，优先统一现有风格 |
| 图标 | 现有 `AppIcons.vue` 或 lucide | 按钮使用图标，避免文字过载 |
| 测试 | Vitest + Playwright | 组件逻辑和端到端页面流 |

推荐目录约定：

```text
frontend/src/
  pages/
    DashboardPage.vue
    SitesPage.vue
    SoftwarePage.vue
    DatabasePage.vue
    FilesPage.vue
    LogsPage.vue
    SettingsPage.vue
  components/
    layout/
    modals/
    service/
    site/
    software/
    forms/
  stores/
    useServiceStore.ts
    useSiteStore.ts
    useSettingsStore.ts
    useDatabaseStore.ts
    useUiStore.ts
  services/
    api.ts
    errors.ts
  types/
    index.ts
```

## 3. 路由结构

当前项目使用 `App.vue` 内部 `currentPage` 状态切换页面，第一阶段可以继续保持，不强制引入 Vue Router。

| 页面 ID | 页面文件 | 导航名称 | 是否 P0 |
| --- | --- | --- | --- |
| `dashboard` | `DashboardPage.vue` | 首页 | 是 |
| `sites` | `SitesPage.vue` | 网站 | 是 |
| `software` | `SoftwarePage.vue` | 软件 | 是 |
| `logs` | `LogsPage.vue` | 日志 | 是 |
| `settings` | `SettingsPage.vue` | 设置 | 是 |
| `database` | `DatabasePage.vue` | 数据库 | P1 |
| `files` | `FilesPage.vue` | 文件 | P2 |

路由/导航规则：

- 左侧导航只改变 `currentPage`。
- 页面切换不清空全局 state。
- 页面切换时可按需刷新当前页面数据，但不得阻塞导航。
- 首页、网站、软件、日志、设置必须可从任意页面进入。

## 4. 页面清单

| 页面 | 核心功能 | 主要接口 |
| --- | --- | --- |
| 首页 | 服务大卡片、启动/停止/重启、启动全部/停止全部、快捷入口、最近错误；MinIO 桶权限与连接信息复制 | `state.get`、`service.start`、`service.stop`、`service.restart`、`suite.start`、`suite.stop`、`minio.buckets.list` |
| 软件 | 软件列表、检测本机、导入、bundled 安装、下载、删除 | `software.detectLocal`、`runtime.import`、`software.installBundled`、`software.downloadInstall`、`software.uninstall` |
| 网站 | 站点列表、新建、编辑、启用、停用、删除、打开、配置、切 PHP | `site.create`、`site.update`、`site.enable`、`site.disable`、`site.delete`、`site.switchPhp` |
| 日志 | 结构化操作记录、搜索、结果筛选、自动刷新、清空 | `log.list`、`log.clear` |
| 设置 | 常规、网络、路径、安全、备份、配置文件 | `settings.get`、`settings.update`、`settings.paths`、`config.get`、`config.save` |
| 数据库 | MySQL 状态、phpMyAdmin、创建库、改密、导入导出 | `database.create`、`database.export`、`database.import`、`database.backups` |
| 文件 | 只读目录浏览 | `files.list`、`open_folder` |

## 5. 组件拆分

| 组件 | 位置建议 | 职责 |
| --- | --- | --- |
| `ServiceCard.vue` | `components/service/` | 首页服务大卡片，封装状态、主按钮、次按钮 |
| `SuiteActions.vue` | `components/service/` | 启动全部/停止全部/刷新 |
| `RuntimeCard.vue` | `components/software/` | 软件卡片，安装/导入/删除/进度 |
| `RuntimeImportModal.vue` | `components/modals/` | 导入运行环境表单 |
| `SiteFormModal.vue` | `components/modals/` | 新建/编辑站点表单 |
| `SiteTable.vue` | `components/site/` | 站点列表和行内操作 |
| `ConfigEditorModal.vue` | `components/modals/` | 配置读取、编辑、保存、备份提示 |
| `PortCheckModal.vue` | `components/modals/` | 端口检查输入和结果 |
| `ConfirmDialog.vue` | `components/modals/` | 删除、停用、切换 PHP 等二次确认 |
| `ErrorDetailPanel.vue` | `components/` | failed/degraded 错误详情和建议 |
| `EmptyState.vue` | `components/` | 通用空状态 |
| `LoadingBlock.vue` | `components/` | 页面/卡片 loading |

组件原则：

- 卡片、弹窗、表单组件不得直接持有复杂业务流程，只触发 store action。
- 页面负责组合组件和处理页面级弹窗显示。
- store action 负责调用接口、刷新 state、统一错误处理。

## 6. 状态管理方案

### 6.1 Store 划分

| Store | 状态 | Action |
| --- | --- | --- |
| `useServiceStore` | `services`、`configFiles`、`loadingByService`、`lastError` | `fetchState`、`startService`、`stopService`、`restartService`、`startSuite`、`stopSuite` |
| `useSiteStore` | `sites`、`selectedSite`、`formMode`、`loadingBySite` | `createSite`、`updateSite`、`deleteSite`、`enableSite`、`disableSite`、`switchPhp` |
| `useSettingsStore` | `settings`、`saving`、`pathErrors` | `fetchSettings`、`updateSettings`、`updatePaths` |
| `useDatabaseStore` | `databases`、`backups`、`mysqlStatus` | `createDatabase`、`exportDatabase`、`importDatabase`、`loadBackups` |
| `useUiStore` | `currentPage`、`toast`、`activeModal`、`globalLoading` | `showToast`、`openModal`、`closeModal`、`navigate` |

### 6.2 AppState 同步规则

- `get_state` 返回后，由统一方法拆分写入 service/site/software/database/settings store。
- 修改接口如果返回 `{ state }`，必须用返回的 state 覆盖本地 state。
- 修改接口只返回 message 时，必须再次调用 `get_state`。
- 前端不得自行把服务状态改成 running，只能显示后端 state。

## 7. 表单与校验规则

| 表单 | 字段 | 前端校验 |
| --- | --- | --- |
| 运行环境导入 | runtime 类型、安装目录、端口 | 类型必选、路径非空、端口 1-65535 |
| 新建站点 | 域名、根目录、端口、服务器、PHP | 域名格式、绝对路径、端口范围、PHP 站点必选 PHP |
| 编辑站点 | 同新建 | 展示变更摘要，保存前校验 |
| 端口检查 | 端口、地址、协议 | 端口范围，地址默认 127.0.0.1 |
| 配置保存 | 文件内容 | 内容大小限制，空内容需确认 |
| 设置 | port、phpMyAdmin URL、路径 | URL 合法、路径绝对、端口范围 |
| 数据库创建 | 数据库名、用户名、密码 | 名称格式、密码长度 |

前端校验只能做第一层保护；后端仍必须重复校验。

## 8. 接口调用规则

统一封装建议：

```text
services/api.ts
  invokeCommand(command, params)
  getState()
  normalizeError(error)
```

规则：

- 所有 Tauri `invoke` 必须通过统一 API 封装或 store action。
- 捕获错误后转成统一 UI 错误对象：`title`、`message`、`code`、`details`、`actions`。
- 修改接口必须有 loading 状态。
- 禁止在模板里直接写复杂 `invoke`。
- 禁止前端 mock 成功。

## 9. loading、empty、error 状态

| 状态 | 要求 |
| --- | --- |
| 页面 loading | 首次进入页面用骨架屏或紧凑 loading，不闪旧数据 |
| 卡片 loading | 只锁当前卡片按钮，不锁全页 |
| 表单 loading | 提交按钮 loading，取消按钮可按风险决定禁用 |
| empty | 每页都有明确空状态和下一步动作 |
| error | 错误必须显示原因和下一步动作，不只显示“失败” |
| retry | 可重试操作提供重试按钮 |

## 10. 权限控制逻辑

第一阶段无登录权限，但前端仍需做系统能力可见性控制：

| 条件 | UI 行为 |
| --- | --- |
| 服务未安装 | 启动按钮隐藏或替换为“安装/导入” |
| PHP 未安装 | 新建 PHP 站点时禁用提交，引导软件页 |
| MySQL 未安装 | 数据库操作禁用，引导软件页 |
| hosts 无权限 | 写 hosts 相关操作显示权限提示 |
| 配置文件不存在 | 编辑器显示不存在状态，按后端允许决定是否可创建 |
| 站点 disabled | 打开站点前提示先启用 |

## 11. 与后端/数据库依赖说明

- 前端依赖 `shared` 类型字段稳定；字段变更必须同步 `types/index.ts`。
- 前端依赖后端返回结构化错误；若当前后端只有字符串错误，前端先展示 message，并在 `COMMUNICATION.md` 登记增强需求。
- 前端依赖数据库表中的 `service_instances`、`sites`、`software`、`runtimes`、`config_files`、`settings`、`operation_logs` 间接通过 AppState 暴露。
- 前端不得直接读 SQLite。

## 12. 前端任务包

### FE-PKG-01：首页与全局状态

- [ ] 任务编号：FE-001
  模块：首页服务卡片
  目标：重构首页为简洁的大尺寸服务启动卡片。
  使用者：新手开发者、PHP 开发者。
  使用位置：首页首屏。
  输入：`AppState.services`。
  输出：服务卡片状态、主按钮、次按钮、错误摘要。
  实现说明：抽取 `ServiceCard` 组件；每张卡片最多展示服务名、状态、端口、主按钮、次按钮、错误摘要；移除非必要 tag 和长说明；根据 `installed/state` 映射按钮。
  依赖接口：`state.get`。
  依赖表：`service_instances`。
  状态处理：未安装、已停止、启动中、运行中、停止中、失败、降级、未知。
  异常情况：Agent 无响应时显示全局错误；服务数据为空时显示引导。
  验收标准：首页首屏以服务卡片为主；状态文案准确；未安装服务不显示可启动假按钮。
  测试要求：组件测试覆盖 8 种服务状态；Playwright 截图检查首页无拥挤说明文字。

- [x] 任务编号：FE-002
  模块：单服务操作
  目标：首页卡片支持启动、停止、重启。
  使用者：所有本地用户。
  使用位置：首页服务卡片。
  输入：serviceId、用户点击动作。
  输出：后端返回 state 写入 store。
  实现说明：在 `useServiceStore` 中实现 `startService`、`stopService`、`restartService`；按钮 loading 存入 `loadingByService`；操作完成后用返回 state 刷新。
  依赖接口：`service.start`、`service.stop`、`service.restart`。
  依赖表：`service_instances`、`operation_logs`。
  状态处理：操作中禁用当前按钮；失败保留原状态并显示错误详情入口。
  异常情况：端口占用、配置错误、服务未安装、启动超时。
  验收标准：前端不自行设置 running；失败 toast 包含原因；当前卡片 loading 不影响其他卡片。
  测试要求：mock invoke 成功/失败/慢响应三类用例；e2e 覆盖点击按钮状态变化。

- [x] 任务编号：FE-003
  模块：启动全部/停止全部
  目标：完成套件级批量启动和停止入口。
  使用者：希望一键启动环境的新手。
  使用位置：首页全局操作区。
  输入：用户点击启动全部/停止全部。
  输出：服务汇总结果和刷新后的 state。
  实现说明：新增 `SuiteActions`；调用 `suite_start`、`suite_stop`；展示成功、失败、跳过摘要；失败项可点击进入日志。
  依赖接口：`suite.start`、`suite.stop`。
  依赖表：`service_instances`、`operation_logs`。
  状态处理：全局按钮 loading；单项失败不阻止展示汇总。
  异常情况：部分服务未安装、端口冲突、Agent 失败。
  验收标准：批量操作有汇总，不只 toast 一句成功；失败服务卡片显示 failed。
  测试要求：覆盖全成功、部分失败、全部跳过。

- [ ] 任务编号：FE-004
  模块：全局 state 同步
  目标：统一 AppState 获取和分发。
  使用者：所有页面。
  使用位置：`App.vue`、stores。
  输入：`get_state` 返回值。
  输出：services、sites、software、databases、settings、configFiles。
  实现说明：建立统一 `applyAppState` 方法；避免各页面自行解析不完整 state；顶部刷新按钮调用所有必要 store。
  依赖接口：`state.get`、`state.getFast`。
  依赖表：所有通过 AppState 暴露的表。
  状态处理：首次加载、刷新中、刷新失败。
  异常情况：返回字段缺失时使用安全默认值并登记问题。
  验收标准：页面切换不丢 state；刷新按钮能刷新所有页面共享数据。
  测试要求：单元测试覆盖字段缺失、空数组、正常 state。

- [ ] 任务编号：FE-005
  模块：全局错误详情
  目标：提供统一错误展示和诊断入口。
  使用者：遇到服务失败的用户。
  使用位置：首页、软件页、网站页、设置页。
  输入：接口错误 message/code/details。
  输出：错误弹窗或面板。
  实现说明：新增 `ErrorDetailPanel`；错误包含标题、原因、影响对象、建议操作、日志入口；支持从服务卡片 failed 状态打开。
  依赖接口：所有修改接口。
  依赖表：`operation_logs`。
  状态处理：无错误时隐藏；错误详情可关闭但不清空服务状态。
  异常情况：后端只返回字符串时前端仍展示 message。
  验收标准：任何关键操作失败不只显示“失败”；用户能进入日志或配置。
  测试要求：覆盖字符串错误、结构化错误、无错误详情。

### FE-PKG-02：软件与运行环境

- [ ] 任务编号：FE-006
  模块：软件列表与筛选
  目标：完成软件页卡片列表、分类、状态筛选和搜索。
  使用者：安装和管理 runtime 的用户。
  使用位置：软件页。
  输入：`AppState.software`。
  输出：按分类和状态过滤后的软件卡片。
  实现说明：抽取 `RuntimeCard`；展示软件名、分类、安装状态、安装方式、本地可用、内置可用、路径和操作按钮。
  依赖接口：`state.get`。
  依赖表：`software`。
  状态处理：已安装、未安装、可导入、安装中、异常、空结果。
  异常情况：software 字段缺失时降级展示。
  验收标准：用户能快速识别 Nginx/PHP 是否已安装；未安装显示安装/导入入口。
  测试要求：组件测试覆盖筛选、搜索、空状态。

- [ ] 任务编号：FE-007
  模块：检测本机环境
  目标：完成本机 runtime 检测入口和结果展示。
  使用者：已有本地 Nginx/PHP 的用户。
  使用位置：软件页顶部。
  输入：用户点击检测。
  输出：检测结果映射到软件卡片或结果面板。
  实现说明：调用 `software_detect_local`；检测中显示 loading；检测到完整 runtime 时展示“导入本机”；不完整时展示缺失文件。
  依赖接口：`software.detectLocal`。
  依赖表：不直接依赖，结果可能影响 `software`。
  状态处理：检测中、检测成功、未发现、发现不完整、检测失败。
  异常情况：后端扫描失败、权限不足。
  验收标准：检测不会自动登记 runtime；必须用户确认导入。
  测试要求：覆盖发现完整、发现不完整、未发现、失败。

- [ ] 任务编号：FE-008
  模块：导入运行环境弹窗
  目标：完成 Nginx/PHP 本地导入表单。
  使用者：已有 runtime 目录的用户。
  使用位置：软件页导入按钮。
  输入：runtimeType、installPath、port。
  输出：调用导入接口并刷新 state。
  实现说明：新增 `RuntimeImportModal`；路径输入配选择目录按钮；端口输入仅对需要端口的 runtime 展示；提交前做基本校验。
  依赖接口：`runtime.import`。
  依赖表：`runtimes`、`software`、`service_instances`、`config_files`。
  状态处理：表单空值、路径为空、端口非法、提交中、成功、失败。
  异常情况：关键文件缺失、版本探测失败、端口占用、重复导入。
  验收标准：导入成功后首页卡片可启动；失败时弹窗保留用户输入。
  测试要求：覆盖空路径、非法端口、导入成功、导入失败。

- [ ] 任务编号：FE-009
  模块：bundled 安装
  目标：完成随包 runtime 安装交互。
  使用者：没有本机环境的新手。
  使用位置：软件卡片安装按钮。
  输入：softwareId。
  输出：安装进度和刷新 state。
  实现说明：卡片按钮调用 `software_install_bundled`；若后端提供阶段则展示阶段；安装期间禁用删除/导入。
  依赖接口：`software.installBundled`。
  依赖表：`software`、`runtimes`、`service_instances`。
  状态处理：准备、解压、校验、登记、完成、失败。
  异常情况：随包缺失、解压失败、校验失败。
  验收标准：安装成功后软件卡片显示已安装，首页对应服务可启动。
  测试要求：覆盖成功、失败、重复点击防抖。

- [ ] 任务编号：FE-010
  模块：在线下载安装
  目标：完成下载进度轮询和安装阶段展示。
  使用者：需要在线安装 runtime 的用户。
  使用位置：软件页。
  输入：softwareId。
  输出：下载百分比、阶段、最终 state。
  实现说明：点击后调用 `software_download_install`；并轮询 `software_download_progress`；阶段结束后停止轮询；失败展示重试。
  依赖接口：`software.downloadInstall`、`software.downloadProgress`。
  依赖表：`software`、`runtimes`。
  状态处理：idle、downloading、extracting、verifying、done、failed。
  异常情况：网络失败、校验失败、重复下载。
  验收标准：进度来自后端，不使用假进度；失败可重试。
  测试要求：使用 mock 进度覆盖阶段切换。

- [ ] 任务编号：FE-011
  模块：移除运行环境
  目标：完成 runtime 删除/移除交互。
  使用者：清理未使用 runtime 的用户。
  使用位置：软件卡片。
  输入：softwareId。
  输出：调用删除接口并刷新 state。
  实现说明：点击删除弹二次确认；提示不会删除网站源码；后端返回被引用错误时展示被哪些站点引用。
  依赖接口：`software.uninstall`。
  依赖表：`software`、`runtimes`、`service_instances`、`sites`。
  状态处理：确认、删除中、删除成功、删除失败。
  异常情况：服务运行中、runtime 被站点引用、权限不足。
  验收标准：被引用 runtime 删除失败时提示清楚；不误导用户以为源码被删除。
  测试要求：覆盖确认取消、成功、被引用失败。

### FE-PKG-03：网站与 PHP

- [ ] 任务编号：FE-012
  模块：站点列表
  目标：完成站点列表、筛选、行内操作布局。
  使用者：管理本地项目的用户。
  使用位置：网站页。
  输入：`AppState.sites`、PHP runtime 列表。
  输出：站点表格/卡片。
  实现说明：抽取 `SiteTable`；列包含域名、端口、根目录、服务器、PHP、状态、操作；支持搜索、状态筛选、PHP 筛选。
  依赖接口：`state.get`。
  依赖表：`sites`、`runtimes`。
  状态处理：active、disabled、error、空列表。
  异常情况：站点根目录不存在时行内提示。
  验收标准：用户能从列表执行打开、目录、编辑、配置、启停、删除。
  测试要求：组件测试覆盖筛选和不同状态行。

- [ ] 任务编号：FE-013
  模块：新建站点表单
  目标：完成新建站点弹窗和前端校验。
  使用者：创建本地域名站点的新手。
  使用位置：网站页新建按钮。
  输入：域名、根目录、端口、server、phpRuntimeId、站点类型。
  输出：调用 `site_create`。
  实现说明：新增/完善 `SiteFormModal`；字段按 PRD 分基础信息、运行环境、高级配置；PHP 站点必须选择 PHP；静态站点可不选。
  依赖接口：`site.create`。
  依赖表：`sites`、`runtimes`、`service_instances`。
  状态处理：空值、域名格式错、端口非法、无 PHP、提交中、成功、失败。
  异常情况：域名重复、目录不存在、端口占用、hosts 无权限、配置错误。
  验收标准：失败时弹窗不清空；成功后列表新增并高亮。
  测试要求：覆盖空表单、PHP 站点无 PHP、成功创建、后端失败。

- [ ] 任务编号：FE-014
  模块：编辑站点表单
  目标：完成站点编辑回显、变更摘要和保存。
  使用者：需要修改端口/目录/PHP 的用户。
  使用位置：网站列表编辑按钮。
  输入：siteId、修改后的字段。
  输出：调用 `site_update` 或 `site_switch_php`。
  实现说明：编辑模式复用 `SiteFormModal`；打开时预填数据；保存前显示域名/端口/目录/PHP 变更摘要；PHP 变更按后端能力走统一保存或单独切换。
  依赖接口：`site.update`、`site.switchPhp`。
  依赖表：`sites`、`runtimes`。
  状态处理：未修改、提交中、保存成功、保存失败。
  异常情况：新端口占用、目录无效、reload 失败。
  验收标准：保存失败旧数据仍显示；成功后列表刷新。
  测试要求：覆盖回显、无变化保存、字段修改、后端失败。

- [ ] 任务编号：FE-015
  模块：站点启用/停用
  目标：完成站点状态切换交互。
  使用者：临时下线本地站点的用户。
  使用位置：站点列表操作列。
  输入：siteId、目标动作。
  输出：调用 `site_enable` 或 `site_disable`。
  实现说明：active 行显示停用；disabled 行显示启用；操作前二次确认；按钮 loading；失败回弹。
  依赖接口：`site.enable`、`site.disable`。
  依赖表：`sites`、`operation_logs`。
  状态处理：active、disabled、切换中、失败。
  异常情况：目录缺失、端口占用、hosts 无权限、Nginx reload 失败。
  验收标准：状态不得乐观更新为成功；以后端返回 state 为准。
  测试要求：覆盖启用成功、停用成功、失败回弹。

- [ ] 任务编号：FE-016
  模块：删除站点
  目标：完成站点删除确认和结果刷新。
  使用者：清理不再需要站点的用户。
  使用位置：站点列表删除按钮。
  输入：siteId。
  输出：调用 `site_delete`。
  实现说明：二次确认弹窗明确“不删除源码目录”；可要求输入域名确认；成功后从列表移除。
  依赖接口：`site.delete`。
  依赖表：`sites`、`operation_logs`。
  状态处理：确认、删除中、成功、失败。
  异常情况：Nginx reload 失败、hosts 无权限、配置恢复失败。
  验收标准：删除失败时站点仍在列表；提示不删除源码。
  测试要求：覆盖取消、确认成功、失败。

- [ ] 任务编号：FE-017
  模块：打开站点/目录
  目标：完成打开 URL 和打开项目目录。
  使用者：日常调试项目的用户。
  使用位置：站点列表行。
  输入：domain、port、document_root。
  输出：调用系统打开 URL/目录。
  实现说明：打开 URL 时 80 端口可省略；disabled 站点点击时提示先启用；目录不存在时提示编辑站点。
  依赖接口：`open_url`、`open_folder`。
  依赖表：`sites`。
  状态处理：active 可打开、disabled 提示、目录缺失错误。
  异常情况：系统打开失败。
  验收标准：不使用 `window.open` 绕过 Tauri；错误有提示。
  测试要求：覆盖 URL 生成、80 端口省略、disabled 提示。

- [ ] 任务编号：FE-018
  模块：站点 PHP 切换
  目标：完成站点行内或弹窗内 PHP 切换体验。
  使用者：多 PHP 版本开发者。
  使用位置：站点列表 PHP 列或编辑弹窗。
  输入：siteId、phpRuntimeId。
  输出：调用 `site_switch_php`。
  实现说明：只展示 installed 且完整的 PHP runtime；切换前确认；切换中禁用当前站点操作；成功后显示新版本。
  依赖接口：`site.switchPhp`。
  依赖表：`sites`、`runtimes`、`service_instances`。
  状态处理：无 PHP 可选、切换中、成功、失败。
  异常情况：目标 PHP 不完整、FastCGI 启动失败、Nginx reload 失败。
  验收标准：失败时 UI 保持旧 PHP；不做乐观切换。
  测试要求：覆盖无 PHP、成功、失败保留旧值。

### FE-PKG-04：配置、日志、诊断、设置、数据库、文件

- [ ] 任务编号：FE-019
  模块：配置编辑器
  目标：完成配置读取、编辑、保存、备份提示。
  使用者：高级用户和排障用户。
  使用位置：首页服务卡片、网站页、设置页。
  输入：fileId 或 siteId。
  输出：配置内容、保存结果。
  实现说明：完善 `ConfigEditorModal`；展示文件名、路径、存在状态、内容编辑区；保存前提示风险；保存后展示成功/失败；失败可显示恢复备份入口。
  依赖接口：`config.get`、`config.save`、`site.config`。
  依赖表：`config_files`、`operation_logs`。
  状态处理：加载中、文件不存在、只读/无权限、保存中、保存成功、保存失败。
  异常情况：配置校验失败、路径不可信、文件过大。
  验收标准：保存失败显示 stderr 或错误原因；不吞掉错误。
  测试要求：覆盖读取成功、不存在、保存成功、保存失败。

- [x] 任务编号：FE-020
  模块：端口检查弹窗
  目标：完成端口输入、检查和结果展示。
  使用者：处理端口冲突的用户。
  使用位置：首页快捷入口、设置页网络区、错误详情。
  输入：port、address、protocol。
  输出：端口可用/占用结果。
  实现说明：完善 `PortCheckModal`；支持从错误详情带入端口；展示 PID、进程名、建议操作。
  依赖接口：`port.check`。
  依赖表：无，必要时写操作日志由后端决定。
  状态处理：空端口、非法端口、检查中、可用、占用、未知。
  异常情况：后端无法获取 PID、权限不足。
  验收标准：占用结果不只显示 true/false，必须有可理解说明。
  测试要求：覆盖空闲、占用、后端失败。

- [ ] 任务编号：FE-021
  模块：日志页
  目标：完成结构化操作日志查看、搜索、结果筛选、刷新和清空。
  使用者：排障用户和测试人员。
  使用位置：日志页。
  输入：固定 `source=operation`、search。
  输出：包含 action、target、success、errorCode、message、details、createdAt 的记录列表。
  实现说明：只保留操作日志；按成功/失败筛选；展示脱敏后的请求详情；自动刷新可开关；清空危险操作二次确认。
  依赖接口：`log.list`、`log.clear`。
  依赖表：`operation_logs`。
  状态处理：加载中、空日志、搜索无结果、清空中。
  异常情况：数据库读取失败、结构化详情缺失。
  验收标准：关键用户操作有记录；密码/token/配置正文不以明文展示；清空只作用 `operation_logs`。
  测试要求：覆盖结构化展示、搜索、结果筛选、脱敏、清空确认和空状态。

- [ ] 任务编号：FE-022
  模块：设置页常规/网络/路径
  目标：完成设置展示和保存。
  使用者：配置默认行为的用户。
  使用位置：设置页。
  输入：autostart、startSuiteOnLaunch、port、phpMyAdminUrl、dataDir、runtimeDir、backupDir。
  输出：调用设置保存接口。
  实现说明：按分区展示；端口、URL、路径做校验；保存按钮按分区 loading；路径可打开目录。
  依赖接口：`settings.get`、`settings.update`、`settings.paths`、`open_folder`。
  依赖表：`settings`。
  状态处理：加载中、字段错误、保存中、保存成功、保存失败。
  异常情况：非法 URL、相对路径、路径不可写。
  验收标准：非法设置不能提交；保存后刷新 state。
  测试要求：覆盖 URL/端口/路径校验和保存失败。

- [ ] 任务编号：FE-023
  模块：设置页安全/备份/配置列表
  目标：完成 hosts 权限检查、MySQL root 密码、备份列表和配置文件入口。
  使用者：排障和维护用户。
  使用位置：设置页。
  输入：root 密码、配置文件 ID、备份路径。
  输出：调用对应接口或打开配置编辑器。
  实现说明：配置文件列表展示名称、路径、存在状态、编辑按钮；备份删除二次确认；root 密码表单不回显明文。
  依赖接口：`db_root_password`、`db_backups`、`db_delete_backup`、`config.get`。
  依赖表：`settings`、`config_files`。
  状态处理：无配置文件、无备份、删除中、密码保存中。
  异常情况：备份文件不存在、密码错误、配置路径不存在。
  验收标准：密码不展示明文；配置编辑入口可打开真实配置。
  测试要求：覆盖空备份、删除确认、配置编辑入口。

- [ ] 任务编号：FE-024
  模块：数据库页
  目标：完成 MySQL 状态判断、phpMyAdmin、创建库、改密、导入导出、备份列表。
  使用者：需要本地数据库的开发者。
  使用位置：数据库页。
  输入：数据库名、用户、密码、导入路径、phpMyAdmin URL。
  输出：数据库列表和操作结果。
  实现说明：未安装 MySQL 时禁用操作并引导软件页；phpMyAdmin URL 来自设置；创建/改密/导入/导出使用弹窗或行内按钮。
  依赖接口：`database.create`、`database.changePassword`、`database.rootPassword`、`database.export`、`database.import`、`database.backups`、`open_url`。
  依赖表：`databases`、`settings`。
  状态处理：MySQL 未安装、未启动、运行中、操作中、失败。
  异常情况：root 密码错误、库名非法、导入文件不存在。
  验收标准：未安装时不显示可用假状态；SQL 失败不乐观更新列表。
  测试要求：覆盖未安装禁用、创建成功、创建失败、phpMyAdmin URL。

- [x] 任务编号：FE-025
  模块：文件页
  目标：完成只读目录浏览。
  使用者：快速查看项目目录的用户。
  使用位置：文件页。
  输入：path。
  输出：文件/目录列表。
  实现说明：路径输入 + 选择目录 + 刷新；列表展示名称、类型、大小、修改时间；只提供打开目录/复制路径，不提供删除移动重命名。
  依赖接口：`files.list`、`open_folder`。
  依赖表：无。
  状态处理：空路径、加载中、目录为空、路径不存在、无权限。
  异常情况：目录不存在、读取失败。
  验收标准：有效目录可浏览；高风险写操作不存在。
  测试要求：覆盖有效目录、空目录、错误路径。

- [x] 任务编号：FE-026
  模块：Playwright 全流程测试
  目标：建立前端端到端验收脚本，覆盖 PRD P0 链路。
  使用者：测试和审查模型。
  使用位置：`tests/e2e/`。
  输入：运行中的前端应用和可控后端。
  输出：测试报告。
  实现说明：覆盖首页加载、服务按钮、软件导入入口、站点表单、日志页、设置页；真实服务测试由系统测试补充，前端 e2e 可使用测试夹具。
  依赖接口：所有 P0 页面接口。
  依赖表：按接口间接依赖。
  状态处理：测试前清理状态或使用隔离数据。
  异常情况：后端未启动、接口失败。
  验收标准：`npx playwright test` 通过，失败截图可定位。
  测试要求：本任务本身必须提供 e2e 用例和运行说明。

## 13. 前端交付要求

每完成一个前端任务包必须：

- [ ] 运行前端类型检查或构建。
- [ ] 运行相关组件测试。
- [ ] 若涉及页面流程，运行 Playwright 对应用例。
- [ ] 对照本文件勾选任务。
- [ ] 若接口/字段不一致，写入 `docs/COMMUNICATION.md`。
- [ ] 按 Git 规范提交，例如 `feat(frontend): complete dashboard service cards`。

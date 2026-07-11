# WinServer GitFlow 分支与发布规范

> **强制规范**：自 2026-07-11 起，本仓库按本文档执行 GitFlow（适配版）。  
> 目标：**`main` 永远可发布**，日常集成不破坏线上可用性，缺陷可快速热修。

关联文档：

- `docs/RELEASE_CHECKLIST.md` — 发版 / 合并前检查清单
- `docs/PROJECT_DOCUMENT.md` — 项目总文档
- `README.md` — 开发与构建入口

---

## 1. 为什么用 GitFlow

WinServer 是本机「服务面板」类软件，可用性要求高于普通工具：

| 风险 | 无分支策略时的后果 | GitFlow 如何降低风险 |
| --- | --- | --- |
| 未完成功能合进默认分支 | 用户装到半成品，服务启停/配置失败 | 功能只在 `feature/*`，经 `develop` 验证再进 `main` |
| 紧急修复与大功能缠在一起 | 修一个 bug 被迫带上未测代码 | `hotfix/*` 只从 `main` 拉，修完回灌 `develop` |
| 版本边界不清 | 无法回滚、无法标版本 | `release/*` 定版、打 tag、走清单 |
| 直接推 `main` | 一次失误全员不可用 | `main` 只接受 release/hotfix 合并 |

---

## 2. 分支模型

```text
                    ┌──────── hotfix/x.y.z ────────┐
                    │                              │
                    ▼                              ▼
  ●─────────────── main (生产 / 始终可发布) ────────●──── tag vX.Y.Z
  │                  ▲                              │
  │                  │ release/x.y.z                │
  │                  │                              │
  ●─────────────── develop (集成 / 下一版) ─────────●
  ▲                  ▲
  │ feature/a        │ feature/b
  │                  │
```

### 2.1 长期分支

| 分支 | 角色 | 保护规则（强制） |
| --- | --- | --- |
| `main` | **生产线**。仅包含已发布或即将发布的稳定代码 | 禁止日常直接 push；禁止未完成功能；合并须走 PR + 清单 |
| `develop` | **集成线**。下一版本的集成分支，默认可编译、默认可测 | 禁止 force-push；功能合并用 PR；保持「随时可开 release」 |

### 2.2 短期分支

| 前缀 | 从哪拉 | 合回哪 | 用途 |
| --- | --- | --- | --- |
| `feature/<topic>` | `develop` | `develop` | 新功能、增强（例：`feature/minio-bucket-policy`） |
| `bugfix/<topic>` | `develop` | `develop` | 非紧急缺陷（例：`bugfix/redis-port-save`） |
| `release/x.y.z` | `develop` | `main` **且** 回灌 `develop` | 冻结发版：只修 bug、改版本号、写变更说明 |
| `hotfix/x.y.z` | `main` | `main` **且** 回灌 `develop` | 生产紧急修复 |

> 命名一律小写、用短横线：`feature/service-config-modal`，不要用空格或中文。

### 2.3 历史分支（过渡）

仓库中曾存在的非 GitFlow 分支（如 `codex/*`、`fix/*`）视为**遗留**：

1. 有价值的改动：整理后以 `feature/*` 或 `bugfix/*` 从最新 `develop` 重开，再 PR。
2. 已合并或废弃：本地删除，勿再继续在其上开发。
3. **从现在起禁止新建** `codex/*`、无类型前缀的随意分支名。

---

## 3. 日常工作流（必须遵守）

### 3.1 开发新功能

```powershell
git checkout develop
git pull   # 有远程时
git checkout -b feature/minio-bucket-policy

# ... 开发、本地验证 ...

git add <相关文件>
git commit -m "feat(minio): 内置桶权限与信息卡片"
# 推远程后开 PR：feature/* -> develop
```

规则：

- 一个 feature 分支只做一件事；大需求拆多个 feature。
- **不要**从 `main` 拉 feature（除非做 hotfix）。
- **不要**把 `main` 的未发布实验直接堆在默认分支上。

### 3.2 缺陷修复（非紧急）

```powershell
git checkout develop
git checkout -b bugfix/nginx-reload-fail
# 修复 + 测试
# PR: bugfix/* -> develop
```

### 3.3 发版（release）

当 `develop` 达到可发布质量：

```powershell
git checkout develop
git checkout -b release/0.2.0

# 只允许：
# - 修 release 阻塞 bug
# - 版本号 / 变更说明 / 文档
# - 跑完整 RELEASE_CHECKLIST

# 完成后：
# 1) PR: release/0.2.0 -> main
# 2) 在 main 打 tag: v0.2.0
# 3) 将 main 合并回 develop（或 PR back-merge）
# 4) 删除 release 分支
```

### 3.4 热修（hotfix）

生产已发布版本出现严重问题（无法启动、数据损坏、端口/服务错误等）：

```powershell
git checkout main
git checkout -b hotfix/0.2.1

# 最小改动修复 + 验证
# 1) PR: hotfix -> main，打 tag v0.2.1
# 2) 必须合并回 develop，避免「热修丢在生产、集成线仍坏」
# 3) 删除 hotfix 分支
```

---

## 4. 合并策略与 PR 要求

| 目标分支 | 允许来源 | 合并方式建议 |
| --- | --- | --- |
| `develop` | `feature/*`、`bugfix/*`、`release/*` 回灌、`hotfix/*` 回灌 | squash 或 merge commit 均可；保持历史可读 |
| `main` | **仅** `release/*`、`hotfix/*` | 推荐 merge commit（保留发版节点）；打 annotated tag |

每个 PR 至少包含：

1. **目的**：解决什么问题 / 交付什么能力  
2. **风险**：对服务启停、配置、数据目录的影响  
3. **验证**：按 `docs/RELEASE_CHECKLIST.md` 中对应级别勾选结果  
4. **回滚**：出问题如何恢复（回退 tag / 关功能 / 还原配置）

禁止：

- 未说明验证结果就合入 `develop` / `main`
- 在 `main` 上直接 `commit` 日常功能
- `git push --force` 到 `main` 或 `develop`
- 把密钥、本机绝对路径机密、大型二进制误提交进仓库

---

## 5. 提交信息约定（Conventional Commits）

```text
<type>(<scope>): <简述>

# type
feat     新功能
fix      缺陷
docs     文档
refactor 重构（行为不变）
test     测试
chore    构建/工具/杂项
perf     性能
revert   回滚

# scope 示例
minio, redis, nginx, mysql, site, desktop, agent, ui
```

示例：

- `feat(minio): 配置页内置桶权限与连接信息卡片`
- `fix(agent): 修复服务启动时端口占用误判`
- `docs(gitflow): 建立分支与发版规范`

---

## 6. 版本与 Tag

- 语义化版本：`MAJOR.MINOR.PATCH`（如 `0.2.0`）
- 生产 tag：`v0.2.0`（annotated）
- 热修：`v0.2.1`
- 预发布（可选）：`v0.3.0-rc.1`（打在 `release/*` 验证阶段）

```powershell
git checkout main
git tag -a v0.2.0 -m "Release v0.2.0"
# 有远程时: git push origin v0.2.0
```

---

## 7. 可用性门禁（Definition of Done）

合入 `develop` 前（feature/bugfix）：

- [ ] 相关单元测试 / 现有测试通过（至少改动模块）
- [ ] 本地可启动桌面端或明确说明仅后端改动
- [ ] 不破坏：服务列表、启停、配置读写的基本路径
- [ ] 无已知崩溃路径；错误对用户可读

合入 `main` 前（release/hotfix）：

- [ ] 完整执行 `docs/RELEASE_CHECKLIST.md`
- [ ] 版本号与 CHANGELOG/说明已更新（若项目已维护）
- [ ] 关键产物可安装/可运行（Tauri build 或约定发布方式）
- [ ] 回滚方案已写在 PR 描述中

**可用性红线**（触发即不可合 `main`）：

1. 应用无法启动或白屏  
2. 核心服务（Nginx / MySQL / Redis 等已安装项）无法启停  
3. 配置保存导致服务无法再次启动  
4. 数据目录被错误删除或覆盖  
5. 未处理的权限/路径问题导致静默失败  

---

## 8. 当前仓库落地状态

| 项 | 状态 |
| --- | --- |
| 长期分支 `main` | 已有 |
| 长期分支 `develop` | 已建立（自 GitFlow 启用日起） |
| 规范文档 | 本文 + `RELEASE_CHECKLIST.md` |
| PR 模板 | `.github/pull_request_template.md` |
| CI 门禁 | `.github/workflows/ci.yml`（推远程后生效） |
| 远程仓库 | 若尚未配置，需 `git remote add origin <url>` 后推送 `main`/`develop` |

### 8.1 启用后第一次同步（有远程时）

```powershell
git push -u origin main
git push -u origin develop
```

建议在托管平台设置：

- `main`、`develop`：禁止 force-push；要求 PR  
- `main`：仅维护者可合并；可选要求 CI 通过  

### 8.2 本地进行中的工作如何迁入 GitFlow

若当前在 `codex/*` 等旧分支且有未提交改动：

1. **不要**把未完成改动直接 commit 到 `main`  
2. 基于最新 `develop` 开 `feature/<topic>`  
3. 用 `git stash` / cherry-pick / 手工迁移把改动搬到新 feature 分支  
4. 自测后 PR → `develop`  

---

## 9. 快速命令速查

```powershell
# 开始功能
git checkout develop; git pull; git checkout -b feature/my-topic

# 开始热修
git checkout main; git pull; git checkout -b hotfix/0.2.1

# 开始发版
git checkout develop; git pull; git checkout -b release/0.2.0

# 查看分支是否偏离规范
git branch
```

---

## 10. 违规处理

| 行为 | 处理 |
| --- | --- |
| 直接向 `main` 提交功能 | 拒绝合入；改走 feature → develop → release |
| 跳过检查清单合入 `main` | 必须补验证或回滚 |
| 热修未回灌 `develop` | 立即 back-merge，避免再次发版丢修复 |
| 提交密钥或本机敏感信息 | 立刻轮换密钥 + 从历史清理（必要时重写历史并通知协作方） |

---

**维护人**：项目开发者 / Agent 助手  
**生效日期**：2026-07-11  
**修订**：规范变更须改本文并通知所有协作者；重大变更走 `docs` 类 PR 合入 `develop`。

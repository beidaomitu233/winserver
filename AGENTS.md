# Agent / 协作者须知

本仓库已启用 **GitFlow（适配版）**。自动化助手与人工开发均须遵守。

## 必读

1. `docs/GITFLOW.md` — 分支、合并、tag、红线  
2. `docs/RELEASE_CHECKLIST.md` — 合入 develop / main 前检查清单  
3. PR 模板：`.github/pull_request_template.md`

## 硬性规则

| 规则 | 说明 |
| --- | --- |
| 默认开发基线 | 从 **`develop`** 拉 `feature/*` 或 `bugfix/*` |
| 禁止 | 直接在 `main` 上堆日常功能；对 `main`/`develop` force-push |
| 合入 `main` | 仅 `release/*` 或 `hotfix/*`，并完成发版清单 |
| 热修 | 必须回灌 `develop` |
| 可用性 | 不得引入「无法启动 / 服务假运行 / 配置写坏无法重启 / 删用户数据」 |

## 提交

使用 Conventional Commits，例如：

```text
feat(minio): ...
fix(agent): ...
docs(gitflow): ...
```

## 当前有未完成工作在旧分支时

1. 不要把半成品直接推进 `main`  
2. 从最新 `develop` 建 `feature/<topic>` 再迁移改动  
3. 自测后 PR → `develop`  

## 文档索引

- 项目总览：`docs/PROJECT_DOCUMENT.md`
- 架构：`docs/architecture.md`
- 验收：`docs/验收文档.md`

## 摘要

<!-- 用 1～3 句话说明改了什么、为什么改 -->

## 类型

- [ ] `feature` → 合入 `develop`
- [ ] `bugfix` → 合入 `develop`
- [ ] `release` → 合入 `main`（并回灌 `develop`）
- [ ] `hotfix` → 合入 `main`（并回灌 `develop`）
- [ ] `docs` / `chore` / 其他

## 影响面（可用性）

| 项 | 说明 |
| --- | --- |
| 影响服务 | 无 / nginx / mysql / redis / minio / … |
| 配置文件 | 无 / 路径与行为 |
| 数据目录 | 无破坏 / 有迁移（说明） |
| 用户可见行为 | |

## 验证

请按 `docs/RELEASE_CHECKLIST.md` 勾选对应级别：

- [ ] 已完成 **集成级（A）**
- [ ] 若合入 `main`：已完成 **发版级（B）**
- [ ] 本地命令结果（粘贴关键通过项）：

```text
cargo test ...
vue-tsc ...
手动冒烟：...
```

## 回滚方案

<!-- 如何快速恢复可用性：回退 tag / 还原配置 / 关闭入口 等 -->

## 清单确认

- [ ] 分支命名符合 `docs/GITFLOW.md`（`feature/*` `bugfix/*` `release/*` `hotfix/*`）
- [ ] 未 force-push `main` / `develop`
- [ ] 未提交密钥、本机隐私路径、无关大文件
- [ ] PR 目标分支正确（功能→`develop`，发版/热修→`main`）

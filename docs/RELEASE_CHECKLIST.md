# WinServer 发布与合并检查清单

配合 `docs/GITFLOW.md` 使用。  
**合入 `main` 必须完整勾选「发版级」**；合入 `develop` 至少完成「集成级」。

---

## A. 集成级（feature / bugfix → develop）

### A1 代码与范围

- [ ] 改动范围与 PR 描述一致，无夹带无关大重构
- [ ] 无调试残留（`console.log` 刷屏、临时硬编码密码写入仓库等）
- [ ] 配置默认值安全（不把生产密钥写进默认模板）

### A2 构建与测试

- [ ] `cargo test -p winserver-agent`（或至少相关单测）通过
- [ ] 前端类型检查：`cd frontend; npx vue-tsc --noEmit` 通过（有 UI 改动时）
- [ ] 涉及服务/runtime 时：`node tests/verify-runtime-resources.js` 通过（若适用）

### A3 冒烟（本地）

- [ ] 应用能启动到首页
- [ ] 与本次改动相关的主路径手动点通一次
- [ ] 错误提示对用户可读（非裸 panic / 空白失败）

### A4 可用性影响声明

在 PR 中写明（可复制）：

```text
影响服务：无 / nginx / mysql / redis / minio / ...
配置文件：无 / 会改 xxx
数据目录：无破坏性操作 / 有迁移（说明）
回滚：还原 commit / 重装配置 / ...
```

---

## B. 发版级（release / hotfix → main）

在完成 **A 全部** 后继续：

### B1 版本

- [ ] 版本号已按约定更新（应用 / 包版本若存在）
- [ ] 准备 annotated tag：`vX.Y.Z`
- [ ] 变更说明已整理（用户可见的修复与功能）

### B2 核心可用性（红线）

- [ ] 冷启动：安装后或干净数据目录下可启动
- [ ] 首页服务卡片状态可信（运行中 / 已停止 / 未安装）
- [ ] 一键启动 / 单独启停至少覆盖：已安装的 Nginx、MySQL（或当前环境实际已装项）
- [ ] 站点：创建或打开已有站点流程无崩溃
- [ ] 数据库页（若启用）：列表加载正常
- [ ] 配置：Redis / MinIO / Nginx 等可视化配置可打开；保存后提示需重启的文案正确
- [ ] MinIO（若安装）：信息卡片与桶列表在服务运行且凭证正确时可加载

### B3 回归与资源

- [ ] 无误删用户 data 目录
- [ ] 端口冲突时有明确错误，而不是假「已启动」
- [ ] 日志页可打开；关键操作有日志痕迹（若适用）

### B4 发布动作

- [ ] PR 已审查（自审或他审）
- [ ] 合并 `main` 后打 tag
- [ ] **回灌 `develop`**（hotfix / release 均必须）
- [ ] 删除已完成的 `release/*` 或 `hotfix/*` 分支

### B5 发布后观察（30 分钟内）

- [ ] 本机再启动一次确认
- [ ] 若已分发给用户：收集「无法启动 / 服务起不来」反馈通道畅通

---

## C. 热修加急通道（仅严重故障）

仅当生产不可用时启用，仍需：

1. 从 `main` 拉 `hotfix/x.y.z`
2. **最小补丁**（禁止顺手做功能）
3. 完成 B2 红线相关项
4. 合 `main` → tag → **立刻合回 `develop`**

---

## D. 命令备忘

```powershell
# Agent 测试
cargo test -p winserver-agent --manifest-path Cargo.toml

# 前端类型
cd frontend; npx vue-tsc --noEmit

# 桌面开发运行
cd frontend; npm run tauri dev

# 桌面构建（发版前）
cd frontend; npm run tauri build
```

---

**原则**：宁可不发版，也不把红线问题送进 `main`。可用性优先于功能数量。

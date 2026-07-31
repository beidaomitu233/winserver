# WinServer 自动更新与版本发布

本文面向具有仓库写入权限的维护者，说明如何从 `develop` 发布一个能够被旧客户端自动发现、下载、验证并安装的新版本。

WinServer 使用 Tauri v2 updater，通过 GitHub Release 分发 Windows NSIS 安装包、签名文件和 `latest.json`。客户端内置公钥，只会安装由配套私钥签名的更新包。

关联规范：

- `docs/GITFLOW.md`：分支、合并、tag 与回灌规则
- `docs/RELEASE_CHECKLIST.md`：合入 `develop` / `main` 前检查清单
- `.github/workflows/release.yml`：tag 触发的实际发布流水线

## 1. 一次性准备

### 1.1 配置 GitHub Actions Secret

进入仓库：

`Settings → Secrets and variables → Actions → Repository secrets`

新增：

| 名称 | 值 | 必需 |
| --- | --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | `frontend/src-tauri/.tauri/winserver-updater.key` 的完整内容 | 是 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 创建签名密钥时设置的密码 | 有密码时 |

无密码密钥不需要创建第二个 Secret。不要使用 Actions Variables 保存私钥，因为 Variables 是明文配置。

### 1.2 备份签名密钥

本机密钥文件：

```text
frontend/src-tauri/.tauri/winserver-updater.key
frontend/src-tauri/.tauri/winserver-updater.key.pub
```

两个文件都被 `.gitignore` 排除。客户端需要的公钥内容已经写入 `frontend/src-tauri/tauri.conf.json`，私钥不能提交到 Git。

至少保留一份安全的离线私钥备份。私钥丢失后，已安装客户端无法验证使用另一把密钥签名的新版本；更换密钥必须先通过旧密钥签名的过渡版本下发新公钥。

### 1.3 验证发布地址

`frontend/src-tauri/tauri.conf.json` 的 updater endpoint 必须指向当前仓库：

```text
https://github.com/beidaomitu233/winserver/releases/latest/download/latest.json
```

若仓库迁移或改名，必须在发布前同步修改该地址并先发布过渡版本。

### 1.4 Bundled runtime 构建资产

`runtime/` 包含体积较大的第三方程序并被 Git 忽略。GitHub runner 在编译前会从 `v1.0.0` Release 下载：

```text
winserver-runtime-bundle-v1.zip
```

`.github/workflows/release.yml` 固定校验该文件的 SHA-256，再解压到 `runtime/`。构建资产至少包含：

```text
nginx-1.26.3.zip
mysql-8.0.12-winx64.zip
redis-7.2.4/
minio/
```

新增、删除或升级 runtime 时，维护者必须重新生成构建资产、上传一个新的不可变文件名（例如 `winserver-runtime-bundle-v2.zip`），并在同一个 PR 中更新下载 URL、SHA-256、`tauri.conf.json` 的 resources 列表和资源验证脚本。不要用 `--clobber` 静默替换已经被工作流引用的同名资产，否则旧提交将无法复现构建。

## 2. 版本号约定

使用语义化版本：

- `MAJOR`：不兼容变更，例如 `2.0.0`
- `MINOR`：向后兼容的新功能，例如 `1.2.0`
- `PATCH`：向后兼容的修复，例如 `1.1.1`

一个正式版本需要同步修改：

| 文件 | 字段 |
| --- | --- |
| `frontend/src-tauri/tauri.conf.json` | `version` |
| `frontend/package.json` | `version` |
| `frontend/package-lock.json` | 根 package 的 `version` |
| `frontend/src-tauri/Cargo.toml` | `[package].version` |
| `Cargo.lock` | `winserver-desktop` package version，由 Cargo 命令刷新 |

最终 tag 必须是 `v` 加应用版本，例如版本 `1.1.1` 对应 `v1.1.1`。工作流会检查 tag 与 `tauri.conf.json` 是否一致，不一致时拒绝发布。

## 3. 每次发布新版本的完整流程

以下示例发布 `1.1.1`。请替换命令中的版本号，不要直接复制旧版本。

### 第 1 步：将已完成的功能合入 develop

功能必须从最新 `develop` 开发，并通过 PR 合入：

```powershell
git checkout feature/your-topic
git status
git push -u origin feature/your-topic
```

创建 PR：

```text
feature/your-topic → develop
```

PR 至少写明改动目的、影响范围、验证结果和回滚方式。CI 通过且审查完成后再合并。

### 第 2 步：从最新 develop 创建 release 分支

```powershell
git checkout develop
git pull --ff-only origin develop
git checkout -b release/1.1.1
```

`release/*` 只允许版本、变更说明、发布文档和 release 阻塞修复，不再加入普通功能。

### 第 3 步：同步所有版本号

修改 `frontend/src-tauri/tauri.conf.json` 和 `frontend/src-tauri/Cargo.toml`：

```json
"version": "1.1.1"
```

```toml
[package]
version = "1.1.1"
```

让 npm 同步 `package.json` 与 `package-lock.json`：

```powershell
cd frontend
npm version 1.1.1 --no-git-tag-version
cd ..
```

刷新 Cargo lockfile，并确认所有版本源：

```powershell
cargo check --workspace
rg -n '1\.1\.1' frontend/package.json frontend/package-lock.json frontend/src-tauri/Cargo.toml frontend/src-tauri/tauri.conf.json Cargo.lock
```

### 第 4 步：整理用户可见的更新说明

PR 和 GitHub Release 的说明应包含：

- 新增功能
- 用户可感知的修复
- 配置、数据或兼容性变化
- 已知限制
- 必要的升级注意事项

不要只粘贴 commit 列表，也不要在公开说明中包含私钥、本机绝对路径、密码或内部日志。

### 第 5 步：执行集成与发布检查

在仓库根目录执行：

```powershell
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
node tests/verify-runtime-resources.js

cd frontend
npm ci
npm run build
cd ..
```

有 UI 改动时运行 Playwright：

```powershell
npx playwright test
```

然后逐项完成 `docs/RELEASE_CHECKLIST.md`，特别验证：

- 冷启动和已有数据目录启动
- Nginx、MySQL、PHP 等已安装服务的启停
- 站点、数据库、日志和配置基本路径
- 配置失败回滚、端口冲突和重复操作
- 安装包不会删除或覆盖用户数据目录

任何“无法启动、服务假运行、配置写坏无法重启、删除用户数据”的问题都必须阻止发布。

### 第 6 步：提交 release 分支

```powershell
git add frontend/package.json frontend/package-lock.json frontend/src-tauri/Cargo.toml frontend/src-tauri/tauri.conf.json Cargo.lock
git commit -m "chore(release): prepare v1.1.1"
git push -u origin release/1.1.1
```

若 release 分支还包含更新说明或阻塞修复，应显式把对应文件加入提交，不要使用未检查的 `git add -A`。

### 第 7 步：创建 release → main PR

创建 PR：

```text
release/1.1.1 → main
```

建议使用 merge commit 保留发布节点。PR 描述中必须包含：

- 发布版本与主要变化
- `RELEASE_CHECKLIST` 结果
- 安装包/冒烟验证结果
- 用户数据影响
- 回滚方案：撤下 Release、回退到前一个 tag，必要时发布 hotfix

CI 和审查通过后才能合并到 `main`。

### 第 8 步：在 main 创建并推送 tag

合并后更新本地 `main`：

```powershell
git checkout main
git pull --ff-only origin main
git tag -a v1.1.1 -m "WinServer v1.1.1"
git push origin v1.1.1
```

不要在 release PR 合并前推 tag，不要把 tag 打在 release 分支的旧提交上。tag push 会触发 `.github/workflows/release.yml`。

### 第 9 步：观察 GitHub Actions

打开：

`GitHub → Actions → release → 对应 v1.1.1 任务`

工作流会自动：

1. 校验 tag 与应用版本一致；
2. 安装前端依赖；
3. 执行 Rust 测试、Clippy 和 runtime 资源检查；
4. 编译 Windows NSIS 安装包；
5. 使用 `TAURI_SIGNING_PRIVATE_KEY` 生成 updater 签名；
6. 创建 GitHub Release；
7. 上传安装包、签名和 `latest.json`；
8. 将 manifest 中的 GitHub Asset API URL 规范化为可匿名下载的 `releases/download/<tag>/<installer>` URL，并执行 HTTP 校验。

任务失败时不要手工上传一个未签名安装包冒充自动更新产物。修复原因后删除错误 tag/草稿 Release，再在正确提交上重新创建 tag，或者按版本策略发布新的 patch 版本。

### 第 10 步：检查 Release 产物

打开对应 GitHub Release，确认至少包含：

- `WinServer_1.1.1_x64-setup.exe`
- updater 签名文件（通常以 `.sig` 结尾）
- `latest.json`

打开 `latest.json` 检查：

- `version` 为 `1.1.1`
- Windows x64 下载 URL 指向本次 Release
- `signature` 非空
- 每个平台的 `url` 使用 `https://github.com/<owner>/<repo>/releases/download/...`，不能是返回 JSON 元数据的 `https://api.github.com/repos/.../assets/...`
- 更新说明与发布时间正确

### 第 11 步：用旧版本客户端验证更新

不要在测试前直接卸载旧版。推荐保留一台或一个 Windows 测试环境安装 `1.1.0`，再发布 `1.1.1`：

1. 启动 `1.1.0`，进入“设置 → 版本”；
2. 点击“检查更新”；
3. 预期显示“发现新版本 v1.1.1”和更新说明；
4. 点击“立即更新到 v1.1.1”；
5. 观察下载进度、签名验证、被动安装和应用重启；
6. 重启后版本显示 `v1.1.1`；
7. 检查原有站点、数据库记录、设置、日志及 runtime 仍存在；
8. 再次检查更新，预期显示“当前已是最新版本”。

还需覆盖：断网、错误签名、重复点击、下载中退出、安装失败和 GitHub 暂时不可用等场景。异常情况下旧版本应继续可用，不能删除用户数据。

### 第 12 步：回灌 develop

release 合入 `main` 后必须把发布提交回灌 `develop`：

```powershell
git checkout develop
git pull --ff-only origin develop
git merge --no-ff main -m "chore(release): back-merge v1.1.1 into develop"
git push origin develop
```

也可以通过 `main → develop` PR 完成回灌。确认 `main`、`develop` 都包含发布提交后，再删除 `release/1.1.1` 分支。

## 4. 客户端如何发现更新

客户端进入“设置 → 版本”后请求：

```text
https://github.com/beidaomitu233/winserver/releases/latest/download/latest.json
```

处理顺序：

```text
读取当前 Tauri 应用版本
  → 请求 latest.json
  → 比较语义化版本
  → 展示版本和更新说明
  → 用户确认下载
  → 校验 updater 签名
  → NSIS 被动覆盖安装
  → 自动重新启动
```

GitHub 的“Latest release”必须是要下发的正式版本。预发布版本不要覆盖正式 latest 指向，除非客户端和工作流明确支持预发布通道。

## 5. 发布失败与回滚

| 场景 | 处理方式 |
| --- | --- |
| tag 与版本不一致 | 删除错误 tag，在版本匹配的 `main` 提交上重建 |
| Actions 找不到私钥 | 检查 Repository Secret 名称是否严格为 `TAURI_SIGNING_PRIVATE_KEY` |
| 私钥密码错误 | 更新 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，不要修改客户端公钥 |
| Release 缺少 `latest.json` | 检查 tauri-action 和 `createUpdaterArtifacts`，不要宣告发布完成 |
| CI 提示 `runtime/... doesn't exist` | 检查 runtime 构建资产 URL、SHA-256、压缩包目录结构和 Restore 步骤 |
| 客户端提示签名错误 | 立即停止分发，核对签名私钥是否与内置公钥配对 |
| 新版本存在严重缺陷 | 撤下 Latest 标记并从 `main` 创建 `hotfix/*`，发布更高 patch 版本 |
| 更新安装失败 | 保留旧版本与用户数据，收集 Actions、客户端和 NSIS 日志进一步验证 |

已发布版本不要复用同一个 tag 或覆盖同名 Release 产物。修复后应递增 patch 版本，使客户端和缓存能够明确区分新产物。

## 6. 发布完成标准

- [ ] 所有版本源和 tag 完全一致
- [ ] release workflow 全部通过
- [ ] Release 包含安装包、签名和 `latest.json`
- [ ] 旧版本能够发现、下载、安装并重启到新版本
- [ ] 再次检查更新显示最新版本
- [ ] 用户站点、数据库记录、设置、日志和 runtime 未丢失
- [ ] `main` 已回灌 `develop`
- [ ] 发布说明和回滚方式可供其他维护者复现


# WinServer 自动更新发布说明

WinServer 使用 Tauri v2 updater，并通过 GitHub Release 分发 Windows NSIS 安装包、签名文件和 `latest.json`。客户端仅安装通过内置公钥验证的更新包。

## 一次性配置

1. 妥善备份本机忽略文件 `frontend/src-tauri/.tauri/winserver-updater.key`。私钥丢失后，已安装的客户端将无法接收使用新密钥签名的更新。
2. 在 GitHub 仓库的 Actions secrets 中新增：
   - `TAURI_SIGNING_PRIVATE_KEY`：私钥文件的完整内容；
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：私钥密码。当前开发密钥未设置密码时可以留空，正式发布前建议换成带密码的生产密钥，并同步替换 `tauri.conf.json` 中的公钥。
3. 确认仓库 Release 地址与 `frontend/src-tauri/tauri.conf.json` 中的 updater endpoint 一致。

私钥和公钥文件都已由 `.gitignore` 排除；仓库中只保留客户端验证所需的公钥文本。

## 发布流程

遵循 `docs/GITFLOW.md` 和 `docs/RELEASE_CHECKLIST.md`：

1. 将功能分支合入 `develop`，从 `develop` 创建 `release/<version>`。
2. 修改 `frontend/src-tauri/tauri.conf.json` 中的 `version`，例如 `1.1.0`，完成发版清单后合入 `main`。
3. 在 `main` 对应提交创建完全匹配的 tag，例如应用版本 `1.1.0` 必须使用 `v1.1.0`。
4. 推送 tag。`.github/workflows/release.yml` 会执行测试和 Clippy，生成签名 NSIS 安装包并发布 `latest.json`。
5. 检查 GitHub Release 至少包含 NSIS 安装包、对应签名文件和 `latest.json`，再用旧版本客户端验证“设置 → 版本 → 检查更新 → 立即更新”。

版本号与 tag 不一致时工作流会主动失败，避免发布清单指向错误版本。

## 回归检查

- 无新版本：显示“当前已是最新版本”，不出现安装按钮。
- 有新版本：展示目标版本和更新说明；重复点击不会并发发起检查或安装。
- 下载中：展示进度且禁用重复操作。
- 签名、网络或清单异常：保留当前版本并显示错误，不执行安装。
- 更新成功：NSIS 以被动模式覆盖安装，应用随后重新启动，版本号变为目标版本，用户数据目录保持不变。


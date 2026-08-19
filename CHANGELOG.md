# 变更日志

本项目的所有重要变更都会记录在此文件中。

## [未发布]

### 新增
- GitHub 设备流登录（OAuth Device Flow）
  - 前端在设置页输入 OAuth Client ID 即可生成设备码、浏览器授权、自动获取 Token 并写入系统钥匙串
  - 新增 `deviceFlowApi.ts`、`SettingsView` 设备流授权面板与轮询逻辑
- 后端 `github_device_flow` 模块与 `commands/device_flow.rs`
  - 对接 `github.com/login/device/code` 与 `github.com/login/oauth/access_token`
  - 新增 Tauri 命令：`github_device_login_start` / `github_device_login_poll` / `github_store_device_token`
- Git 操作认证回退到系统凭据
  - `git_ops/remote.rs` 新增 `try_system_https_credentials`，依次尝试 git credential helper、设备流写入的 GitHub Token、macOS 钥匙串直读
  - 解决 push/fetch 在未显式配置 Token 时仍能使用已登录账号的问题

### 变更
- `keychain` 模块新增常量 `GITHUB_TOKEN_ACCOUNT`，供设备流 Token 与认证回退共享存储
- `settingsStore` 新增 `githubOAuthClientId` 字段与 `updateGitHubOAuthClientId` 方法
# CaoGit 本地源码开发与打包指南

本文档介绍如何在本地环境中搭建 CaoGit 的开发环境，以及如何从源码构建出各平台的安装包。

## 项目简介

CaoGit 是一个基于 **Tauri 2.0 + Vue 3 + TypeScript + Rust** 构建的跨平台 Git 图形化客户端。

| 端 | 技术 | 目录 |
|----|------|------|
| 前端 | Vue 3 + Vite + TypeScript | `src/` |
| 后端 | Rust（Tauri / git2） | `src-tauri/` |
| 测试 | Vitest | `tests/unit/` |
| 自动化构建 | GitHub Actions | `.github/workflows/` |

---

## 一、环境准备

### 1.1 基础环境

| 工具 | 版本要求 | 说明 |
|------|---------|------|
| Git | >= 2.x | 源码管理，`git2` 后端已内置 libgit2，运行时可不依赖系统 git |
| Node.js | **20 LTS 及以上**（CI 使用 Node 20） | 前端构建 |
| npm | 随 Node.js 附带 | 依赖安装 |
| Rust | stable 工具链 | 后端编译 |
| Cargo | 随 Rust 工具链附带 | Rust 依赖管理 |

安装 Rust 推荐使用 [rustup](https://rustup.rs/)：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 1.2 平台依赖

**macOS**
```bash
# 安装 Xcode Command Line Tools（首次必装）
xcode-select --install
```
打包 DMG 还需要 `hdiutil`（系统自带）和 `codesign`（随 CLT 提供）。

**Windows**
- 安装 [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（勾选 "Desktop development with C++"）
- WebView2 运行时：Windows 10/11 系统已内置，无需额外安装

**Linux（Ubuntu/Debian）**
```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev \
  librsvg2-dev patchelf libssl-dev libgtk-3-dev build-essential
```

---

## 二、获取源码

```bash
git clone https://github.com/WNLUO/CaoGit.git
cd CaoGit
```

---

## 三、安装依赖

```bash
# 1. 安装前端依赖
npm install

# 2. 预编译 Rust 依赖（可选，验证工具链是否就绪）
cd src-tauri && cargo check
cd ..
```

> Rust 依赖首次下载/编译耗时较长（git2 会编译 vendored-openssl），请耐心等待。

---

## 四、本地开发

### 4.1 完整开发模式（前端 + 后端联调）

```bash
npm run tauri:dev
```

该命令会：
1. 启动 Vite 开发服务器（`http://localhost:1420`，端口固定，见 `vite.config.ts`）
2. 编译并运行 Rust 后端，拉起应用窗口
3. 前端与 Rust 代码修改后均可热更新/自动重编译

> 端口 `1420` 被占用会导致启动失败（`strictPort: true`），可先释放端口或修改 `vite.config.ts`。

### 4.2 仅前端开发

```bash
npm run dev
```

仅启动 Vite 开发服务器，适合纯 UI 调试，但无法调用 Rust 命令（Tauri API 会失效）。

### 4.3 运行测试

```bash
npm run test          # vitest 监听模式
npm run test:run      # 单次运行
npm run test:coverage # 覆盖率报告
```

---

## 五、本地构建打包

> 所有构建产物默认输出到 `src-tauri/target/release/bundle/`。

### 5.1 macOS 标准打包（推荐）

```bash
npm run tauri:build
```

内部流程（`package.json` 定义）：
1. `tauri build --bundles app` → 生成 `CaoGit.app`
2. `scripts/create-localized-dmg.sh` → 应用本地化资源并生成带背景图的 DMG

产物：
- `src-tauri/target/release/bundle/macos/CaoGit.app`
- `src-tauri/target/release/bundle/dmg/CaoGit_<版本号>_aarch64.dmg`

### 5.2 仅打包 .app（不生成 DMG）

```bash
npx tauri build --bundles app
```

### 5.3 直接打包 DMG

```bash
npm run tauri:build:dmg
```

### 5.4 App Store 版本打包

```bash
npm run tauri:build:appstore
```

内部流程：
1. `npm run build:appstore`（读取 `.env.appstore`，注入 `VITE_APPSTORE=true`）
2. 预置本地化资源
3. `cargo build --release --features appstore --no-default-features`
4. 使用 `src-tauri/tauri.conf.appstore.json` 打包
5. 完成后对 `.app` 应用本地化

> 需要持有 `Apple Distribution` 证书（`T5P2UCK36A`），证书存放在系统钥匙串中。

### 5.5 免签名本地构建

默认 `tauri.conf.json` 配置了开发者签名身份。若本机没有证书，使用无签名配置：

```bash
npx tauri build --config src-tauri/tauri.conf.local.json
```

`tauri.conf.local.json` 中 `signingIdentity` 为 `null`、`hardenedRuntime` 为 `false`，适合无证书的本地调试构建。

### 5.6 Windows / Linux 打包

`npm run tauri:build` 中的 DMG 脚本依赖 macOS 的 `bash`/`hdiutil`，**仅在 macOS 上可用**。在 Windows / Linux 上请直接使用 Tauri 命令：

```bash
# Windows / Linux 通用
npm run build
npx tauri build
```

产物（各自平台）：
- Windows：`bundle/msi/*.msi`、`bundle/nsis/*.exe`
- Linux：`bundle/appimage/*.AppImage`、`bundle/deb/*.deb`

---

## 六、构建产物说明

| 平台 | 产物路径（`src-tauri/target/release/bundle/` 下） | 说明 |
|------|--------------------------------------------------|------|
| macOS | `macos/CaoGit.app` | 应用包 |
| macOS | `dmg/CaoGit_<版本>_aarch64.dmg` | 安装镜像（Apple Silicon） |
| Windows | `msi/*.msi` / `nsis/*.exe` | 安装程序 |
| Linux | `appimage/*.AppImage` / `deb/*.deb` | 安装包 |

调试用中间产物位于 `src-tauri/target/debug/` 与 `src-tauri/target/release/`。

---

## 七、版本号管理

版本号需在**三处保持一致**：

1. `package.json` → `"version"`
2. `src-tauri/tauri.conf.json` → `"version"`
3. `src-tauri/Cargo.toml` → `[package] version`

`package.json` 已声明版本自增脚本：

```bash
npm run bump:major
npm run bump:minor
npm run bump:patch
```

> 注意：以上脚本依赖根目录 `scripts/bump-version.js`，该文件当前不在仓库中，执行会报错。如无此脚本，请手动同步修改上述三个文件。

GitHub Actions 构建时会校验 Git 标签与 `tauri.conf.json` 版本号一致，不一致将直接失败。

---

## 八、打包相关配置说明

### 8.1 三套 Tauri 配置

| 配置文件 | 用途 |
|---------|------|
| `src-tauri/tauri.conf.json` | 默认配置：Developer ID 签名、自动更新（默认 feature） |
| `src-tauri/tauri.conf.local.json` | 本地免签名调试配置（版本号独立维护，可能滞后） |
| `src-tauri/tauri.conf.appstore.json` | App Store 专用：Apple Distribution 签名、App Store entitlements |

### 8.2 本地化

- 语言资源：`src-tauri/resources/` 下的 `zh_CN.lproj`、`zh-Hans.lproj`、`en.lproj`
- `src-tauri/build.rs`：构建时自动生成/补齐本地化资源目录与 `InfoPlist.strings`
- `src-tauri/scripts/add-localization.sh`：向打包后的 `.app` 注入本地化并 ad-hoc 重签名
- `src-tauri/scripts/create-localized-dmg.sh`：打包 DMG 前自动执行本地化

### 8.3 代码签名与公证

- 发布版（`tauri.conf.json`）：Developer ID Application 签名，`hardenedRuntime: true`，entitlements 见 `src-tauri/entitlements.plist`
- App Store 版（`tauri.conf.appstore.json`）：Apple Distribution 签名，entitlements 见 `src-tauri/entitlements-appstore.plist`
- 公证（notarization）仅在 GitHub Actions 发布流程中执行，需要 `APPLE_ID` / `APPLE_TEAM_ID` / `APPLE_APP_PASSWORD` 三个仓库 Secret
- 本地打包默认不公证，分发后用户首次打开可能被 macOS 拦截，可用以下命令放行：

```bash
xattr -cr /Applications/CaoGit.app
```

### 8.4 自动更新

- 默认 feature 为 `auto-update`（DMG 分发版），通过 GitHub Releases API 检查更新并下载安装（自定义实现，非 Tauri updater 插件）
- `appstore` feature 会禁用自动更新，改用 App Store 更新提示
- `.env.updater` 存放更新签名密钥等环境变量（`TAURI_SIGNING_PRIVATE_KEY`）

---

## 九、发布流程（GitHub Actions）

本地调试通过后，通过推送 Git 标签触发 CI 自动构建三平台安装包：

```bash
# 方式一：使用仓库脚本
./release.sh v0.3.0 "Release v0.3.0"

# 方式二：手动操作
git add .
git commit -m "feat: 新功能"
git tag -a v0.3.0 -m "Release v0.3.0"
git push origin v0.3.0
```

触发后：
1. `verify-version` 校验标签与 `tauri.conf.json` 版本一致
2. `build-macos` 构建并签名 DMG（含公证）
3. 上传产物到 Release

> 详见 `.github/RELEASE_GUIDE.md` 与 `.github/QUICK_START.md`。

---

## 十、常见问题（FAQ）

**Q1：`npm run tauri:dev` 启动失败，提示端口被占用？**
`vite.config.ts` 中 `strictPort: true`，端口固定 `1420`。释放该端口后重试。

**Q2：Rust 编译特别慢？**
首次编译需编译全部 Rust 依赖（含 vendored-openssl），属正常现象。后续构建有缓存会明显加快。可先执行 `cd src-tauri && cargo check` 预编译。

**Q3：本地打包报签名错误？**
本机无对应开发者证书时，使用免签名配置：
```bash
npx tauri build --config src-tauri/tauri.conf.local.json
```

**Q4：`npm run tauri:build` 在 Windows/Linux 上失败？**
`tauri:build` 脚本中的 DMG 生成依赖 macOS 的 `bash`/`hdiutil`，仅支持 macOS。其他平台请使用 `npx tauri build`。

**Q5：打包出的应用在 macOS 上提示"已损坏，无法打开"？**
未公证的应用会被 Gatekeeper 拦截，执行：
```bash
xattr -cr /Applications/CaoGit.app
```

**Q6：构建报"版本号不一致"？**
检查 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 三处版本号是否一致，Git 标签是否以 `v` 开头且与配置版本相同。

**Q7：修改 Rust 代码后如何快速验证？**
`npm run tauri:dev` 会自动重编译 Rust 并重启应用；如需构建产物则用 `npx tauri build --debug`（调试版，不打包）。

# 光合日历 · 桌面 Agent（SchedulePCagent）

本目录包含：**产品文档** + **桌面观察运行时（Rust + Tauri + React）** 初版实现。桌面窗口产品名为 **「光合桌面观察」**（调试壳，非最终商品名）。

## 文档索引

| 文档 | 说明 |
|------|------|
| [**docs/运行教程.md**](./docs/运行教程.md) | **从零启动：目录、命令、方舟 Key、常见报错（ENOENT / cargo / exe 占用 / 白屏）** |
| [docs/MVPv1.md](./docs/MVPv1.md) | 个人桌面 harness：观察层 / 时间线 / CLI 调试 / 节奏引擎路线 |
| [docs/PRDv1.md](./docs/PRDv1.md) | 产品方向与分阶段路线 |
| [docs/flutter_to_desktop_product_analysis.md](./docs/flutter_to_desktop_product_analysis.md) | 与 Flutter 主应用的关系 |
| [docs/数据与运行说明.md](./docs/数据与运行说明.md) | **数据路径、字段设计、白屏排查、WebView 与停录是否丢数据** |
| [docs/气泡球调参.md](./docs/气泡球调参.md) | **气泡窗口宽高、阴影、object-fit、右键菜单等调参入口** |

### 火山方舟 AI 分析（豆包）

1. 在 `SchedulePCagent` 下复制 [`.env.example`](./.env.example) 为 **`.env`**，填写 `ARK_API_KEY`（**不要**把 `.env` 提交到 Git）。
2. 启动 `npm run tauri:dev`，点击 **「AI 分析今日」**：会把今日事件 JSON + 近 15 分钟统计发给方舟，模型默认 `doubao-seed-2-0-lite-260215`，可用 `ARK_MODEL` / `ARK_API_BASE` 覆盖。
3. 若曾在聊天/截图里泄露过 Key，请到火山引擎控制台 **轮换密钥**。

### 桌面气泡球（光合日历）

- `npm run tauri:dev` 后会同时打开主窗口与 **`bubble`** 小窗：无边框、透明背景、置顶、不占任务栏（见 `src-tauri/tauri.conf.json`）。
- 调节大小、阴影、图片裁切、右键菜单：见 **[docs/气泡球调参.md](./docs/气泡球调参.md)** 与 **`src/BubbleBall.tsx` 顶部注释**。
- 背景图为仓库内 [`images/光合日历气泡球背景.png`](./images/光合日历气泡球背景.png)。在球体区域 **按住拖动** 可移动窗口；**双击** 打开主窗口；**右键** 弹出功能菜单（打开主界面 / 隐藏气泡球，菜单通过 Portal 画在窗口内、不压在球体上）。隐藏后主窗口点 **「显示气泡球」** 可再次显示。

## 工程结构

| 路径 | 说明 |
|------|------|
| `crates/harness-core` | `DesktopEvent`（app_switch + clipboard）、`RecordingState` 轮询、`TimelineStore`（含 WAL）、`WindowStats` 时间窗统计 |
| `crates/harness-cli` | 调试 CLI，二进制名 **`harness`** |
| `src-tauri/` | Tauri 2：`invoke` 时间线 / 前台 / **后台记录** / `analyze_window_minutes` |
| `src/` | React + TypeScript + Tailwind（已预留，后续可 `shadcn init`） |
| `data/` | 默认 SQLite 目录（`harness.db*`），随仓库在 D 盘；见 [`data/README.md`](./data/README.md) |

## 前置条件

- **Rust**（`rustc` / `cargo`）— 本仓库作者环境为 **装在 D 盘**，避免占满 C 盘：
  - 用户变量：`RUSTUP_HOME` = `D:\Rust\rustup`，`CARGO_HOME` = `D:\Rust\cargo`
  - `Path` 前置：`D:\Rust\cargo\bin`
  - 一键脚本（新机器或重装）：以 PowerShell 执行 [`scripts/install-rust-d-drive.ps1`](./scripts/install-rust-d-drive.ps1)（需已装 **Visual Studio Build Tools / MSVC** 以使用 `x86_64-pc-windows-msvc`）
  - 设置后请 **新开终端**，或重启 Cursor，再运行 `cargo` / `npm run tauri:dev`
  - 若终端仍找不到 `cargo`：运行 **`npm run tauri:dev`**（或 `npm run tauri -- dev`），会通过 [`scripts/run-with-rust-env.mjs`](./scripts/run-with-rust-env.mjs) 把 `D:\Rust\cargo\bin` 加入本次进程 `PATH`；也可在 PowerShell 临时执行：`$env:Path = 'D:\Rust\cargo\bin;' + $env:Path`
- Node.js 18+、npm

## 开发命令

```bash
cd SchedulePCagent
npm install
```

### 调试 CLI（显微镜）

```bash
cargo run -p harness-cli -- observe status
cargo run -p harness-cli -- observe run --interval-secs 2
cargo run -p harness-cli -- timeline today
cargo run -p harness-cli -- timeline today --json
cargo run -p harness-cli -- analyze window --minutes 15
cargo run -p harness-cli -- analyze window --minutes 30 --json
```

数据库默认：**本仓库 `data/harness.db`**（`npm run tauri:*` 会设 `GUANGHE_DATA_DIR`）；在仓库根执行 `cargo run -p harness-cli` 时也会用 `./data/`。可用 `--db` 或 `HARNESS_DB_PATH` / `GUANGHE_DATA_DIR` 覆盖。详见 [数据与运行说明](./docs/数据与运行说明.md)。**导出 JSON 给 AI**：`npm run export:timeline-today` → `data/export/timeline-today-*.json`。

### 桌面壳（Tauri + Vite）

```bash
npm run tauri:dev
```

生产构建：

```bash
npm run tauri:build
```

（`tauri.conf.json` 里 `bundle.active` 当前为 `false`，便于无图标先跑通；发布前再打开并配置 `icons`。）

## 已实现（阶段 A）

- 剪贴板文本变化写入 `clipboard` 事件（预览截断、记录总字符数）
- SQLite `WAL`，便于 Tauri 后台写线程与界面读连接并存
- Tauri 内 **开始记录 / 停止记录**（2s 轮询，与 CLI 共用 `RecordingState`）
- `analyze window` / `analyze_window_minutes`：近 N 分钟 app_switch 次数、clipboard 次数、主导应用 Top10

## 下一步（阶段 B+）

- 浏览器上下文、Rhythm Engine v0、`shadcn/ui`、托盘与全局快捷键、对接 `schedule_backend` AI

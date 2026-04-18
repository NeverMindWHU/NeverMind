/**
 * Prepends Cargo's bin dir to PATH (and sets default D: rustup dirs on Windows)
 * so `npm run tauri` works in terminals that didn't pick up User env vars yet.
 */
import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const isWin = process.platform === "win32";

const defaultCargoHome = isWin ? "D:\\Rust\\cargo" : path.join(process.env.HOME || "", ".cargo");
const defaultRustupHome = isWin ? "D:\\Rust\\rustup" : path.join(process.env.HOME || "", ".rustup");

if (!process.env.CARGO_HOME) process.env.CARGO_HOME = defaultCargoHome;
if (!process.env.RUSTUP_HOME) process.env.RUSTUP_HOME = defaultRustupHome;

const cargoBin = path.join(process.env.CARGO_HOME, "bin");
const cargoExe = path.join(cargoBin, isWin ? "cargo.exe" : "cargo");

// 数据库落在仓库内 data/，避免默认写进 C 盘 AppData；可用环境变量覆盖
if (!process.env.GUANGHE_DATA_DIR) {
  process.env.GUANGHE_DATA_DIR = path.join(root, "data");
}

if (existsSync(cargoExe)) {
  process.env.PATH = cargoBin + path.delimiter + (process.env.PATH || "");
} else {
  console.warn(
    `[run-with-rust-env] 未找到 ${cargoExe}。请安装 Rust 或设置 CARGO_HOME / 将 cargo 加入 PATH。`
  );
}

const args = process.argv.slice(2);
if (args.length === 0) {
  console.error("用法: npm run tauri -- dev    或    npm run tauri -- build");
  process.exit(1);
}

const tauriCli = path.join(root, "node_modules", ".bin", isWin ? "tauri.cmd" : "tauri");
if (!existsSync(tauriCli)) {
  console.error("未找到 node_modules/.bin/tauri，请先 npm install");
  process.exit(1);
}

const result = spawnSync(tauriCli, args, {
  cwd: root,
  stdio: "inherit",
  env: process.env,
  shell: isWin,
});

process.exit(result.status ?? 1);

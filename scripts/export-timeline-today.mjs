/**
 * 导出「今日时间线」为 JSON 文件，方便粘贴给不支持 .db 的 AI。
 * 使用与 Tauri 相同的 GUANGHE_DATA_DIR（仓库 data/）。
 */
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const dataDir = path.join(root, "data");
process.env.GUANGHE_DATA_DIR = dataDir;

const isWin = process.platform === "win32";
if (!process.env.CARGO_HOME) {
  process.env.CARGO_HOME = isWin ? "D:\\Rust\\cargo" : path.join(process.env.HOME || "", ".cargo");
}
const cargoExe = path.join(process.env.CARGO_HOME, "bin", isWin ? "cargo.exe" : "cargo");
if (existsSync(cargoExe)) {
  process.env.PATH =
    path.join(process.env.CARGO_HOME, "bin") + path.delimiter + (process.env.PATH || "");
}

const outDir = path.join(dataDir, "export");
mkdirSync(outDir, { recursive: true });
const day = new Date().toISOString().slice(0, 10);
const outFile = path.join(outDir, `timeline-today-${day}.json`);

const r = spawnSync(
  existsSync(cargoExe) ? cargoExe : "cargo",
  [
    "run",
    "-p",
    "harness-cli",
    "--quiet",
    "--",
    "timeline",
    "today",
    "--json",
  ],
  {
    cwd: root,
    env: { ...process.env, GUANGHE_DATA_DIR: dataDir },
    encoding: "utf-8",
    shell: isWin,
  }
);

if (r.status !== 0) {
  console.error(r.stderr || r.stdout || "cargo failed");
  process.exit(r.status ?? 1);
}

writeFileSync(outFile, r.stdout || "[]\n", "utf-8");
console.log(`已写入: ${outFile}`);
console.log("可直接把该 JSON 文件内容拖给 AI，或打开复制。");

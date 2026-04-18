# 本地数据库目录（默认）

运行时会在本目录生成：

- `harness.db` — SQLite 主库文件  
- `harness.db-wal` / `harness.db-shm` — 见上级 [`docs/数据与运行说明.md`](../docs/数据与运行说明.md)

通过 `npm run tauri:dev` / `tauri:build` 启动时，会自动设置 `GUANGHE_DATA_DIR` 指向这里，因此**数据跟仓库走（例如在 D 盘）**，而不是 `C:\Users\...\AppData\Local\...`。

覆盖方式：

- `HARNESS_DB_PATH`：直接指定 `.db` 文件的完整路径  
- `GUANGHE_DATA_DIR`：指定目录（程序使用其中的 `harness.db`）

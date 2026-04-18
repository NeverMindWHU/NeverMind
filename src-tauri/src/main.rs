fn main() {
    // 从 .env 加载本地配置（若不存在则静默忽略）。
    // 真实的豆包客户端构建在 `nevermind_tauri::run()` 内完成，
    // 缺失 ARK_API_KEY 会在应用初始化阶段直接报错。
    let _ = dotenvy::dotenv();

    nevermind_tauri::run();
}

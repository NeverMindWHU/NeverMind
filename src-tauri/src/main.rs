fn main() {
    // 从 .env 加载本地配置（若不存在则静默忽略）。
    let _ = dotenvy::dotenv();

    // 预热默认 LLM 客户端配置；真实调用仍由应用外壳中的命令层接管。
    let _ = nevermind_tauri::ai::default_client();

    nevermind_tauri::run();
}

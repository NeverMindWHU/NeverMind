use nevermind_tauri::{ai, db::Database};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从 .env 加载本地配置（若不存在则静默忽略）。
    let _ = dotenvy::dotenv();

    let database = Database::connect("sqlite:nevermind.db").await?;
    database.migrate().await?;

    // 构建默认 LLM 客户端：有 ARK_API_KEY 则为真实客户端，否则回退 Mock。
    let _llm = ai::default_client();

    Ok(())
}

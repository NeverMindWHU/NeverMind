pub mod commands;
pub mod ai;
pub mod db;
pub mod models;
pub mod scheduler;
pub mod state;
pub mod utils;

use crate::{db::Database, state::AppState, utils::error::AppResult};

async fn init_app_state(database_url: &str) -> AppResult<AppState> {
    let database = Database::connect(database_url).await?;
    database.migrate().await?;
    // 生产启动路径：必须成功构建豆包客户端，保证所有 IPC 调用都走真实 LLM。
    let llm = crate::ai::require_ark_client()?;
    Ok(AppState::from_pool_with_llm(
        database.pool().clone(),
        llm,
    ))
}

pub fn run() {
    let app_state = tauri::async_runtime::block_on(init_app_state("sqlite:nevermind.db"))
        .unwrap_or_else(|err| {
            panic!(
                "应用初始化失败: {}\n请确认 .env 中已配置 ARK_API_KEY（参考 .env.example）。",
                err
            )
        });

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::ipc::generate_cards,
            commands::ipc::list_generated_cards,
            commands::ipc::review_generated_cards,
            commands::ipc::list_due_reviews,
            commands::ipc::submit_review_result,
            commands::ipc::get_review_dashboard,
            commands::ipc::get_settings,
            commands::ipc::update_settings,
            commands::ipc::list_model_profiles,
            commands::ipc::save_model_profile,
            commands::ipc::test_model_profile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

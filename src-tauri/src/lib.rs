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
    Ok(AppState::from_pool(database.pool().clone()))
}

pub fn run() {
    let app_state = tauri::async_runtime::block_on(init_app_state("sqlite:nevermind.db"))
        .unwrap_or_else(|err| panic!("failed to initialize application state: {}", err));

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::ipc::generate_cards,
            commands::ipc::list_generated_cards,
            commands::ipc::review_generated_cards,
            commands::ipc::list_due_reviews,
            commands::ipc::submit_review_result,
            commands::ipc::get_review_dashboard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

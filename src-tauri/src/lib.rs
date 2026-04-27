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

    let shortcut_str = tauri::async_runtime::block_on(async {
        if let Ok(Some(settings)) = app_state.settings_dao.get_settings().await {
            settings.screenshot_shortcut
        } else {
            "ctrl+shift+a".to_string()
        }
    });

    let mut shortcut_builder = tauri_plugin_global_shortcut::Builder::new();
    if let Ok(s) = shortcut_str.parse::<tauri_plugin_global_shortcut::Shortcut>() {
        shortcut_builder = shortcut_builder.with_shortcuts([s]).unwrap_or_else(|e| {
            eprintln!("Failed to register shortcut from settings: {}", e);
            tauri_plugin_global_shortcut::Builder::new()
        });
    }

    tauri::Builder::default()
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .plugin(tauri_plugin_notification::init())
        .plugin(
            shortcut_builder
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        use tauri::Manager;
                        let buf: tauri::State<'_, crate::commands::screenshot::ScreenshotBuffer> = app.state();
                        let _ = tauri::async_runtime::block_on(
                            crate::commands::screenshot::spawn_screenshot_windows(app.clone(), buf)
                        );
                    }
                })
                .build(),
        )
        .manage(app_state)
        .manage(crate::commands::screenshot::ScreenshotBuffer(
            std::sync::Mutex::new(std::collections::HashMap::new()),
        ))
        .setup(|app| {
            commands::tray::setup_tray(app)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })
        })
        .invoke_handler(tauri::generate_handler![
            commands::ipc::generate_cards,
            commands::ipc::list_generated_cards,
            commands::ipc::review_generated_cards,
            commands::ipc::list_due_reviews,
            commands::ipc::list_upcoming_reviews,
            commands::ipc::submit_review_result,
            commands::ipc::get_review_dashboard,
            commands::ipc::get_settings,
            commands::ipc::update_settings,
            commands::ipc::list_model_profiles,
            commands::ipc::save_model_profile,
            commands::ipc::test_model_profile,
            commands::ipc::clear_library,
            commands::ipc::library_search_by_keyword,
            commands::ipc::library_search_by_question,
            commands::ipc::library_list_keyword_buckets,
            commands::screenshot::capture_screen,
            commands::screenshot::capture_monitor,
            commands::screenshot::get_captured_monitor,
            commands::screenshot::spawn_screenshot_window,
            commands::screenshot::spawn_screenshot_windows,
            commands::screenshot::close_screenshot_windows,
            commands::tray::sync_tray_generation_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

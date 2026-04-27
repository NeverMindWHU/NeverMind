//! 系统托盘：菜单、图标随卡片生成任务状态切换。

use serde::Deserialize;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::commands::screenshot::ScreenshotBuffer;

pub const TRAY_ICON_ID: &str = "nevermind-tray";

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrayGenerationState {
    Idle,
    Running,
    Ready,
    Mixed,
}

fn solid_rgba_icon(r: u8, g: u8, b: u8) -> Image<'static> {
    const S: u32 = 22;
    let n = (S * S) as usize;
    let mut rgba = Vec::with_capacity(n * 4);
    for _ in 0..n {
        rgba.extend_from_slice(&[r, g, b, 255]);
    }
    Image::new_owned(rgba, S, S)
}

fn icon_for_state(state: TrayGenerationState) -> Image<'static> {
    match state {
        TrayGenerationState::Idle => solid_rgba_icon(110, 110, 118),
        TrayGenerationState::Running => solid_rgba_icon(59, 130, 246),
        TrayGenerationState::Ready => solid_rgba_icon(34, 197, 94),
        TrayGenerationState::Mixed => solid_rgba_icon(245, 158, 11),
    }
}

fn show_and_focus_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn navigate_main_to_generate(app: &AppHandle) {
    show_and_focus_main(app);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.eval("window.location.hash = '#/generate'");
    }
}

fn spawn_screenshot_from_tray(app: &AppHandle) {
    let handle = app.clone();
    let buf: tauri::State<'_, ScreenshotBuffer> = handle.state();
    let _ = tauri::async_runtime::block_on(
        crate::commands::screenshot::spawn_screenshot_windows(handle.clone(), buf),
    );
}

/// 创建托盘图标与菜单；在 `.manage(ScreenshotBuffer)` 之后于 `setup` 中调用。
pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "nevermind-tray-show", "显示主窗口", true, None::<&str>)?;
    let gen = MenuItem::with_id(app, "nevermind-tray-generate", "生成卡片", true, None::<&str>)?;
    let shot = MenuItem::with_id(app, "nevermind-tray-screenshot", "区域截图", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "nevermind-tray-quit", "退出 NeverMind", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &gen, &shot, &quit])?;

    let icon = icon_for_state(TrayGenerationState::Idle);

    let _tray = TrayIconBuilder::with_id(TRAY_ICON_ID)
        .icon(icon)
        .menu(&menu)
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "nevermind-tray-show" => show_and_focus_main(app),
                "nevermind-tray-generate" => navigate_main_to_generate(app),
                "nevermind-tray-screenshot" => spawn_screenshot_from_tray(app),
                "nevermind-tray-quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                show_and_focus_main(app);
            }
        })
        .build(app)?;

    Ok(())
}

#[tauri::command]
pub fn sync_tray_generation_state(
    app: AppHandle,
    state: TrayGenerationState,
) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ICON_ID) else {
        return Ok(());
    };
    let icon = icon_for_state(state);
    tray
        .set_icon(Some(icon))
        .map_err(|e| e.to_string())?;
    let title = match state {
        TrayGenerationState::Idle => "",
        TrayGenerationState::Running => "生成中",
        TrayGenerationState::Ready => "有待预览批次",
        TrayGenerationState::Mixed => "生成中·有待预览",
    };
    if title.is_empty() {
        let _ = tray.set_title(None::<&str>);
    } else {
        let _ = tray.set_title(Some(title));
    }
    Ok(())
}

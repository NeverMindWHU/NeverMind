use tauri::AppHandle;
use tauri_utils::config::WindowConfig;
use xcap::Monitor;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Mutex;

/// Pre-captured screenshot bytes, keyed by monitor index.
/// Populated *before* overlay windows are opened so we never capture the overlay itself.
pub struct ScreenshotBuffer(pub Mutex<HashMap<usize, Vec<u8>>>);

/// Returns the pre-captured PNG bytes for a specific monitor.
/// Called by the frontend overlay after the window has been shown.
#[tauri::command]
pub async fn get_captured_monitor(
    index: usize,
    buffer: tauri::State<'_, ScreenshotBuffer>,
) -> Result<Vec<u8>, String> {
    let map = buffer.0.lock().map_err(|e| e.to_string())?;
    map.get(&index)
        .cloned()
        .ok_or_else(|| format!("No pre-captured image for monitor {}", index))
}

/// Captures a specific monitor by index on-demand (kept for ad-hoc use).
#[tauri::command]
pub async fn capture_monitor(index: usize) -> Result<Vec<u8>, String> {
    do_capture_monitor(index)
}

/// Legacy single-monitor capture kept for compatibility.
#[tauri::command]
pub async fn capture_screen() -> Result<Vec<u8>, String> {
    do_capture_monitor(0)
}

fn do_capture_monitor(index: usize) -> Result<Vec<u8>, String> {
    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    let monitor = monitors
        .into_iter()
        .nth(index)
        .ok_or_else(|| format!("Monitor {} not found", index))?;

    let image = monitor.capture_image().map_err(|e| e.to_string())?;

    let mut bytes: Vec<u8> = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(bytes)
}

/// 1. Captures ALL monitors first (while desktop is still visible).
/// 2. Stores PNGs in ScreenshotBuffer.
/// 3. Opens one fullscreen overlay window per monitor.
#[tauri::command]
pub async fn spawn_screenshot_windows(
    app: AppHandle,
    buffer: tauri::State<'_, ScreenshotBuffer>,
) -> Result<(), String> {
    use tauri::Manager;

    let monitors = Monitor::all().map_err(|e| e.to_string())?;

    // ── Step 1: capture every monitor while desktop is still visible ─────
    let mut captures: Vec<(usize, Vec<u8>, f64, f64, f64, f64)> = Vec::new();
    for (i, monitor) in monitors.iter().enumerate() {
        let image = monitor.capture_image().map_err(|e| e.to_string())?;
        let mut bytes: Vec<u8> = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;

        let mon_x = monitor.x().map_err(|e| e.to_string())? as f64;
        let mon_y = monitor.y().map_err(|e| e.to_string())? as f64;
        let mon_w = monitor.width().map_err(|e| e.to_string())? as f64;
        let mon_h = monitor.height().map_err(|e| e.to_string())? as f64;

        captures.push((i, bytes, mon_x, mon_y, mon_w, mon_h));
    }

    // ── Step 2: store in the shared buffer ───────────────────────────────
    {
        let mut map = buffer.0.lock().map_err(|e| e.to_string())?;
        map.clear();
        for (i, bytes, _, _, _, _) in &captures {
            map.insert(*i, bytes.clone());
        }
    }

    // ── Step 3: open overlay windows (screenshots already taken) ─────────
    for (i, _, mon_x, mon_y, mon_w, mon_h) in &captures {
        let label = format!("screenshot-{}", i);

        if app.get_webview_window(&label).is_some() {
            continue;
        }

        let url = format!("index.html#/screenshot?monitor={}", i);

        let mut config = WindowConfig::default();
        config.label = label;
        config.url = tauri::WebviewUrl::App(url.into());
        config.transparent = true;
        config.decorations = false;
        config.always_on_top = true;
        config.skip_taskbar = true;
        config.fullscreen = true;
        config.x = Some(*mon_x);
        config.y = Some(*mon_y);
        config.width = *mon_w;
        config.height = *mon_h;
        config.title = "Screenshot Overlay".to_string();

        let win = tauri::WebviewWindowBuilder::from_config(&app, &config)
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?;
        // 未聚焦时浏览器收不到 keydown，导致 idle 阶段 Esc 无法退出
        let _ = win.set_focus();
    }

    Ok(())
}

/// Legacy single-window spawn.
#[tauri::command]
pub async fn spawn_screenshot_window(app: AppHandle, buffer: tauri::State<'_, ScreenshotBuffer>) -> Result<(), String> {
    spawn_screenshot_windows(app, buffer).await
}

/// Closes all open screenshot overlay windows and clears the buffer.
#[tauri::command]
pub async fn close_screenshot_windows(
    app: AppHandle,
    buffer: tauri::State<'_, ScreenshotBuffer>,
) -> Result<(), String> {
    use tauri::Manager;

    // Close windows
    for i in 0..16 {
        let label = format!("screenshot-{}", i);
        if let Some(win) = app.get_webview_window(&label) {
            win.close().map_err(|e| e.to_string())?;
        }
    }
    if let Some(win) = app.get_webview_window("screenshot") {
        win.close().map_err(|e| e.to_string())?;
    }

    // Clear buffer
    if let Ok(mut map) = buffer.0.lock() {
        map.clear();
    }

    Ok(())
}

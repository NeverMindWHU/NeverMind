mod ark;
pub mod nevermind;

use harness_core::{
    build_learning_snapshot_event, current_foreground, default_db_path, RecordingState,
    TimelineStore,
};
use tauri::Manager;
use tauri::PhysicalPosition;
use tauri::Runtime;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use nevermind::{
    db::Database,
    state::AppState as NevermindState,
    utils::error::AppResult,
};

async fn init_nevermind_state(database_url: &str) -> AppResult<NevermindState> {
    let database = Database::connect(database_url).await?;
    database.migrate().await?;
    let llm = nevermind::ai::require_ark_client()?;
    Ok(NevermindState::from_pool_with_llm(
        database.pool().clone(),
        llm,
    ))
}

/// 与前端 `BubbleBall.tsx` 中球体边长（逻辑像素）一致。
const BUBBLE_BALL_LOGICAL_PX: f64 = 72.0;

/// 右键菜单打开时：整窗需接收点击，不能穿透。
static BUBBLE_MENU_OPEN: AtomicBool = AtomicBool::new(false);

/// 手边 AI 助手面板打开时：整窗需接收点击，不能穿透。
static BUBBLE_ASSISTANT_OPEN: AtomicBool = AtomicBool::new(false);

struct RecorderInner {
    stop: Option<Arc<AtomicBool>>,
    join: Option<thread::JoinHandle<()>>,
}

struct RecorderState {
    recorder: Mutex<RecorderInner>,
}

impl Default for RecorderState {
    fn default() -> Self {
        Self {
            recorder: Mutex::new(RecorderInner {
                stop: None,
                join: None,
            }),
        }
    }
}

#[tauri::command]
fn get_timeline_today() -> Result<String, String> {
    let db = default_db_path();
    let store = TimelineStore::open(&db).map_err(|e| e.to_string())?;
    let events = store.events_today_local().map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&events).map_err(|e| e.to_string())
}

/// 按本地日历日拉取时间线（用于「回顾」页与某日学习卡片）。
#[tauri::command]
fn get_timeline_local_date(year: i32, month: u32, day: u32) -> Result<String, String> {
    let d = chrono::NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| "无效日期".to_string())?;
    let db = default_db_path();
    let store = TimelineStore::open(&db).map_err(|e| e.to_string())?;
    let events = store.events_on_local_day(d).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&events).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_foreground_snapshot() -> Result<String, String> {
    let s = current_foreground().map_err(|e| e.to_string())?;
    serde_json::to_string(&s).map_err(|e| e.to_string())
}

#[tauri::command]
fn analyze_window_minutes(minutes: i64) -> Result<String, String> {
    if minutes < 1 || minutes > 24 * 60 {
        return Err("minutes 必须在 1..=1440 范围内".into());
    }
    let db = default_db_path();
    let store = TimelineStore::open(&db).map_err(|e| e.to_string())?;
    let stats = store
        .analyze_window_minutes(minutes)
        .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&stats).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_recording(state: tauri::State<'_, RecorderState>) -> Result<(), String> {
    let mut g = state.recorder.lock().map_err(|e| e.to_string())?;
    if g.join.is_some() {
        return Err("已在记录中".into());
    }
    let stop = Arc::new(AtomicBool::new(false));
    let stop_t = stop.clone();
    let db = default_db_path();
    let handle = thread::spawn(move || {
        let store = match TimelineStore::open(&db) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[harness] 打开数据库失败: {e}");
                return;
            }
        };
        let mut rec = RecordingState::new();
        while !stop_t.load(Ordering::SeqCst) {
            if let Err(e) = rec.poll_once(&store) {
                eprintln!("[harness] 采集 tick 失败: {e}");
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
    g.stop = Some(stop);
    g.join = Some(handle);
    Ok(())
}

#[tauri::command]
fn stop_recording(state: tauri::State<'_, RecorderState>) -> Result<(), String> {
    let mut g = state.recorder.lock().map_err(|e| e.to_string())?;
    if let Some(s) = &g.stop {
        s.store(true, Ordering::SeqCst);
    }
    if let Some(h) = g.join.take() {
        h.join().map_err(|e| format!("停止记录线程: {e:?}"))?;
    }
    g.stop = None;
    Ok(())
}

#[tauri::command]
fn get_recording_state(state: tauri::State<'_, RecorderState>) -> Result<bool, String> {
    let g = state.recorder.lock().map_err(|e| e.to_string())?;
    Ok(g.join.is_some())
}

/// 快捷键「学习快照」：主屏 PNG + 前台应用信息写入时间线（与轮询记录独立，可并行使用）。
#[tauri::command]
fn record_learning_snapshot() -> Result<String, String> {
    let ev = build_learning_snapshot_event().map_err(|e| e.to_string())?;
    let db = default_db_path();
    let store = TimelineStore::open(&db).map_err(|e| e.to_string())?;
    store.insert_event(&ev).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&ev).map_err(|e| e.to_string())
}

/// 解析数据目录下的相对路径（供前端 `convertFileSrc` 加载截图等本地文件）。
#[tauri::command]
fn resolve_data_file(rel: String) -> Result<String, String> {
    harness_core::resolve_data_relative_path(&rel)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

/// 从气泡球切回主调试窗口并聚焦。
#[tauri::command]
fn focus_main_window(app: tauri::AppHandle) -> Result<(), String> {
    let Some(w) = app.get_webview_window("main") else {
        return Err("找不到主窗口".into());
    };
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// 若用户点了「隐藏」，可从主窗口再次显示气泡球（不抢主窗口焦点，便于继续点主界面开关）。
#[tauri::command]
fn show_bubble_window(app: tauri::AppHandle) -> Result<(), String> {
    let Some(w) = app.get_webview_window("bubble") else {
        return Err("找不到气泡窗口".into());
    };
    w.show().map_err(|e| e.to_string())?;
    Ok(())
}

/// 主窗口侧栏：关闭气泡球窗口。
#[tauri::command]
fn hide_bubble_window(app: tauri::AppHandle) -> Result<(), String> {
    let Some(w) = app.get_webview_window("bubble") else {
        return Err("找不到气泡窗口".into());
    };
    w.hide().map_err(|e| e.to_string())?;
    Ok(())
}

/// 主窗口同步侧栏开关状态。
#[tauri::command]
fn is_bubble_visible(app: tauri::AppHandle) -> Result<bool, String> {
    let Some(w) = app.get_webview_window("bubble") else {
        return Ok(false);
    };
    w.is_visible().map_err(|e| e.to_string())
}

/// 气泡右键菜单展开时由前端调用，用于穿透逻辑：菜单打开期间整窗接收鼠标。
#[tauri::command]
fn set_bubble_menu_open(open: bool) {
    BUBBLE_MENU_OPEN.store(open, Ordering::Relaxed);
}

#[tauri::command]
fn set_bubble_assistant_open(open: bool) {
    BUBBLE_ASSISTANT_OPEN.store(open, Ordering::Relaxed);
}

/// 桌面气泡上的「手边助手」：短对话，可选一张配图（Base64，无 data: 前缀）。
#[tauri::command]
async fn bubble_assistant_chat(
    text: String,
    image_mime: Option<String>,
    image_base64: Option<String>,
) -> Result<String, String> {
    let trimmed = text.trim();
    let has_img = image_base64
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if trimmed.is_empty() && !has_img {
        return Err("请输入文字或添加一张图片".into());
    }

    let user_line = if trimmed.is_empty() {
        "请根据图片内容简要回答（中文）。"
    } else {
        trimmed
    };

    let system = "你是「光合桌面观察」里常驻桌面的手边助手，回答简短、可执行，优先中文；不要编造用户未提供的信息。";

    if has_img {
        let mime = image_mime
            .unwrap_or_else(|| "image/png".into())
            .trim()
            .to_string();
        if !mime.starts_with("image/") {
            return Err("图片类型无效".into());
        }
        let b64 = image_base64.unwrap();
        if b64.len() > 6_000_000 {
            return Err("图片过大，请选较小的截图或照片".into());
        }
        match ark::chat_completion_with_optional_image(system, user_line, &mime, &b64).await {
            Ok(s) => Ok(s),
            Err(e) => {
                let fallback_prompt = format!(
                    "{}\n\n（说明：多模态请求失败：{}，请仅根据上述文字回复。）",
                    user_line, e
                );
                ark::chat_completion(system, &fallback_prompt).await
            }
        }
    } else {
        ark::chat_completion(system, user_line).await
    }
}

/// 置顶气泡窗体较大时，透明区会挡住下层主窗口；仅在「球」区域接收点击，其余穿透到桌面/主窗口。
fn spawn_bubble_cursor_passthrough(app: tauri::AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(50));
        let Some(bubble) = app.get_webview_window("bubble") else {
            continue;
        };
        if !bubble.is_visible().unwrap_or(false) {
            continue;
        }
        if BUBBLE_MENU_OPEN.load(Ordering::Relaxed)
            || BUBBLE_ASSISTANT_OPEN.load(Ordering::Relaxed)
        {
            let _ = bubble.set_ignore_cursor_events(false);
            continue;
        }
        let Ok(scale) = bubble.scale_factor() else {
            continue;
        };
        let Ok(inner_pos) = bubble.inner_position() else {
            continue;
        };
        let Ok(inner_size) = bubble.inner_size() else {
            continue;
        };
        let Ok(cursor) = app.cursor_position() else {
            continue;
        };
        let ball_px = BUBBLE_BALL_LOGICAL_PX * scale;
        let left = inner_pos.x as f64 + inner_size.width as f64 - ball_px;
        let top = inner_pos.y as f64 + inner_size.height as f64 - ball_px;
        let cx = cursor.x;
        let cy = cursor.y;
        let in_ball = cx >= left && cx <= left + ball_px && cy >= top && cy <= top + ball_px;
        let _ = bubble.set_ignore_cursor_events(!in_ball);
    });
}

/// 气泡球首次出现：主显示器工作区右下角（留出边距，避开任务栏）。
fn position_bubble_bottom_right<R: Runtime>(bubble: &tauri::WebviewWindow<R>) {
    let Ok(Some(monitor)) = bubble.primary_monitor() else {
        return;
    };
    let wa = monitor.work_area();
    let Ok(outer) = bubble.outer_size() else {
        return;
    };
    let margin: i32 = 16;
    let x = wa.position.x + wa.size.width as i32 - outer.width as i32 - margin;
    let y = wa.position.y + wa.size.height as i32 - outer.height as i32 - margin;
    let _ = bubble.set_position(PhysicalPosition::new(x, y));
}

fn truncate_chars(s: &str, max: usize) -> String {
    let mut out = String::with_capacity(max.saturating_add(32));
    for (i, ch) in s.chars().enumerate() {
        if i >= max {
            out.push_str("\n\n…(内容过长已截断，仅发送前 ");
            out.push_str(&max.to_string());
            out.push_str(" 个字符给模型)");
            break;
        }
        out.push(ch);
    }
    out
}

/// 读取今日时间线 + 近 15 分钟统计，调用火山方舟做简短中文分析。
#[tauri::command]
async fn ai_analyze_today() -> Result<String, String> {
    let db = default_db_path();
    let store = TimelineStore::open(&db).map_err(|e| e.to_string())?;
    let events = store.events_today_local().map_err(|e| e.to_string())?;
    let events_json =
        serde_json::to_string_pretty(&events).map_err(|e| e.to_string())?;
    let stats = store
        .analyze_window_minutes(15)
        .map_err(|e| e.to_string())?;
    let stats_json = serde_json::to_string_pretty(&stats).map_err(|e| e.to_string())?;

    let events_part = truncate_chars(&events_json, 18_000);
    let user_prompt = format!(
        "下面是我今天在电脑上的采集数据（桌面观察 MVP）。\n\n\
         【今日事件 JSON 数组】每个元素有 type 字段：app_switch 表示前台切换（可选含前台窗口截图 image_rel、width_px、height_px），clipboard 表示剪贴板文本变化（可能含隐私，分析时不要复述具体内容），learning_snapshot 表示用户主动保存的学习快照（主显示器截图路径 image_rel，不要复述路径细节），browser_moment 表示 Chrome/哔哩哔哩学习时刻（learning_content 为解析出的学习主题，另有 page_title、summary、keywords，截图路径 image_rel）。\n\
         {}\n\n\
         【近 15 分钟统计 JSON】\n\
         {}\n\n\
         请用中文简洁回答：\n\
         1) 当前节奏概括（1～2 句）\n\
         2) 是否像「卡住 / 分心 / 高频切换」等（若有依据）\n\
         3) 一条可执行的下一步建议（10～20 字以内优先）",
        events_part, stats_json
    );

    let system = "你是「光合桌面观察」里的节奏分析助手，只根据用户提供的结构化日志做推断，不要编造未出现的事实；对剪贴板内容保持隐私，不要逐字引用。";

    ark::chat_completion(system, &user_prompt).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 优先仓库根目录 .env（npm run tauri:dev 时 cwd 多为仓库根）
    let _ = dotenvy::dotenv();
    let _ = dotenvy::from_path("../.env");

    let nevermind_state =
        tauri::async_runtime::block_on(init_nevermind_state("sqlite:nevermind.db")).unwrap_or_else(
            |err| {
                panic!(
                    "NeverMind 复习与卡片模块初始化失败: {}\n请确认 .env 中已配置 ARK_API_KEY（参考 .env.example）。",
                    err
                )
            },
        );

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::default().build())
        .setup(|app| {
            if let Some(bubble) = app.get_webview_window("bubble") {
                let _ = bubble.set_shadow(false);
                position_bubble_bottom_right(&bubble);
            }
            spawn_bubble_cursor_passthrough(app.handle().clone());
            Ok(())
        })
        .manage(nevermind_state)
        .manage(RecorderState::default())
        .invoke_handler(tauri::generate_handler![
            get_timeline_today,
            get_timeline_local_date,
            get_foreground_snapshot,
            analyze_window_minutes,
            start_recording,
            stop_recording,
            get_recording_state,
            record_learning_snapshot,
            resolve_data_file,
            focus_main_window,
            show_bubble_window,
            hide_bubble_window,
            is_bubble_visible,
            set_bubble_menu_open,
            set_bubble_assistant_open,
            bubble_assistant_chat,
            ai_analyze_today,
            nevermind::commands::ipc::generate_cards,
            nevermind::commands::ipc::list_generated_cards,
            nevermind::commands::ipc::review_generated_cards,
            nevermind::commands::ipc::list_due_reviews,
            nevermind::commands::ipc::submit_review_result,
            nevermind::commands::ipc::get_review_dashboard,
            nevermind::commands::ipc::get_settings,
            nevermind::commands::ipc::update_settings,
            nevermind::commands::ipc::list_model_profiles,
            nevermind::commands::ipc::save_model_profile,
            nevermind::commands::ipc::test_model_profile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NeverMind");
}

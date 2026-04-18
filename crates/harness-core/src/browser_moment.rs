//! 浏览器前台启发式：用窗口标题近似「当前页签」，在标签切换 / 哔哩哔哩等时机截取前台窗口。

use crate::error::{HarnessError, Result};
use crate::event::DesktopEvent;
use crate::default_data_dir;
use std::fs;
use std::path::Path;

/// 常见 Chromium / Edge 可执行名（小写比较）
pub fn is_browser_app(exe_name: &str) -> bool {
    let n = exe_name.to_lowercase();
    n == "chrome.exe"
        || n == "msedge.exe"
        || n == "brave.exe"
        || n == "vivaldi.exe"
        || n == "opera.exe"
        || n == "arc.exe"
        || n == "firefox.exe"
        || n.contains("chromium")
}

/// 当前「学习场景」仅 **Google Chrome**（网页哔哩哔哩与一般浏览；窗口标题近似页签）。
pub fn is_chrome_exe(exe_name: &str) -> bool {
    exe_name.to_lowercase() == "chrome.exe"
}

/// 哔哩哔哩 Windows 客户端（预留：不参与当前学习卡片采集，避免与「仅 Chrome」产品范围混淆）
pub fn is_bilibili_desktop_exe(exe_name: &str) -> bool {
    let n = exe_name.to_lowercase();
    n.contains("bilibili") || exe_name.contains("哔哩")
}

pub fn is_learning_focus_tracked_exe(exe_name: &str) -> bool {
    is_chrome_exe(exe_name)
}

pub fn is_bilibili_title(title: &str) -> bool {
    let t = title.to_lowercase();
    t.contains("bilibili") || title.contains("哔哩")
}

/// 从窗口标题中尽量抽出「页面标题」（去掉末尾 ` - 浏览器名`）。
pub fn extract_page_title_line(full: &str) -> String {
    const SUFFIXES: &[&str] = &[
        " - Google Chrome",
        " - Chromium",
        " - Microsoft​ Edge",
        " - Microsoft Edge",
        " - Brave",
        " - Vivaldi",
        " - Opera",
        " - Arc",
    ];
    let mut s = full.to_string();
    for suf in SUFFIXES {
        if let Some(pos) = s.rfind(suf) {
            s.truncate(pos);
            break;
        }
    }
    s.trim().to_string()
}

fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// 去掉 B 站窗口标题里常见的站点后缀，得到视频/合集标题。
pub fn strip_bilibili_site_suffix(line: &str) -> String {
    let mut t = line.trim();
    const SUFFIXES: &[&str] = &[
        " - 哔哩哔哩 (Official)",
        " - bilibili (Official)",
        " - 哔哩哔哩",
        " - bilibili",
    ];
    for suf in SUFFIXES {
        if let Some(i) = t.rfind(suf) {
            t = t[..i].trim();
            break;
        }
    }
    if let Some(pos) = t.rfind(" - ") {
        let right = t[pos + 3..].trim();
        let rl = right.to_lowercase();
        if rl.contains("bilibili") || right.contains("哔哩") {
            t = t[..pos].trim();
        }
    }
    t.to_string()
}

fn chrome_primary_title(page_line: &str) -> String {
    let t = page_line.trim();
    if t.is_empty() {
        return String::new();
    }
    for sep in [" | ", " · "] {
        if let Some(p) = t.find(sep) {
            let left = t[..p].trim();
            if char_count(left) >= 4 {
                return left.to_string();
            }
        }
    }
    if let Some(pos) = t.rfind(" - ") {
        let left = t[..pos].trim();
        let right = t[pos + 3..].trim();
        if char_count(left) >= 4 && char_count(right) <= 28 {
            return left.to_string();
        }
    }
    t.to_string()
}

fn is_blank_chrome_tab_page_line(page_line: &str) -> bool {
    let s = page_line.trim();
    if s.is_empty() {
        return true;
    }
    let lower = s.to_lowercase();
    lower.contains("新标签页")
        || lower.starts_with("new tab")
        || lower == "new tab"
        || lower.contains("about:blank")
}

/// 从页签一行解析「学习内容」主题（用于学习卡片与关键词）
pub fn extract_learning_content(trigger: &str, page_line: &str) -> String {
    let t = page_line.trim();
    if t.is_empty() {
        return String::new();
    }
    match trigger {
        "bilibili_open" | "bilibili_video" | "bilibili" => strip_bilibili_site_suffix(t),
        "chrome_new_tab" => {
            if is_blank_chrome_tab_page_line(page_line) {
                "空白新标签页".to_string()
            } else {
                chrome_primary_title(page_line)
            }
        }
        "chrome_tab" | "tab_switch" => chrome_primary_title(page_line),
        _ => chrome_primary_title(page_line),
    }
}

fn is_stop_token(tok: &str) -> bool {
    const STOP: &[&str] = &[
        "的", "了", "和", "与", "或", "在", "是", "为", "这", "那", "我", "你", "他", "她", "它",
        "吗", "呢", "吧", "啊", "之", "等", "及", "被", "从", "让", "将", "对", "把", "到", "也",
        "个", "中", "上", "下", "第", "集", "期", "官方", "双语", "视频", "教程", "合集", "转载",
        "the", "and", "for", "with", "from", "this", "that",
    ];
    let x = tok.trim();
    char_count(x) < 2 || STOP.iter().any(|&s| s == x)
}

fn push_keyword(out: &mut Vec<String>, s: String) {
    let s = s.trim().to_string();
    if s.is_empty() || is_stop_token(&s) {
        return;
    }
    if out.iter().any(|x: &String| x == &s) {
        return;
    }
    if char_count(&s) > 24 {
        return;
    }
    out.push(s);
}

fn tokenize_title_hints(primary: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in primary.chars() {
        if ch.is_whitespace()
            || matches!(
                ch,
                '，' | '。' | '、' | '；' | '：' | '｜' | '|' | '/' | '\\' | '·' | '—' | '－' | '-'
                    | '【' | '】' | '(' | ')' | '（' | '）' | '[' | ']' | '「' | '」' | '#' | '@'
            )
        {
            if !cur.is_empty() {
                let piece = cur.trim_matches(|c| c == '【' || c == '】' || c == '「' || c == '」');
                if char_count(piece) >= 2 {
                    push_keyword(&mut out, piece.to_string());
                }
                cur.clear();
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        let piece = cur.trim_matches(|c| c == '【' || c == '】' || c == '「' || c == '」');
        if char_count(piece) >= 2 {
            push_keyword(&mut out, piece.to_string());
        }
    }
    out
}

fn heuristic_keywords(trigger: &str, learning: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    match trigger {
        "bilibili_open" | "bilibili_video" | "bilibili" => {
            push_keyword(&mut out, "哔哩哔哩".into());
            push_keyword(&mut out, "视频学习".into());
        }
        "chrome_new_tab" => {
            push_keyword(&mut out, "Chrome".into());
            push_keyword(&mut out, "新标签".into());
        }
        "chrome_tab" | "tab_switch" => {
            push_keyword(&mut out, "Chrome".into());
            push_keyword(&mut out, "网页".into());
        }
        _ => {
            push_keyword(&mut out, "Chrome".into());
        }
    }
    for t in tokenize_title_hints(learning) {
        push_keyword(&mut out, t);
        if out.len() >= 8 {
            break;
        }
    }
    if out.len() < 3 && char_count(learning) >= 2 {
        let compact: String = learning.chars().filter(|c| !c.is_whitespace()).take(16).collect();
        if char_count(&compact) >= 2 {
            push_keyword(&mut out, compact);
        }
    }
    out.truncate(8);
    out
}

fn summary_zh(trigger: &str, learning: &str) -> String {
    let head = if char_count(learning) > 80 {
        learning.chars().take(80).collect::<String>() + "…"
    } else {
        learning.to_string()
    };
    match trigger {
        "bilibili_open" => format!("进入哔哩哔哩：{}", head),
        "bilibili_video" => format!("哔哩哔哩：{}", head),
        "chrome_new_tab" => "Chrome：空白新标签页".to_string(),
        "chrome_tab" => format!("Chrome：{}", head),
        "bilibili" => format!("哔哩哔哩：{}", head),
        "tab_switch" => format!("Chrome：{}", head),
        _ => format!("浏览器：{}", head),
    }
}

/// 是否应在此时生成一条带截图的「学习聚焦」时刻。
///
/// **Chrome**：网页哔哩哔哩（标题含哔哩/bilibili）或一般标签（含新标签页启发式）。
/// `prev_full` 为**同一 exe** 下上一次窗口标题；`None` 表示尚无同进程记录。
pub fn should_record_learning_focus_moment(
    full_title: &str,
    prev_full: Option<&str>,
    app_exe: &str,
) -> Option<&'static str> {
    if full_title.is_empty() || !is_learning_focus_tracked_exe(app_exe) {
        return None;
    }

    let in_bilibili_context = is_bilibili_title(full_title);

    if in_bilibili_context {
        match prev_full {
            Some(p) if p != full_title => {
                if is_bilibili_title(p) {
                    Some("bilibili_video")
                } else {
                    Some("bilibili_open")
                }
            }
            None => Some("bilibili_open"),
            _ => None,
        }
    } else {
        match prev_full {
            Some(p) if p != full_title => {
                let page = extract_page_title_line(full_title);
                if is_blank_chrome_tab_page_line(&page) {
                    Some("chrome_new_tab")
                } else {
                    Some("chrome_tab")
                }
            }
            _ => None,
        }
    }
}

/// 截取当前前台窗口客户区为 PNG，返回事件（不写库）。
pub fn build_browser_moment_event(trigger: &str, browser_app: String, window_title: String) -> Result<DesktopEvent> {
    #[cfg(windows)]
    {
        let page_line = extract_page_title_line(&window_title);
        let mut learning_content = extract_learning_content(trigger, &page_line);
        if learning_content.is_empty() {
            learning_content = page_line.clone();
        }
        let summary = summary_zh(trigger, &learning_content);
        let keywords = heuristic_keywords(trigger, &learning_content);
        let (image_rel, w, h) = windows_impl::capture_foreground_client_png_file("bm")?;
        Ok(DesktopEvent::browser_moment_now(
            trigger.to_string(),
            browser_app,
            window_title,
            page_line,
            learning_content,
            summary,
            keywords,
            image_rel,
            w,
            h,
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = (trigger, browser_app, window_title);
        Err(HarnessError::UnsupportedPlatform)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use image::RgbaImage;
    use std::ffi::c_void;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
        GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HGDIOBJ, SRCCOPY,
    };
    use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
    use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetForegroundWindow};

    /// PW_CLIENTONLY | PW_RENDERFULLCONTENT（2）— 尽量抓到 Chromium 硬件加速内容。
    fn print_window_flags_full() -> PRINT_WINDOW_FLAGS {
        PRINT_WINDOW_FLAGS(1u32 | 2u32)
    }

    /// `name_prefix`：保存文件名前缀，如 `bm`（浏览器时刻）、`fg`（前台切换）。
    pub fn capture_foreground_client_png_file(name_prefix: &str) -> Result<(String, u32, u32)> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            return Err(HarnessError::Windows("无前台窗口".into()));
        }

        let mut r = RECT::default();
        unsafe { GetClientRect(hwnd, &mut r) }
            .map_err(|e| HarnessError::Windows(format!("GetClientRect: {e}")))?;
        let cw = (r.right - r.left).max(1);
        let ch = (r.bottom - r.top).max(1);

        let data_dir = default_data_dir();
        let day = chrono::Local::now().format("%Y-%m-%d").to_string();
        let snap_dir = data_dir.join("snapshots").join(&day);
        fs::create_dir_all(&snap_dir).map_err(HarnessError::Io)?;
        let prefix = name_prefix.trim().trim_end_matches('-');
        let pfx = if prefix.is_empty() { "cap" } else { prefix };
        let name = format!("{}-{}.png", pfx, chrono::Utc::now().timestamp_millis());
        let abs_path = snap_dir.join(&name);

        let (w, h, rgba) = unsafe {
            let hdc_src = GetDC(Some(hwnd));
            if hdc_src.is_invalid() {
                return Err(HarnessError::Windows("GetDC(前台客户区) 失败".into()));
            }
            let hdc_mem = CreateCompatibleDC(Some(hdc_src));
            if hdc_mem.is_invalid() {
                let _ = ReleaseDC(Some(hwnd), hdc_src);
                return Err(HarnessError::Windows("CreateCompatibleDC 失败".into()));
            }
            let hbmp = CreateCompatibleBitmap(hdc_src, cw, ch);
            if hbmp.is_invalid() {
                let _ = DeleteDC(hdc_mem);
                let _ = ReleaseDC(Some(hwnd), hdc_src);
                return Err(HarnessError::Windows("CreateCompatibleBitmap 失败".into()));
            }
            let old: HGDIOBJ = SelectObject(hdc_mem, hbmp.into());

            let ok_print = PrintWindow(hwnd, hdc_mem, print_window_flags_full()).as_bool();
            if !ok_print {
                let _ = BitBlt(
                    hdc_mem,
                    0,
                    0,
                    cw,
                    ch,
                    Some(hdc_src),
                    0,
                    0,
                    SRCCOPY,
                )
                .is_ok();
            }

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: cw,
                    biHeight: -ch,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0 as u32,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [Default::default()],
            };

            let px = (cw * ch * 4) as usize;
            let mut bgra = vec![0u8; px];
            let lines = GetDIBits(
                hdc_mem,
                hbmp,
                0,
                ch as u32,
                Some(bgra.as_mut_ptr() as *mut c_void),
                &mut bmi,
                DIB_RGB_COLORS,
            );
            let _ = SelectObject(hdc_mem, old);
            let _ = DeleteObject(hbmp.into());
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(Some(hwnd), hdc_src);

            if lines == 0 {
                return Err(HarnessError::Windows("GetDIBits 失败".into()));
            }

            let w = cw as u32;
            let h = ch as u32;
            let mut rgba = Vec::with_capacity(px);
            for chunk in bgra.chunks_exact(4) {
                rgba.push(chunk[2]);
                rgba.push(chunk[1]);
                rgba.push(chunk[0]);
                rgba.push(chunk[3]);
            }
            (w, h, rgba)
        };

        let img = RgbaImage::from_raw(w, h, rgba).ok_or_else(|| HarnessError::Windows("图像缓冲无效".into()))?;
        img.save(&abs_path).map_err(|e| HarnessError::Windows(format!("保存 PNG: {e}")))?;

        let rel = normalize_rel_path(
            abs_path
                .strip_prefix(&data_dir)
                .map_err(|_| HarnessError::Windows("截图路径不在数据目录内".into()))?,
        );
        Ok((rel, w, h))
    }

    fn normalize_rel_path(path: &Path) -> String {
        path.iter()
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// 供 [`crate::recording`] 等复用：截取当前前台窗口客户区为 PNG（相对数据目录的路径）。
#[cfg(windows)]
pub fn capture_foreground_window_png(name_prefix: &str) -> Result<(String, u32, u32)> {
    windows_impl::capture_foreground_client_png_file(name_prefix)
}

#[cfg(not(windows))]
pub fn capture_foreground_window_png(_name_prefix: &str) -> Result<(String, u32, u32)> {
    Err(HarnessError::UnsupportedPlatform)
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Normalized desktop events (MVPv1 subset). Serialized with `type` tag for JSON/SQLite payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DesktopEvent {
    AppSwitch {
        app: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// 前台窗口客户区截图（与浏览器时刻相同采集方式），旧数据无此字段。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        image_rel: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width_px: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height_px: Option<u32>,
        at: DateTime<Utc>,
    },
    /// Clipboard changed; `text_preview` is truncated for storage. `char_len` is full UTF-8 char count when known.
    Clipboard {
        text_preview: String,
        char_len: u32,
        truncated: bool,
        at: DateTime<Utc>,
    },
    /// 用户通过快捷键触发的「学习快照」：主显示器截图，路径相对于应用数据目录（见 `default_data_dir`）。
    LearningSnapshot {
        /// 例如 `snapshots/2026-04-18/ls-1713456789012.png`
        image_rel: String,
        app: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        width_px: u32,
        height_px: u32,
        at: DateTime<Utc>,
    },
    /// 浏览器前台启发式：页签标题变化 / 哔哩哔哩等关键时机，截取前台窗口客户区。
    BrowserMoment {
        /// `bilibili_open` | `bilibili_video` | `chrome_new_tab` | `chrome_tab`（旧数据可能为 `bilibili` | `tab_switch`）
        trigger: String,
        browser_app: String,
        /// 完整窗口标题（含浏览器后缀）
        window_title: String,
        /// 从标题解析的页面一行（近似当前标签）
        page_title: String,
        /// 从页签/站点标题抽取的「学习内容」主题（如 B 站视频标题、网页主标题）
        #[serde(default)]
        learning_content: String,
        /// 简短中文摘要（展示用）
        summary: String,
        keywords: Vec<String>,
        image_rel: String,
        width_px: u32,
        height_px: u32,
        at: DateTime<Utc>,
    },
}

impl DesktopEvent {
    pub fn recorded_at(&self) -> DateTime<Utc> {
        match self {
            DesktopEvent::AppSwitch { at, .. }
            | DesktopEvent::Clipboard { at, .. }
            | DesktopEvent::LearningSnapshot { at, .. }
            | DesktopEvent::BrowserMoment { at, .. } => *at,
        }
    }

    pub fn app_switch_now(app: String, title: Option<String>) -> Self {
        DesktopEvent::AppSwitch {
            app,
            title,
            image_rel: None,
            width_px: None,
            height_px: None,
            at: Utc::now(),
        }
    }

    pub fn app_switch_with_window_shot(
        app: String,
        title: Option<String>,
        image_rel: String,
        width_px: u32,
        height_px: u32,
    ) -> Self {
        DesktopEvent::AppSwitch {
            app,
            title,
            image_rel: Some(image_rel),
            width_px: Some(width_px),
            height_px: Some(height_px),
            at: Utc::now(),
        }
    }

    pub fn clipboard_now(text_preview: String, char_len: u32, truncated: bool) -> Self {
        DesktopEvent::Clipboard {
            text_preview,
            char_len,
            truncated,
            at: Utc::now(),
        }
    }

    pub fn learning_snapshot_now(
        image_rel: String,
        app: String,
        title: Option<String>,
        width_px: u32,
        height_px: u32,
    ) -> Self {
        DesktopEvent::LearningSnapshot {
            image_rel,
            app,
            title,
            width_px,
            height_px,
            at: Utc::now(),
        }
    }

    pub fn browser_moment_now(
        trigger: String,
        browser_app: String,
        window_title: String,
        page_title: String,
        learning_content: String,
        summary: String,
        keywords: Vec<String>,
        image_rel: String,
        width_px: u32,
        height_px: u32,
    ) -> Self {
        DesktopEvent::BrowserMoment {
            trigger,
            browser_app,
            window_title,
            page_title,
            learning_content,
            summary,
            keywords,
            image_rel,
            width_px,
            height_px,
            at: Utc::now(),
        }
    }
}

//! One polling tick: foreground `app_switch`（Windows 下尽量附带前台窗口截图）+ clipboard。
//! 当前学习场景下，**仅 Google Chrome** 在窗口标题变化时写入 `BrowserMoment`（含客户区截图；含网页哔哩哔哩）。

use crate::browser_moment::{
    build_browser_moment_event, capture_foreground_window_png, is_learning_focus_tracked_exe,
    should_record_learning_focus_moment,
};
use crate::clipboard::{clipboard_sequence_number, try_read_unicode_clipboard_event};
use crate::current_foreground;
use crate::error::Result;
use crate::event::DesktopEvent;
use crate::store::TimelineStore;

/// Tracks last seen foreground and clipboard sequence so we only persist on change.
#[derive(Debug, Default)]
pub struct RecordingState {
    last_fg: Option<(String, Option<String>)>,
    last_clipboard_seq: Option<u32>,
    /// 仅当 `app` 为 Chrome 时：`(exe, 完整窗口标题)`，用于检测标签/页面标题变化。
    last_browser_title: Option<(String, String)>,
    /// 上次自动浏览器截图时间（毫秒时间戳），用于防抖。
    last_browser_shot_ms: Option<i64>,
}

impl RecordingState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Poll once: insert `app_switch` / `clipboard` rows when changed.
    pub fn poll_once(&mut self, store: &TimelineStore) -> Result<()> {
        let fg = current_foreground()?;
        let key = (fg.app.clone(), fg.title.clone());
        if self.last_fg.as_ref() != Some(&key) {
            self.last_fg = Some(key.clone());
            let ev = {
                #[cfg(windows)]
                {
                    match capture_foreground_window_png("fg") {
                        Ok((rel, w, h)) => DesktopEvent::app_switch_with_window_shot(
                            fg.app.clone(),
                            fg.title.clone(),
                            rel,
                            w,
                            h,
                        ),
                        Err(e) => {
                            tracing::warn!(target: "harness", "前台切换截图失败（仍记录进程）: {e}");
                            DesktopEvent::app_switch_now(fg.app.clone(), fg.title.clone())
                        }
                    }
                }
                #[cfg(not(windows))]
                {
                    DesktopEvent::app_switch_now(fg.app.clone(), fg.title.clone())
                }
            };
            store.insert_event(&ev)?;
        }

        if is_learning_focus_tracked_exe(&fg.app) {
            let full = fg.title.clone().unwrap_or_default();
            if !full.is_empty() {
                let prev = self.last_browser_title.as_ref();
                let prev_same_app = prev
                    .filter(|(a, _)| a == &fg.app)
                    .map(|(_, t)| t.as_str());
                let trigger = should_record_learning_focus_moment(&full, prev_same_app, &fg.app);
                if let Some(tr) = trigger {
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let min_gap = match tr {
                        "bilibili_video" | "bilibili_open" => 4_000i64,
                        "chrome_new_tab" => 6_000i64,
                        "chrome_tab" => 12_000i64,
                        _ => 8_000i64,
                    };
                    let ok = self
                        .last_browser_shot_ms
                        .map(|p| now_ms - p >= min_gap)
                        .unwrap_or(true);
                    if ok {
                        match build_browser_moment_event(tr, fg.app.clone(), full.clone()) {
                            Ok(ev) => {
                                store.insert_event(&ev)?;
                                self.last_browser_shot_ms = Some(now_ms);
                            }
                            Err(e) => {
                                tracing::warn!(target: "harness", "学习聚焦时刻截图失败: {e}");
                            }
                        }
                    }
                }
                self.last_browser_title = Some((fg.app.clone(), full));
            }
        } else {
            self.last_browser_title = None;
        }

        let seq = clipboard_sequence_number()?;
        match self.last_clipboard_seq {
            None => {
                self.last_clipboard_seq = Some(seq);
            }
            Some(prev) if seq != prev => {
                self.last_clipboard_seq = Some(seq);
                if let Some(ev) = try_read_unicode_clipboard_event()? {
                    store.insert_event(&ev)?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}

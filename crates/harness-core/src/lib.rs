mod browser_moment;
mod clipboard;
mod error;
mod event;
mod foreground;
mod learning_snapshot;
mod recording;
mod store;

pub use error::HarnessError;
pub use event::DesktopEvent;
pub use foreground::{current_foreground, ForegroundSnapshot};
pub use browser_moment::{
    build_browser_moment_event, extract_learning_content, is_bilibili_desktop_exe, is_bilibili_title,
    is_browser_app, is_chrome_exe, is_learning_focus_tracked_exe, should_record_learning_focus_moment,
    strip_bilibili_site_suffix,
};
pub use learning_snapshot::build_learning_snapshot_event;
pub use recording::RecordingState;
pub use store::{AppCount, TimelineStore, WindowStats};

/// Resolves the SQLite file path. Priority:
/// 1. `HARNESS_DB_PATH` — full path to the `.db` file.
/// 2. `GUANGHE_DATA_DIR` — directory; uses `harness.db` inside it (`npm run tauri:*` 会设为仓库内 `data/`).
/// 3. Current dir is repo root (存在 `package.json`) → `./data/harness.db`（便于 `cargo run` 时在 SchedulePCagent 下落到 D 盘项目目录）。
/// 4. Fallback: `%LOCALAPPDATA%/GuangheDesktopObserve/harness.db`（仅当以上都不适用时）。
pub fn default_db_path() -> std::path::PathBuf {
    use std::path::PathBuf;

    if let Ok(p) = std::env::var("HARNESS_DB_PATH") {
        return PathBuf::from(p);
    }
    default_data_dir().join("harness.db")
}

/// 应用数据目录（截图 `snapshots/` 等；若设置了 `HARNESS_DB_PATH` 则为该文件所在目录）。
pub fn default_data_dir() -> std::path::PathBuf {
    use std::path::{Path, PathBuf};

    if let Ok(p) = std::env::var("HARNESS_DB_PATH") {
        let pb = Path::new(&p);
        if let Some(parent) = pb.parent() {
            return parent.to_path_buf();
        }
        return PathBuf::from(".");
    }
    if let Ok(dir) = std::env::var("GUANGHE_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("package.json").is_file() {
            return cwd.join("data");
        }
    }
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("GuangheDesktopObserve")
}

/// 将相对于 [`default_data_dir`] 的路径解析为绝对路径（禁止 `..` 等逃逸）。
pub fn resolve_data_relative_path(rel: &str) -> std::result::Result<std::path::PathBuf, HarnessError> {
    use std::path::{Component, Path, PathBuf};

    let base = default_data_dir();
    std::fs::create_dir_all(&base).map_err(HarnessError::Io)?;
    let base_canon = base.canonicalize().map_err(HarnessError::Io)?;

    let rel_path = Path::new(rel.trim());
    if rel_path.as_os_str().is_empty() {
        return Err(HarnessError::InvalidPath("路径为空".into()));
    }
    if rel_path.is_absolute() {
        return Err(HarnessError::InvalidPath("不允许绝对路径".into()));
    }

    let mut full = PathBuf::new();
    full.push(&base_canon);
    for c in rel_path.components() {
        match c {
            Component::Normal(part) => full.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(HarnessError::InvalidPath("非法的相对路径".into()));
            }
        }
    }

    let full = full.canonicalize().map_err(HarnessError::Io)?;
    if !full.starts_with(&base_canon) {
        return Err(HarnessError::InvalidPath("路径越界".into()));
    }
    Ok(full)
}

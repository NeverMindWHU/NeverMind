use crate::error::{HarnessError, Result};
use crate::event::DesktopEvent;
use chrono::{DateTime, Local, TimeZone, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

pub struct TimelineStore {
    conn: Connection,
}

impl TimelineStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS desktop_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                recorded_at TEXT NOT NULL,
                payload TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_desktop_events_recorded_at
                ON desktop_events (recorded_at);
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn insert_event(&self, event: &DesktopEvent) -> Result<i64> {
        let recorded_at = event.recorded_at().to_rfc3339();
        let payload = serde_json::to_string(event)?;
        self.conn.execute(
            "INSERT INTO desktop_events (recorded_at, payload) VALUES (?1, ?2)",
            params![recorded_at, payload],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Events with `recorded_at >= start` (UTC), ascending by id.
    pub fn events_since_utc(&self, start: DateTime<Utc>) -> Result<Vec<DesktopEvent>> {
        let start_s = start.to_rfc3339();
        let mut stmt = self.conn.prepare(
            "SELECT payload FROM desktop_events WHERE recorded_at >= ?1 ORDER BY id ASC",
        )?;
        let mut rows = stmt.query(params![start_s])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let s: String = row.get(0)?;
            let ev: DesktopEvent = serde_json::from_str(&s)?;
            out.push(ev);
        }
        Ok(out)
    }

    /// All events whose `recorded_at` falls on the **local** calendar day (start inclusive, next day exclusive).
    pub fn events_on_local_day(&self, day: chrono::NaiveDate) -> Result<Vec<DesktopEvent>> {
        let naive_midnight = day
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| HarnessError::InvalidTime(format!("invalid date {day}")))?;
        let start_local = Local
            .from_local_datetime(&naive_midnight)
            .single()
            .ok_or_else(|| {
                HarnessError::InvalidTime(format!("ambiguous local midnight for {day}"))
            })?;
        let end_local = start_local + chrono::Duration::days(1);
        let start_utc = start_local.with_timezone(&Utc).to_rfc3339();
        let end_utc = end_local.with_timezone(&Utc).to_rfc3339();

        let mut stmt = self.conn.prepare(
            "SELECT payload FROM desktop_events WHERE recorded_at >= ?1 AND recorded_at < ?2 ORDER BY id ASC",
        )?;
        let mut rows = stmt.query(params![start_utc, end_utc])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let s: String = row.get(0)?;
            let ev: DesktopEvent = serde_json::from_str(&s)?;
            out.push(ev);
        }
        Ok(out)
    }

    /// Today's events in local timezone.
    pub fn events_today_local(&self) -> Result<Vec<DesktopEvent>> {
        let today = Local::now().date_naive();
        self.events_on_local_day(today)
    }

    /// Stats for the last `minutes` (rolling window from now, UTC-based duration).
    pub fn analyze_window_minutes(&self, minutes: i64) -> Result<WindowStats> {
        let end = Utc::now();
        let start = end - chrono::Duration::minutes(minutes);
        let events = self.events_since_utc(start)?;
        let mut app_switch_count = 0u32;
        let mut clipboard_event_count = 0u32;
        let mut learning_snapshot_count = 0u32;
        let mut browser_moment_count = 0u32;
        let mut app_counts: HashMap<String, u32> = HashMap::new();

        for ev in &events {
            match ev {
                DesktopEvent::AppSwitch { app, .. } => {
                    app_switch_count += 1;
                    *app_counts.entry(app.clone()).or_insert(0) += 1;
                }
                DesktopEvent::Clipboard { .. } => {
                    clipboard_event_count += 1;
                }
                DesktopEvent::LearningSnapshot { .. } => {
                    learning_snapshot_count += 1;
                }
                DesktopEvent::BrowserMoment { .. } => {
                    browser_moment_count += 1;
                }
            }
        }

        let mut dominant: Vec<AppCount> = app_counts
            .into_iter()
            .map(|(app, count)| AppCount { app, count })
            .collect();
        dominant.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.app.cmp(&b.app)));
        dominant.truncate(10);

        Ok(WindowStats {
            window_minutes: minutes,
            interval_start_utc: start,
            interval_end_utc: end,
            app_switch_count,
            clipboard_event_count,
            learning_snapshot_count,
            browser_moment_count,
            dominant_apps: dominant,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AppCount {
    pub app: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowStats {
    pub window_minutes: i64,
    pub interval_start_utc: DateTime<Utc>,
    pub interval_end_utc: DateTime<Utc>,
    pub app_switch_count: u32,
    pub clipboard_event_count: u32,
    pub learning_snapshot_count: u32,
    pub browser_moment_count: u32,
    pub dominant_apps: Vec<AppCount>,
}

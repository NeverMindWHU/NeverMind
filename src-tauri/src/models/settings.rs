use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppSettings {
    pub id: i64,
    pub theme: String,
    pub language: String,
    pub notification_enabled: bool,
    pub review_reminder_enabled: bool,
    pub review_reminder_time: String,
    pub default_model_profile_id: Option<String>,
    pub export_directory: Option<String>,
    pub screenshot_shortcut: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ModelProfile {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub endpoint: String,
    pub model: Option<String>,
    pub timeout_ms: i64,
    pub api_key_secret_ref: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertSettings {
    pub theme: String,
    pub language: String,
    pub notification_enabled: bool,
    pub review_reminder_enabled: bool,
    pub review_reminder_time: String,
    pub default_model_profile_id: Option<String>,
    pub export_directory: Option<String>,
    pub screenshot_shortcut: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertModelProfile {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub endpoint: String,
    pub model: Option<String>,
    pub timeout_ms: i64,
    pub api_key_secret_ref: Option<String>,
    pub is_default: bool,
}

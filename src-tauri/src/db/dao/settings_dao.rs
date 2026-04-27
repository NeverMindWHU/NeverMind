use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;

use crate::{
    models::settings::{AppSettings, ModelProfile, UpsertModelProfile, UpsertSettings},
    utils::error::AppResult,
};

#[async_trait]
pub trait SettingsDao: Send + Sync {
    async fn get_settings(&self) -> AppResult<Option<AppSettings>>;
    async fn upsert_settings(&self, input: &UpsertSettings) -> AppResult<AppSettings>;
    async fn list_model_profiles(&self) -> AppResult<Vec<ModelProfile>>;
    async fn upsert_model_profile(&self, input: &UpsertModelProfile) -> AppResult<ModelProfile>;
}

#[derive(Clone)]
pub struct SqliteSettingsDao {
    pool: SqlitePool,
}

impl SqliteSettingsDao {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettingsDao for SqliteSettingsDao {
    async fn get_settings(&self) -> AppResult<Option<AppSettings>> {
        let settings = sqlx::query_as::<_, AppSettings>(
            r#"
            SELECT id, theme, language, notification_enabled, review_reminder_enabled,
                   review_reminder_time, default_model_profile_id, export_directory, screenshot_shortcut, updated_at
            FROM settings
            WHERE id = 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(settings)
    }

    async fn upsert_settings(&self, input: &UpsertSettings) -> AppResult<AppSettings> {
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO settings (
                id, theme, language, notification_enabled, review_reminder_enabled,
                review_reminder_time, default_model_profile_id, export_directory, screenshot_shortcut, updated_at
            )
            VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                theme = excluded.theme,
                language = excluded.language,
                notification_enabled = excluded.notification_enabled,
                review_reminder_enabled = excluded.review_reminder_enabled,
                review_reminder_time = excluded.review_reminder_time,
                default_model_profile_id = excluded.default_model_profile_id,
                export_directory = excluded.export_directory,
                screenshot_shortcut = excluded.screenshot_shortcut,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&input.theme)
        .bind(&input.language)
        .bind(input.notification_enabled)
        .bind(input.review_reminder_enabled)
        .bind(&input.review_reminder_time)
        .bind(&input.default_model_profile_id)
        .bind(&input.export_directory)
        .bind(&input.screenshot_shortcut)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(self.get_settings().await?.expect("settings should exist after upsert"))
    }

    async fn list_model_profiles(&self) -> AppResult<Vec<ModelProfile>> {
        let profiles = sqlx::query_as::<_, ModelProfile>(
            r#"
            SELECT id, name, provider, endpoint, model, timeout_ms,
                   api_key_secret_ref, is_default, created_at, updated_at
            FROM model_profiles
            ORDER BY is_default DESC, created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(profiles)
    }

    async fn upsert_model_profile(&self, input: &UpsertModelProfile) -> AppResult<ModelProfile> {
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO model_profiles (
                id, name, provider, endpoint, model, timeout_ms,
                api_key_secret_ref, is_default, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                provider = excluded.provider,
                endpoint = excluded.endpoint,
                model = excluded.model,
                timeout_ms = excluded.timeout_ms,
                api_key_secret_ref = excluded.api_key_secret_ref,
                is_default = excluded.is_default,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&input.id)
        .bind(&input.name)
        .bind(&input.provider)
        .bind(&input.endpoint)
        .bind(&input.model)
        .bind(input.timeout_ms)
        .bind(&input.api_key_secret_ref)
        .bind(input.is_default)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let profile = sqlx::query_as::<_, ModelProfile>(
            r#"
            SELECT id, name, provider, endpoint, model, timeout_ms,
                   api_key_secret_ref, is_default, created_at, updated_at
            FROM model_profiles
            WHERE id = ?
            "#,
        )
        .bind(&input.id)
        .fetch_one(&self.pool)
        .await?;

        Ok(profile)
    }
}

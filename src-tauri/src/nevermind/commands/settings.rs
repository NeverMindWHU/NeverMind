use std::{path::Path, time::Instant};

use chrono::{DateTime, NaiveTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;
use uuid::Uuid;

use crate::nevermind::{
    models::settings::{AppSettings, ModelProfile, UpsertModelProfile, UpsertSettings},
    state::AppState,
    utils::error::{AppError, AppResult},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: T,
}

impl<T> CommandResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSettingsData {
    pub export_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsData {
    pub theme: String,
    pub language: String,
    pub notification_enabled: bool,
    pub review_reminder_enabled: bool,
    pub review_reminder_time: String,
    pub default_model_profile_id: Option<String>,
    pub storage: StorageSettingsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsInput {
    pub theme: String,
    pub language: String,
    pub notification_enabled: bool,
    pub review_reminder_enabled: bool,
    pub review_reminder_time: String,
    pub storage: StorageSettingsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsData {
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelProfileItem {
    pub profile_id: String,
    pub name: String,
    pub provider: String,
    pub endpoint: String,
    pub is_default: bool,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListModelProfilesData {
    pub items: Vec<ModelProfileItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveModelProfileInput {
    pub profile_id: Option<String>,
    pub name: String,
    pub provider: String,
    pub endpoint: String,
    pub api_key: String,
    pub model: Option<String>,
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveModelProfileData {
    pub profile_id: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestModelProfileInput {
    pub profile_id: Option<String>,
    pub provider: String,
    pub endpoint: String,
    pub api_key: String,
    pub model: Option<String>,
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestModelProfileData {
    pub reachable: bool,
    pub latency_ms: u128,
}

const DEFAULT_THEME: &str = "system";
const DEFAULT_LANGUAGE: &str = "zh-CN";
const DEFAULT_REVIEW_REMINDER_TIME: &str = "09:00";
pub async fn get_settings(state: &AppState) -> AppResult<CommandResponse<AppSettingsData>> {
    let data = state
        .settings_dao
        .get_settings()
        .await?
        .map(map_settings)
        .unwrap_or_else(default_settings);

    Ok(CommandResponse::ok(data))
}

pub async fn update_settings(
    state: &AppState,
    input: UpdateSettingsInput,
) -> AppResult<CommandResponse<UpdateSettingsData>> {
    validate_settings_input(&input)?;

    let default_model_profile_id = state
        .settings_dao
        .get_settings()
        .await?
        .and_then(|settings| settings.default_model_profile_id);

    let saved = state
        .settings_dao
        .upsert_settings(&UpsertSettings {
            theme: input.theme,
            language: input.language,
            notification_enabled: input.notification_enabled,
            review_reminder_enabled: input.review_reminder_enabled,
            review_reminder_time: input.review_reminder_time,
            default_model_profile_id,
            export_directory: input.storage.export_directory,
        })
        .await?;

    Ok(CommandResponse::ok(UpdateSettingsData {
        updated_at: saved.updated_at,
    }))
}

pub async fn list_model_profiles(
    state: &AppState,
) -> AppResult<CommandResponse<ListModelProfilesData>> {
    let items = state
        .settings_dao
        .list_model_profiles()
        .await?
        .into_iter()
        .map(map_model_profile)
        .collect();

    Ok(CommandResponse::ok(ListModelProfilesData { items }))
}

pub async fn save_model_profile(
    state: &AppState,
    input: SaveModelProfileInput,
) -> AppResult<CommandResponse<SaveModelProfileData>> {
    validate_model_profile_fields(
        &input.name,
        &input.provider,
        &input.endpoint,
        &input.api_key,
        input.timeout_ms,
    )?;

    let existing_profiles = state.settings_dao.list_model_profiles().await?;
    let profile_id = input.profile_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let is_first_profile = existing_profiles.is_empty();
    let is_default = existing_profiles
        .iter()
        .find(|profile| profile.id == profile_id)
        .map(|profile| profile.is_default)
        .unwrap_or(is_first_profile);

    let saved = state
        .settings_dao
        .upsert_model_profile(&UpsertModelProfile {
            id: profile_id.clone(),
            name: input.name,
            provider: input.provider,
            endpoint: normalize_endpoint(&input.endpoint)?,
            model: input.model,
            timeout_ms: input.timeout_ms,
            // 当前先直接存入 DB 字段，后续可以切换为系统钥匙串/安全存储引用。
            api_key_secret_ref: Some(input.api_key),
            is_default,
        })
        .await?;

    if is_first_profile {
        let existing_settings = state.settings_dao.get_settings().await?;
        let defaults = existing_settings
            .as_ref()
            .map(app_settings_to_upsert)
            .unwrap_or_else(|| UpsertSettings {
                theme: DEFAULT_THEME.to_string(),
                language: DEFAULT_LANGUAGE.to_string(),
                notification_enabled: true,
                review_reminder_enabled: true,
                review_reminder_time: DEFAULT_REVIEW_REMINDER_TIME.to_string(),
                default_model_profile_id: Some(saved.id.clone()),
                export_directory: None,
            });

        state
            .settings_dao
            .upsert_settings(&UpsertSettings {
                default_model_profile_id: Some(saved.id.clone()),
                ..defaults
            })
            .await?;
    }

    Ok(CommandResponse::ok(SaveModelProfileData {
        profile_id: saved.id,
        updated_at: saved.updated_at,
    }))
}

pub async fn test_model_profile(
    input: TestModelProfileInput,
) -> AppResult<CommandResponse<TestModelProfileData>> {
    validate_model_profile_fields(
        "temporary-profile",
        &input.provider,
        &input.endpoint,
        &input.api_key,
        input.timeout_ms,
    )?;

    let request_url = chat_completions_url(&input.provider, &input.endpoint)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(input.timeout_ms as u64))
        .build()
        .map_err(|err| AppError::ModelConnectionFailed {
            message: format!("初始化 HTTP 客户端失败: {}", err),
        })?;

    let started_at = Instant::now();
    let response = client
        .post(request_url)
        .bearer_auth(input.api_key)
        .json(&json!({
            "model": input.model.unwrap_or_else(|| "health-check".to_string()),
            "messages": [
                { "role": "user", "content": "ping" }
            ],
            "max_tokens": 1,
            "temperature": 0
        }))
        .send()
        .await
        .map_err(|err| {
            if err.is_timeout() {
                AppError::ModelConnectionFailed {
                    message: "模型请求超时".to_string(),
                }
            } else {
                AppError::ModelConnectionFailed {
                    message: format!("模型请求失败: {}", err),
                }
            }
        })?;

    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(AppError::ModelAuthFailed),
        status if status.is_success() => Ok(CommandResponse::ok(TestModelProfileData {
            reachable: true,
            latency_ms: started_at.elapsed().as_millis(),
        })),
        status => Err(AppError::ModelConnectionFailed {
            message: format!("模型返回 HTTP {}", status.as_u16()),
        }),
    }
}

fn default_settings() -> AppSettingsData {
    AppSettingsData {
        theme: DEFAULT_THEME.to_string(),
        language: DEFAULT_LANGUAGE.to_string(),
        notification_enabled: true,
        review_reminder_enabled: true,
        review_reminder_time: DEFAULT_REVIEW_REMINDER_TIME.to_string(),
        default_model_profile_id: None,
        storage: StorageSettingsData {
            export_directory: None,
        },
    }
}

fn map_settings(settings: AppSettings) -> AppSettingsData {
    AppSettingsData {
        theme: settings.theme,
        language: settings.language,
        notification_enabled: settings.notification_enabled,
        review_reminder_enabled: settings.review_reminder_enabled,
        review_reminder_time: settings.review_reminder_time,
        default_model_profile_id: settings.default_model_profile_id,
        storage: StorageSettingsData {
            export_directory: settings.export_directory,
        },
    }
}

fn map_model_profile(profile: ModelProfile) -> ModelProfileItem {
    ModelProfileItem {
        profile_id: profile.id,
        name: profile.name,
        provider: profile.provider,
        endpoint: profile.endpoint,
        is_default: profile.is_default,
        is_available: true,
    }
}

fn app_settings_to_upsert(settings: &AppSettings) -> UpsertSettings {
    UpsertSettings {
        theme: settings.theme.clone(),
        language: settings.language.clone(),
        notification_enabled: settings.notification_enabled,
        review_reminder_enabled: settings.review_reminder_enabled,
        review_reminder_time: settings.review_reminder_time.clone(),
        default_model_profile_id: settings.default_model_profile_id.clone(),
        export_directory: settings.export_directory.clone(),
    }
}

fn validate_settings_input(input: &UpdateSettingsInput) -> AppResult<()> {
    validate_theme(&input.theme)?;
    validate_language(&input.language)?;
    validate_review_reminder_time(&input.review_reminder_time)?;

    if let Some(export_directory) = &input.storage.export_directory {
        validate_export_directory(export_directory)?;
    }

    Ok(())
}

fn validate_model_profile_fields(
    name: &str,
    provider: &str,
    endpoint: &str,
    api_key: &str,
    timeout_ms: i64,
) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::InvalidSettings {
            message: "模型配置名称不能为空".to_string(),
        });
    }
    validate_provider(provider)?;
    normalize_endpoint(endpoint)?;
    if api_key.trim().is_empty() {
        return Err(AppError::InvalidSettings {
            message: "apiKey 不能为空".to_string(),
        });
    }
    if timeout_ms <= 0 {
        return Err(AppError::InvalidSettings {
            message: "timeoutMs 必须大于 0".to_string(),
        });
    }

    Ok(())
}

fn validate_theme(theme: &str) -> AppResult<()> {
    match theme {
        "light" | "dark" | "system" => Ok(()),
        other => Err(AppError::InvalidSettings {
            message: format!("theme 非法: {}", other),
        }),
    }
}

fn validate_language(language: &str) -> AppResult<()> {
    match language {
        "zh-CN" | "en-US" => Ok(()),
        other => Err(AppError::InvalidSettings {
            message: format!("language 非法: {}", other),
        }),
    }
}

fn validate_provider(provider: &str) -> AppResult<()> {
    match provider {
        "openai-compatible" | "qwen" | "custom" => Ok(()),
        other => Err(AppError::InvalidSettings {
            message: format!("provider 非法: {}", other),
        }),
    }
}

fn validate_review_reminder_time(value: &str) -> AppResult<()> {
    NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| AppError::InvalidTimeFormat {
        message: format!("reviewReminderTime 非法: {}", value),
    })?;
    Ok(())
}

fn validate_export_directory(path: &str) -> AppResult<()> {
    if !Path::new(path).is_absolute() {
        return Err(AppError::InvalidPath {
            message: format!("导出目录必须为绝对路径: {}", path),
        });
    }

    Ok(())
}

fn normalize_endpoint(endpoint: &str) -> AppResult<String> {
    let trimmed = endpoint.trim();
    let parsed = Url::parse(trimmed).map_err(|_| AppError::InvalidSettings {
        message: format!("endpoint 非法: {}", endpoint),
    })?;

    match parsed.scheme() {
        "http" | "https" => Ok(trimmed.trim_end_matches('/').to_string()),
        _ => Err(AppError::InvalidSettings {
            message: format!("endpoint 必须使用 http/https: {}", endpoint),
        }),
    }
}

fn chat_completions_url(provider: &str, endpoint: &str) -> AppResult<String> {
    validate_provider(provider)?;
    let normalized = normalize_endpoint(endpoint)?;
    if normalized.ends_with("/chat/completions") {
        Ok(normalized)
    } else {
        Ok(format!("{}/chat/completions", normalized))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::{migrate::Migrator, sqlite::SqlitePoolOptions};

    use super::{
        get_settings, save_model_profile, update_settings, validate_review_reminder_time,
        SaveModelProfileInput, StorageSettingsData, UpdateSettingsInput,
    };
    use crate::nevermind::state::AppState;

    static TEST_MIGRATOR: Migrator = sqlx::migrate!("./migrations");

    #[tokio::test]
    async fn get_settings_returns_defaults_when_table_is_empty() {
        let state = setup_test_state().await;

        let response = get_settings(&state).await.unwrap();

        assert!(response.success);
        assert_eq!(response.data.theme, "system");
        assert_eq!(response.data.language, "zh-CN");
        assert_eq!(response.data.review_reminder_time, "09:00");
        assert_eq!(response.data.storage.export_directory, None);
    }

    #[tokio::test]
    async fn update_settings_persists_values() {
        let state = setup_test_state().await;

        let response = update_settings(
            &state,
            UpdateSettingsInput {
                theme: "dark".to_string(),
                language: "en-US".to_string(),
                notification_enabled: false,
                review_reminder_enabled: true,
                review_reminder_time: "08:30".to_string(),
                storage: StorageSettingsData {
                    export_directory: Some("/tmp/nevermind".to_string()),
                },
            },
        )
        .await
        .unwrap();

        assert!(response.success);

        let settings = state.settings_dao.get_settings().await.unwrap().unwrap();
        assert_eq!(settings.theme, "dark");
        assert_eq!(settings.language, "en-US");
        assert!(!settings.notification_enabled);
        assert_eq!(settings.review_reminder_time, "08:30");
        assert_eq!(settings.export_directory.as_deref(), Some("/tmp/nevermind"));
    }

    #[tokio::test]
    async fn save_first_model_profile_sets_default_profile_id() {
        let state = setup_test_state().await;

        let response = save_model_profile(
            &state,
            SaveModelProfileInput {
                profile_id: None,
                name: "Qwen Default".to_string(),
                provider: "qwen".to_string(),
                endpoint: "https://api.example.com".to_string(),
                api_key: "secret".to_string(),
                model: Some("demo-model".to_string()),
                timeout_ms: 30_000,
            },
        )
        .await
        .unwrap();

        assert!(response.success);

        let settings = state.settings_dao.get_settings().await.unwrap().unwrap();
        assert_eq!(
            settings.default_model_profile_id.as_deref(),
            Some(response.data.profile_id.as_str())
        );

        let profiles = state.settings_dao.list_model_profiles().await.unwrap();
        assert_eq!(profiles.len(), 1);
        assert!(profiles[0].is_default);
        assert_eq!(profiles[0].api_key_secret_ref.as_deref(), Some("secret"));
    }

    #[test]
    fn invalid_review_time_is_rejected() {
        let err = validate_review_reminder_time("25:61").unwrap_err();
        assert_eq!(err.code(), "INVALID_TIME_FORMAT");
    }

    async fn setup_test_state() -> AppState {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        TEST_MIGRATOR.run(&pool).await.unwrap();
        AppState::from_pool(pool)
    }
}

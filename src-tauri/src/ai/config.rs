//! 火山引擎方舟（Volcengine Ark / 豆包）LLM 运行时配置。
//!
//! 所有字段都支持环境变量覆盖，便于在本地开发、CI、生产之间切换。
//! 接口文档：<https://www.volcengine.com/docs/82379>

use std::time::Duration;

use crate::utils::error::{AppError, AppResult};

/// 默认指向北京区端点。
pub const DEFAULT_API_BASE: &str = "https://ark.cn-beijing.volces.com/api/v3";
/// 默认模型 ID。
pub const DEFAULT_MODEL: &str = "doubao-seed-2-0-lite-260215";
/// 默认请求超时。
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
/// 默认采样温度。卡片生成偏事实性，取中等值即可。
pub const DEFAULT_TEMPERATURE: f32 = 0.7;

#[derive(Debug, Clone)]
pub struct ArkConfig {
    /// API Key，必填；建议通过环境变量 `ARK_API_KEY` 注入，切勿硬编码。
    pub api_key: String,
    /// Ark 的基础 URL，例如 `https://ark.cn-beijing.volces.com/api/v3`。
    pub api_base: String,
    /// 模型 ID（又称 endpoint_id），例如 `doubao-seed-2-0-lite-260215`。
    pub model: String,
    /// 单次请求超时。
    pub timeout: Duration,
    /// 采样温度。
    pub temperature: f32,
}

impl ArkConfig {
    /// 从环境变量读取配置。
    ///
    /// | 变量 | 必填 | 默认值 |
    /// |---|---|---|
    /// | `ARK_API_KEY`  | 是 | —— |
    /// | `ARK_API_BASE` | 否 | `DEFAULT_API_BASE` |
    /// | `ARK_MODEL`    | 否 | `DEFAULT_MODEL` |
    pub fn from_env() -> AppResult<Self> {
        let api_key = env_non_empty("ARK_API_KEY").ok_or_else(|| AppError::AiUnavailable {
            message: "ARK_API_KEY 未设置，请在 .env 或运行环境中提供".into(),
        })?;

        let api_base = env_non_empty("ARK_API_BASE").unwrap_or_else(|| DEFAULT_API_BASE.to_string());
        let model = env_non_empty("ARK_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string());

        Ok(Self {
            api_key,
            api_base: api_base.trim_end_matches('/').to_string(),
            model,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            temperature: DEFAULT_TEMPERATURE,
        })
    }

    /// 拼接 `/chat/completions` 端点。
    pub fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.api_base)
    }
}

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_completions_url_joins_correctly() {
        let cfg = ArkConfig {
            api_key: "k".into(),
            api_base: "https://example.com/api/v3".into(),
            model: "m".into(),
            timeout: Duration::from_secs(1),
            temperature: 0.5,
        };
        assert_eq!(
            cfg.chat_completions_url(),
            "https://example.com/api/v3/chat/completions"
        );
    }

    #[test]
    fn from_env_missing_key_returns_unavailable() {
        // 保证该测试下 ARK_API_KEY 不存在。
        std::env::remove_var("ARK_API_KEY");
        let err = ArkConfig::from_env().unwrap_err();
        assert_eq!(err.code(), "AI_UNAVAILABLE");
    }
}

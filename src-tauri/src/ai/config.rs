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
/// 纯文本通路默认超时。
///
/// 30s 对 200–500 tokens 的文本场景足够，但对多模态会大概率触顶，
/// 所以这里稍微放宽一点。
pub const DEFAULT_TEXT_TIMEOUT_MS: u64 = 90_000;
/// 多模态（含图片）通路默认超时。
///
/// 图片场景耗时来源：
/// 1. 前端 `data:image/...;base64,` 上传（多张合计可到几十 MB）；
/// 2. 豆包视觉理解 + 文本生成（seed-2.0 通常 15–60s，峰值更久）；
/// 3. 响应下载。
/// 三段串起来 30s 基本不够用，默认放到 3 分钟更稳。
pub const DEFAULT_VISION_TIMEOUT_MS: u64 = 180_000;
/// 连接阶段（TCP/TLS）超时。与整体超时独立，避免连接卡住吃掉全部预算。
pub const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 15_000;
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
    /// 连接（TCP/TLS）超时。
    pub connect_timeout: Duration,
    /// 纯文本通路的单次请求超时。
    pub text_timeout: Duration,
    /// 多模态（带图片）通路的单次请求超时。
    pub vision_timeout: Duration,
    /// 采样温度。
    pub temperature: f32,
}

impl ArkConfig {
    /// 从环境变量读取配置。
    ///
    /// | 变量 | 必填 | 默认值 |
    /// |---|---|---|
    /// | `ARK_API_KEY`            | 是 | —— |
    /// | `ARK_API_BASE`           | 否 | `DEFAULT_API_BASE` |
    /// | `ARK_MODEL`              | 否 | `DEFAULT_MODEL` |
    /// | `ARK_TIMEOUT_MS`         | 否 | `DEFAULT_TEXT_TIMEOUT_MS`（文本通路） |
    /// | `ARK_VISION_TIMEOUT_MS`  | 否 | `DEFAULT_VISION_TIMEOUT_MS`（多模态通路） |
    /// | `ARK_CONNECT_TIMEOUT_MS` | 否 | `DEFAULT_CONNECT_TIMEOUT_MS` |
    pub fn from_env() -> AppResult<Self> {
        let api_key = env_non_empty("ARK_API_KEY").ok_or_else(|| AppError::AiUnavailable {
            message: "ARK_API_KEY 未设置，请在 .env 或运行环境中提供".into(),
        })?;

        let api_base = env_non_empty("ARK_API_BASE").unwrap_or_else(|| DEFAULT_API_BASE.to_string());
        let model = env_non_empty("ARK_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string());

        let text_timeout_ms = env_u64("ARK_TIMEOUT_MS").unwrap_or(DEFAULT_TEXT_TIMEOUT_MS);
        let vision_timeout_ms =
            env_u64("ARK_VISION_TIMEOUT_MS").unwrap_or(DEFAULT_VISION_TIMEOUT_MS);
        let connect_timeout_ms =
            env_u64("ARK_CONNECT_TIMEOUT_MS").unwrap_or(DEFAULT_CONNECT_TIMEOUT_MS);

        Ok(Self {
            api_key,
            api_base: api_base.trim_end_matches('/').to_string(),
            model,
            connect_timeout: Duration::from_millis(connect_timeout_ms),
            text_timeout: Duration::from_millis(text_timeout_ms),
            vision_timeout: Duration::from_millis(vision_timeout_ms),
            temperature: DEFAULT_TEMPERATURE,
        })
    }

    /// 拼接 `/chat/completions` 端点。
    pub fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.api_base)
    }

    /// 根据请求是否带图片，返回整体请求超时。
    pub fn timeout_for(&self, has_images: bool) -> Duration {
        if has_images {
            self.vision_timeout
        } else {
            self.text_timeout
        }
    }
}

fn env_u64(key: &str) -> Option<u64> {
    env_non_empty(key).and_then(|v| v.parse::<u64>().ok())
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

    fn sample_cfg() -> ArkConfig {
        ArkConfig {
            api_key: "k".into(),
            api_base: "https://example.com/api/v3".into(),
            model: "m".into(),
            connect_timeout: Duration::from_secs(5),
            text_timeout: Duration::from_secs(30),
            vision_timeout: Duration::from_secs(120),
            temperature: 0.5,
        }
    }

    #[test]
    fn chat_completions_url_joins_correctly() {
        let cfg = sample_cfg();
        assert_eq!(
            cfg.chat_completions_url(),
            "https://example.com/api/v3/chat/completions"
        );
    }

    #[test]
    fn timeout_for_picks_vision_when_images_present() {
        let cfg = sample_cfg();
        assert_eq!(cfg.timeout_for(false), Duration::from_secs(30));
        assert_eq!(cfg.timeout_for(true), Duration::from_secs(120));
    }

    #[test]
    fn from_env_missing_key_returns_unavailable() {
        // 保证该测试下 ARK_API_KEY 不存在。
        std::env::remove_var("ARK_API_KEY");
        let err = ArkConfig::from_env().unwrap_err();
        assert_eq!(err.code(), "AI_UNAVAILABLE");
    }
}

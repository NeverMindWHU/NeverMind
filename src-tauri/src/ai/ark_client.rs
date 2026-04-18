//! 火山引擎方舟（豆包）LLM 客户端实现。
//!
//! 对齐 OpenAI 兼容的 `/chat/completions` 接口：
//! - Authorization: `Bearer {ARK_API_KEY}`
//! - Body 与 OpenAI ChatCompletion 对齐
//!
//! 文档：<https://www.volcengine.com/docs/82379>

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::{
    ai::{client::LlmClient, config::ArkConfig},
    utils::error::{AppError, AppResult},
};

pub struct ArkLlmClient {
    config: ArkConfig,
    http: reqwest::Client,
}

impl ArkLlmClient {
    pub fn new(config: ArkConfig) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| AppError::AiUnavailable {
                message: format!("HTTP 客户端初始化失败: {}", e),
            })?;
        Ok(Self { config, http })
    }
}

#[async_trait]
impl LlmClient for ArkLlmClient {
    async fn complete(&self, prompt: &str) -> AppResult<String> {
        let body = json!({
            "model": self.config.model,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "temperature": self.config.temperature,
        });

        let response = self
            .http
            .post(self.config.chat_completions_url())
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(map_request_error)?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::AiUnavailable {
                message: format!("Ark 返回 HTTP {}: {}", status, truncate(&text, 500)),
            });
        }

        let parsed: ArkChatResponse =
            response.json().await.map_err(|e| AppError::AiResponseInvalid {
                message: format!("解析方舟响应失败: {}", e),
            })?;

        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| AppError::AiResponseInvalid {
                message: "方舟返回了空内容".into(),
            })
    }
}

fn map_request_error(e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::AiTimeout
    } else {
        AppError::AiUnavailable {
            message: format!("请求方舟失败: {}", e),
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push_str("...");
        out
    }
}

// ---------------------------------------------------------------------------
// 响应体结构（只解析我们用到的字段，其他字段忽略）
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ArkChatResponse {
    choices: Vec<ArkChoice>,
}

#[derive(Debug, Deserialize)]
struct ArkChoice {
    message: ArkMessage,
}

#[derive(Debug, Deserialize)]
struct ArkMessage {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{parser::parse_cards, prompt::build_prompt};

    /// 真实调用方舟 API 的联通性测试。
    ///
    /// 默认 `#[ignore]`，CI 不会跑。开发者本地需要验证联通时：
    /// ```bash
    /// cargo test -p nevermind-tauri --lib ai::ark_client::tests::live_ark_call -- --ignored --nocapture
    /// ```
    /// 需要在 shell 或 `.env` 中提供 `ARK_API_KEY`。
    #[tokio::test]
    #[ignore]
    async fn live_ark_call() {
        let _ = dotenvy::dotenv();
        let config = ArkConfig::from_env().expect("缺少 ARK_API_KEY");
        let client = ArkLlmClient::new(config).expect("初始化客户端失败");

        let prompt = build_prompt(
            "艾宾浩斯遗忘曲线说明了记忆随时间按指数规律衰减。",
            Some("遗忘曲线"),
            None,
        );

        let raw = client.complete(&prompt).await.expect("方舟调用失败");
        eprintln!("--- Ark 原始响应 ---\n{}\n--- end ---", raw);

        let cards = parse_cards(&raw).expect("解析卡片失败");
        assert!(!cards.is_empty(), "应返回至少一张卡片");
        for card in &cards {
            assert!(!card.keyword.is_empty());
            assert!(!card.definition.is_empty());
        }
    }
}

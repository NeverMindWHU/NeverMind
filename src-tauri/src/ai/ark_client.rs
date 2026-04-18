//! 火山引擎方舟（豆包）LLM 客户端实现。
//!
//! 对齐 OpenAI 兼容的 `/chat/completions` 接口：
//! - Authorization: `Bearer {ARK_API_KEY}`
//! - Body 与 OpenAI ChatCompletion 对齐，`messages[].content` 可为字符串或
//!   `[{type: "text", text}, {type: "image_url", image_url: {url}}]` 数组。
//!
//! 文档：
//! - 对话 API：<https://www.volcengine.com/docs/82379/1494384>
//! - 图片理解：<https://www.volcengine.com/docs/82379/1362931>

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    ai::{
        client::{ChatRequest, LlmClient},
        config::ArkConfig,
    },
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

    /// 将 `ChatRequest` 转为 OpenAI 兼容的 `messages` 数组。
    ///
    /// - 没图片：走字符串 content（和此前行为完全一致）
    /// - 有图片：走数组 content —— `[{type:"text",...}, {type:"image_url",...}*]`
    fn build_messages(request: &ChatRequest) -> Value {
        if request.images.is_empty() {
            return json!([
                { "role": "user", "content": request.text }
            ]);
        }

        let mut parts: Vec<Value> = Vec::with_capacity(request.images.len() + 1);
        if !request.text.is_empty() {
            parts.push(json!({ "type": "text", "text": request.text }));
        }
        for img in &request.images {
            parts.push(json!({
                "type": "image_url",
                "image_url": { "url": img.url }
            }));
        }

        json!([
            { "role": "user", "content": parts }
        ])
    }

    async fn send(&self, body: Value) -> AppResult<String> {
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

#[async_trait]
impl LlmClient for ArkLlmClient {
    async fn complete(&self, prompt: &str) -> AppResult<String> {
        self.complete_chat(ChatRequest::from_text(prompt)).await
    }

    async fn complete_chat(&self, request: ChatRequest) -> AppResult<String> {
        let body = json!({
            "model": self.config.model,
            "messages": Self::build_messages(&request),
            "temperature": self.config.temperature,
        });
        self.send(body).await
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
    use crate::ai::{
        client::ImageInput,
        parser::parse_cards,
        prompt::build_prompt,
    };

    #[test]
    fn build_messages_text_only_uses_string_content() {
        let req = ChatRequest::from_text("hello");
        let msgs = ArkLlmClient::build_messages(&req);
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"], "hello");
    }

    #[test]
    fn build_messages_with_images_uses_parts_array() {
        let req = ChatRequest::from_text("描述这张图")
            .with_images(vec![ImageInput::new("https://example.com/a.png")]);
        let msgs = ArkLlmClient::build_messages(&req);
        let parts = &msgs[0]["content"];
        assert!(parts.is_array(), "有图片时 content 必须是数组");
        let parts = parts.as_array().unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0]["type"], "text");
        assert_eq!(parts[0]["text"], "描述这张图");
        assert_eq!(parts[1]["type"], "image_url");
        assert_eq!(parts[1]["image_url"]["url"], "https://example.com/a.png");
    }

    #[test]
    fn build_messages_empty_text_with_image_skips_text_part() {
        let req = ChatRequest::default()
            .with_images(vec![ImageInput::new("https://example.com/b.png")]);
        let msgs = ArkLlmClient::build_messages(&req);
        let parts = msgs[0]["content"].as_array().unwrap();
        assert_eq!(parts.len(), 1, "无文字指令时只有一个 image_url part");
        assert_eq!(parts[0]["type"], "image_url");
    }

    /// 真实调用方舟 API 的联通性测试（文本）。
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
            false,
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

    /// 真实调用方舟 **多模态** API 的联通性测试。
    ///
    /// 使用一张 Wikipedia 的公网图片，让豆包根据图片提取卡片。
    /// 运行：
    /// ```bash
    /// cargo test -p nevermind-tauri --lib ai::ark_client::tests::live_ark_vision_call -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore]
    async fn live_ark_vision_call() {
        let _ = dotenvy::dotenv();
        let config = ArkConfig::from_env().expect("缺少 ARK_API_KEY");
        let client = ArkLlmClient::new(config).expect("初始化客户端失败");

        // 使用稳定的公开图床。Wikipedia CDN 从方舟服务端下载常超时。
        let image_url = "https://picsum.photos/id/180/400/300";

        let prompt = build_prompt(
            "请直接描述图片内容，并生成 1 张关键词卡片，关键词取图中最主要的物体或场景。",
            None,
            Some("图片识别演示"),
            true,
        );
        let request = ChatRequest::from_text(prompt)
            .with_images(vec![ImageInput::new(image_url)]);

        let raw = client
            .complete_chat(request)
            .await
            .expect("方舟多模态调用失败");
        eprintln!("--- Ark 多模态原始响应 ---\n{}\n--- end ---", raw);

        let cards = parse_cards(&raw).expect("解析卡片失败");
        assert!(!cards.is_empty(), "应返回至少一张卡片");
        for card in &cards {
            assert!(!card.keyword.is_empty());
            assert!(!card.definition.is_empty());
        }
    }
}

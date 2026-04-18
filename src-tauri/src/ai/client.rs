use async_trait::async_trait;

use crate::utils::error::AppResult;

/// 多模态输入中的一张图片。
///
/// `url` 既可以是 `http(s)://` 可公网访问的链接，也可以是
/// `data:image/<mime>;base64,<payload>` 形式的内联数据 URL。
/// 火山方舟（豆包）图片理解 API 与 OpenAI 兼容，两种格式都接受。
#[derive(Debug, Clone)]
pub struct ImageInput {
    pub url: String,
}

impl ImageInput {
    pub fn new<S: Into<String>>(url: S) -> Self {
        Self { url: url.into() }
    }
}

/// 一次 LLM 补全请求的统一入参：文本 + 可选图片。
///
/// - 纯文本场景：`text` 非空、`images` 为空
/// - 图片 + 文字指令：两者都非空
/// - 纯图片场景：`text` 可以为空，调用方仍需保证 prompt 里有足够指令
///
/// 业务层只构造这一个结构，底层客户端决定要不要走多模态通道。
#[derive(Debug, Clone, Default)]
pub struct ChatRequest {
    pub text: String,
    pub images: Vec<ImageInput>,
}

impl ChatRequest {
    pub fn from_text<S: Into<String>>(text: S) -> Self {
        Self {
            text: text.into(),
            images: Vec::new(),
        }
    }

    pub fn with_images(mut self, images: Vec<ImageInput>) -> Self {
        self.images = images;
        self
    }

    pub fn has_images(&self) -> bool {
        !self.images.is_empty()
    }
}

/// LLM 客户端抽象：所有模型供应商（Qwen / OpenAI 兼容 / 本地）都实现这个 trait。
/// Service 层只依赖 `&dyn LlmClient`，可以在运行时或测试里自由替换实现。
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// 纯文本补全。保留此方法用于兼容只需要文本的调用点。
    async fn complete(&self, prompt: &str) -> AppResult<String>;

    /// 多模态补全：文本 + 可选图片。
    ///
    /// 默认实现退化到 `complete(text)`，忽略图片——这样不支持视觉的客户端
    /// （如 `MockLlmClient`）无需改动；真正的视觉模型客户端应 override。
    async fn complete_chat(&self, request: ChatRequest) -> AppResult<String> {
        self.complete(&request.text).await
    }
}

/// 不联网的 Mock 实现，固定返回一张结构完整的卡片。
/// 用于本地开发与集成测试，保证没有 API Key 时也能跑通整条生成链路。
pub struct MockLlmClient;

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, _prompt: &str) -> AppResult<String> {
        Ok(r#"
        {
          "cards": [
            {
              "keyword": "示例关键词",
              "definition": "这是一张用于开发调试的 Mock 知识卡片",
              "explanation": "当未配置真实 LLM 时，系统会返回这张固定卡片，便于前后端联调与自动化测试。",
              "relatedTerms": ["Mock", "联调", "本地开发"],
              "scenarios": ["单元测试", "集成测试", "无网络演示"],
              "sourceExcerpt": "由 MockLlmClient 返回，无真实来源"
            }
          ]
        }
        "#
        .to_string())
    }
}

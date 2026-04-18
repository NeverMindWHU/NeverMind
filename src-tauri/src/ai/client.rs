use async_trait::async_trait;

use crate::utils::error::AppResult;

/// LLM 客户端抽象：所有模型供应商（Qwen / OpenAI 兼容 / 本地）都实现这个 trait。
/// Service 层只依赖 `&dyn LlmClient`，可以在运行时或测试里自由替换实现。
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// 发起一次补全请求，返回模型的原始文本输出（通常是一段 JSON 或被 Markdown 代码块包裹的 JSON）。
    async fn complete(&self, prompt: &str) -> AppResult<String>;
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

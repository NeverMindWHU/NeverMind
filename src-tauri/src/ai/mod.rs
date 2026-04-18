pub mod ark_client;
pub mod client;
pub mod config;
pub mod parser;
pub mod prompt;

use std::sync::Arc;

pub use ark_client::ArkLlmClient;
pub use client::{LlmClient, MockLlmClient};
pub use config::ArkConfig;

/// 根据环境变量构建默认的 LLM 客户端。
///
/// - `ARK_API_KEY` 存在且可解析 → 返回真实的 [`ArkLlmClient`]
/// - 其他情况（未设置 / 客户端构造失败）→ 回退到 [`MockLlmClient`]，并在 stderr 打印原因
///
/// 这样生产环境一行不改自动接真实模型，测试与开发环境没配 Key 也能跑通。
pub fn default_client() -> Arc<dyn LlmClient> {
    match ArkConfig::from_env().and_then(ArkLlmClient::new) {
        Ok(client) => Arc::new(client),
        Err(e) => {
            eprintln!(
                "[ai] Ark 客户端不可用，回退到 MockLlmClient。原因: {}",
                e
            );
            Arc::new(MockLlmClient)
        }
    }
}

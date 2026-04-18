pub mod ark_client;
pub mod client;
pub mod config;
pub mod parser;
pub mod prompt;

use std::sync::Arc;

pub use ark_client::ArkLlmClient;
pub use client::{LlmClient, MockLlmClient};
pub use config::ArkConfig;

use crate::utils::error::AppResult;

/// 严格构建豆包（Ark）客户端。
///
/// 读取环境变量 `ARK_API_KEY` / `ARK_API_BASE` / `ARK_MODEL`，失败即返回错误。
///
/// **生产路径必须使用此函数**，保证每次调用都走真实豆包，不会静默降级到 Mock。
pub fn require_ark_client() -> AppResult<Arc<dyn LlmClient>> {
    let config = ArkConfig::from_env()?;
    let client = ArkLlmClient::new(config)?;
    Ok(Arc::new(client))
}

/// 宽松构建默认 LLM 客户端：有 `ARK_API_KEY` 则用真实 Ark，否则回退 Mock。
///
/// **仅用于开发 / 离线演示 / 脚本**等不强制真实 AI 的场景。
/// Tauri 桌面应用的生产启动流程请使用 [`require_ark_client`]。
pub fn default_client() -> Arc<dyn LlmClient> {
    match require_ark_client() {
        Ok(client) => client,
        Err(e) => {
            eprintln!(
                "[ai] Ark 客户端不可用，回退到 MockLlmClient。原因: {}",
                e
            );
            Arc::new(MockLlmClient)
        }
    }
}

use std::sync::Arc;

use sqlx::SqlitePool;

use crate::nevermind::ai::{LlmClient, MockLlmClient};
use crate::nevermind::db::dao::{
    card_dao::{CardDao, SqliteCardDao},
    review_dao::{ReviewDao, SqliteReviewDao},
    settings_dao::{SettingsDao, SqliteSettingsDao},
};

pub struct AppState {
    pub card_dao: Arc<dyn CardDao>,
    pub review_dao: Arc<dyn ReviewDao>,
    pub settings_dao: Arc<dyn SettingsDao>,
    /// 生成卡片链路使用的 LLM 客户端。
    /// 生产路径应注入真实豆包客户端（见 `ai::require_ark_client`）。
    pub llm: Arc<dyn LlmClient>,
}

impl AppState {
    pub fn new(
        card_dao: Arc<dyn CardDao>,
        review_dao: Arc<dyn ReviewDao>,
        settings_dao: Arc<dyn SettingsDao>,
        llm: Arc<dyn LlmClient>,
    ) -> Self {
        Self {
            card_dao,
            review_dao,
            settings_dao,
            llm,
        }
    }

    /// 使用指定 LLM 客户端构建 AppState。生产路径请走这个入口。
    pub fn from_pool_with_llm(pool: SqlitePool, llm: Arc<dyn LlmClient>) -> Self {
        Self {
            card_dao: Arc::new(SqliteCardDao::new(pool.clone())),
            review_dao: Arc::new(SqliteReviewDao::new(pool.clone())),
            settings_dao: Arc::new(SqliteSettingsDao::new(pool)),
            llm,
        }
    }

    /// 仅用于测试 / 离线场景：LLM 字段默认注入 `MockLlmClient`。
    /// **生产启动严禁使用此方法**，否则豆包会被旁路。
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self::from_pool_with_llm(pool, Arc::new(MockLlmClient))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nevermind::ai::LlmClient;
    use crate::nevermind::utils::error::AppResult;
    use async_trait::async_trait;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingLlm {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl LlmClient for CountingLlm {
        async fn complete(&self, _prompt: &str) -> AppResult<String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok("ok".into())
        }
    }

    #[tokio::test]
    async fn from_pool_with_llm_injects_custom_client() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        let counting = Arc::new(CountingLlm {
            calls: AtomicUsize::new(0),
        });
        let state = AppState::from_pool_with_llm(pool, counting.clone());

        // 验证 AppState.llm 指向注入的客户端，而不是 Mock。
        state.llm.complete("ping").await.unwrap();
        assert_eq!(counting.calls.load(Ordering::SeqCst), 1);
    }
}

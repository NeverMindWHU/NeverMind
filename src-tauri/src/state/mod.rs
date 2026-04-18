use std::sync::Arc;

use sqlx::SqlitePool;

use crate::db::dao::{
    card_dao::{CardDao, SqliteCardDao},
    review_dao::{ReviewDao, SqliteReviewDao},
    settings_dao::{SettingsDao, SqliteSettingsDao},
};

pub struct AppState {
    pub card_dao: Arc<dyn CardDao>,
    pub review_dao: Arc<dyn ReviewDao>,
    pub settings_dao: Arc<dyn SettingsDao>,
}

impl AppState {
    pub fn new(
        card_dao: Arc<dyn CardDao>,
        review_dao: Arc<dyn ReviewDao>,
        settings_dao: Arc<dyn SettingsDao>,
    ) -> Self {
        Self {
            card_dao,
            review_dao,
            settings_dao,
        }
    }

    pub fn from_pool(pool: SqlitePool) -> Self {
        Self {
            card_dao: Arc::new(SqliteCardDao::new(pool.clone())),
            review_dao: Arc::new(SqliteReviewDao::new(pool.clone())),
            settings_dao: Arc::new(SqliteSettingsDao::new(pool)),
        }
    }
}

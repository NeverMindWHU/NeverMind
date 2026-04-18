use async_trait::async_trait;
use chrono::Utc;
use sqlx::{QueryBuilder, SqlitePool};

use crate::{
    models::card::{Card, GenerationBatch, NewCard, NewGenerationBatch, UpdateCardStatus},
    utils::error::{AppError, AppResult},
};

#[async_trait]
pub trait CardDao: Send + Sync {
    async fn create_generation_batch(&self, batch: &NewGenerationBatch) -> AppResult<()>;
    async fn insert_cards(&self, cards: &[NewCard]) -> AppResult<()>;
    async fn list_cards_by_batch(&self, batch_id: &str) -> AppResult<Vec<Card>>;
    async fn review_generated_cards(&self, batch_id: &str, update: &UpdateCardStatus) -> AppResult<()>;
    async fn get_generation_batch(&self, batch_id: &str) -> AppResult<GenerationBatch>;
}

#[derive(Clone)]
pub struct SqliteCardDao {
    pool: SqlitePool,
}

impl SqliteCardDao {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CardDao for SqliteCardDao {
    async fn create_generation_batch(&self, batch: &NewGenerationBatch) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO generation_batches (id, source_type, source_text, selected_keyword, context_title, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&batch.id)
        .bind(&batch.source_type)
        .bind(&batch.source_text)
        .bind(&batch.selected_keyword)
        .bind(&batch.context_title)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn insert_cards(&self, cards: &[NewCard]) -> AppResult<()> {
        if cards.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;
        for card in cards {
            let now = Utc::now();
            sqlx::query(
                r#"
                INSERT INTO cards (
                    id, batch_id, keyword, definition, explanation, source_excerpt,
                    status, created_at, updated_at, next_review_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&card.id)
            .bind(&card.batch_id)
            .bind(&card.keyword)
            .bind(&card.definition)
            .bind(&card.explanation)
            .bind(&card.source_excerpt)
            .bind(&card.status)
            .bind(now)
            .bind(now)
            .bind(card.next_review_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_cards_by_batch(&self, batch_id: &str) -> AppResult<Vec<Card>> {
        let cards = sqlx::query_as::<_, Card>(
            r#"
            SELECT id, batch_id, keyword, definition, explanation, source_excerpt,
                   status, created_at, updated_at, next_review_at
            FROM cards
            WHERE batch_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(cards)
    }

    async fn review_generated_cards(&self, batch_id: &str, update: &UpdateCardStatus) -> AppResult<()> {
        let batch = self.get_generation_batch(batch_id).await?;
        let _ = batch;

        let mut tx = self.pool.begin().await?;

        if !update.accepted_ids.is_empty() {
            let now = Utc::now();

            // 1) 把选中的卡片标记为 accepted
            let mut builder = QueryBuilder::new(
                "UPDATE cards SET status = 'accepted', updated_at = ",
            );
            builder.push_bind(now);
            builder.push(" WHERE batch_id = ");
            builder.push_bind(batch_id);
            builder.push(" AND id IN (");
            {
                let mut separated = builder.separated(", ");
                for id in &update.accepted_ids {
                    separated.push_bind(id);
                }
            }
            builder.push(")");
            builder.build().execute(&mut *tx).await?;

            // 2) 把这些卡对应的 review_schedule 在未来的 due_at 拉到 now，
            //    确保用户一旦接受就能在复习队列里立刻看到。
            //    已经到期（due_at <= now）或者 status != 'pending' 的行不动。
            let mut schedule_builder = QueryBuilder::new(
                "UPDATE review_schedule SET due_at = ",
            );
            schedule_builder.push_bind(now);
            schedule_builder.push(", updated_at = ");
            schedule_builder.push_bind(now);
            schedule_builder.push(" WHERE status = 'pending' AND due_at > ");
            schedule_builder.push_bind(now);
            schedule_builder.push(" AND card_id IN (");
            {
                let mut separated = schedule_builder.separated(", ");
                for id in &update.accepted_ids {
                    separated.push_bind(id);
                }
            }
            schedule_builder.push(")");
            schedule_builder.build().execute(&mut *tx).await?;
        }

        if !update.rejected_ids.is_empty() {
            let mut builder = QueryBuilder::new(
                "UPDATE cards SET status = 'rejected', updated_at = ",
            );
            builder.push_bind(Utc::now());
            builder.push(" WHERE batch_id = ");
            builder.push_bind(batch_id);
            builder.push(" AND id IN (");
            {
                let mut separated = builder.separated(", ");
                for id in &update.rejected_ids {
                    separated.push_bind(id);
                }
            }
            builder.push(")");
            builder.build().execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_generation_batch(&self, batch_id: &str) -> AppResult<GenerationBatch> {
        let batch = sqlx::query_as::<_, GenerationBatch>(
            r#"
            SELECT id, source_type, source_text, selected_keyword, context_title, created_at
            FROM generation_batches
            WHERE id = ?
            "#,
        )
        .bind(batch_id)
        .fetch_optional(&self.pool)
        .await?;

        batch.ok_or(AppError::NotFound {
            entity: "generation_batch",
        })
    }
}

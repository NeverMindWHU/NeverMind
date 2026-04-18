use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::{
    models::review::{DueReviewItem, NewReviewLog, NewReviewSchedule, ReviewLog, ReviewSchedule},
    utils::error::{AppError, AppResult},
};

#[async_trait]
pub trait ReviewDao: Send + Sync {
    async fn create_schedule(&self, schedule: &NewReviewSchedule) -> AppResult<()>;
    async fn list_due_reviews(&self, limit: i64) -> AppResult<Vec<DueReviewItem>>;
    async fn count_due_reviews(&self) -> AppResult<i64>;
    /// 列出**尚未到期**的 pending review，按到期时间升序。
    /// 供"提前复习下一轮"入口使用。
    async fn list_upcoming_reviews(&self, limit: i64) -> AppResult<Vec<DueReviewItem>>;
    /// 尚未到期（`due_at > now`）的 pending review 总数。
    async fn count_upcoming_reviews(&self) -> AppResult<i64>;
    async fn get_schedule(&self, review_id: &str) -> AppResult<ReviewSchedule>;
    async fn update_schedule_after_review(
        &self,
        review_id: &str,
        next_step: i64,
        next_due_at: chrono::DateTime<Utc>,
        status: &str,
    ) -> AppResult<()>;
    async fn insert_review_log(&self, log: &NewReviewLog) -> AppResult<ReviewLog>;
    async fn count_completed_reviews_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> AppResult<i64>;
    async fn get_next_due_at(&self) -> AppResult<Option<DateTime<Utc>>>;
    async fn list_completed_review_days_desc(&self, limit: i64) -> AppResult<Vec<String>>;
}

#[derive(Clone)]
pub struct SqliteReviewDao {
    pool: SqlitePool,
}

impl SqliteReviewDao {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReviewDao for SqliteReviewDao {
    async fn create_schedule(&self, schedule: &NewReviewSchedule) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO review_schedule (id, card_id, review_step, due_at, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&schedule.id)
        .bind(&schedule.card_id)
        .bind(schedule.review_step)
        .bind(schedule.due_at)
        .bind(&schedule.status)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_due_reviews(&self, limit: i64) -> AppResult<Vec<DueReviewItem>> {
        let items = sqlx::query_as::<_, DueReviewItem>(
            r#"
            SELECT rs.id AS review_id,
                   c.id AS card_id,
                   c.keyword,
                   c.question,
                   c.keywords,
                   c.definition,
                   c.explanation,
                   rs.review_step,
                   rs.due_at
            FROM review_schedule rs
            INNER JOIN cards c ON c.id = rs.card_id
            WHERE rs.status = 'pending'
              AND c.status = 'accepted'
              AND rs.due_at <= ?
            ORDER BY rs.due_at ASC
            LIMIT ?
            "#,
        )
        .bind(Utc::now())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    async fn count_due_reviews(&self) -> AppResult<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM review_schedule rs
            INNER JOIN cards c ON c.id = rs.card_id
            WHERE rs.status = 'pending'
              AND c.status = 'accepted'
              AND rs.due_at <= ?
            "#,
        )
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    async fn list_upcoming_reviews(&self, limit: i64) -> AppResult<Vec<DueReviewItem>> {
        // 与 list_due_reviews 对称，只是条件改为 due_at > now。
        // 按最早到期排序，"最接近到期"的卡先上。
        let items = sqlx::query_as::<_, DueReviewItem>(
            r#"
            SELECT rs.id AS review_id,
                   c.id AS card_id,
                   c.keyword,
                   c.question,
                   c.keywords,
                   c.definition,
                   c.explanation,
                   rs.review_step,
                   rs.due_at
            FROM review_schedule rs
            INNER JOIN cards c ON c.id = rs.card_id
            WHERE rs.status = 'pending'
              AND c.status = 'accepted'
              AND rs.due_at > ?
            ORDER BY rs.due_at ASC
            LIMIT ?
            "#,
        )
        .bind(Utc::now())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    async fn count_upcoming_reviews(&self) -> AppResult<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM review_schedule rs
            INNER JOIN cards c ON c.id = rs.card_id
            WHERE rs.status = 'pending'
              AND c.status = 'accepted'
              AND rs.due_at > ?
            "#,
        )
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    async fn get_schedule(&self, review_id: &str) -> AppResult<ReviewSchedule> {
        let schedule = sqlx::query_as::<_, ReviewSchedule>(
            r#"
            SELECT id, card_id, review_step, due_at, status, created_at, updated_at
            FROM review_schedule
            WHERE id = ?
            "#,
        )
        .bind(review_id)
        .fetch_optional(&self.pool)
        .await?;

        schedule.ok_or(AppError::NotFound {
            entity: "review_schedule",
        })
    }

    async fn update_schedule_after_review(
        &self,
        review_id: &str,
        next_step: i64,
        next_due_at: chrono::DateTime<Utc>,
        status: &str,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE review_schedule
            SET review_step = ?, due_at = ?, status = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(next_step)
        .bind(next_due_at)
        .bind(status)
        .bind(Utc::now())
        .bind(review_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn insert_review_log(&self, log: &NewReviewLog) -> AppResult<ReviewLog> {
        sqlx::query(
            r#"
            INSERT INTO review_logs (
                id, review_schedule_id, card_id, result, previous_step, next_step, reviewed_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&log.id)
        .bind(&log.review_schedule_id)
        .bind(&log.card_id)
        .bind(&log.result)
        .bind(log.previous_step)
        .bind(log.next_step)
        .bind(log.reviewed_at)
        .execute(&self.pool)
        .await?;

        let saved = sqlx::query_as::<_, ReviewLog>(
            r#"
            SELECT id, review_schedule_id, card_id, result, previous_step, next_step, reviewed_at
            FROM review_logs
            WHERE id = ?
            "#,
        )
        .bind(&log.id)
        .fetch_one(&self.pool)
        .await?;

        Ok(saved)
    }

    async fn count_completed_reviews_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> AppResult<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM review_logs
            WHERE reviewed_at >= ?
              AND reviewed_at < ?
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    async fn get_next_due_at(&self) -> AppResult<Option<DateTime<Utc>>> {
        // `SELECT MIN(...)` 是聚合查询：即使 WHERE 匹配不到任何行，SQLite
        // 也会返回 **一行 NULL**。若把标量类型写成非 Option 的 `DateTime<Utc>`，
        // `fetch_optional` 会拿到 `Some(row)` 再去把 NULL 解码到 `DateTime<Utc>`，
        // 触发 `invalid datetime: ` 错误。因此这里显式用 Option 作为标量类型，
        // 并用 `fetch_one`（聚合永远返回一行）避免歧义。
        let next_due_at: Option<DateTime<Utc>> = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            r#"
            SELECT MIN(rs.due_at)
            FROM review_schedule rs
            INNER JOIN cards c ON c.id = rs.card_id
            WHERE rs.status = 'pending'
              AND c.status = 'accepted'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(next_due_at)
    }

    async fn list_completed_review_days_desc(&self, limit: i64) -> AppResult<Vec<String>> {
        let days = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT substr(reviewed_at, 1, 10) AS review_day
            FROM review_logs
            ORDER BY review_day DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(days)
    }
}

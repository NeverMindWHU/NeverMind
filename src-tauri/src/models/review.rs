use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReviewSchedule {
    pub id: String,
    pub card_id: String,
    pub review_step: i64,
    pub due_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReviewLog {
    pub id: String,
    pub review_schedule_id: String,
    pub card_id: String,
    pub result: String,
    pub previous_step: i64,
    pub next_step: i64,
    pub reviewed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DueReviewItem {
    pub review_id: String,
    pub card_id: String,
    pub keyword: String,
    pub definition: String,
    pub explanation: String,
    pub review_step: i64,
    pub due_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewReviewSchedule {
    pub id: String,
    pub card_id: String,
    pub review_step: i64,
    pub due_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewReviewLog {
    pub id: String,
    pub review_schedule_id: String,
    pub card_id: String,
    pub result: String,
    pub previous_step: i64,
    pub next_step: i64,
    pub reviewed_at: DateTime<Utc>,
}

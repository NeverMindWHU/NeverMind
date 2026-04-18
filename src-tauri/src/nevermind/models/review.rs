use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewResult {
    Done,
    Remembered,
    Forgotten,
    Skipped,
}

impl ReviewResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Remembered => "remembered",
            Self::Forgotten => "forgotten",
            Self::Skipped => "skipped",
        }
    }
}

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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Card {
    pub id: String,
    pub batch_id: Option<String>,
    pub keyword: String,
    pub definition: String,
    pub explanation: String,
    pub source_excerpt: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub next_review_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GenerationBatch {
    pub id: String,
    pub source_type: String,
    pub source_text: String,
    pub selected_keyword: Option<String>,
    pub context_title: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGenerationBatch {
    pub id: String,
    pub source_type: String,
    pub source_text: String,
    pub selected_keyword: Option<String>,
    pub context_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCard {
    pub id: String,
    pub batch_id: Option<String>,
    pub keyword: String,
    pub definition: String,
    pub explanation: String,
    pub source_excerpt: Option<String>,
    pub status: String,
    pub next_review_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCardStatus {
    pub accepted_ids: Vec<String>,
    pub rejected_ids: Vec<String>,
}

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

// ============================================================================
// 跨模块共享 DTO —— 对齐 docs/architecture/contracts/card-generation.md
// 前端、其他后端模块（如复习）均可通过这些类型协作，无需重复定义。
// ============================================================================

/// 生成/展示阶段对外暴露的单张卡片。
/// 注意：`related_terms` / `scenarios` / `review_history` 暂未在 `cards` 表中持久化，
/// 仅在 AI 首次生成时有值，从 DB 读取时退化为空数组。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedCard {
    pub card_id: String,
    pub keyword: String,
    pub definition: String,
    pub explanation: String,
    pub related_terms: Vec<String>,
    pub scenarios: Vec<String>,
    pub source_excerpt: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub review_history: Vec<String>,
    pub next_review_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedCardBatchResult {
    pub batch_id: String,
    pub cards: Vec<GeneratedCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewedGeneratedCardsResult {
    pub batch_id: String,
    pub accepted_count: i64,
    pub rejected_count: i64,
    pub pending_count: i64,
}

impl From<Card> for GeneratedCard {
    fn from(c: Card) -> Self {
        Self {
            card_id: c.id,
            keyword: c.keyword,
            definition: c.definition,
            explanation: c.explanation,
            related_terms: Vec::new(),
            scenarios: Vec::new(),
            source_excerpt: c.source_excerpt,
            status: c.status,
            created_at: c.created_at,
            review_history: Vec::new(),
            next_review_at: c.next_review_at,
        }
    }
}

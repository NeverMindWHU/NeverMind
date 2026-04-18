//! 卡片生成链路 —— 对齐 `docs/architecture/contracts/card-generation.md`。
//!
//! 本模块只做业务编排，不直接依赖 Tauri，任何外壳（Tauri Command、CLI、HTTP）
//! 都可以薄薄包一层把下面三个函数暴露出去。

use std::collections::HashSet;

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ai::{parser::parse_cards, prompt::build_prompt, LlmClient},
    db::dao::{card_dao::CardDao, review_dao::ReviewDao},
    models::{
        card::{
            GeneratedCard, GeneratedCardBatchResult, NewCard, NewGenerationBatch,
            ReviewedGeneratedCardsResult, UpdateCardStatus,
        },
        review::NewReviewSchedule,
    },
    utils::error::{AppError, AppResult},
};

// ============================================================================
// 输入结构（对齐契约文档中 Command 的 Input 字段）
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateCardsInput {
    pub source_text: String,
    #[serde(default)]
    pub selected_keyword: Option<String>,
    #[serde(default)]
    pub context_title: Option<String>,
    pub source_type: String,
    #[serde(default)]
    pub model_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewGeneratedCardsInput {
    pub batch_id: String,
    #[serde(default)]
    pub accept_card_ids: Vec<String>,
    #[serde(default)]
    pub reject_card_ids: Vec<String>,
}

// ============================================================================
// 常量
// ============================================================================

const MAX_SOURCE_TEXT_LEN: usize = 5000;

/// 新卡片的首次复习延迟（小时）。与复习节点序列 [1, 1, 3, 7, 15, 30] 中首个节点对齐。
/// 后续真正的排期推进由复习模块（scheduler）负责，此处只负责"起点"。
const FIRST_REVIEW_DELAY_HOURS: i64 = 24;

// ============================================================================
// 业务函数
// ============================================================================

/// 生成一批结构化卡片：调 AI → 标准化 → 写 `generation_batches` + `cards` +
/// 初始化 `review_schedule`。
///
/// 卡片初始 `status = "pending"`，只有用户在预览阶段点「接受」后才会流入复习队列
/// （`ReviewDao::list_due_reviews` 会过滤 `cards.status = 'accepted'`）。
pub async fn generate_cards(
    llm: &dyn LlmClient,
    card_dao: &dyn CardDao,
    review_dao: &dyn ReviewDao,
    input: GenerateCardsInput,
) -> AppResult<GeneratedCardBatchResult> {
    validate_generate_input(&input)?;

    let prompt = build_prompt(
        &input.source_text,
        input.selected_keyword.as_deref(),
        input.context_title.as_deref(),
    );
    let raw_response = llm.complete(&prompt).await?;
    let parsed_cards = parse_cards(&raw_response)?;

    let batch_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let first_due_at = now + Duration::hours(FIRST_REVIEW_DELAY_HOURS);

    card_dao
        .create_generation_batch(&NewGenerationBatch {
            id: batch_id.clone(),
            source_type: input.source_type.clone(),
            source_text: input.source_text.clone(),
            selected_keyword: input.selected_keyword.clone(),
            context_title: input.context_title.clone(),
        })
        .await?;

    let new_cards: Vec<NewCard> = parsed_cards
        .iter()
        .map(|p| NewCard {
            id: Uuid::new_v4().to_string(),
            batch_id: Some(batch_id.clone()),
            keyword: p.keyword.clone(),
            definition: p.definition.clone(),
            explanation: p.explanation.clone(),
            source_excerpt: p.source_excerpt.clone(),
            status: "pending".into(),
            next_review_at: Some(first_due_at),
        })
        .collect();

    card_dao.insert_cards(&new_cards).await?;

    for card in &new_cards {
        review_dao
            .create_schedule(&NewReviewSchedule {
                id: Uuid::new_v4().to_string(),
                card_id: card.id.clone(),
                review_step: 1,
                due_at: first_due_at,
                status: "pending".into(),
            })
            .await?;
    }

    let cards: Vec<GeneratedCard> = new_cards
        .into_iter()
        .zip(parsed_cards.into_iter())
        .map(|(new_card, parsed)| GeneratedCard {
            card_id: new_card.id,
            keyword: new_card.keyword,
            definition: new_card.definition,
            explanation: new_card.explanation,
            related_terms: parsed.related_terms,
            scenarios: parsed.scenarios,
            source_excerpt: new_card.source_excerpt,
            status: new_card.status,
            created_at: now,
            review_history: Vec::new(),
            next_review_at: new_card.next_review_at,
        })
        .collect();

    Ok(GeneratedCardBatchResult { batch_id, cards })
}

/// 根据批次 ID 拉取该批已生成的卡片，用于预览阶段回显。
///
/// 注意：从数据库读出的卡片不包含 `related_terms` / `scenarios` / `review_history`
/// （当前 schema 未持久化这些字段），这些字段会退化为空数组。
pub async fn list_generated_cards(
    card_dao: &dyn CardDao,
    batch_id: &str,
) -> AppResult<GeneratedCardBatchResult> {
    // 先确认批次存在；批次不存在时返回 GENERATION_BATCH_NOT_FOUND。
    card_dao.get_generation_batch(batch_id).await?;

    let cards = card_dao
        .list_cards_by_batch(batch_id)
        .await?
        .into_iter()
        .map(GeneratedCard::from)
        .collect();

    Ok(GeneratedCardBatchResult {
        batch_id: batch_id.to_string(),
        cards,
    })
}

/// 对一批已生成卡片执行"接受 / 取消"操作。
/// 未出现在任一列表中的卡片保持 `pending`。
pub async fn review_generated_cards(
    card_dao: &dyn CardDao,
    input: ReviewGeneratedCardsInput,
) -> AppResult<ReviewedGeneratedCardsResult> {
    validate_review_input(&input)?;
    card_dao.get_generation_batch(&input.batch_id).await?;

    card_dao
        .review_generated_cards(
            &input.batch_id,
            &UpdateCardStatus {
                accepted_ids: input.accept_card_ids.clone(),
                rejected_ids: input.reject_card_ids.clone(),
            },
        )
        .await?;

    let all_cards = card_dao.list_cards_by_batch(&input.batch_id).await?;
    let mut accepted = 0i64;
    let mut rejected = 0i64;
    let mut pending = 0i64;
    for card in &all_cards {
        match card.status.as_str() {
            "accepted" => accepted += 1,
            "rejected" => rejected += 1,
            _ => pending += 1,
        }
    }

    Ok(ReviewedGeneratedCardsResult {
        batch_id: input.batch_id,
        accepted_count: accepted,
        rejected_count: rejected,
        pending_count: pending,
    })
}

// ============================================================================
// 输入校验
// ============================================================================

fn validate_generate_input(input: &GenerateCardsInput) -> AppResult<()> {
    let text = input.source_text.trim();
    if text.is_empty() {
        return Err(AppError::Validation {
            message: "sourceText 不能为空".into(),
        });
    }
    if text.chars().count() > MAX_SOURCE_TEXT_LEN {
        return Err(AppError::Validation {
            message: format!("sourceText 超过 {} 字符", MAX_SOURCE_TEXT_LEN),
        });
    }
    match input.source_type.as_str() {
        "manual" | "selection" | "import" => Ok(()),
        other => Err(AppError::Validation {
            message: format!("sourceType 非法: {}", other),
        }),
    }
}

fn validate_review_input(input: &ReviewGeneratedCardsInput) -> AppResult<()> {
    let accepted: HashSet<&String> = input.accept_card_ids.iter().collect();
    let rejected: HashSet<&String> = input.reject_card_ids.iter().collect();
    if !accepted.is_disjoint(&rejected) {
        return Err(AppError::InvalidReviewOperation {
            message: "acceptCardIds 与 rejectCardIds 不允许重复".into(),
        });
    }
    Ok(())
}

// ============================================================================
// 集成测试：使用内存数据库 + MockLlmClient 验证完整链路
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ai::MockLlmClient,
        db::{
            dao::{card_dao::SqliteCardDao, review_dao::SqliteReviewDao},
            Database,
        },
    };

    async fn setup() -> (Database, SqliteCardDao, SqliteReviewDao) {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.migrate().await.unwrap();
        let card_dao = SqliteCardDao::new(db.pool().clone());
        let review_dao = SqliteReviewDao::new(db.pool().clone());
        (db, card_dao, review_dao)
    }

    fn sample_input() -> GenerateCardsInput {
        GenerateCardsInput {
            source_text: "艾宾浩斯遗忘曲线描述了记忆随时间衰减的规律。".into(),
            selected_keyword: Some("遗忘曲线".into()),
            context_title: None,
            source_type: "manual".into(),
            model_profile_id: None,
        }
    }

    #[tokio::test]
    async fn full_generation_flow() {
        let (_db, card_dao, review_dao) = setup().await;
        let llm = MockLlmClient;

        let out = generate_cards(&llm, &card_dao, &review_dao, sample_input())
            .await
            .unwrap();

        assert!(!out.batch_id.is_empty());
        assert_eq!(out.cards.len(), 1);
        assert_eq!(out.cards[0].status, "pending");
        assert!(!out.cards[0].related_terms.is_empty(), "首次生成应带出 relatedTerms");

        // list 回来时，扩展字段退化为空数组（DB 未持久化）
        let listed = list_generated_cards(&card_dao, &out.batch_id).await.unwrap();
        assert_eq!(listed.cards.len(), 1);
        assert!(listed.cards[0].related_terms.is_empty());

        // 接受该卡片
        let card_id = out.cards[0].card_id.clone();
        let reviewed = review_generated_cards(
            &card_dao,
            ReviewGeneratedCardsInput {
                batch_id: out.batch_id.clone(),
                accept_card_ids: vec![card_id],
                reject_card_ids: vec![],
            },
        )
        .await
        .unwrap();

        assert_eq!(reviewed.accepted_count, 1);
        assert_eq!(reviewed.rejected_count, 0);
        assert_eq!(reviewed.pending_count, 0);
    }

    #[tokio::test]
    async fn list_on_unknown_batch_returns_not_found() {
        let (_db, card_dao, _review_dao) = setup().await;
        let err = list_generated_cards(&card_dao, "non-existent").await.unwrap_err();
        assert_eq!(err.code(), "GENERATION_BATCH_NOT_FOUND");
    }

    #[test]
    fn validate_generate_input_rejects_empty_text() {
        let mut input = sample_input();
        input.source_text = "   ".into();
        let err = validate_generate_input(&input).unwrap_err();
        assert_eq!(err.code(), "INVALID_INPUT");
    }

    #[test]
    fn validate_generate_input_rejects_unknown_source_type() {
        let mut input = sample_input();
        input.source_type = "weird".into();
        let err = validate_generate_input(&input).unwrap_err();
        assert_eq!(err.code(), "INVALID_INPUT");
    }

    #[test]
    fn validate_review_input_rejects_overlap() {
        let err = validate_review_input(&ReviewGeneratedCardsInput {
            batch_id: "b".into(),
            accept_card_ids: vec!["x".into()],
            reject_card_ids: vec!["x".into()],
        })
        .unwrap_err();
        assert_eq!(err.code(), "INVALID_REVIEW_OPERATION");
    }
}

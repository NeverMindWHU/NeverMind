//! 卡片生成链路 —— 对齐 `docs/architecture/contracts/card-generation.md`。
//!
//! 本模块只做业务编排，不直接依赖 Tauri，任何外壳（Tauri Command、CLI、HTTP）
//! 都可以薄薄包一层把下面三个函数暴露出去。

use std::collections::HashSet;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ai::{
        client::{ChatRequest, ImageInput},
        parser::parse_cards,
        prompt::build_prompt,
        LlmClient,
    },
    db::dao::{card_dao::CardDao, review_dao::ReviewDao},
    models::{
        card::{
            GeneratedCard, GeneratedCardBatchResult, NewCard, NewGenerationBatch,
            ReviewedGeneratedCardsResult, UpdateCardStatus,
        },
    },
    scheduler::planner::build_initial_schedule,
    utils::error::{AppError, AppResult},
};

// ============================================================================
// 输入结构（对齐契约文档中 Command 的 Input 字段）
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateCardsInput {
    /// 原始文本。纯图片生成场景下允许为空字符串，此时必须提供 `image_urls`。
    #[serde(default)]
    pub source_text: String,
    #[serde(default)]
    pub selected_keyword: Option<String>,
    #[serde(default)]
    pub context_title: Option<String>,
    pub source_type: String,
    #[serde(default)]
    pub model_profile_id: Option<String>,
    /// 图片输入。每项可以是 `http(s)://` URL 或 `data:image/<mime>;base64,...`
    /// 形式的内联数据 URL。`source_text` 与 `image_urls` 至少一个非空。
    #[serde(default)]
    pub image_urls: Vec<String>,
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
const MAX_IMAGE_COUNT: usize = 8;

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

    let has_images = !input.image_urls.is_empty();
    let prompt = build_prompt(
        &input.source_text,
        input.selected_keyword.as_deref(),
        input.context_title.as_deref(),
        has_images,
    );

    let request = ChatRequest {
        text: prompt,
        images: input
            .image_urls
            .iter()
            .map(|url| ImageInput::new(url.clone()))
            .collect(),
    };
    let raw_response = llm.complete_chat(request).await?;
    let parsed_cards = parse_cards(&raw_response)?;

    let batch_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    // 纯图片场景下 source_text 为空，用一段人类可读描述占位写入批次；
    // 图片 URL（尤其 base64 data URL）不持久化，避免数据库膨胀。
    let persisted_source_text = if input.source_text.trim().is_empty() && has_images {
        format!("[图片输入 {} 张]", input.image_urls.len())
    } else {
        input.source_text.clone()
    };

    card_dao
        .create_generation_batch(&NewGenerationBatch {
            id: batch_id.clone(),
            source_type: input.source_type.clone(),
            source_text: persisted_source_text,
            selected_keyword: input.selected_keyword.clone(),
            context_title: input.context_title.clone(),
        })
        .await?;

    let mut schedules = Vec::with_capacity(parsed_cards.len());
    let new_cards: Vec<NewCard> = parsed_cards
        .iter()
        .map(|p| {
            let card_id = Uuid::new_v4().to_string();
            let schedule = build_initial_schedule(Uuid::new_v4().to_string(), card_id.clone(), now);
            let next_review_at = schedule.due_at;
            schedules.push(schedule);

            NewCard {
                id: card_id,
                batch_id: Some(batch_id.clone()),
                keyword: p.keyword.clone(),
                definition: p.definition.clone(),
                explanation: p.explanation.clone(),
                source_excerpt: p.source_excerpt.clone(),
                status: "pending".into(),
                next_review_at: Some(next_review_at),
            }
        })
        .collect();

    card_dao.insert_cards(&new_cards).await?;

    for schedule in &schedules {
        review_dao.create_schedule(schedule).await?;
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
    let has_images = !input.image_urls.is_empty();

    if text.is_empty() && !has_images {
        return Err(AppError::Validation {
            message: "sourceText 与 imageUrls 至少需要提供一项".into(),
        });
    }
    if text.chars().count() > MAX_SOURCE_TEXT_LEN {
        return Err(AppError::Validation {
            message: format!("sourceText 超过 {} 字符", MAX_SOURCE_TEXT_LEN),
        });
    }

    if has_images {
        if input.image_urls.len() > MAX_IMAGE_COUNT {
            return Err(AppError::Validation {
                message: format!("imageUrls 最多 {} 张", MAX_IMAGE_COUNT),
            });
        }
        for (idx, url) in input.image_urls.iter().enumerate() {
            let trimmed = url.trim();
            if trimmed.is_empty() {
                return Err(AppError::Validation {
                    message: format!("imageUrls[{}] 不能为空", idx),
                });
            }
            let ok = trimmed.starts_with("http://")
                || trimmed.starts_with("https://")
                || trimmed.starts_with("data:image/");
            if !ok {
                return Err(AppError::Validation {
                    message: format!(
                        "imageUrls[{}] 必须以 http(s):// 或 data:image/ 开头",
                        idx
                    ),
                });
            }
        }
    }

    match input.source_type.as_str() {
        "manual" | "selection" | "import" | "image" => Ok(()),
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
        models::review::ReviewSchedule,
        scheduler::ebbinghaus::first_review,
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
            image_urls: Vec::new(),
        }
    }

    #[tokio::test]
    async fn full_generation_flow() {
        let (db, card_dao, review_dao) = setup().await;
        let llm = MockLlmClient;

        let out = generate_cards(&llm, &card_dao, &review_dao, sample_input())
            .await
            .unwrap();

        assert!(!out.batch_id.is_empty());
        assert_eq!(out.cards.len(), 1);
        assert_eq!(out.cards[0].status, "pending");
        assert!(!out.cards[0].related_terms.is_empty(), "首次生成应带出 relatedTerms");
        let expected_first_review = first_review(out.cards[0].created_at);
        assert_eq!(out.cards[0].next_review_at, Some(expected_first_review.next_due_at));

        let saved_schedule = sqlx::query_as::<_, ReviewSchedule>(
            r#"
            SELECT id, card_id, review_step, due_at, status, created_at, updated_at
            FROM review_schedule
            WHERE card_id = ?
            "#,
        )
        .bind(&out.cards[0].card_id)
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(saved_schedule.review_step, expected_first_review.next_step);
        assert_eq!(saved_schedule.due_at, expected_first_review.next_due_at);
        assert_eq!(saved_schedule.status, expected_first_review.status);

        // list 回来时，扩展字段退化为空数组（DB 未持久化）
        let listed = list_generated_cards(&card_dao, &out.batch_id).await.unwrap();
        assert_eq!(listed.cards.len(), 1);
        assert!(listed.cards[0].related_terms.is_empty());
        assert_eq!(listed.cards[0].next_review_at, Some(expected_first_review.next_due_at));

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

    /// 回归测试：用户接受卡片后，若 review_schedule 原本定在未来，
    /// 应被拉回到 now，使卡片立即出现在复习队列里。
    #[tokio::test]
    async fn accepting_cards_pulls_future_schedule_due_at_to_now() {
        let (db, card_dao, review_dao) = setup().await;

        // 模拟旧数据：手动插入一张 pending 卡片 + 未来到期的 schedule。
        // 这正是改动前用户库里那 3 张卡的状态。
        card_dao
            .create_generation_batch(&NewGenerationBatch {
                id: "batch-legacy".into(),
                source_type: "manual".into(),
                source_text: "legacy".into(),
                selected_keyword: None,
                context_title: None,
            })
            .await
            .unwrap();

        let now = Utc::now();
        let future_due = now + chrono::Duration::days(1);
        card_dao
            .insert_cards(&[NewCard {
                id: "card-legacy".into(),
                batch_id: Some("batch-legacy".into()),
                keyword: "legacy".into(),
                definition: "def".into(),
                explanation: "exp".into(),
                source_excerpt: None,
                status: "pending".into(),
                next_review_at: Some(future_due),
            }])
            .await
            .unwrap();
        review_dao
            .create_schedule(&crate::models::review::NewReviewSchedule {
                id: "sched-legacy".into(),
                card_id: "card-legacy".into(),
                review_step: 1,
                due_at: future_due,
                status: "pending".into(),
            })
            .await
            .unwrap();

        // 改动前：accept 不会动 schedule；现在应当把 due_at 拉到 <= now。
        review_generated_cards(
            &card_dao,
            ReviewGeneratedCardsInput {
                batch_id: "batch-legacy".into(),
                accept_card_ids: vec!["card-legacy".into()],
                reject_card_ids: vec![],
            },
        )
        .await
        .unwrap();

        let schedule = sqlx::query_as::<_, ReviewSchedule>(
            r#"
            SELECT id, card_id, review_step, due_at, status, created_at, updated_at
            FROM review_schedule
            WHERE id = 'sched-legacy'
            "#,
        )
        .fetch_one(db.pool())
        .await
        .unwrap();

        assert!(
            schedule.due_at <= Utc::now(),
            "接受后 due_at 应被拉到 <= now（实际 {:?}）",
            schedule.due_at
        );

        // 既然 status=accepted 且 due_at <= now，list_due_reviews 必须能拿到它。
        let items = review_dao.list_due_reviews(10).await.unwrap();
        assert!(
            items.iter().any(|x| x.card_id == "card-legacy"),
            "刚接受的卡应立即出现在 list_due_reviews 结果中"
        );
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

    #[test]
    fn validate_generate_input_rejects_empty_text_and_no_images() {
        let mut input = sample_input();
        input.source_text = "   ".into();
        input.image_urls = vec![];
        let err = validate_generate_input(&input).unwrap_err();
        assert_eq!(err.code(), "INVALID_INPUT");
    }

    #[test]
    fn validate_generate_input_accepts_image_only() {
        let mut input = sample_input();
        input.source_text = String::new();
        input.source_type = "image".into();
        input.image_urls = vec!["https://example.com/a.png".into()];
        validate_generate_input(&input).unwrap();
    }

    #[test]
    fn validate_generate_input_rejects_bad_image_scheme() {
        let mut input = sample_input();
        input.image_urls = vec!["ftp://example.com/a.png".into()];
        let err = validate_generate_input(&input).unwrap_err();
        assert_eq!(err.code(), "INVALID_INPUT");
    }

    #[test]
    fn validate_generate_input_accepts_data_url() {
        let mut input = sample_input();
        input.source_text = String::new();
        input.image_urls = vec!["data:image/png;base64,AAAA".into()];
        validate_generate_input(&input).unwrap();
    }

    // ---- 透传：自定义 LLM 验证 image_urls 到了 complete_chat ----

    use std::sync::Mutex;

    struct CapturingLlm {
        last: Mutex<Option<ChatRequest>>,
        response: String,
    }

    #[async_trait::async_trait]
    impl LlmClient for CapturingLlm {
        async fn complete(&self, prompt: &str) -> AppResult<String> {
            *self.last.lock().unwrap() = Some(ChatRequest::from_text(prompt));
            Ok(self.response.clone())
        }
        async fn complete_chat(&self, request: ChatRequest) -> AppResult<String> {
            *self.last.lock().unwrap() = Some(request);
            Ok(self.response.clone())
        }
    }

    fn mock_response() -> String {
        r#"{"cards":[{"keyword":"遗忘曲线","definition":"d","explanation":"e","relatedTerms":[],"scenarios":[],"sourceExcerpt":""}]}"#.to_string()
    }

    #[tokio::test]
    async fn generate_cards_forwards_images_to_llm() {
        let (_db, card_dao, review_dao) = setup().await;
        let llm = CapturingLlm {
            last: Mutex::new(None),
            response: mock_response(),
        };

        let input = GenerateCardsInput {
            source_text: String::new(),
            selected_keyword: None,
            context_title: Some("Ebbinghaus Forgetting Curve".into()),
            source_type: "image".into(),
            model_profile_id: None,
            image_urls: vec![
                "https://example.com/a.png".into(),
                "data:image/png;base64,AAAA".into(),
            ],
        };
        generate_cards(&llm, &card_dao, &review_dao, input).await.unwrap();

        let captured = llm.last.lock().unwrap().clone().expect("llm 未被调用");
        assert_eq!(captured.images.len(), 2, "两张图片都必须透传给 LLM");
        assert_eq!(captured.images[0].url, "https://example.com/a.png");
        assert!(captured.images[1].url.starts_with("data:image/png;base64,"));
        assert!(captured.text.contains("随附图片"), "prompt 应提示模型看图");
    }

    #[tokio::test]
    async fn generate_cards_persists_placeholder_source_text_for_image_only() {
        let (db, card_dao, review_dao) = setup().await;
        let llm = CapturingLlm {
            last: Mutex::new(None),
            response: mock_response(),
        };
        let input = GenerateCardsInput {
            source_text: String::new(),
            selected_keyword: None,
            context_title: None,
            source_type: "image".into(),
            model_profile_id: None,
            image_urls: vec!["https://example.com/x.png".into()],
        };
        let out = generate_cards(&llm, &card_dao, &review_dao, input).await.unwrap();

        let (source_text, source_type): (String, String) = sqlx::query_as(
            "SELECT source_text, source_type FROM generation_batches WHERE id = ?",
        )
        .bind(&out.batch_id)
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(source_type, "image");
        assert!(
            source_text.starts_with("[图片输入"),
            "纯图片批次应写入占位符而不是空串，实际: {source_text}"
        );
    }
}

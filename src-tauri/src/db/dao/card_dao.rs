use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, SqlitePool};

use crate::{
    models::card::{
        parse_keywords_json, serialize_keywords, Card, GenerationBatch, KeywordBucket,
        KeywordBucketQuestion, NewCard, NewGenerationBatch, UpdateCardStatus,
    },
    utils::error::{AppError, AppResult},
};

/// 聚合视图每个桶下保留的最多"问题摘要"数。
const KEYWORD_BUCKET_SAMPLE_LIMIT: usize = 5;

#[async_trait]
pub trait CardDao: Send + Sync {
    async fn create_generation_batch(&self, batch: &NewGenerationBatch) -> AppResult<()>;
    async fn insert_cards(&self, cards: &[NewCard]) -> AppResult<()>;
    async fn list_cards_by_batch(&self, batch_id: &str) -> AppResult<Vec<Card>>;
    async fn review_generated_cards(
        &self,
        batch_id: &str,
        update: &UpdateCardStatus,
    ) -> AppResult<()>;
    async fn get_generation_batch(&self, batch_id: &str) -> AppResult<GenerationBatch>;

    // ---- 宝库：搜索 & 聚合 --------------------------------------------------

    /// 按关键词精确匹配（忽略大小写、去首尾空白）查询卡片。
    ///
    /// 命中规则：某张卡的 `keywords` JSON 数组里包含该关键词，或其
    /// 主关键词 `keyword` 等于该词（兼容老数据）。
    ///
    /// `only_accepted = true` 时只返回 `status = 'accepted'` 的卡；默认 `false`。
    async fn search_cards_by_keyword(
        &self,
        keyword: &str,
        only_accepted: bool,
    ) -> AppResult<Vec<Card>>;

    /// 按问题文本模糊搜索：在 `question`、`definition`、`explanation` 三列做 LIKE。
    async fn search_cards_by_question(
        &self,
        query: &str,
        only_accepted: bool,
        limit: i64,
    ) -> AppResult<Vec<Card>>;

    /// 以关键词为一级维度聚合当前"可见"的卡片，跨批次。
    /// 老数据（`keywords` 为空）会以其主关键词 `keyword` 作为桶 key 兜底。
    async fn list_keyword_buckets(&self, only_accepted: bool) -> AppResult<Vec<KeywordBucket>>;
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
            let keywords_json = serialize_keywords(&card.keywords);
            sqlx::query(
                r#"
                INSERT INTO cards (
                    id, batch_id, keyword, question, keywords,
                    definition, explanation, source_excerpt,
                    status, created_at, updated_at, next_review_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&card.id)
            .bind(&card.batch_id)
            .bind(&card.keyword)
            .bind(&card.question)
            .bind(&keywords_json)
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
            SELECT id, batch_id, keyword, question, keywords,
                   definition, explanation, source_excerpt,
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

    async fn review_generated_cards(
        &self,
        batch_id: &str,
        update: &UpdateCardStatus,
    ) -> AppResult<()> {
        let batch = self.get_generation_batch(batch_id).await?;
        let _ = batch;

        let mut tx = self.pool.begin().await?;

        if !update.accepted_ids.is_empty() {
            let now = Utc::now();

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

            // 把这些卡对应的 review_schedule 在未来的 due_at 拉到 now，
            // 确保用户一旦接受就能在复习队列里立刻看到。
            // 已经到期（due_at <= now）或者 status != 'pending' 的行不动。
            let mut schedule_builder =
                QueryBuilder::new("UPDATE review_schedule SET due_at = ");
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

    // ---- 宝库 API ----------------------------------------------------------

    async fn search_cards_by_keyword(
        &self,
        keyword: &str,
        only_accepted: bool,
    ) -> AppResult<Vec<Card>> {
        let needle = keyword.trim();
        if needle.is_empty() {
            return Ok(Vec::new());
        }

        // 策略：
        // - 在 JSON 列上用 LIKE '%"needle"%' 做宽松匹配（关键词正好是 JSON 字面量）
        // - 同时用 LOWER(keyword) = LOWER(needle) 覆盖老数据
        // 这里对 needle 做了转义（`"` / `%` / `_`）防误伤。
        let escaped = needle.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%\"{}\"%", escaped);
        let lowered = needle.to_lowercase();

        let mut builder = QueryBuilder::new(
            r#"
            SELECT id, batch_id, keyword, question, keywords,
                   definition, explanation, source_excerpt,
                   status, created_at, updated_at, next_review_at
            FROM cards
            WHERE ("#,
        );
        builder.push("keywords LIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" ESCAPE '\\'");
        builder.push(" OR LOWER(keyword) = ");
        builder.push_bind(lowered);
        builder.push(")");
        if only_accepted {
            builder.push(" AND status = 'accepted'");
        }
        builder.push(" ORDER BY created_at DESC");

        let cards = builder
            .build_query_as::<Card>()
            .fetch_all(&self.pool)
            .await?;
        Ok(cards)
    }

    async fn search_cards_by_question(
        &self,
        query: &str,
        only_accepted: bool,
        limit: i64,
    ) -> AppResult<Vec<Card>> {
        let needle = query.trim();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        let escaped = needle
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{}%", escaped);
        let limit = limit.clamp(1, 200);

        let mut builder = QueryBuilder::new(
            r#"
            SELECT id, batch_id, keyword, question, keywords,
                   definition, explanation, source_excerpt,
                   status, created_at, updated_at, next_review_at
            FROM cards
            WHERE ("#,
        );
        builder.push("question   LIKE "); builder.push_bind(pattern.clone()); builder.push(" ESCAPE '\\'");
        builder.push(" OR definition LIKE "); builder.push_bind(pattern.clone()); builder.push(" ESCAPE '\\'");
        builder.push(" OR explanation LIKE "); builder.push_bind(pattern.clone()); builder.push(" ESCAPE '\\'");
        builder.push(" OR keyword   LIKE "); builder.push_bind(pattern); builder.push(" ESCAPE '\\'");
        builder.push(")");
        if only_accepted {
            builder.push(" AND status = 'accepted'");
        }
        builder.push(" ORDER BY created_at DESC LIMIT ");
        builder.push_bind(limit);

        let cards = builder
            .build_query_as::<Card>()
            .fetch_all(&self.pool)
            .await?;
        Ok(cards)
    }

    async fn list_keyword_buckets(&self, only_accepted: bool) -> AppResult<Vec<KeywordBucket>> {
        // 不在 SQL 里 json_each，而是一次性把所有卡片读出来在内存里聚合。
        // 单机知识库规模下（千级）完全 OK，日后量级上来再上 FTS5 / json1 优化。
        let mut builder = QueryBuilder::new(
            r#"
            SELECT id, batch_id, keyword, question, keywords,
                   definition, explanation, source_excerpt,
                   status, created_at, updated_at, next_review_at
            FROM cards
            WHERE 1=1"#,
        );
        if only_accepted {
            builder.push(" AND status = 'accepted'");
        } else {
            // 宝库默认只展示"还活着"的卡：排除 rejected。
            builder.push(" AND status != 'rejected'");
        }
        builder.push(" ORDER BY created_at DESC");

        let rows = builder
            .build_query_as::<Card>()
            .fetch_all(&self.pool)
            .await?;

        // keyword（大小写敏感保留原始形态，但用 lower 去重） → bucket
        use std::collections::HashMap;
        struct Acc {
            keyword_display: String,
            count: i64,
            last_updated_at: DateTime<Utc>,
            samples: Vec<KeywordBucketQuestion>,
        }
        let mut map: HashMap<String, Acc> = HashMap::new();

        for card in rows {
            // 每张卡用 effective_keywords（老数据会自动回退到 [keyword]）
            let kws = card.effective_keywords();
            let question_text = card.effective_question();
            for kw in kws {
                let key = kw.trim().to_lowercase();
                if key.is_empty() {
                    continue;
                }
                let acc = map.entry(key.clone()).or_insert_with(|| Acc {
                    keyword_display: kw.clone(),
                    count: 0,
                    last_updated_at: card.updated_at,
                    samples: Vec::new(),
                });
                acc.count += 1;
                if card.updated_at > acc.last_updated_at {
                    acc.last_updated_at = card.updated_at;
                }
                if acc.samples.len() < KEYWORD_BUCKET_SAMPLE_LIMIT {
                    acc.samples.push(KeywordBucketQuestion {
                        card_id: card.id.clone(),
                        question: question_text.clone(),
                        status: card.status.clone(),
                        created_at: card.created_at,
                    });
                }
            }
        }

        let mut buckets: Vec<KeywordBucket> = map
            .into_iter()
            .map(|(_, v)| KeywordBucket {
                keyword: v.keyword_display,
                question_count: v.count,
                sample_questions: v.samples,
                last_updated_at: v.last_updated_at,
            })
            .collect();

        // 排序：问题数量优先，其次最近更新时间，最后按关键词字典序。
        buckets.sort_by(|a, b| {
            b.question_count
                .cmp(&a.question_count)
                .then(b.last_updated_at.cmp(&a.last_updated_at))
                .then(a.keyword.cmp(&b.keyword))
        });
        Ok(buckets)
    }
}

/// 一个小工具，对外只把 `keywords` JSON 列 parse 出来，供上层 in-memory 过滤使用。
/// DAO 已经做了初步过滤（SQL LIKE），但 LIKE 命中的是"任意位置子串"，为了更精确，
/// 上层可以再用这个 helper 做一次严格 equals 过滤。
pub fn card_keywords_contains_exact(card: &Card, keyword_lower: &str) -> bool {
    let parsed = parse_keywords_json(&card.keywords);
    parsed
        .iter()
        .any(|k| k.trim().to_lowercase() == keyword_lower)
        || card.keyword.trim().to_lowercase() == keyword_lower
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::Database, models::card::NewCard};

    async fn setup() -> SqliteCardDao {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.migrate().await.unwrap();
        SqliteCardDao::new(db.pool().clone())
    }

    fn new_card(
        id: &str,
        batch: &str,
        keyword: &str,
        question: &str,
        keywords: Vec<&str>,
        status: &str,
    ) -> NewCard {
        NewCard {
            id: id.into(),
            batch_id: Some(batch.into()),
            keyword: keyword.into(),
            question: question.into(),
            keywords: keywords.into_iter().map(String::from).collect(),
            definition: "d".into(),
            explanation: "e".into(),
            source_excerpt: None,
            status: status.into(),
            next_review_at: None,
        }
    }

    async fn seed(dao: &SqliteCardDao) {
        dao.create_generation_batch(&NewGenerationBatch {
            id: "b1".into(),
            source_type: "manual".into(),
            source_text: "".into(),
            selected_keyword: None,
            context_title: None,
        })
        .await
        .unwrap();

        dao.insert_cards(&[
            new_card(
                "c1",
                "b1",
                "闭包",
                "什么是闭包？",
                vec!["闭包", "作用域", "词法环境"],
                "accepted",
            ),
            new_card(
                "c2",
                "b1",
                "闭包",
                "闭包和普通函数有什么区别？",
                vec!["闭包", "函数", "引用"],
                "accepted",
            ),
            new_card(
                "c3",
                "b1",
                "动态规划",
                "什么是动态规划？",
                vec!["动态规划", "最优子结构", "状态转移"],
                "pending",
            ),
            new_card(
                "c4",
                "b1",
                "废弃卡",
                "废弃卡是什么？",
                vec!["废弃卡"],
                "rejected",
            ),
        ])
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn search_by_keyword_finds_cross_batch_hits() {
        let dao = setup().await;
        seed(&dao).await;
        let hits = dao.search_cards_by_keyword("闭包", false).await.unwrap();
        let ids: Vec<&str> = hits.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"c1") && ids.contains(&"c2"));
        assert!(!ids.contains(&"c3"));
    }

    #[tokio::test]
    async fn search_by_keyword_only_accepted() {
        let dao = setup().await;
        seed(&dao).await;
        let hits = dao
            .search_cards_by_keyword("动态规划", true)
            .await
            .unwrap();
        assert_eq!(hits.len(), 0, "c3 是 pending，only_accepted=true 应排除");
        let hits2 = dao
            .search_cards_by_keyword("动态规划", false)
            .await
            .unwrap();
        assert_eq!(hits2.len(), 1);
    }

    #[tokio::test]
    async fn search_by_question_matches_fuzzy_text() {
        let dao = setup().await;
        seed(&dao).await;
        let hits = dao
            .search_cards_by_question("区别", false, 50)
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "c2");
    }

    #[tokio::test]
    async fn list_keyword_buckets_aggregates_and_sorts() {
        let dao = setup().await;
        seed(&dao).await;
        let buckets = dao.list_keyword_buckets(false).await.unwrap();
        // "闭包" 出现在 c1、c2 中 → count = 2，应排第一
        assert_eq!(buckets[0].keyword, "闭包");
        assert_eq!(buckets[0].question_count, 2);
        // rejected 的 c4 不应出现
        assert!(!buckets.iter().any(|b| b.keyword == "废弃卡"));
    }

    #[tokio::test]
    async fn list_keyword_buckets_only_accepted_excludes_pending() {
        let dao = setup().await;
        seed(&dao).await;
        let buckets = dao.list_keyword_buckets(true).await.unwrap();
        assert!(!buckets.iter().any(|b| b.keyword == "动态规划"));
    }

    #[test]
    fn card_keywords_contains_exact_matches_case_insensitive() {
        let c = Card {
            id: "x".into(),
            batch_id: None,
            keyword: "Rust".into(),
            question: "".into(),
            keywords: r#"["Rust","Cargo"]"#.into(),
            definition: "".into(),
            explanation: "".into(),
            source_excerpt: None,
            status: "accepted".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            next_review_at: None,
        };
        assert!(card_keywords_contains_exact(&c, "rust"));
        assert!(card_keywords_contains_exact(&c, "cargo"));
        assert!(!card_keywords_contains_exact(&c, "python"));
    }
}

//! 宝库（Library）相关业务逻辑。
//!
//! 为了让前端把"知识宝库"做成以**关键词**为一级维度的视图，这里提供 3 个能力：
//!
//! 1. `search_by_keyword`：根据关键词精确查询该词下的所有卡片（跨批次）。
//! 2. `search_by_question`：对 `question` / `definition` / `explanation` / `keyword`
//!    四列做模糊匹配，用于"按问题找卡"。
//! 3. `list_keyword_buckets`：按关键词聚合，返回"每个关键词下有几张卡 + 最近若干问题"，
//!    作为宝库首屏的列表。
//!
//! 所有输入都以 `camelCase` 命名，方便前端直接用 `invoke` 调用。

use serde::{Deserialize, Serialize};

use crate::{
    db::dao::card_dao::CardDao,
    models::card::{GeneratedCard, KeywordBucket},
    utils::error::AppResult,
};

// ---------------------------------------------------------------------------
// 入参 / 返回 DTO
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchByKeywordInput {
    pub keyword: String,
    /// 可选：是否只看已入库（accepted）。默认 `false`，即宝库全览（排除 rejected）。
    #[serde(default)]
    pub only_accepted: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchByQuestionInput {
    pub query: String,
    #[serde(default)]
    pub only_accepted: bool,
    /// 最多返回条数，默认 50。
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListKeywordBucketsInput {
    #[serde(default)]
    pub only_accepted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCardsResult {
    pub keyword: Option<String>,
    pub query: Option<String>,
    pub cards: Vec<GeneratedCard>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordBucketsResult {
    pub buckets: Vec<KeywordBucket>,
}

// ---------------------------------------------------------------------------
// Service 实现
// ---------------------------------------------------------------------------

pub async fn search_by_keyword(
    card_dao: &dyn CardDao,
    input: SearchByKeywordInput,
) -> AppResult<SearchCardsResult> {
    let keyword = input.keyword.trim();
    if keyword.is_empty() {
        return Ok(SearchCardsResult {
            keyword: None,
            query: None,
            cards: Vec::new(),
        });
    }
    let rows = card_dao
        .search_cards_by_keyword(keyword, input.only_accepted)
        .await?;
    // DAO 已经用 `LIKE '%"keyword"%'` 做过初筛；再在应用层用 effective_keywords
    // 做一次严格过滤，避免诸如 keyword="js" 误命中 "jsonl" 这种子串问题。
    let needle = keyword.to_lowercase();
    let cards: Vec<GeneratedCard> = rows
        .into_iter()
        .filter(|c| {
            c.effective_keywords()
                .iter()
                .any(|k| k.trim().to_lowercase() == needle)
        })
        .map(GeneratedCard::from)
        .collect();
    Ok(SearchCardsResult {
        keyword: Some(keyword.to_string()),
        query: None,
        cards,
    })
}

pub async fn search_by_question(
    card_dao: &dyn CardDao,
    input: SearchByQuestionInput,
) -> AppResult<SearchCardsResult> {
    let query = input.query.trim();
    if query.is_empty() {
        return Ok(SearchCardsResult {
            keyword: None,
            query: None,
            cards: Vec::new(),
        });
    }
    let limit = input.limit.unwrap_or(50);
    let rows = card_dao
        .search_cards_by_question(query, input.only_accepted, limit)
        .await?;
    let cards = rows.into_iter().map(GeneratedCard::from).collect();
    Ok(SearchCardsResult {
        keyword: None,
        query: Some(query.to_string()),
        cards,
    })
}

pub async fn list_keyword_buckets(
    card_dao: &dyn CardDao,
    input: ListKeywordBucketsInput,
) -> AppResult<KeywordBucketsResult> {
    let buckets = card_dao.list_keyword_buckets(input.only_accepted).await?;
    Ok(KeywordBucketsResult { buckets })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::{dao::card_dao::SqliteCardDao, Database},
        models::card::{NewCard, NewGenerationBatch},
    };

    async fn setup() -> SqliteCardDao {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.migrate().await.unwrap();
        SqliteCardDao::new(db.pool().clone())
    }

    fn make(id: &str, batch: &str, kws: Vec<&str>, q: &str, status: &str) -> NewCard {
        NewCard {
            id: id.into(),
            batch_id: Some(batch.into()),
            keyword: kws[0].into(),
            question: q.into(),
            keywords: kws.into_iter().map(String::from).collect(),
            definition: "def".into(),
            explanation: "exp".into(),
            source_excerpt: None,
            status: status.into(),
            next_review_at: None,
        }
    }

    async fn seed(dao: &SqliteCardDao) {
        dao.create_generation_batch(&NewGenerationBatch {
            id: "b".into(),
            source_type: "manual".into(),
            source_text: "".into(),
            selected_keyword: None,
            context_title: None,
        })
        .await
        .unwrap();
        dao.insert_cards(&[
            make("c1", "b", vec!["js", "async"], "async 是什么？", "accepted"),
            make("c2", "b", vec!["jsonl", "纯文本"], "jsonl 是什么？", "accepted"),
            make("c3", "b", vec!["闭包"], "闭包是什么？", "pending"),
        ])
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn search_by_keyword_does_exact_match_not_substring() {
        let dao = setup().await;
        seed(&dao).await;
        let r = search_by_keyword(
            &dao,
            SearchByKeywordInput {
                keyword: "js".into(),
                only_accepted: false,
            },
        )
        .await
        .unwrap();
        let ids: Vec<&str> = r.cards.iter().map(|c| c.card_id.as_str()).collect();
        assert_eq!(ids, vec!["c1"], "不应匹配到 c2 的 jsonl");
    }

    #[tokio::test]
    async fn search_by_question_fuzzy() {
        let dao = setup().await;
        seed(&dao).await;
        let r = search_by_question(
            &dao,
            SearchByQuestionInput {
                query: "是什么".into(),
                only_accepted: false,
                limit: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(r.cards.len(), 3);
    }

    #[tokio::test]
    async fn list_keyword_buckets_via_service() {
        let dao = setup().await;
        seed(&dao).await;
        let r = list_keyword_buckets(
            &dao,
            ListKeywordBucketsInput {
                only_accepted: false,
            },
        )
        .await
        .unwrap();
        // 不会包含 rejected。accepted + pending 都算活的：
        // js, async, jsonl, 纯文本, 闭包 共 5 个桶。
        assert_eq!(r.buckets.len(), 5);
    }

    #[tokio::test]
    async fn empty_inputs_return_empty_results() {
        let dao = setup().await;
        seed(&dao).await;
        let r = search_by_keyword(
            &dao,
            SearchByKeywordInput {
                keyword: "   ".into(),
                only_accepted: false,
            },
        )
        .await
        .unwrap();
        assert!(r.cards.is_empty());
        let r = search_by_question(
            &dao,
            SearchByQuestionInput {
                query: "".into(),
                only_accepted: false,
                limit: None,
            },
        )
        .await
        .unwrap();
        assert!(r.cards.is_empty());
    }
}

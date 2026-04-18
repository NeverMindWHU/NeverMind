use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 卡片在 DB 层的原始行。
///
/// `keywords_json` 存储形如 `["k1","k2","k3"]` 的 JSON 字符串。
/// 对外暴露的 `GeneratedCard` 已经把它解析回 `Vec<String>`，
/// 调用方一般不会直接看到 JSON。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Card {
    pub id: String,
    pub batch_id: Option<String>,
    /// 旧模型残留：当前作为"主关键词（首选）"保留，用于老数据兜底。
    /// 新写入时一般等于 `keywords[0]`。
    pub keyword: String,
    /// 新：完整问题文本（疑问句）。旧数据为空串，读取时走 fallback。
    #[sqlx(default)]
    pub question: String,
    /// 新：3 个关键词，JSON 数组字符串。旧数据为 "[]"。
    #[sqlx(default)]
    pub keywords: String,
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
    /// 主关键词（通常取 `keywords[0]`），用于与旧查询/展示兼容。
    pub keyword: String,
    /// 完整问题文本（疑问句）。调用方构造时必填。
    pub question: String,
    /// 3 个关键词。调用方构造时填完整 3 个；持久化时序列化为 JSON 字符串。
    pub keywords: Vec<String>,
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
    /// 主关键词（一般等于 `keywords[0]`），与老版 UI 兼容。
    pub keyword: String,
    /// 完整问题文本（疑问句）。
    pub question: String,
    /// 3 个关键词（已从 JSON 列解析回 `Vec<String>`）。
    pub keywords: Vec<String>,
    pub definition: String,
    pub explanation: String,
    pub related_terms: Vec<String>,
    pub scenarios: Vec<String>,
    pub source_excerpt: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub review_history: Vec<String>,
    pub next_review_at: Option<DateTime<Utc>>,
    /// 所属批次（便于前端从宝库跳回该卡的上下文）。
    pub batch_id: Option<String>,
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

/// 宝库"按关键词桶"聚合视图的一项。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordBucket {
    pub keyword: String,
    pub question_count: i64,
    /// 该关键词下的若干问题摘要（最多若干条，按最近创建时间排序）。
    pub sample_questions: Vec<KeywordBucketQuestion>,
    pub last_updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordBucketQuestion {
    pub card_id: String,
    pub question: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl Card {
    /// 解析存储在 DB 里的 `keywords` JSON 列，失败时回退到空数组。
    pub fn parsed_keywords(&self) -> Vec<String> {
        parse_keywords_json(&self.keywords)
    }

    /// 读时兜底：老卡片 `question` 可能为空串，统一回填成"xxx是什么？"。
    pub fn effective_question(&self) -> String {
        let q = self.question.trim();
        if !q.is_empty() {
            return q.to_string();
        }
        format!("{}是什么？", self.keyword)
    }

    /// 读时兜底：老卡片 `keywords` 可能为空数组，至少回退到 `[keyword]`。
    pub fn effective_keywords(&self) -> Vec<String> {
        let mut kws = self.parsed_keywords();
        if kws.is_empty() && !self.keyword.trim().is_empty() {
            kws.push(self.keyword.clone());
        }
        kws
    }
}

/// 把 `Vec<String>` 序列化成 `["..","..",..]` 形式，入库前调用。
pub fn serialize_keywords(keywords: &[String]) -> String {
    // serde_json 对 Vec<String> 的序列化是稳定的，不会失败。
    serde_json::to_string(keywords).unwrap_or_else(|_| "[]".to_string())
}

/// 解析 DB 里的 `keywords` JSON 列，任何失败都安全回退为空。
pub fn parse_keywords_json(raw: &str) -> Vec<String> {
    if raw.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

impl From<Card> for GeneratedCard {
    fn from(c: Card) -> Self {
        let question = c.effective_question();
        let keywords = c.effective_keywords();
        Self {
            card_id: c.id,
            keyword: c.keyword,
            question,
            keywords,
            definition: c.definition,
            explanation: c.explanation,
            related_terms: Vec::new(),
            scenarios: Vec::new(),
            source_excerpt: c.source_excerpt,
            status: c.status,
            created_at: c.created_at,
            review_history: Vec::new(),
            next_review_at: c.next_review_at,
            batch_id: c.batch_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_card(keyword: &str, question: &str, keywords_json: &str) -> Card {
        Card {
            id: "id".into(),
            batch_id: None,
            keyword: keyword.into(),
            question: question.into(),
            keywords: keywords_json.into(),
            definition: "def".into(),
            explanation: "exp".into(),
            source_excerpt: None,
            status: "pending".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            next_review_at: None,
        }
    }

    #[test]
    fn effective_question_fallbacks_to_template_when_empty() {
        let c = make_card("闭包", "", "[]");
        assert_eq!(c.effective_question(), "闭包是什么？");
    }

    #[test]
    fn effective_question_uses_stored_when_present() {
        let c = make_card("闭包", "闭包是如何实现的？", "[]");
        assert_eq!(c.effective_question(), "闭包是如何实现的？");
    }

    #[test]
    fn effective_keywords_fallbacks_to_single_keyword_when_empty() {
        let c = make_card("闭包", "", "[]");
        assert_eq!(c.effective_keywords(), vec!["闭包".to_string()]);
    }

    #[test]
    fn effective_keywords_parses_json_array() {
        let c = make_card("闭包", "", r#"["闭包","作用域","词法环境"]"#);
        assert_eq!(
            c.effective_keywords(),
            vec![
                "闭包".to_string(),
                "作用域".to_string(),
                "词法环境".to_string()
            ]
        );
    }

    #[test]
    fn serialize_keywords_round_trips() {
        let kws = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let s = serialize_keywords(&kws);
        assert_eq!(parse_keywords_json(&s), kws);
    }

    #[test]
    fn parse_keywords_json_tolerates_garbage() {
        assert!(parse_keywords_json("").is_empty());
        assert!(parse_keywords_json("not-json").is_empty());
        assert!(parse_keywords_json("[").is_empty());
    }
}

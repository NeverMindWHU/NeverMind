use serde::Deserialize;

use crate::utils::error::{AppError, AppResult};

/// LLM 原始输出中的单张卡片结构。
///
/// 新协议（v2）字段：
/// - `question`：完整问题文本（疑问句）。疑问句输入 → 原样；陈述句 → 拼成"xxx是什么？"。
/// - `keywords`：**恰好 3 个**关键词。若模型只给 1~2 个也容忍，但上层会尝试补齐。
///
/// 兼容旧协议（v1）：`keyword + definition + explanation`，parser 在这种情况下
/// 会自动把 `keyword` 折算成 `keywords = [keyword]`、`question = "<keyword>是什么？"`。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedCard {
    /// v2 主字段；v1 没有。
    #[serde(default)]
    pub question: String,
    /// v2 主字段；v1 没有。
    #[serde(default)]
    pub keywords: Vec<String>,

    /// v1 必需；v2 也保留作为"主关键词"（用于兼容旧 UI、老索引）。
    /// v2 里可能是 v2 keywords[0]，也可能模型没提供 → 由 parser 回填。
    #[serde(default)]
    pub keyword: String,

    pub definition: String,
    pub explanation: String,

    #[serde(default)]
    pub related_terms: Vec<String>,
    #[serde(default)]
    pub scenarios: Vec<String>,
    #[serde(default)]
    pub source_excerpt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ParsedResponse {
    cards: Vec<ParsedCard>,
}

/// 从 LLM 返回的原始字符串中解析出卡片列表。
///
/// 容忍：
/// - Markdown 代码块围栏（```json ... ```）
/// - v1 老 schema（无 `question` / `keywords`）：自动回填 `question = "<keyword>是什么？"`、
///   `keywords = [keyword]`
/// - v2 缺失主关键词：取 `keywords[0]`；若 `keywords` 也为空则报错。
pub fn parse_cards(raw: &str) -> AppResult<Vec<ParsedCard>> {
    let json = extract_json_body(raw);
    let mut parsed: ParsedResponse = serde_json::from_str(&json).map_err(|e| {
        AppError::AiResponseInvalid {
            message: format!("JSON 解析失败: {}", e),
        }
    })?;

    if parsed.cards.is_empty() {
        return Err(AppError::AiResponseInvalid {
            message: "模型未返回任何卡片".into(),
        });
    }

    for (i, card) in parsed.cards.iter_mut().enumerate() {
        // 1) 定义/解释必填
        if card.definition.trim().is_empty() {
            return Err(AppError::AiResponseInvalid {
                message: format!("第 {} 张卡片缺少 definition", i + 1),
            });
        }

        // 2) keywords 规整：去空白 / 去重，保留首次出现顺序
        card.keywords = normalize_keywords(&card.keywords);

        // 3) 回填 keyword（主关键词）：优先取已存在的 keyword，再 fallback 到 keywords[0]
        if card.keyword.trim().is_empty() {
            if let Some(first) = card.keywords.first() {
                card.keyword = first.clone();
            }
        }
        if card.keyword.trim().is_empty() {
            return Err(AppError::AiResponseInvalid {
                message: format!("第 {} 张卡片缺少关键词（keyword/keywords 均为空）", i + 1),
            });
        }

        // 4) 若 v1 老 schema 没给 keywords，至少保证 keywords = [keyword]
        if card.keywords.is_empty() {
            card.keywords = vec![card.keyword.clone()];
        }

        // 5) 回填 question：
        //    - 空 → "xxx是什么？"
        //    - 模型返回的是陈述句（没以 "？/?" 结尾，也没有常见疑问词） → 也拼成"xxx是什么？"
        //      ——让用户在宝库里看到统一的疑问句形式。
        let trimmed_q = card.question.trim();
        if trimmed_q.is_empty() {
            card.question = format!("{}是什么？", card.keyword);
        } else if !looks_like_question(trimmed_q) {
            card.question = format!("{}是什么？", card.keyword);
        } else {
            card.question = trimmed_q.to_string();
        }
    }

    Ok(parsed.cards)
}

fn normalize_keywords(raw: &[String]) -> Vec<String> {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::with_capacity(raw.len());
    for k in raw {
        let t = k.trim();
        if t.is_empty() {
            continue;
        }
        let key = t.to_string();
        if seen.insert(key.clone()) {
            out.push(key);
        }
    }
    out
}

/// 粗粒度判断一段文本是否已经是疑问句。
///
/// 覆盖中英文常见标记：`？`/`?` 结尾，或首尾出现「什么/为什么/怎么/如何/怎样/
/// 哪/吗/能否/是否/与……的区别/what/why/how/...」等。命中任一就算疑问句。
fn looks_like_question(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    if t.ends_with('？') || t.ends_with('?') {
        return true;
    }
    let lower = t.to_lowercase();
    const ZH_MARKERS: &[&str] = &[
        "什么", "为什么", "怎么", "怎样", "如何", "哪", "是否", "能否", "吗", "区别",
    ];
    const EN_MARKERS: &[&str] = &[
        "what ", "why ", "how ", "when ", "where ", "which ", "who ", "whose ", "can ", "could ",
        "should ", "would ", "is ", "are ", "does ", "do ",
    ];
    for m in ZH_MARKERS {
        if t.contains(m) {
            return true;
        }
    }
    for m in EN_MARKERS {
        if lower.starts_with(m) {
            return true;
        }
    }
    false
}

/// 剥离 Markdown 代码块围栏，提取纯 JSON。
fn extract_json_body(raw: &str) -> String {
    let trimmed = raw.trim();
    let stripped = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.trim_start())
        .unwrap_or(trimmed);
    stripped
        .strip_suffix("```")
        .unwrap_or(stripped)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v2_full_payload() {
        let raw = r#"{"cards":[{
            "question":"什么是闭包？",
            "keywords":["闭包","作用域","词法环境"],
            "definition":"D",
            "explanation":"E"
        }]}"#;
        let cards = parse_cards(raw).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].question, "什么是闭包？");
        assert_eq!(cards[0].keywords.len(), 3);
        // 主关键词回填为 keywords[0]
        assert_eq!(cards[0].keyword, "闭包");
    }

    #[test]
    fn parses_v1_legacy_payload_and_fills_question_keywords() {
        // 旧 schema：只有 keyword + definition + explanation
        let raw = r#"{"cards":[{"keyword":"闭包","definition":"D","explanation":"E"}]}"#;
        let cards = parse_cards(raw).unwrap();
        assert_eq!(cards[0].question, "闭包是什么？");
        assert_eq!(cards[0].keywords, vec!["闭包".to_string()]);
    }

    #[test]
    fn declarative_question_is_rewritten_to_xxx_shi_shenme() {
        let raw = r#"{"cards":[{
            "question":"闭包是一种特殊的函数",
            "keywords":["闭包"],
            "definition":"D","explanation":"E"
        }]}"#;
        let cards = parse_cards(raw).unwrap();
        // 陈述句 → 改写
        assert_eq!(cards[0].question, "闭包是什么？");
    }

    #[test]
    fn parses_json_with_markdown_fence() {
        let raw = "```json\n{\"cards\":[{\"keyword\":\"X\",\"definition\":\"Y\",\"explanation\":\"Z\"}]}\n```";
        let cards = parse_cards(raw).unwrap();
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn rejects_empty_cards_array() {
        let raw = r#"{"cards":[]}"#;
        let err = parse_cards(raw).unwrap_err();
        assert_eq!(err.code(), "AI_RESPONSE_INVALID");
    }

    #[test]
    fn rejects_missing_any_keyword_source() {
        let raw = r#"{"cards":[{"definition":"Y","explanation":"Z"}]}"#;
        let err = parse_cards(raw).unwrap_err();
        assert_eq!(err.code(), "AI_RESPONSE_INVALID");
    }

    #[test]
    fn normalizes_duplicate_and_whitespace_keywords() {
        let raw = r#"{"cards":[{
            "question":"什么是 A？",
            "keywords":["A","  A  ","B",""],
            "definition":"D","explanation":"E"
        }]}"#;
        let cards = parse_cards(raw).unwrap();
        assert_eq!(cards[0].keywords, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn looks_like_question_covers_common_markers() {
        assert!(looks_like_question("这是什么？"));
        assert!(looks_like_question("What is closure"));
        assert!(looks_like_question("如何理解闭包"));
        assert!(!looks_like_question("闭包是一种特殊函数"));
    }
}

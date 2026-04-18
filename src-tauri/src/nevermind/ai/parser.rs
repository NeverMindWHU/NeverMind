use serde::Deserialize;

use crate::nevermind::utils::error::{AppError, AppResult};

/// LLM 原始输出中的单张卡片结构。
/// 字段命名用 `camelCase` 与 Prompt 要求一致；解析为 Rust `snake_case`。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedCard {
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
/// 容忍常见的 Markdown 代码块围栏（```json ... ```）。
pub fn parse_cards(raw: &str) -> AppResult<Vec<ParsedCard>> {
    let json = extract_json_body(raw);
    let parsed: ParsedResponse = serde_json::from_str(&json).map_err(|e| {
        AppError::AiResponseInvalid {
            message: format!("JSON 解析失败: {}", e),
        }
    })?;

    if parsed.cards.is_empty() {
        return Err(AppError::AiResponseInvalid {
            message: "模型未返回任何卡片".into(),
        });
    }

    for (i, card) in parsed.cards.iter().enumerate() {
        if card.keyword.trim().is_empty() || card.definition.trim().is_empty() {
            return Err(AppError::AiResponseInvalid {
                message: format!("第 {} 张卡片缺少 keyword 或 definition", i + 1),
            });
        }
    }

    Ok(parsed.cards)
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
    fn parses_plain_json() {
        let raw = r#"{"cards":[{"keyword":"X","definition":"Y","explanation":"Z"}]}"#;
        let cards = parse_cards(raw).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].keyword, "X");
    }

    #[test]
    fn parses_json_with_markdown_fence() {
        let raw = "```json\n{\"cards\":[{\"keyword\":\"X\",\"definition\":\"Y\",\"explanation\":\"Z\"}]}\n```";
        let cards = parse_cards(raw).unwrap();
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn fills_optional_fields_with_defaults() {
        let raw = r#"{"cards":[{"keyword":"X","definition":"Y","explanation":"Z"}]}"#;
        let cards = parse_cards(raw).unwrap();
        assert!(cards[0].related_terms.is_empty());
        assert!(cards[0].scenarios.is_empty());
        assert!(cards[0].source_excerpt.is_none());
    }

    #[test]
    fn rejects_empty_cards_array() {
        let raw = r#"{"cards":[]}"#;
        let err = parse_cards(raw).unwrap_err();
        assert_eq!(err.code(), "AI_RESPONSE_INVALID");
    }

    #[test]
    fn rejects_missing_required_fields() {
        let raw = r#"{"cards":[{"keyword":"","definition":"Y","explanation":"Z"}]}"#;
        let err = parse_cards(raw).unwrap_err();
        assert_eq!(err.code(), "AI_RESPONSE_INVALID");
    }
}

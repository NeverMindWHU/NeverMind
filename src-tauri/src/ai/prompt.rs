/// 根据用户输入构造发送给 LLM 的 Prompt。
///
/// 要求模型严格返回 JSON，格式与 `parser::ParsedCard` 对齐：
/// ```json
/// {
///   "cards": [
///     { "keyword": "...", "definition": "...", "explanation": "...",
///       "relatedTerms": ["..."], "scenarios": ["..."], "sourceExcerpt": "..." }
///   ]
/// }
/// ```
pub fn build_prompt(
    source_text: &str,
    selected_keyword: Option<&str>,
    context_title: Option<&str>,
) -> String {
    let keyword_hint = match selected_keyword {
        Some(k) if !k.trim().is_empty() => format!("请重点围绕关键词「{}」提炼。\n", k.trim()),
        _ => String::new(),
    };
    let context_hint = match context_title {
        Some(c) if !c.trim().is_empty() => format!("来源标题：{}\n", c.trim()),
        _ => String::new(),
    };

    format!(
        r#"你是一名知识卡片生成助手。请阅读下列内容，提炼 1 到 3 张结构化的知识卡片。

严格按如下 JSON 返回，不要输出任何额外说明或 Markdown 代码块以外的文字：
{{
  "cards": [
    {{
      "keyword": "简洁的关键词或术语",
      "definition": "正式、精准的定义",
      "explanation": "更通俗的解释",
      "relatedTerms": ["关联词1", "关联词2"],
      "scenarios": ["应用场景1", "应用场景2"],
      "sourceExcerpt": "原文中最能支撑该卡片的一句话，没有则留空字符串"
    }}
  ]
}}

{context_hint}{keyword_hint}
原始内容：
{source_text}
"#,
        context_hint = context_hint,
        keyword_hint = keyword_hint,
        source_text = source_text.trim(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_embeds_source_text() {
        let p = build_prompt("艾宾浩斯遗忘曲线。", Some("遗忘曲线"), Some("心理学笔记"));
        assert!(p.contains("艾宾浩斯遗忘曲线"));
        assert!(p.contains("遗忘曲线"));
        assert!(p.contains("心理学笔记"));
    }

    #[test]
    fn prompt_without_optional_fields() {
        let p = build_prompt("只给原文。", None, None);
        assert!(p.contains("只给原文"));
        assert!(!p.contains("来源标题"));
        assert!(!p.contains("请重点围绕关键词"));
    }
}

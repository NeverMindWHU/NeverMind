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
///
/// 参数：
/// - `source_text`：原文（图片场景下可能为空串）
/// - `selected_keyword`：用户显式选中的关键词（可选）
/// - `context_title`：来源标题（可选）
/// - `has_images`：是否还附带了图片输入。`true` 时 prompt 会告知模型
///   "另附图片"，并在原文为空时要求模型根据图片提炼；这样同一段 prompt
///   可用于纯文本、纯图片、图文混合三种情况。
pub fn build_prompt(
    source_text: &str,
    selected_keyword: Option<&str>,
    context_title: Option<&str>,
    has_images: bool,
) -> String {
    let keyword_hint = match selected_keyword {
        Some(k) if !k.trim().is_empty() => format!("请重点围绕关键词「{}」提炼。\n", k.trim()),
        _ => String::new(),
    };
    let context_hint = match context_title {
        Some(c) if !c.trim().is_empty() => format!("来源标题：{}\n", c.trim()),
        _ => String::new(),
    };

    let trimmed_text = source_text.trim();
    let content_section = match (trimmed_text.is_empty(), has_images) {
        (false, false) => format!("原始内容：\n{}\n", trimmed_text),
        (false, true) => format!(
            "原始内容（另附 {} 张图片，需结合图文一起理解）：\n{}\n",
            "若干", trimmed_text
        ),
        (true, true) => "原始内容以图片形式提供，请根据随附图片的视觉内容提炼卡片。\n".to_string(),
        (true, false) => {
            // 调用方应已在业务层拦截，这里给个安全兜底避免 prompt 完全空白。
            "原始内容：（无）\n".to_string()
        }
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

{context_hint}{keyword_hint}{content_section}"#,
        context_hint = context_hint,
        keyword_hint = keyword_hint,
        content_section = content_section,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_embeds_source_text() {
        let p = build_prompt(
            "艾宾浩斯遗忘曲线。",
            Some("遗忘曲线"),
            Some("心理学笔记"),
            false,
        );
        assert!(p.contains("艾宾浩斯遗忘曲线"));
        assert!(p.contains("遗忘曲线"));
        assert!(p.contains("心理学笔记"));
    }

    #[test]
    fn prompt_without_optional_fields() {
        let p = build_prompt("只给原文。", None, None, false);
        assert!(p.contains("只给原文"));
        assert!(!p.contains("来源标题"));
        assert!(!p.contains("请重点围绕关键词"));
    }

    #[test]
    fn prompt_image_only_mentions_image_section() {
        let p = build_prompt("", None, Some("一张示意图"), true);
        assert!(p.contains("随附图片"), "纯图片场景必须提示模型看图");
        assert!(p.contains("一张示意图"));
    }

    #[test]
    fn prompt_text_plus_image_keeps_both_hints() {
        let p = build_prompt("这是一段补充说明。", Some("遗忘曲线"), None, true);
        assert!(p.contains("这是一段补充说明"));
        assert!(p.contains("结合图文"), "图文混合模式应提示结合图片");
        assert!(p.contains("遗忘曲线"));
    }
}

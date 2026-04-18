/// 根据用户输入构造发送给 LLM 的 Prompt。
///
/// v2 输出协议（与 `parser::ParsedCard` 对齐）：
/// ```json
/// {
///   "cards": [
///     {
///       "question": "<疑问句形式的问题>",
///       "keywords": ["<关键词 1>", "<关键词 2>", "<关键词 3>"],
///       "definition": "<精准的定义>",
///       "explanation": "<通俗解释>",
///       "relatedTerms": ["..."],
///       "scenarios": ["..."],
///       "sourceExcerpt": "<原文摘录>"
///     }
///   ]
/// }
/// ```
///
/// 输入形态 → 生成策略：
/// - `selected_keyword` 非空  → 围绕该关键词提炼 1~3 张卡，每张恰好 3 个关键词
///   （其中首个为用户给定的关键词或其紧密相关概念）。
/// - `selected_keyword` 为空  → 由模型自主挑选恰好 3 张卡，每张恰好 3 个关键词；
///   各张卡的主关键词（`keywords[0]`）必须互不相同。
///
/// 疑问/陈述句规则（由模型执行，parser 兜底）：
/// - 若原文或意图本身是疑问句 → 原样作为 `question`
/// - 若是陈述句 → 将该知识点改写为疑问句，统一风格 "xxx是什么？" 或 "xxx 与 yyy 有什么区别？"
pub fn build_prompt(
    source_text: &str,
    selected_keyword: Option<&str>,
    context_title: Option<&str>,
    has_images: bool,
) -> String {
    let has_user_keyword = selected_keyword
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);

    let (card_count_hint, keyword_hint) = if has_user_keyword {
        let k = selected_keyword.unwrap().trim();
        (
            "请根据内容提炼 1 到 3 张结构化的知识卡片。".to_string(),
            format!(
                "请整批围绕关键词「{}」展开，每张卡片的 `keywords` 数组的第 0 项应当是该关键词本身或其紧密相关的子概念。\n",
                k
            ),
        )
    } else {
        (
            "请从下列内容中自主挑选**恰好 3 个不同主题**，为每个主题生成一张卡片，最终返回正好 3 张卡片。".to_string(),
            "未指定关注关键词时，每张卡片 `keywords[0]`（主关键词）必须互不相同，覆盖内容里最值得记忆的 3 个知识点。\n".to_string(),
        )
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
        r#"你是一名知识卡片生成助手。{card_count_hint}

每张卡片需要同时具备：
1. `question`：一个清晰的**疑问句**。
   - 如果用户原始内容本身就是疑问句，请保留其问法（允许改写得更精确）。
   - 如果原始内容是陈述句，请把其核心知识改写成疑问句；最常见的形式是 "xxx是什么？"、
     "xxx 为什么会 yyy？"、"xxx 和 yyy 有什么区别？" 等。绝不要在 `question` 里填陈述句。
2. `keywords`：**恰好 3 个**互不相同的关键词/术语，按相关性从主到次排序。
   - 关键词必须是**短语或术语**（2–8 字为佳），不要整句话。
   - 同批次里，不同卡片的 `keywords[0]`（主关键词）不可重复。
3. `definition`：正式、精准的定义。
4. `explanation`：更通俗的解释，可包含直觉类比。
5. `relatedTerms` / `scenarios`：可选的补充。
6. `sourceExcerpt`：原文里最能支撑该卡片的一句话；没有则填空字符串。

严格按如下 JSON 返回，不要输出任何额外说明或 Markdown 代码块以外的文字：
{{
  "cards": [
    {{
      "question": "……？",
      "keywords": ["主关键词", "关键词2", "关键词3"],
      "definition": "正式、精准的定义",
      "explanation": "更通俗的解释",
      "relatedTerms": ["关联词1", "关联词2"],
      "scenarios": ["应用场景1", "应用场景2"],
      "sourceExcerpt": "原文中最能支撑该卡片的一句话，没有则留空字符串"
    }}
  ]
}}

{context_hint}{keyword_hint}{content_section}"#,
        card_count_hint = card_count_hint,
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
        assert!(p.contains("1 到 3 张"));
        // v2：总是要求 question + keywords 协议
        assert!(p.contains("\"question\""));
        assert!(p.contains("\"keywords\""));
        assert!(p.contains("恰好 3 个"));
    }

    #[test]
    fn prompt_without_optional_fields_requires_three_cards() {
        let p = build_prompt("只给原文。", None, None, false);
        assert!(p.contains("只给原文"));
        assert!(!p.contains("来源标题"));
        assert!(
            p.contains("恰好 3 个不同主题"),
            "未指定关键词时 prompt 必须要求 3 张卡，实际 prompt: {p}"
        );
        assert!(
            p.contains("正好 3 张"),
            "未指定关键词时 prompt 必须要求正好 3 张卡，实际 prompt: {p}"
        );
        assert!(
            p.contains("互不相同"),
            "未指定关键词时 prompt 必须要求每张卡的主关键词互不相同"
        );
    }

    #[test]
    fn prompt_with_blank_selected_keyword_treated_as_unset() {
        let p = build_prompt("原文。", Some("   "), None, false);
        assert!(p.contains("恰好 3 个不同主题"));
    }

    #[test]
    fn prompt_explains_declarative_to_question_rewrite() {
        let p = build_prompt("原文。", None, None, false);
        assert!(p.contains("疑问句"), "prompt 必须强调 question 是疑问句");
        assert!(p.contains("xxx是什么"), "prompt 必须给出 xxx是什么？ 作为示例");
        assert!(p.contains("陈述句"), "prompt 必须覆盖陈述句改写规则");
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

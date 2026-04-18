-- 将卡片模型从"单关键词 + 定义 + 解释"升级为
-- "问题 + 3 个关键词 + 答案（definition + explanation）"。
--
-- 设计取舍：
-- - 不引入独立的 keywords / question_keywords 表。关键词数量恒为 3 且由 AI 生成，
--   属于自由文本集合，JSON 列可以在 SQLite 里用 json_each / LIKE 查询，足够当前场景。
-- - 老数据在读时通过代码层 fallback 兼容：
--     * question 为空  → 使用 "{keyword}是什么？"
--     * keywords 为空 → 使用 [keyword]
--   这样不需要数据回填脚本。
-- - 为"按关键词搜索"提供 LIKE 专用索引前置：在 JSON 列上我们用简单 LIKE
--   '%"xxx"%'，结合 idx_cards_keywords 这种表达式索引并非必须；实际量级下
--   全表扫描 + 内存过滤已足够。未来量上来可以加 FTS5。

ALTER TABLE cards ADD COLUMN question TEXT NOT NULL DEFAULT '';
ALTER TABLE cards ADD COLUMN keywords TEXT NOT NULL DEFAULT '[]';

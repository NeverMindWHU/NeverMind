# 卡片生成模块接口契约

## 1. 模块目标

卡片生成模块负责将用户输入的关键词或选中文本转换为结构化知识卡片，并在用户确认后写入本地数据库，同时初始化首个复习计划。

---

## 2. 前后端边界

前端负责：

- 收集输入内容
- 展示生成中、失败、成功状态
- 允许用户二次编辑生成结果
- 触发保存

后端负责：

- 校验输入
- 调用 AI 模型
- 标准化生成结果
- 持久化卡片
- 初始化复习计划

---

## 3. Command 列表

### 3.1 `preview_generated_card`

用途：

- 根据输入文本或关键词生成卡片预览
- 不写入数据库

输入：

```json
{
  "sourceText": "string",
  "selectedKeyword": "string | null",
  "contextTitle": "string | null",
  "sourceType": "manual | selection | import",
  "modelProfileId": "string | null"
}
```

字段说明：

- `sourceText`：原始文本，最少 1 个字符，最大长度建议 5000
- `selectedKeyword`：用户显式选择的关键词，可为空
- `contextTitle`：来源标题，例如文章标题、章节名，可为空
- `sourceType`：来源类型
- `modelProfileId`：指定模型配置，可为空，空时使用默认模型

成功返回：

```json
{
  "success": true,
  "data": {
    "keyword": "艾宾浩斯遗忘曲线",
    "definition": "用于描述记忆随时间衰减规律的理论模型",
    "explanation": "它帮助系统计算什么时候复习更有效",
    "relatedTerms": ["记忆", "复习", "排期"],
    "scenarios": ["考试复习", "长期记忆训练"],
    "mnemonic": "先记后忘，按点再访",
    "sourceExcerpt": "遗忘曲线说明记忆会随时间下降"
  }
}
```

失败返回：

```json
{
  "success": false,
  "error": {
    "code": "AI_TIMEOUT",
    "message": "卡片生成超时",
    "details": null
  }
}
```

### 3.2 `save_generated_card`

用途：

- 保存用户确认后的卡片内容
- 初始化复习计划

输入：

```json
{
  "keyword": "string",
  "definition": "string",
  "explanation": "string",
  "relatedTerms": ["string"],
  "scenarios": ["string"],
  "mnemonic": "string | null",
  "sourceExcerpt": "string | null",
  "sourceType": "manual | selection | import",
  "sourceTitle": "string | null",
  "tags": ["string"]
}
```

成功返回：

```json
{
  "success": true,
  "data": {
    "cardId": "uuid",
    "createdAt": "2026-04-18T10:30:00Z",
    "nextReviewAt": "2026-04-19T10:30:00Z"
  }
}
```

---

## 4. 数据结构约定

### 4.1 `GeneratedCardPreview`

```json
{
  "keyword": "string",
  "definition": "string",
  "explanation": "string",
  "relatedTerms": ["string"],
  "scenarios": ["string"],
  "mnemonic": "string | null",
  "sourceExcerpt": "string | null"
}
```

### 4.2 `SavedCardResult`

```json
{
  "cardId": "string",
  "createdAt": "string",
  "nextReviewAt": "string"
}
```

---

## 5. 错误码

- `INVALID_INPUT`：输入内容为空或超过限制
- `MODEL_NOT_FOUND`：指定模型配置不存在
- `AI_TIMEOUT`：AI 响应超时
- `AI_UNAVAILABLE`：AI 服务不可用
- `AI_RESPONSE_INVALID`：AI 返回结果无法解析
- `DB_WRITE_FAILED`：保存卡片失败

---

## 6. 前端联调规则

- 前端在预览阶段只调用 `preview_generated_card`
- 前端在用户点击确认保存时再调用 `save_generated_card`
- 前端不得假设 AI 返回字段一定完整，需要处理可空字段
- 前端错误提示优先根据 `error.code` 映射，不直接展示原始技术错误

---

## 7. Mock 建议

前端 Mock 数据必须至少覆盖以下情况：

- 正常返回完整卡片
- `mnemonic` 为空
- `relatedTerms` 为空数组
- AI 超时错误
- 保存成功后返回 `cardId` 和 `nextReviewAt`

---

## 8. 数据库影响

本模块至少涉及以下数据写入：

- `cards`
- `review_schedule`

新增字段或卡片结构变化时，必须同步更新：

- 本契约文档
- Rust `models`
- 前端 `types`
- `src-tauri/migrations/`

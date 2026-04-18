# 卡片生成模块接口契约

## 1. 模块目标

卡片生成模块负责将用户输入的关键词或选中文本直接提交给大模型，生成最终结构化知识卡片并立即写入本地数据库，同时初始化首个复习计划。用户后续进入预览阶段时，可以对已生成卡片执行接受或取消操作，但该步骤不与输入动作绑定在一起。

---

## 2. 前后端边界

前端负责：

- 收集输入内容
- 展示生成中、失败、成功状态
- 展示已生成卡片的预览列表
- 在预览阶段触发接受或取消操作

后端负责：

- 校验输入
- 调用 AI 模型
- 标准化生成结果
- 持久化卡片
- 初始化复习计划
- 维护卡片在预览阶段的接受状态

---

## 3. Command 列表

### 3.1 `generate_cards`

用途：

- 根据输入文本或关键词直接生成最终卡片
- 生成后立即写入数据库
- 初始化对应复习计划

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
    "batchId": "uuid",
    "cards": [
      {
        "cardId": "uuid",
        "keyword": "艾宾浩斯遗忘曲线",
        "definition": "用于描述记忆随时间衰减规律的理论模型",
        "explanation": "它帮助系统计算什么时候复习更有效",
        "relatedTerms": ["记忆", "复习", "排期"],
        "scenarios": ["考试复习", "长期记忆训练"],
        "sourceExcerpt": "遗忘曲线说明记忆会随时间下降",
        "status": "pending",
        "createdAt": "2026-04-18T10:30:00Z",
        "reviewHistory": [],
        "nextReviewAt": "2026-04-19T10:30:00Z"
      }
    ]
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

### 3.2 `list_generated_cards`

用途：

- 获取某次生成任务下的已生成卡片，用于预览阶段展示

输入：

```json
{
  "batchId": "uuid"
}
```

成功返回：

```json
{
  "success": true,
  "data": {
    "batchId": "uuid",
    "cards": [
      {
        "cardId": "uuid",
        "keyword": "艾宾浩斯遗忘曲线",
        "definition": "用于描述记忆随时间衰减规律的理论模型",
        "explanation": "它帮助系统计算什么时候复习更有效",
        "relatedTerms": ["记忆", "复习", "排期"],
        "scenarios": ["考试复习", "长期记忆训练"],
        "sourceExcerpt": "遗忘曲线说明记忆会随时间下降",
        "status": "pending",
        "createdAt": "2026-04-18T10:30:00Z",
        "reviewHistory": [],
        "nextReviewAt": "2026-04-19T10:30:00Z"
      }
    ]
  }
}
```

### 3.3 `review_generated_cards`

用途：

- 在单独的预览阶段，对一批已生成卡片执行接受或取消操作

输入：

```json
{
  "batchId": "uuid",
  "acceptCardIds": ["uuid"],
  "rejectCardIds": ["uuid"]
}
```

约束：

- `acceptCardIds` 与 `rejectCardIds` 不允许重复
- 两者都可以为空数组
- 未出现在两个数组中的卡片保持 `pending`

成功返回：

```json
{
  "success": true,
  "data": {
    "batchId": "uuid",
    "acceptedCount": 2,
    "rejectedCount": 1,
    "pendingCount": 0
  }
}
```

---

## 4. 数据结构约定

### 4.1 `GeneratedCard`

```json
{
  "cardId": "string",
  "keyword": "string",
  "definition": "string",
  "explanation": "string",
  "relatedTerms": ["string"],
  "scenarios": ["string"],
  "sourceExcerpt": "string | null",
  "status": "pending | accepted",
  "createdAt": "string",
  "reviewHistory": ["string"],
  "nextReviewAt": "string"

}
```

### 4.2 `GeneratedCardBatchResult`

```json
{
  "batchId": "string",
  "cards": ["GeneratedCard"]
}
```

### 4.3 `ReviewedGeneratedCardsResult`

```json
{
  "batchId": "string",
  "acceptedCount": 0,
  "rejectedCount": 0,
  "pendingCount": 0
}
```

---

## 5. 错误码

- `INVALID_INPUT`：输入内容为空或超过限制
- `MODEL_NOT_FOUND`：指定模型配置不存在
- `AI_TIMEOUT`：AI 响应超时
- `AI_UNAVAILABLE`：AI 服务不可用
- `AI_RESPONSE_INVALID`：AI 返回结果无法解析
- `GENERATION_BATCH_NOT_FOUND`：生成批次不存在
- `INVALID_REVIEW_OPERATION`：接受或取消操作非法
- `DB_WRITE_FAILED`：卡片写入或状态更新失败

---

## 6. 前端联调规则

- 前端在用户输入后直接调用 `generate_cards`，不再单独执行“生成前确认保存”
- 前端在预览页通过 `list_generated_cards` 拉取某批次卡片
- 前端在用户明确操作后，通过 `review_generated_cards` 提交接受和取消结果
- 前端不得假设 AI 返回字段一定完整，需要处理可空字段
- 前端错误提示优先根据 `error.code` 映射，不直接展示原始技术错误
- 前端不应在本地删除取消卡片，以后端返回的状态为准

---

## 7. Mock 建议

前端 Mock 数据必须至少覆盖以下情况：

- 正常返回一批完整卡片
- 单次生成返回多张卡片
- `relatedTerms` 为空数组
- AI 超时错误
- 预览阶段接受部分卡片
- 预览阶段取消部分卡片

---

## 8. 数据库影响

本模块至少涉及以下数据写入：

- `cards`
- `review_schedule`
- `generation_batches` 或等价的生成批次标识

新增字段或卡片结构变化时，必须同步更新：

- 本契约文档
- Rust `models`
- 前端 `types`
- `src-tauri/migrations/`

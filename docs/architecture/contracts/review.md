# 复习模块接口契约

## 1. 模块目标

复习模块负责提供待复习卡片列表、单次复习流程和复习结果回写，并基于用户反馈更新下一次复习时间。

---

## 2. 前后端边界

前端负责：

- 展示今日待复习数量
- 管理翻卡交互
- 提交复习结果
- 展示完成反馈

后端负责：

- 查询到期卡片
- 返回复习所需数据
- 根据用户反馈更新排期
- 写入复习记录
- 触发后续通知逻辑

---

## 3. Command 列表

### 3.1 `list_due_reviews`

用途：

- 获取当前时间点需要复习的卡片列表

输入：

```json
{
  "limit": 20,
  "cursor": "string | null",
  "includeCompletedToday": false
}
```

成功返回：

```json
{
  "success": true,
  "data": {
    "items": [
      {
        "reviewId": "uuid",
        "cardId": "uuid",
        "keyword": "艾宾浩斯遗忘曲线",
        "definition": "用于描述记忆衰减规律的理论模型",
        "explanation": "帮助系统决定何时提醒复习",
        "mnemonic": "先记后忘，按点再访",
        "reviewStep": 1,
        "dueAt": "2026-04-18T09:00:00Z",
        "tags": ["记忆", "学习方法"]
      }
    ],
    "nextCursor": null,
    "summary": {
      "dueCount": 12,
      "completedToday": 3
    }
  }
}
```

### 3.2 `submit_review_result`

用途：

- 提交单张卡片的复习结果

输入：

```json
{
  "reviewId": "uuid",
  "cardId": "uuid",
  "result": "remembered | forgotten | skipped",
  "reviewedAt": "2026-04-18T10:00:00Z"
}
```

成功返回：

```json
{
  "success": true,
  "data": {
    "cardId": "uuid",
    "result": "remembered",
    "previousStep": 1,
    "nextStep": 2,
    "nextReviewAt": "2026-04-19T10:00:00Z",
    "remainingDueCount": 11
  }
}
```

### 3.3 `get_review_dashboard`

用途：

- 获取复习首页摘要数据

成功返回：

```json
{
  "success": true,
  "data": {
    "dueToday": 12,
    "completedToday": 3,
    "streakDays": 8,
    "nextDueAt": "2026-04-18T14:00:00Z"
  }
}
```

---

## 4. 数据结构约定

### 4.1 `DueReviewItem`

```json
{
  "reviewId": "string",
  "cardId": "string",
  "keyword": "string",
  "definition": "string",
  "explanation": "string",
  "mnemonic": "string | null",
  "reviewStep": 1,
  "dueAt": "string",
  "tags": ["string"]
}
```

### 4.2 `ReviewResultPayload`

```json
{
  "result": "remembered | forgotten | skipped",
  "reviewedAt": "string"
}
```

---

## 5. 排期规则约定

- `remembered`：进入下一复习节点
- `forgotten`：回退到初始节点或重置为第一阶段
- `skipped`：不计入掌握情况，可保留原节点并推迟到最近可复习时间

默认节点建议：

- 第 1 次：1 天
- 第 2 次：1 天
- 第 3 次：3 天
- 第 4 次：7 天
- 第 5 次：15 天
- 第 6 次：30 天

实际算法以后端实现为准，但前端必须基于该规则理解返回结果，不自行计算下次时间。

---

## 6. 错误码

- `REVIEW_NOT_FOUND`：复习任务不存在
- `CARD_NOT_FOUND`：卡片不存在
- `INVALID_REVIEW_RESULT`：复习结果非法
- `SCHEDULE_UPDATE_FAILED`：排期更新失败
- `DB_WRITE_FAILED`：写入复习记录失败

---

## 7. 前端联调规则

- 前端展示复习卡片时，不自行决定卡片是否到期，以后端返回列表为准
- 前端提交结果后，以返回的 `nextReviewAt` 和 `remainingDueCount` 更新界面
- 前端不缓存排期计算逻辑，不在本地做算法兜底
- 若列表为空，应显示“今日已完成”而不是报错

---

## 8. Mock 建议

前端 Mock 数据必须覆盖：

- 正常有待复习卡片
- 今日无待复习卡片
- 提交 `remembered`
- 提交 `forgotten`
- 提交 `skipped`
- `REVIEW_NOT_FOUND` 错误

---

## 9. 数据库影响

本模块至少涉及以下数据表：

- `cards`
- `review_schedule`
- `review_logs` 或等价复习记录表

若排期算法字段发生变化，必须同步更新：

- 本契约文档
- 排期算法说明
- Rust 数据模型
- migration 脚本

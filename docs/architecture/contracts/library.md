# 知识宝库模块接口契约

## 1. 模块目标

知识宝库模块负责卡片的集中查询、筛选、详情查看、批量操作以及导入导出能力。

---

## 2. 前后端边界

前端负责：

- 列表展示
- 搜索、筛选、排序与分页交互
- 卡片详情展示
- 批量选择与操作入口

后端负责：

- 查询条件解析
- 数据检索
- 详情读取
- 批量更新
- 导入导出处理

---

## 3. Command 列表

### 3.1 `list_cards`

用途：

- 获取卡片列表

输入：

```json
{
  "query": "string | null",
  "tag": "string | null",
  "status": "all | active | archived",
  "sortBy": "createdAt | updatedAt | nextReviewAt | keyword",
  "sortOrder": "asc | desc",
  "limit": 20,
  "cursor": "string | null"
}
```

成功返回：

```json
{
  "success": true,
  "data": {
    "items": [
      {
        "cardId": "uuid",
        "keyword": "艾宾浩斯遗忘曲线",
        "definition": "用于描述记忆衰减规律的理论模型",
        "tags": ["记忆", "学习方法"],
        "status": "active",
        "createdAt": "2026-04-18T10:00:00Z",
        "updatedAt": "2026-04-18T10:10:00Z",
        "nextReviewAt": "2026-04-19T10:00:00Z"
      }
    ],
    "nextCursor": null,
    "total": 128
  }
}
```

### 3.2 `get_card_detail`

用途：

- 获取单张卡片详情

输入：

```json
{
  "cardId": "uuid"
}
```

成功返回：

```json
{
  "success": true,
  "data": {
    "cardId": "uuid",
    "keyword": "艾宾浩斯遗忘曲线",
    "definition": "用于描述记忆衰减规律的理论模型",
    "explanation": "帮助系统安排复习节奏",
    "relatedTerms": ["记忆", "遗忘", "复习"],
    "scenarios": ["考试复习", "知识管理"],
    "mnemonic": "先记后忘，按点再访",
    "tags": ["记忆", "学习方法"],
    "status": "active",
    "createdAt": "2026-04-18T10:00:00Z",
    "updatedAt": "2026-04-18T10:10:00Z",
    "nextReviewAt": "2026-04-19T10:00:00Z"
  }
}
```

### 3.3 `update_card`

用途：

- 更新卡片内容或标签

输入：

```json
{
  "cardId": "uuid",
  "keyword": "string",
  "definition": "string",
  "explanation": "string",
  "relatedTerms": ["string"],
  "scenarios": ["string"],
  "mnemonic": "string | null",
  "tags": ["string"],
  "status": "active | archived"
}
```

### 3.4 `batch_update_cards`

用途：

- 批量归档、批量恢复、批量打标签

输入：

```json
{
  "cardIds": ["uuid"],
  "action": "archive | restore | addTag | removeTag",
  "tag": "string | null"
}
```

### 3.5 `export_cards`

用途：

- 导出卡片

输入：

```json
{
  "format": "json | csv",
  "cardIds": ["uuid"] 
}
```

成功返回：

```json
{
  "success": true,
  "data": {
    "exportPath": "/absolute/path/export.json",
    "count": 25
  }
}
```

---

## 4. 数据结构约定

### 4.1 `CardListItem`

```json
{
  "cardId": "string",
  "keyword": "string",
  "definition": "string",
  "tags": ["string"],
  "status": "active | archived",
  "createdAt": "string",
  "updatedAt": "string",
  "nextReviewAt": "string | null"
}
```

### 4.2 `CardDetail`

在 `CardListItem` 基础上增加：

- `explanation`
- `relatedTerms`
- `scenarios`
- `mnemonic`

---

## 5. 错误码

- `CARD_NOT_FOUND`：卡片不存在
- `INVALID_QUERY`：查询参数非法
- `INVALID_SORT_FIELD`：排序字段非法
- `INVALID_BATCH_ACTION`：批量操作非法
- `EXPORT_FAILED`：导出失败
- `IMPORT_FAILED`：导入失败
- `DB_READ_FAILED`：数据库读取失败
- `DB_WRITE_FAILED`：数据库写入失败

---

## 6. 前端联调规则

- 列表分页以前端传入的 `limit + cursor` 为准，不依赖页码
- 前端筛选条件必须显式传递，不使用隐式默认值推断
- 前端列表只显示 `CardListItem` 字段，详情页再调用 `get_card_detail`
- 批量操作成功后，前端应刷新当前列表，不假设本地状态一定正确

---

## 7. Mock 建议

前端 Mock 数据必须覆盖：

- 空列表
- 关键词搜索结果
- 按标签过滤结果
- 已归档卡片
- 批量归档成功
- 导出成功和导出失败

---

## 8. 数据库影响

本模块主要涉及：

- `cards`
- `card_tags` 或等价标签关系表
- 导入导出记录表，可选

若筛选字段、状态字段或导出结构有变更，必须同步更新：

- 本契约文档
- 前端筛选类型
- Rust 查询参数模型
- migration 脚本

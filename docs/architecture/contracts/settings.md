# 设置模块接口契约

## 1. 模块目标

设置模块负责用户偏好、模型配置、通知策略和基础应用配置的读取与持久化。

---

## 2. 前后端边界

前端负责：

- 设置表单展示
- 表单校验与保存交互
- 设置成功与失败提示
- 配置项分组展示

后端负责：

- 配置读取与写入
- 默认值补齐
- 敏感配置存储
- 通知能力和模型配置校验

---

## 3. Command 列表

### 3.1 `get_settings`

用途：

- 获取当前用户设置

成功返回：

```json
{
  "success": true,
  "data": {
    "theme": "system",
    "language": "zh-CN",
    "notificationEnabled": true,
    "reviewReminderEnabled": true,
    "reviewReminderTime": "09:00",
    "defaultModelProfileId": "default-qwen",
    "storage": {
      "exportDirectory": "/Users/mac/Documents/NeverMind"
    }
  }
}
```

### 3.2 `update_settings`

用途：

- 更新基础设置

输入：

```json
{
  "theme": "light | dark | system",
  "language": "zh-CN | en-US",
  "notificationEnabled": true,
  "reviewReminderEnabled": true,
  "reviewReminderTime": "09:00",
  "storage": {
    "exportDirectory": "/absolute/path"
  }
}
```

成功返回：

```json
{
  "success": true,
  "data": {
    "updatedAt": "2026-04-18T11:00:00Z"
  }
}
```

### 3.3 `list_model_profiles`

用途：

- 获取 AI 模型配置列表

成功返回：

```json
{
  "success": true,
  "data": {
    "items": [
      {
        "profileId": "default-qwen",
        "name": "Qwen Default",
        "provider": "qwen",
        "endpoint": "https://api.example.com",
        "isDefault": true,
        "isAvailable": true
      }
    ]
  }
}
```

### 3.4 `save_model_profile`

用途：

- 新增或更新模型配置

输入：

```json
{
  "profileId": "string | null",
  "name": "string",
  "provider": "openai-compatible | qwen | custom",
  "endpoint": "string",
  "apiKey": "string",
  "model": "string",
  "timeoutMs": 30000
}
```

### 3.5 `test_model_profile`

用途：

- 测试模型配置是否可用

输入：

```json
{
  "profileId": "string | null",
  "provider": "openai-compatible | qwen | custom",
  "endpoint": "string",
  "apiKey": "string",
  "model": "string",
  "timeoutMs": 30000
}
```

成功返回：

```json
{
  "success": true,
  "data": {
    "reachable": true,
    "latencyMs": 850
  }
}
```

---

## 4. 数据结构约定

### 4.1 `AppSettings`

```json
{
  "theme": "light | dark | system",
  "language": "zh-CN | en-US",
  "notificationEnabled": "boolean",
  "reviewReminderEnabled": "boolean",
  "reviewReminderTime": "HH:mm",
  "defaultModelProfileId": "string | null",
  "storage": {
    "exportDirectory": "string | null"
  }
}
```

### 4.2 `ModelProfile`

```json
{
  "profileId": "string",
  "name": "string",
  "provider": "string",
  "endpoint": "string",
  "model": "string | null",
  "isDefault": "boolean",
  "isAvailable": "boolean"
}
```

说明：

- `apiKey` 不返回给前端明文展示，前端仅在新建或修改时传入
- 已保存的敏感字段建议以后端安全存储方案为准

---

## 5. 错误码

- `INVALID_SETTINGS`：设置项非法
- `INVALID_TIME_FORMAT`：提醒时间格式非法
- `INVALID_PATH`：导出目录非法
- `MODEL_PROFILE_NOT_FOUND`：模型配置不存在
- `MODEL_CONNECTION_FAILED`：模型连接失败
- `MODEL_AUTH_FAILED`：模型鉴权失败
- `DB_WRITE_FAILED`：设置保存失败
- `DB_READ_FAILED`：设置读取失败

---

## 6. 前端联调规则

- 前端不缓存后端未返回的默认值，以 `get_settings` 返回内容为准
- 前端保存设置后，应以接口返回结果更新本地状态
- 模型配置测试与保存必须分开，不把“测试成功”视为“保存成功”
- 敏感字段如 `apiKey` 不在列表页回显完整内容

---

## 7. Mock 建议

前端 Mock 数据必须覆盖：

- 初始默认设置
- 开启与关闭通知
- 无默认模型
- 模型连通性测试成功
- 模型鉴权失败
- 设置保存失败

---

## 8. 数据库影响

本模块主要涉及：

- `settings`
- `model_profiles`

如果后续新增设置项，必须同步更新：

- 本契约文档
- 前端设置表单类型
- Rust 配置模型
- migration 脚本

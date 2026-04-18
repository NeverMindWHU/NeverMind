# NeverMind 后端双人开发分工指南

## 1. 目标

本文档用于指导 NeverMind 在当前阶段由 2 名后端开发者并行推进 `src-tauri` 部分，减少目录冲突、职责重叠和联调返工。

当前后端已经具备以下底层骨架：

- SQLite 数据库接入
- migration 执行能力
- 基础模型 `models`
- DAO trait 与 SQLite DAO 实现
- 统一错误类型

在此基础上，建议按“底层数据层”和“业务流程层”进行拆分。

---

## 2. 角色划分

### 后端 A：底层数据与基础设施负责人

负责目录：

```text
src-tauri/
├── migrations/
└── src/
    ├── db/
    │   └── dao/
    ├── models/
    └── utils/
```

负责内容：

- 维护 SQLite 表结构和 migration
- 维护 `models` 中的持久化模型与输入输出结构
- 维护 DAO trait 和 SQLite DAO 实现
- 维护通用错误类型、数据库连接、事务处理
- 为上层 service 提供稳定的数据访问接口

当前优先模块：

- `settings`
- `library`
- `review` 的数据存储部分
- `card-generation` 的批次与卡片入库部分

### 后端 B：业务流程与能力编排负责人

负责目录：

```text
src-tauri/
└── src/
    ├── ai/
    ├── scheduler/
    ├── notifications/
    ├── tray/
    ├── commands/
    └── state/
```

负责内容：

- 接入 AI 调用与生成流程
- 实现复习排期规则与状态流转
- 实现通知与托盘相关业务逻辑
- 编排上层 command/service 流程
- 将 DAO 能力组装为可供前端调用的完整接口

当前优先模块：

- `card-generation`
- `review`
- `settings` 的 command 层
- `library` 的 command 层

---

## 3. 共同维护的区域

以下目录允许两人协作，但必须提前约定负责文件：

```text
src-tauri/src/commands/
src-tauri/src/state/
```

建议拆分方式：

```text
src-tauri/src/commands/
├── card_generation.rs
├── review.rs
├── library.rs
└── settings.rs
```

推荐归属：

- A 负责 `library.rs`、`settings.rs`
- B 负责 `card_generation.rs`、`review.rs`
- `commands/mod.rs` 由当日最后合并的人统一维护，避免重复冲突

---

## 4. 明确禁止的冲突点

以下内容不要两个人同时修改：

- 同一个 migration 文件
- 同一个 `mod.rs` 或统一导出文件
- 同一个 DAO 文件
- 同一个 command 文件
- 契约文档中的同一段字段定义

推荐规则：

- `migrations/` 只由 A 修改
- B 如果需要新字段，先提需求，再由 A 增加 migration
- `commands/` 按模块文件拆开后各自负责

---

## 5. 推荐开发顺序

建议按下面顺序推进：

### 第一阶段：先稳定底层

A 负责：

- 完善 `migrations/`
- 完善 `db/dao/`
- 完善 `models/`
- 补充数据库相关测试

B 负责：

- 搭 `ai/`
- 搭 `scheduler/`
- 定义 command 层输入输出
- 写 service 流程草稿

### 第二阶段：先打通简单模块

A 负责：

- `settings` 的 DAO 与 model 收口
- `library` 的查询、更新、导出底层实现

B 负责：

- `settings` command
- `library` command
- 基础错误映射和返回结构

### 第三阶段：再打通复杂模块

A 负责：

- `generation_batches`
- `cards`
- `review_schedule`
- `review_logs`

B 负责：

- `generate_cards`
- `review_generated_cards`
- `list_due_reviews`
- `submit_review_result`

### 第四阶段：联调与收尾

两人一起完成：

- command 层联调
- 状态字段校验
- 错误码收口
- 文档回写

---

## 6. 日常协作工作流

建议每天按下面节奏协作：

1. 早上先确认当天会不会新增字段、表或状态枚举
2. 若要改数据库结构，A 先补 migration，再同步给 B
3. B 基于最新模型和 DAO 接口继续推进业务流程
4. 下班前同步当天变更点和第二天依赖项

每天同步至少确认 4 件事：

- 新增了什么表或字段
- 哪些接口的入参或出参改了
- 哪些错误码变了
- 哪些模块已经可以联调

---

## 7. Git 与 PR 规则

推荐分支命名：

```text
feature/backend-settings-storage
feature/backend-library-query
feature/backend-review-flow
feature/backend-card-generation
```

推荐提交流程：

1. 先更新契约文档
2. 再提交模型和 migration
3. 再提交 DAO 或 service
4. 最后提交 command 和联调修改

每个 PR 尽量只做一件事，例如：

- 只加一个 migration
- 只补一个 DAO
- 只补一个 command
- 只补一个模块的联调收口

---

## 8. 新增字段时的标准流程

如果 B 在业务开发时发现需要新增字段：

1. 先修改对应契约文档
2. 通知 A 补 migration
3. A 更新 `models` 和 `dao`
4. B 再更新 command 与业务逻辑
5. 双方一起验证前后兼容性

不要直接在业务代码里先写死一个“临时字段”，否则后面很容易返工。

---

## 9. 当前推荐落点

结合当前仓库状态，建议这样开工：

- A 先继续完善：
  - `src-tauri/src/db/mod.rs`
  - `src-tauri/src/db/dao/`
  - `src-tauri/src/models/`
  - `src-tauri/migrations/`
- B 先开始补：
  - `src-tauri/src/commands/`
  - `src-tauri/src/ai/`
  - `src-tauri/src/scheduler/`
  - `src-tauri/src/notifications/`

---

## 10. 结论

简单来说：

- A 负责“数据怎么存、怎么查、怎么迁移”
- B 负责“业务怎么跑、怎么调 AI、怎么给前端用”

只要坚持“数据库改动先过 A，业务流程编排由 B 收口”，这套双人分工就能比较稳定地跑起来。

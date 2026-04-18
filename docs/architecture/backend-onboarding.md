# NeverMind 后端上手指南

## 1. 这份文档给谁看

本文档面向新加入 NeverMind 后端开发的同学，帮助你快速理解当前 `src-tauri` 的结构、已有基础设施、推荐开发入口和协作方式。

---

## 2. 当前技术栈

后端当前采用：

- Rust
- SQLite
- `sqlx`
- `tokio`
- Tauri 后端目录结构

当前目标不是一次性把全部功能写完，而是先搭好稳定的底层骨架，再逐步补齐 command、AI、复习调度和通知能力。

---

## 3. 先看哪里

建议第一次进入项目时，按下面顺序阅读：

1. [README.md](file:///Users/mac/code/NeverMind/README.md)
2. [parallel-development-plan.md](file:///Users/mac/code/NeverMind/docs/architecture/parallel-development-plan.md)
3. [backend-two-person-guide.md](file:///Users/mac/code/NeverMind/docs/architecture/backend-two-person-guide.md)
4. `docs/architecture/contracts/` 下的 4 份契约文档
5. `src-tauri/src/` 当前已有底层实现

推荐优先阅读的契约文档：

- [card-generation.md](file:///Users/mac/code/NeverMind/docs/architecture/contracts/card-generation.md)
- [review.md](file:///Users/mac/code/NeverMind/docs/architecture/contracts/review.md)
- [library.md](file:///Users/mac/code/NeverMind/docs/architecture/contracts/library.md)
- [settings.md](file:///Users/mac/code/NeverMind/docs/architecture/contracts/settings.md)

---

## 4. 当前后端目录说明

```text
src-tauri/
├── Cargo.toml
├── migrations/
│   └── 0001_init.sql
└── src/
    ├── main.rs
    ├── lib.rs
    ├── db/
    │   ├── mod.rs
    │   └── dao/
    │       ├── card_dao.rs
    │       ├── review_dao.rs
    │       └── settings_dao.rs
    ├── models/
    │   ├── card.rs
    │   ├── review.rs
    │   └── settings.rs
    ├── utils/
    │   └── error.rs
    ├── commands/
    ├── ai/
    ├── scheduler/
    ├── notifications/
    ├── tray/
    └── state/
```

当前已完成的主要是：

- 数据库连接
- migration 执行
- 基础 models
- DAO trait 和 SQLite DAO 实现
- 通用错误类型

当前还没系统补齐的主要是：

- command 层
- AI 调用层
- scheduler
- notification / tray
- service 编排层

---

## 5. 现有代码怎么理解

### `src-tauri/src/db/mod.rs`

作用：

- 负责数据库连接
- 提供 `Database` 封装
- 负责执行 migrations

### `src-tauri/src/db/dao/`

作用：

- 定义每个领域的数据访问接口
- 提供 SQLite 实现

当前已拆为：

- `card_dao.rs`
- `review_dao.rs`
- `settings_dao.rs`

### `src-tauri/src/models/`

作用：

- 存放数据库模型和输入结构
- 给 DAO 和未来的 command/service 共享使用

### `src-tauri/migrations/`

作用：

- 管理数据库结构版本
- 当前初始建表已经在 `0001_init.sql`

---

## 6. 新人第一天建议做什么

建议第一天只做下面几件事：

1. 先跑一次 `cargo check`
2. 阅读 4 份契约文档，理解模块输入输出
3. 对照 `migrations/0001_init.sql` 看表结构
4. 对照 `models/` 和 `dao/` 看字段是否一一对应
5. 再决定自己从哪一层开始接手

不要一上来直接写 command 或 AI 流程，先把底层数据模型和接口边界搞清楚。

---

## 7. 日常开发入口

如果你负责底层数据层，优先从这些目录开始：

- `src-tauri/migrations/`
- `src-tauri/src/models/`
- `src-tauri/src/db/dao/`
- `src-tauri/src/utils/`

如果你负责业务流程层，优先从这些目录开始：

- `src-tauri/src/commands/`
- `src-tauri/src/ai/`
- `src-tauri/src/scheduler/`
- `src-tauri/src/notifications/`
- `src-tauri/src/state/`

---

## 8. 新增一个功能时怎么做

以一个新功能为例，建议按下面顺序推进：

1. 先改对应契约文档
2. 如涉及数据库，先补 migration
3. 再补 models
4. 再补 DAO
5. 再补 service / command
6. 最后联调和补文档

这个顺序能保证上层逻辑不会建立在不稳定的数据结构上。

---

## 9. 常见注意事项

- 不要直接修改旧 migration，新增一个新 migration 文件
- 不要在 command 里直接写复杂 SQL
- 不要让前端字段名和 Rust 字段名各写各的
- 不要先写业务临时字段，后补文档
- 如果契约改了，`docs + models + dao + command` 要一起改

---

## 10. 常用命令

在 `src-tauri/` 目录下常用：

```bash
cargo check
```

后续如果补了测试，可继续使用：

```bash
cargo test
```

---

## 11. 推荐上手路径

如果你是：

- 偏数据层同学：从 `settings` 和 `library` 开始
- 偏业务流程同学：从 `review` 和 `card-generation` 开始

推荐原因：

- `settings`、`library` 更适合熟悉数据库读写和返回结构
- `review`、`card-generation` 更适合熟悉状态流转、AI 和调度逻辑

---

## 12. 你卡住时先检查什么

如果开发时卡住，先看这几项：

- 契约文档有没有定义清楚字段
- 数据表有没有对应字段
- model 和 DAO 是否已经同步
- 当前改动是否应该先补 migration
- 自己是不是改到了另一个同学负责的目录

---

## 13. 结论

新人进入 NeverMind 后端开发时，最重要的不是先写功能，而是先搞清楚三件事：

- 契约怎么定义
- 数据怎么落地
- 目录归谁负责

只要这三件事清楚，上手速度会快很多，后续联调也会顺畅很多。

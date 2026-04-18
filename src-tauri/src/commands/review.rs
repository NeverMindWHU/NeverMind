use chrono::{DateTime, Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    models::review::{DueReviewItem, NewReviewLog, ReviewResult},
    scheduler::ebbinghaus::next_review,
    state::AppState,
    utils::error::{AppError, AppResult},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: T,
}

impl<T> CommandResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListDueReviewsInput {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
    pub include_completed_today: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueReviewCard {
    pub review_id: String,
    pub card_id: String,
    /// 主关键词（兼容旧 UI）。
    pub keyword: String,
    /// v2 新增：完整问题文本（正面展示用）。
    pub question: String,
    /// v2 新增：3 个关键词（复习页背面 tag 展示用）。
    pub keywords: Vec<String>,
    pub definition: String,
    pub explanation: String,
    pub review_step: i64,
    pub due_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueReviewsSummary {
    pub due_count: i64,
    pub completed_today: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDueReviewsData {
    pub items: Vec<DueReviewCard>,
    pub next_cursor: Option<String>,
    pub summary: DueReviewsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListUpcomingReviewsInput {
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingReviewsSummary {
    /// 尚未到期（`due_at > now`）的 pending review 总数。
    pub upcoming_count: i64,
    /// 最早到期时间；若无 upcoming 则为 None。
    pub earliest_due_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUpcomingReviewsData {
    pub items: Vec<DueReviewCard>,
    pub summary: UpcomingReviewsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitReviewResultInput {
    pub review_id: String,
    pub card_id: String,
    pub result: ReviewResult,
    pub reviewed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitReviewResultData {
    pub card_id: String,
    pub result: ReviewResult,
    pub previous_step: i64,
    pub next_step: i64,
    pub next_review_at: DateTime<Utc>,
    pub remaining_due_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDashboardData {
    pub due_today: i64,
    pub completed_today: i64,
    pub streak_days: i64,
    pub next_due_at: Option<DateTime<Utc>>,
}

pub async fn list_due_reviews(
    state: &AppState,
    input: ListDueReviewsInput,
) -> AppResult<CommandResponse<ListDueReviewsData>> {
    let limit = input.limit.unwrap_or(20).clamp(1, 100);
    let items = state.review_dao.list_due_reviews(limit).await?;
    let due_count = state.review_dao.count_due_reviews().await?;
    let completed_today = if input.include_completed_today.unwrap_or(false) {
        count_completed_today(state, Utc::now()).await?
    } else {
        0
    };

    Ok(CommandResponse::ok(ListDueReviewsData {
        items: items.into_iter().map(map_due_review_item).collect(),
        next_cursor: input.cursor.and(None),
        summary: DueReviewsSummary {
            due_count,
            completed_today,
        },
    }))
}

/// 当"今日到期"队列清空时，前端可以调用本命令把**下一轮**（尚未到期）的
/// pending review 提前拉出来让用户先刷。提交结果后仍走正常 Ebbinghaus 推进，
/// 与"准点复习"等价。
pub async fn list_upcoming_reviews(
    state: &AppState,
    input: ListUpcomingReviewsInput,
) -> AppResult<CommandResponse<ListUpcomingReviewsData>> {
    let limit = input.limit.unwrap_or(20).clamp(1, 100);
    let items = state.review_dao.list_upcoming_reviews(limit).await?;
    let upcoming_count = state.review_dao.count_upcoming_reviews().await?;
    let earliest_due_at = items.first().map(|it| it.due_at);

    Ok(CommandResponse::ok(ListUpcomingReviewsData {
        items: items.into_iter().map(map_due_review_item).collect(),
        summary: UpcomingReviewsSummary {
            upcoming_count,
            earliest_due_at,
        },
    }))
}

pub async fn submit_review_result(
    state: &AppState,
    input: SubmitReviewResultInput,
) -> AppResult<CommandResponse<SubmitReviewResultData>> {
    let schedule = state.review_dao.get_schedule(&input.review_id).await?;
    if schedule.card_id != input.card_id {
        return Err(AppError::Validation {
            message: "reviewId 与 cardId 不匹配".to_string(),
        });
    }

    let decision = next_review(schedule.review_step, input.result, input.reviewed_at);
    state
        .review_dao
        .update_schedule_after_review(
            &input.review_id,
            decision.next_step,
            decision.next_due_at,
            decision.status,
        )
        .await?;

    let log = NewReviewLog {
        id: Uuid::new_v4().to_string(),
        review_schedule_id: input.review_id.clone(),
        card_id: input.card_id.clone(),
        result: input.result.as_str().to_string(),
        previous_step: schedule.review_step,
        next_step: decision.next_step,
        reviewed_at: input.reviewed_at,
    };
    state.review_dao.insert_review_log(&log).await?;

    let remaining_due_count = state.review_dao.count_due_reviews().await?;

    Ok(CommandResponse::ok(SubmitReviewResultData {
        card_id: input.card_id,
        result: input.result,
        previous_step: schedule.review_step,
        next_step: decision.next_step,
        next_review_at: decision.next_due_at,
        remaining_due_count,
    }))
}

pub async fn get_review_dashboard(
    state: &AppState,
) -> AppResult<CommandResponse<ReviewDashboardData>> {
    let now = Utc::now();
    let due_today = state.review_dao.count_due_reviews().await?;
    let completed_today = count_completed_today(state, now).await?;
    let next_due_at = state.review_dao.get_next_due_at().await?;
    let streak_days = calculate_streak_days(
        state.review_dao.list_completed_review_days_desc(30).await?,
        now.date_naive(),
    )?;

    Ok(CommandResponse::ok(ReviewDashboardData {
        due_today,
        completed_today,
        streak_days,
        next_due_at,
    }))
}

fn map_due_review_item(item: DueReviewItem) -> DueReviewCard {
    // 老数据 question 可能是空串、keywords 可能是 "[]" / 空串，
    // 这里按宝库同款规则做兜底：question → "{keyword}是什么？"，keywords → [keyword]。
    let question = {
        let q = item.question.trim();
        if q.is_empty() {
            format!("{}是什么？", item.keyword)
        } else {
            q.to_string()
        }
    };
    let mut keywords: Vec<String> = serde_json::from_str(&item.keywords).unwrap_or_default();
    if keywords.is_empty() && !item.keyword.trim().is_empty() {
        keywords.push(item.keyword.clone());
    }

    DueReviewCard {
        review_id: item.review_id,
        card_id: item.card_id,
        keyword: item.keyword,
        question,
        keywords: keywords.clone(),
        definition: item.definition,
        explanation: item.explanation,
        review_step: item.review_step,
        due_at: item.due_at,
        // 旧版 tags 字段保留向后兼容；直接复用 keywords 让前端旧 UI 也能工作。
        tags: keywords,
    }
}

async fn count_completed_today(state: &AppState, now: DateTime<Utc>) -> AppResult<i64> {
    let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = (now.date_naive() + Days::new(1))
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    state
        .review_dao
        .count_completed_reviews_between(start, end)
        .await
}

fn calculate_streak_days(days: Vec<String>, today: NaiveDate) -> AppResult<i64> {
    if days.is_empty() {
        return Ok(0);
    }

    let mut parsed_days = Vec::with_capacity(days.len());
    for day in days {
        let parsed = NaiveDate::parse_from_str(&day, "%Y-%m-%d").map_err(|_| AppError::Validation {
            message: "review_logs 日期格式非法".to_string(),
        })?;
        parsed_days.push(parsed);
    }

    if parsed_days.first().copied() != Some(today) {
        return Ok(0);
    }

    let mut streak = 1_i64;
    let mut expected_day = today;

    for day in parsed_days.into_iter().skip(1) {
        let Some(previous_day) = expected_day.checked_sub_days(Days::new(1)) else {
            break;
        };

        if day == previous_day {
            streak += 1;
            expected_day = day;
        } else if day < previous_day {
            break;
        }
    }

    Ok(streak)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate, Utc};
    use sqlx::{migrate::Migrator, query_scalar, sqlite::SqlitePoolOptions, SqlitePool};

    use super::{
        calculate_streak_days, get_review_dashboard, list_due_reviews, list_upcoming_reviews,
        submit_review_result, ListDueReviewsInput, ListUpcomingReviewsInput,
        SubmitReviewResultInput,
    };
    use crate::{models::review::ReviewResult, state::AppState};

    static TEST_MIGRATOR: Migrator = sqlx::migrate!("./migrations");

    #[test]
    fn streak_counts_consecutive_days_ending_today() {
        let streak = calculate_streak_days(
            vec![
                "2026-04-18".to_string(),
                "2026-04-17".to_string(),
                "2026-04-16".to_string(),
            ],
            NaiveDate::from_ymd_opt(2026, 4, 18).unwrap(),
        )
        .unwrap();

        assert_eq!(streak, 3);
    }

    #[test]
    fn streak_resets_when_today_has_no_completed_reviews() {
        let streak = calculate_streak_days(
            vec!["2026-04-17".to_string(), "2026-04-16".to_string()],
            NaiveDate::from_ymd_opt(2026, 4, 18).unwrap(),
        )
        .unwrap();

        assert_eq!(streak, 0);
    }

    #[tokio::test]
    async fn list_due_reviews_returns_seeded_mock_data() {
        let (state, pool) = setup_test_state().await;
        let now = Utc::now();

        seed_card_and_schedule(
            &pool,
            "review-due-1",
            "card-due-1",
            "已到期卡片 A",
            now - Duration::hours(2),
            1,
            "accepted",
            "pending",
        )
        .await;
        seed_card_and_schedule(
            &pool,
            "review-due-2",
            "card-due-2",
            "已到期卡片 B",
            now - Duration::minutes(30),
            2,
            "accepted",
            "pending",
        )
        .await;
        seed_card_and_schedule(
            &pool,
            "review-future-1",
            "card-future-1",
            "未来卡片",
            now + Duration::days(1),
            1,
            "accepted",
            "pending",
        )
        .await;
        seed_card_and_schedule(
            &pool,
            "review-rejected-1",
            "card-rejected-1",
            "已拒绝卡片",
            now - Duration::hours(1),
            1,
            "rejected",
            "pending",
        )
        .await;
        seed_review_log(&pool, "log-today-1", "review-due-1", "card-due-1", now).await;

        let response = list_due_reviews(
            &state,
            ListDueReviewsInput {
                limit: Some(10),
                cursor: None,
                include_completed_today: Some(true),
            },
        )
        .await
        .unwrap();

        assert!(response.success);
        assert_eq!(response.data.items.len(), 2);
        assert_eq!(response.data.summary.due_count, 2);
        assert_eq!(response.data.summary.completed_today, 1);
        assert_eq!(response.data.items[0].review_id, "review-due-1");
        assert_eq!(response.data.items[1].review_id, "review-due-2");
        // v2：DueReviewCard 会把 keywords 字段解析并回填 tags。老数据（seed 里没写
        // question / keywords）的 tags 会等同 [keyword]，question 会被兜底成
        // "<keyword>是什么？"。
        for item in &response.data.items {
            assert_eq!(item.tags, vec![item.keyword.clone()]);
            assert_eq!(item.keywords, vec![item.keyword.clone()]);
            assert_eq!(item.question, format!("{}是什么？", item.keyword));
        }
    }

    #[tokio::test]
    async fn submit_review_result_updates_schedule_and_inserts_log() {
        let (state, pool) = setup_test_state().await;
        let now = Utc::now();

        seed_card_and_schedule(
            &pool,
            "review-submit-1",
            "card-submit-1",
            "提交测试卡片",
            now - Duration::hours(1),
            1,
            "accepted",
            "pending",
        )
        .await;

        let reviewed_at = now;
        let response = submit_review_result(
            &state,
            SubmitReviewResultInput {
                review_id: "review-submit-1".to_string(),
                card_id: "card-submit-1".to_string(),
                result: ReviewResult::Remembered,
                reviewed_at,
            },
        )
        .await
        .unwrap();

        assert!(response.success);
        assert_eq!(response.data.previous_step, 1);
        assert_eq!(response.data.next_step, 2);
        assert_eq!(response.data.remaining_due_count, 0);
        assert!(response.data.next_review_at > reviewed_at);

        let schedule = state
            .review_dao
            .get_schedule("review-submit-1")
            .await
            .unwrap();
        assert_eq!(schedule.review_step, 2);
        assert_eq!(schedule.status, "pending");
        assert_eq!(schedule.due_at, response.data.next_review_at);

        let log_count: i64 = query_scalar("SELECT COUNT(*) FROM review_logs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(log_count, 1);
    }

    #[tokio::test]
    async fn get_review_dashboard_uses_seeded_mock_data() {
        let (state, pool) = setup_test_state().await;
        let now = Utc::now();

        seed_card_and_schedule(
            &pool,
            "review-dashboard-due",
            "card-dashboard-due",
            "今日到期",
            now - Duration::minutes(10),
            1,
            "accepted",
            "pending",
        )
        .await;
        seed_card_and_schedule(
            &pool,
            "review-dashboard-next",
            "card-dashboard-next",
            "下一次到期",
            now + Duration::hours(5),
            2,
            "accepted",
            "pending",
        )
        .await;
        seed_review_log(
            &pool,
            "log-dashboard-today",
            "review-dashboard-due",
            "card-dashboard-due",
            now - Duration::minutes(5),
        )
        .await;
        seed_review_log(
            &pool,
            "log-dashboard-yesterday",
            "review-dashboard-due",
            "card-dashboard-due",
            now - Duration::days(1),
        )
        .await;

        let response = get_review_dashboard(&state).await.unwrap();

        assert!(response.success);
        assert_eq!(response.data.due_today, 1);
        assert_eq!(response.data.completed_today, 1);
        assert_eq!(response.data.streak_days, 2);
        assert_eq!(
            response.data.next_due_at,
            Some(now - Duration::minutes(10))
        );
    }

    /// 提前复习：只返回 `due_at > now` 的 pending / accepted 卡片，按最早到期排序；
    /// 已到期 / 已拒绝卡片不应混入。
    #[tokio::test]
    async fn list_upcoming_reviews_returns_only_future_pending_cards() {
        let (state, pool) = setup_test_state().await;
        let now = Utc::now();

        seed_card_and_schedule(
            &pool,
            "review-due-past",
            "card-due-past",
            "已到期",
            now - Duration::minutes(5),
            1,
            "accepted",
            "pending",
        )
        .await;
        seed_card_and_schedule(
            &pool,
            "review-up-soon",
            "card-up-soon",
            "明天到期",
            now + Duration::hours(12),
            1,
            "accepted",
            "pending",
        )
        .await;
        seed_card_and_schedule(
            &pool,
            "review-up-later",
            "card-up-later",
            "下周到期",
            now + Duration::days(7),
            2,
            "accepted",
            "pending",
        )
        .await;
        seed_card_and_schedule(
            &pool,
            "review-up-rejected",
            "card-up-rejected",
            "未来但被拒绝",
            now + Duration::days(2),
            1,
            "rejected",
            "pending",
        )
        .await;

        let response = list_upcoming_reviews(
            &state,
            ListUpcomingReviewsInput { limit: Some(10) },
        )
        .await
        .unwrap();

        assert!(response.success);
        assert_eq!(response.data.items.len(), 2);
        assert_eq!(response.data.summary.upcoming_count, 2);
        assert_eq!(response.data.items[0].review_id, "review-up-soon");
        assert_eq!(response.data.items[1].review_id, "review-up-later");
        assert_eq!(
            response.data.summary.earliest_due_at,
            Some(response.data.items[0].due_at)
        );
    }

    /// 提前复习队列为空时，summary 返回 0 / None，不 panic。
    #[tokio::test]
    async fn list_upcoming_reviews_handles_empty_queue() {
        let (state, _pool) = setup_test_state().await;

        let response = list_upcoming_reviews(&state, ListUpcomingReviewsInput::default())
            .await
            .unwrap();

        assert!(response.success);
        assert!(response.data.items.is_empty());
        assert_eq!(response.data.summary.upcoming_count, 0);
        assert_eq!(response.data.summary.earliest_due_at, None);
    }

    /// 回归测试：当库中没有任何 accepted 卡片时，聚合 `SELECT MIN(...)` 会返回一行 NULL。
    /// 历史实现把标量类型写成非 Option 的 `DateTime<Utc>`，导致启动即报
    /// `invalid datetime: ` 错。现在必须安全返回 `next_due_at = None`。
    #[tokio::test]
    async fn get_review_dashboard_returns_none_next_due_when_no_accepted_card() {
        let (state, pool) = setup_test_state().await;

        // 仅写入一张 pending 状态的卡片 + 其 review_schedule，
        // 但 card.status != 'accepted'，因此所有聚合查询都会在 WHERE 阶段过滤掉它。
        seed_card_and_schedule(
            &pool,
            "pending-review",
            "pending-card",
            "未接受的卡",
            Utc::now() + Duration::hours(1),
            1,
            "pending",
            "pending",
        )
        .await;

        let response = get_review_dashboard(&state).await.unwrap();
        assert!(response.success);
        assert_eq!(response.data.due_today, 0);
        assert_eq!(response.data.completed_today, 0);
        assert_eq!(response.data.next_due_at, None);
    }

    async fn setup_test_state() -> (AppState, SqlitePool) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        TEST_MIGRATOR.run(&pool).await.unwrap();

        (AppState::from_pool(pool.clone()), pool)
    }

    async fn seed_card_and_schedule(
        pool: &SqlitePool,
        review_id: &str,
        card_id: &str,
        keyword: &str,
        due_at: chrono::DateTime<Utc>,
        review_step: i64,
        card_status: &str,
        schedule_status: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO cards (
                id, batch_id, keyword, definition, explanation, source_excerpt,
                status, created_at, updated_at, next_review_at
            )
            VALUES (?, NULL, ?, ?, ?, NULL, ?, ?, ?, ?)
            "#,
        )
        .bind(card_id)
        .bind(keyword)
        .bind(format!("{keyword} 定义"))
        .bind(format!("{keyword} 解释"))
        .bind(card_status)
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(due_at)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            INSERT INTO review_schedule (
                id, card_id, review_step, due_at, status, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(review_id)
        .bind(card_id)
        .bind(review_step)
        .bind(due_at)
        .bind(schedule_status)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(pool)
        .await
        .unwrap();
    }

    async fn seed_review_log(
        pool: &SqlitePool,
        log_id: &str,
        review_id: &str,
        card_id: &str,
        reviewed_at: chrono::DateTime<Utc>,
    ) {
        sqlx::query(
            r#"
            INSERT INTO review_logs (
                id, review_schedule_id, card_id, result, previous_step, next_step, reviewed_at
            )
            VALUES (?, ?, ?, 'remembered', 1, 2, ?)
            "#,
        )
        .bind(log_id)
        .bind(review_id)
        .bind(card_id)
        .bind(reviewed_at)
        .execute(pool)
        .await
        .unwrap();
    }

}

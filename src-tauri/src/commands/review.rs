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
    pub keyword: String,
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
    DueReviewCard {
        review_id: item.review_id,
        card_id: item.card_id,
        keyword: item.keyword,
        definition: item.definition,
        explanation: item.explanation,
        review_step: item.review_step,
        due_at: item.due_at,
        tags: Vec::new(),
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
        calculate_streak_days, get_review_dashboard, list_due_reviews, submit_review_result,
        ListDueReviewsInput, SubmitReviewResultInput,
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
        assert!(response.data.items.iter().all(|item| item.tags.is_empty()));
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

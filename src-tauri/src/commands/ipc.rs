use tauri::State;

use crate::{
    ai::MockLlmClient,
    commands::{generate, review},
    models::card::{GeneratedCardBatchResult, ReviewedGeneratedCardsResult},
    state::AppState,
    utils::error::CommandError,
};

#[tauri::command]
pub async fn generate_cards(
    state: State<'_, AppState>,
    input: generate::GenerateCardsInput,
) -> Result<GeneratedCardBatchResult, CommandError> {
    let llm = MockLlmClient;
    generate::generate_cards(
        &llm,
        state.card_dao.as_ref(),
        state.review_dao.as_ref(),
        input,
    )
    .await
    .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_generated_cards(
    state: State<'_, AppState>,
    batch_id: String,
) -> Result<GeneratedCardBatchResult, CommandError> {
    generate::list_generated_cards(state.card_dao.as_ref(), &batch_id)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn review_generated_cards(
    state: State<'_, AppState>,
    input: generate::ReviewGeneratedCardsInput,
) -> Result<ReviewedGeneratedCardsResult, CommandError> {
    generate::review_generated_cards(state.card_dao.as_ref(), input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn list_due_reviews(
    state: State<'_, AppState>,
    input: review::ListDueReviewsInput,
) -> Result<review::CommandResponse<review::ListDueReviewsData>, CommandError> {
    review::list_due_reviews(state.inner(), input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn submit_review_result(
    state: State<'_, AppState>,
    input: review::SubmitReviewResultInput,
) -> Result<review::CommandResponse<review::SubmitReviewResultData>, CommandError> {
    review::submit_review_result(state.inner(), input)
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn get_review_dashboard(
    state: State<'_, AppState>,
) -> Result<review::CommandResponse<review::ReviewDashboardData>, CommandError> {
    review::get_review_dashboard(state.inner())
        .await
        .map_err(CommandError::from)
}

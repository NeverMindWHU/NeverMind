use sqlx::Error as SqlxError;
use sqlx::migrate::MigrateError;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] SqlxError),
    #[error("migration error: {0}")]
    Migration(#[from] MigrateError),
    #[error("not found: {entity}")]
    NotFound { entity: &'static str },
    #[error("validation error: {message}")]
    Validation { message: String },
    #[error("invalid review operation: {message}")]
    InvalidReviewOperation { message: String },
    #[error("ai timeout")]
    AiTimeout,
    #[error("ai unavailable: {message}")]
    AiUnavailable { message: String },
    #[error("ai response invalid: {message}")]
    AiResponseInvalid { message: String },
}

impl AppError {
    /// 返回契约文档约定的稳定错误码，供前端按 `error.code` 映射提示文案。
    /// 其他模块新增 `NotFound.entity` 时，在此处补充映射即可。
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Database(_) | AppError::Migration(_) => "DB_WRITE_FAILED",
            AppError::NotFound { entity } => match *entity {
                "generation_batch" => "GENERATION_BATCH_NOT_FOUND",
                "review_schedule" => "REVIEW_NOT_FOUND",
                "card" => "CARD_NOT_FOUND",
                _ => "NOT_FOUND",
            },
            AppError::Validation { .. } => "INVALID_INPUT",
            AppError::InvalidReviewOperation { .. } => "INVALID_REVIEW_OPERATION",
            AppError::AiTimeout => "AI_TIMEOUT",
            AppError::AiUnavailable { .. } => "AI_UNAVAILABLE",
            AppError::AiResponseInvalid { .. } => "AI_RESPONSE_INVALID",
        }
    }
}

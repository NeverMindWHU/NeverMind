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
}

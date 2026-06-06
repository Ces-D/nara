use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use brainiac_core::error::BrainiacError;
use cadence_core::error::CadenceError;
use titans_tower::UserFacingError;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    Axum(#[from] axum::Error),
    #[error(transparent)]
    Serenity(#[from] serenity::Error),
    #[error("invalid input: {0}")]
    Field(#[from] titans_tower::FieldParseError),
    #[error(transparent)]
    Tower(#[from] titans_tower::TowerError),
    #[error(transparent)]
    Brainiac(#[from] BrainiacError),
    #[error(transparent)]
    Bean(#[from] bean::error::BeanError),
    #[error(transparent)]
    Cadence(#[from] CadenceError),
    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Multipart(#[from] axum::extract::multipart::MultipartError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("payload too large: {0}")]
    PayloadTooLarge(String),
    #[error("configuration error: {0}")]
    Config(String),
}

impl ServiceError {
    fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) | Self::Field(_) | Self::Multipart(_) => StatusCode::BAD_REQUEST,
            Self::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Brainiac(_)
            | Self::Bean(_)
            | Self::Cadence(_)
            | Self::Io(_)
            | Self::Join(_)
            | Self::Axum(_)
            | Self::Serenity(_)
            | Self::SerdeJson(_)
            | Self::Tower(_)
            | Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// True if the error message is safe to surface to the caller as-is.
/// Internal errors get logged server-side and replaced with a generic body.
impl titans_tower::UserFacingError for ServiceError {
    fn is_user_facing(&self) -> bool {
        matches!(
            self,
            Self::BadRequest(_) | Self::PayloadTooLarge(_) | Self::Field(_) | Self::Multipart(_)
        )
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = if self.is_user_facing() {
            self.to_string()
        } else {
            log::error!("internal error: {self}");
            "internal error".to_string()
        };
        (status, body).into_response()
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

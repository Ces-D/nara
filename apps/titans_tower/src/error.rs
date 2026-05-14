use crate::discord;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use brainiac_core::database::BrainiacDbError;
use konan_core::print_ops::KonanDbError;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    Axum(#[from] axum::Error),
    #[error(transparent)]
    Serenity(#[from] serenity::Error),
    #[error("invalid input: {0}")]
    Field(#[from] discord::FieldParseError),
    #[error(transparent)]
    Konan(#[from] KonanDbError),
    #[error(transparent)]
    Brainiac(#[from] BrainiacDbError),
    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Multipart(#[from] axum::extract::multipart::MultipartError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
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
            Self::Konan(_)
            | Self::Brainiac(_)
            | Self::Io(_)
            | Self::Join(_)
            | Self::Axum(_)
            | Self::Serenity(_)
            | Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// True if the error message is safe to surface to the caller as-is.
    /// Internal errors get logged server-side and replaced with a generic body.
    pub fn is_user_facing(&self) -> bool {
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

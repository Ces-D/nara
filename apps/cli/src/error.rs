use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("missing required environment variable: {0}")]
    MissingEnv(&'static str),

    #[error("invalid base URL `{url}`: {source}")]
    InvalidBaseUrl {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("server returned {status}: {body}")]
    Server { status: StatusCode, body: String },

    #[error("invalid task JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("invalid recurrence rule: {0}")]
    RRule(String),

    #[error("invalid file path: {0}")]
    InvalidPath(String),
}

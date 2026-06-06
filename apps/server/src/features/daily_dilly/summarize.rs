use super::collect::PersonDay;
use async_trait::async_trait;
use cadence_core::error::CadenceError;

/// Produces a `(title, summary)` for one person's day of messages.
#[async_trait]
pub trait Summarizer: Send + Sync {
    async fn summarize(&self, person: &PersonDay) -> Result<(String, String), CadenceError>;
}

/// Placeholder summarizer: `summary` is the messages joined by newline and
/// `title` is the first line, truncated to 80 chars.
///
/// TODO: replace with an LLM-backed [`Summarizer`] implementation.
pub struct JoinSummarizer;

#[async_trait]
impl Summarizer for JoinSummarizer {
    async fn summarize(&self, person: &PersonDay) -> Result<(String, String), CadenceError> {
        let summary = person.messages.join("\n");
        let title: String = summary
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(80)
            .collect();
        let title = if title.trim().is_empty() {
            "Daily summary".to_string()
        } else {
            title
        };
        Ok((title, summary))
    }
}

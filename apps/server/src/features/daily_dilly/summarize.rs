use super::collect::PersonDay;
use ai::{AssistantContent, Context, Message, complete, providers::openai};
use async_trait::async_trait;
use cadence_core::error::CadenceError;
use serde::Deserialize;

/// Env var holding the OpenAI API token used to generate daily-dilly summaries.
const SUMMARY_API_TOKEN_ENV: &str = "NARA_DAILLY_DILLY_SUMMARY_TOKEN";

/// Model used to generate summaries. Change here to swap models.
const SUMMARY_MODEL: &str = "gpt-5-mini";

/// System prompt steering the model to turn one person's raw daily notes into
/// an organized, skimmable review they can read the following day.
const SUMMARY_SYSTEM_PROMPT: &str = "\
You are a personal daily assistant. You turn one person's raw notes from a single \
day into an organized review they can read the next morning to pick up where they \
left off. The input is a chat-style log of that person's own messages, oldest \
first. Messages may be reminders, to-dos, notes, facts, decisions, or stray \
thoughts — unstructured and sometimes terse.

Your job is organization and synthesis, not interpretation. Read everything and \
produce a structured summary using only these sections, in this order, omitting \
any section that has no content:

- **Action items / reminders** — things to do, with any dates, times, or \
  deadlines that were mentioned.
- **Decisions** — choices the person clearly made.
- **Notes & facts** — information worth remembering.
- **Open questions** — things the person left unresolved or wanted to think about.
- **Ambiguities to clarify** — details that are unclear, underspecified, or could \
  mean more than one thing.

Rules:
- Organize and synthesize; do not invent information, and do not resolve \
  ambiguity yourself. Preserve uncertainty.
- Separate confirmed facts from assumptions or opinions; do not infer decisions \
  that were not explicitly made.
- When something is unclear, list it under \"Ambiguities to clarify\" so the \
  person can resolve it the following day. Do not ask the user questions \
  directly and do not expect any clarifying information.
- Ignore greetings, jokes, and off-topic chatter.
- Write for the person themselves reviewing tomorrow: concise, skimmable, \
  markdown bullet lists.

Respond with ONLY a JSON object, with no markdown code fences, in exactly this \
shape:
{\"title\": \"<a short 3-8 word headline capturing the day>\", \"summary\": \"<the markdown summary>\"}";

/// Produces a `(title, summary)` for one person's day of messages.
#[async_trait]
pub trait Summarizer: Send + Sync {
    async fn summarize(&self, person: &PersonDay) -> Result<(String, String), CadenceError>;
}

/// Placeholder summarizer: `summary` is the messages joined by newline and
/// `title` is the first line, truncated to 80 chars.
///
/// Retained as the fallback used by [`AiSummarizer`] when the LLM call fails and
/// as the default when no API token is configured.
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

/// Shape the model is asked to return.
#[derive(Deserialize)]
struct SummaryJson {
    title: String,
    summary: String,
}

/// LLM-backed summarizer using OpenAI via the `ai` crate.
pub struct AiSummarizer {
    api_key: String,
}

impl AiSummarizer {
    /// Build from the [`SUMMARY_API_TOKEN_ENV`] env var; `Err` if it is unset.
    pub fn from_env() -> Result<Self, CadenceError> {
        let api_key = std::env::var(SUMMARY_API_TOKEN_ENV)
            .map_err(|_| CadenceError::Channel(format!("missing {SUMMARY_API_TOKEN_ENV}")))?;
        Ok(Self { api_key })
    }

    /// Run the actual LLM call. Returns `Err` on any provider, network, or
    /// parse failure so the caller can fall back.
    async fn try_summarize(&self, person: &PersonDay) -> Result<(String, String), CadenceError> {
        // Swap providers here if ever needed — this is the only OpenAI-specific spot.
        let provider = openai::builder()
            .api_key(Some(self.api_key.as_str()))
            .build()
            .map_err(|e| CadenceError::Channel(e.to_string()))?;
        let model = provider
            .model(SUMMARY_MODEL)
            .build()
            .map_err(|e| CadenceError::Channel(e.to_string()))?;

        let user = format!(
            "Person: {}\nMessages from today (oldest first):\n{}",
            person.display,
            person.messages.join("\n"),
        );
        let ctx = Context::builder()
            .system_prompt(SUMMARY_SYSTEM_PROMPT)
            .message(Message::user_text(user))
            .build();

        // `None` options => no temperature is sent (gpt-5 reasoning models
        // reject a custom temperature).
        let resp = complete(model, ctx, None)
            .await
            .map_err(|e| CadenceError::Channel(e.to_string()))?;

        let text: String = resp
            .content
            .iter()
            .filter_map(|c| match c {
                AssistantContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect();

        let parsed: SummaryJson = serde_json::from_str(strip_fences(&text))
            .map_err(|e| CadenceError::Channel(format!("bad summary JSON: {e}")))?;
        Ok((parsed.title, parsed.summary))
    }
}

#[async_trait]
impl Summarizer for AiSummarizer {
    async fn summarize(&self, person: &PersonDay) -> Result<(String, String), CadenceError> {
        match self.try_summarize(person).await {
            Ok(r) => Ok(r),
            Err(e) => {
                log::warn!(
                    "daily-dilly: AI summary failed for {}: {e}; using join fallback",
                    person.username
                );
                JoinSummarizer.summarize(person).await
            }
        }
    }
}

/// Strip a single leading ```` ```json ````/```` ``` ```` fence and a trailing
/// ```` ``` ```` fence, if present, so fenced JSON still parses.
fn strip_fences(text: &str) -> &str {
    let trimmed = text.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    // Drop an optional language tag on the opening fence line.
    let rest = match rest.split_once('\n') {
        Some((_lang, body)) => body,
        None => rest,
    };
    rest.trim_end().strip_suffix("```").unwrap_or(rest).trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unfenced_json() {
        let raw = r#"{"title": "Busy Tuesday", "summary": "- did things"}"#;
        let parsed: SummaryJson = serde_json::from_str(strip_fences(raw)).unwrap();
        assert_eq!(parsed.title, "Busy Tuesday");
        assert_eq!(parsed.summary, "- did things");
    }

    #[test]
    fn parses_fenced_json() {
        let raw = "```json\n{\"title\": \"Busy Tuesday\", \"summary\": \"- did things\"}\n```";
        let parsed: SummaryJson = serde_json::from_str(strip_fences(raw)).unwrap();
        assert_eq!(parsed.title, "Busy Tuesday");
        assert_eq!(parsed.summary, "- did things");
    }

    #[test]
    fn parses_bare_fenced_json() {
        let raw = "```\n{\"title\": \"t\", \"summary\": \"s\"}\n```";
        let parsed: SummaryJson = serde_json::from_str(strip_fences(raw)).unwrap();
        assert_eq!(parsed.title, "t");
        assert_eq!(parsed.summary, "s");
    }
}

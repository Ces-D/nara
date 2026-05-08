use crate::database::models::{ItemState, Rating};
use chrono::{DateTime, TimeDelta, Utc};
use fsrs::{DEFAULT_PARAMETERS, FSRS, FSRSError, MemoryState, NextStates};

const DEFAULT_DESIRED_RETENTION: f32 = 0.9;

pub struct Scheduler {
    fsrs: FSRS,
}

impl Scheduler {
    pub fn new() -> Result<Self, FSRSError> {
        let f = FSRS::new(Some(&DEFAULT_PARAMETERS))?;
        Ok(Self { fsrs: f })
    }

    pub fn preview(&self, item: &ItemState, now: DateTime<Utc>) -> Result<NextStates, FSRSError> {
        let memory = memory_state_of(item);
        let elapsed = days_elapsed(item, now);
        self.fsrs
            .next_states(memory, DEFAULT_DESIRED_RETENTION, elapsed)
    }

    pub fn process_review(
        &self,
        item: &ItemState,
        rating: Rating,
        now: DateTime<Utc>,
    ) -> Result<(MemoryState, DateTime<Utc>), FSRSError> {
        let next_states = self.preview(item, now)?;

        let chosen = match rating {
            Rating::Again => next_states.again,
            Rating::Hard => next_states.hard,
            Rating::Good => next_states.good,
            Rating::Easy => next_states.easy,
        };

        let interval_secs = (chosen.interval * 86400.0).round() as i64;
        let due_at = now
            + TimeDelta::try_seconds(interval_secs).ok_or_else(|| FSRSError::OptimalNotFound)?;

        Ok((chosen.memory, due_at))
    }
}

fn memory_state_of(item: &ItemState) -> Option<MemoryState> {
    match (item.stability, item.difficulty) {
        (Some(s), Some(d)) => Some(MemoryState {
            stability: s,
            difficulty: d,
        }),
        _ => None,
    }
}

fn days_elapsed(item: &ItemState, now: DateTime<Utc>) -> u32 {
    item.last_reviewed_at
        .map(|reviewed| now.signed_duration_since(reviewed).num_days().max(0) as u32)
        .unwrap_or(0)
}

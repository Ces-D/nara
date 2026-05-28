use crate::{channels::ChannelRegistry, database::CadenceDBPool, error::CadenceError};
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::{collections::HashMap, marker::PhantomData, sync::Arc, time::Duration};

/// A payload type that knows its own task identifier. The `TASK_TYPE` constant
/// is the only place a task-type string is authored: producers (`enqueue::<T>`,
/// `schedule::<T>`, `JobOutcome::spawn::<T>`) and consumers (`Handler<T>`) all
/// derive it from the same `T`, so the producer/consumer payload shape is
/// linked at the type level.
pub trait Task: Serialize + DeserializeOwned + Send + Sync + 'static {
    const TASK_TYPE: &'static str;
}

#[async_trait]
pub trait Handler<T: Task>: Send + Sync + 'static {
    async fn run(&self, payload: T, ctx: &JobContext) -> Result<JobOutcome, CadenceError>;
}

#[async_trait]
trait ErasedHandler: Send + Sync {
    async fn run(
        &self,
        payload: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<JobOutcome, CadenceError>;
}

struct HandlerAdapter<T: Task, H: Handler<T>> {
    handler: H,
    _marker: PhantomData<fn() -> T>,
}

#[async_trait]
impl<T: Task, H: Handler<T>> ErasedHandler for HandlerAdapter<T, H> {
    async fn run(
        &self,
        payload: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<JobOutcome, CadenceError> {
        let typed: T =
            serde_json::from_value(payload).map_err(|e| CadenceError::Channel(e.to_string()))?;
        self.handler.run(typed, ctx).await
    }
}

#[derive(Default)]
pub struct TaskRegistry {
    handlers: HashMap<&'static str, Arc<dyn ErasedHandler>>,
}

impl TaskRegistry {
    pub fn register<T: Task, H: Handler<T>>(&mut self, handler: H) {
        let adapter = HandlerAdapter::<T, H> {
            handler,
            _marker: PhantomData,
        };
        if self
            .handlers
            .insert(T::TASK_TYPE, Arc::new(adapter))
            .is_some()
        {
            panic!("duplicate task_type registration {}", T::TASK_TYPE);
        }
    }

    pub(crate) async fn dispatch(
        &self,
        task_type: &str,
        payload: serde_json::Value,
        ctx: &JobContext<'_>,
    ) -> Option<Result<JobOutcome, CadenceError>> {
        let handler = self.handlers.get(task_type)?.clone();
        Some(handler.run(payload, ctx).await)
    }
}

pub enum JobOutcome {
    Done,
    Spawn {
        task_type: String,
        payload: serde_json::Value,
        artifact_ref: Option<String>,
        delay: Duration,
    },
    Retry {
        after: Duration,
    },
    Failed(CadenceError),
}

impl JobOutcome {
    /// Build a `Spawn` outcome for a typed child task. The task_type comes from
    /// `T::TASK_TYPE`, so the child's payload shape is enforced at compile time.
    pub fn spawn<T: Task>(payload: T) -> Result<Self, CadenceError> {
        Self::spawn_with(payload, None, Duration::ZERO)
    }

    pub fn spawn_with<T: Task>(
        payload: T,
        artifact_ref: Option<String>,
        delay: Duration,
    ) -> Result<Self, CadenceError> {
        let payload =
            serde_json::to_value(payload).map_err(|e| CadenceError::Channel(e.to_string()))?;
        Ok(JobOutcome::Spawn {
            task_type: T::TASK_TYPE.to_string(),
            payload,
            artifact_ref,
            delay,
        })
    }
}

pub struct JobContext<'a> {
    pub job_id: i64,
    pub artifact_ref: Option<String>,
    pub channels: &'a ChannelRegistry,
    pub db: &'a CadenceDBPool,
}

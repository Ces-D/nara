use crate::{channels::ChannelRegistry, database::CadenceDBPool, error::CadenceError};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc, time::Duration};

#[derive(Default)]
pub struct TaskRegistry {
    handlers: HashMap<&'static str, Arc<dyn Schedulable>>,
}

impl TaskRegistry {
    pub fn register<T: Schedulable + 'static>(&mut self, handler: T) {
        let key = handler.task_type();
        if self.handlers.insert(key, Arc::new(handler)).is_some() {
            panic!("duplicate task_type registration {key}");
        }
    }

    pub fn get(&self, task_type: &str) -> Option<Arc<dyn Schedulable>> {
        self.handlers.get(task_type).cloned()
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

pub struct JobContext<'a> {
    pub job_id: i64,
    pub payload: serde_json::Value,
    pub artifact_ref: Option<String>,
    pub channels: &'a ChannelRegistry,
    pub db: &'a CadenceDBPool,
}

#[async_trait]
pub trait Schedulable: Send + Sync {
    fn task_type(&self) -> &'static str;
    async fn run(&self, ctx: &JobContext) -> Result<JobOutcome, CadenceError>;
}

use crate::print_ops::{
    self, FileBuildHandler, KonanPrintChannel, KonanPrintDeliverHandler, OutlineBuildHandler,
    PrintFileTask, TrackerBuildHandler,
};
use crate::template::{BoxOutline, HabitTracker};
use cadence_core::{
    channels::ChannelRegistry,
    database::{self, CadenceDBPool, CreateSchedule, Schedule},
    error::CadenceError,
    registry::TaskRegistry,
};
use chrono::{DateTime, Utc};
use rrule::{RRule, Unvalidated};

/// Konan-facing facade over cadence_core. Owns the konan cadence pool and
/// exposes one method per HTTP / Discord entry point so callers never type a
/// task_type string or hand-build a `CreateJob` / `CreateSchedule`.
#[derive(Clone)]
pub struct KonanScheduler {
    pool: CadenceDBPool,
}

impl KonanScheduler {
    pub fn new(pool: CadenceDBPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &CadenceDBPool {
        &self.pool
    }

    pub fn register_handlers(tasks: &mut TaskRegistry) {
        tasks.register::<BoxOutline, _>(OutlineBuildHandler);
        tasks.register::<HabitTracker, _>(TrackerBuildHandler);
        tasks.register::<PrintFileTask, _>(FileBuildHandler);
        tasks.register::<print_ops::KonanDeliverPayload, _>(KonanPrintDeliverHandler);
    }

    pub fn register_channels(channels: &mut ChannelRegistry) {
        channels.register(KonanPrintChannel);
    }

    // ~~~~~~~~~~~~ one-shot enqueue

    pub async fn print_outline(&self, outline: BoxOutline) -> Result<i64, CadenceError> {
        cadence_core::enqueue::<BoxOutline>(&self.pool, outline).await
    }

    pub async fn print_tracker(&self, tracker: HabitTracker) -> Result<i64, CadenceError> {
        cadence_core::enqueue::<HabitTracker>(&self.pool, tracker).await
    }

    pub async fn print_file(&self, task: PrintFileTask) -> Result<i64, CadenceError> {
        cadence_core::enqueue::<PrintFileTask>(&self.pool, task).await
    }

    // ~~~~~~~~~~~~ schedule (recurring / future)

    pub async fn schedule_outline(
        &self,
        name: String,
        outline: BoxOutline,
        rrule: Option<RRule<Unvalidated>>,
        start: DateTime<Utc>,
    ) -> Result<i64, CadenceError> {
        cadence_core::schedule::<BoxOutline>(&self.pool, name, outline, rrule, None, start).await
    }

    pub async fn schedule_tracker(
        &self,
        name: String,
        tracker: HabitTracker,
        rrule: Option<RRule<Unvalidated>>,
        start: DateTime<Utc>,
    ) -> Result<i64, CadenceError> {
        cadence_core::schedule::<HabitTracker>(&self.pool, name, tracker, rrule, None, start).await
    }

    // ~~~~~~~~~~~~ schedule admin (low-level passthroughs used by HTTP / Discord)

    pub async fn create_schedule_raw(&self, schedule: CreateSchedule) -> Result<i64, CadenceError> {
        database::create_schedule(&self.pool, schedule).await
    }

    pub async fn list_schedules(&self) -> Result<Vec<Schedule>, CadenceError> {
        database::list_schedules(&self.pool).await
    }

    pub async fn delete_schedule(&self, id: i64) -> Result<(), CadenceError> {
        database::delete_schedule(&self.pool, id).await
    }
}

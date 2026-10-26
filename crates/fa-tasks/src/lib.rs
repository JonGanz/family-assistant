//! fa-tasks (M2): task lists, recurrence, assignment, routines. See arch/Tasks.md.

mod error;
mod ingest;
mod jobs;
mod materialize;
mod producers;
mod schedule;
mod service;
mod strategy;
mod tick;

pub use error::{Result, TaskError};
pub use ingest::{MailTaskMode, MailTaskRequest};
pub use jobs::{default_schedules, handlers};
pub use schedule::{
    advance_rrule, after_done_next, first_due, move_into_season, preview_dates, validate_rrule,
};
pub use service::{Clock, StatusFilter, TaskQuery, TaskService, TaskViewMode, Viewer};
pub use strategy::{AssignmentStrategy, Fixed, Pool, Rotation, strategy_for};

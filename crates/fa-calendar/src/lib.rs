//! fa-calendar (M1): source discovery, sync, recurrence expansion, range queries and jobs.
//! See arch/Feeds.md sections 3 and 6.

mod error;
pub mod expand;
mod jobs;
mod remote;
mod service;
pub mod tz_windows;

pub use error::{CalendarError, Result, scrub};
pub use jobs::{default_schedules, enqueue_due, handlers, prune};
pub use remote::ConnectorConfig;
pub use service::{CalendarService, mask_url, window_of};

/// Longest range `CalendarService::events` serves.
pub const MAX_RANGE_DAYS: i64 = 400;

//! M2 request and response DTOs (plan/M2.md, Domain types).
//!
//! Times of day are `"HH:MM"`, seasons `"MM-DD"`.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::authz::ListCompletionPolicy;
use crate::entities::*;
use crate::ids::*;
use crate::timefmt;

macro_rules! dto {
    ($(#[$m:meta])* $v:vis struct $name:ident { $($body:tt)* }) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
        #[ts(export, export_to = "../../../apps/web/src/gen/types/")]
        $v struct $name { $($body)* }
    };
}

// ---- task lists ----

dto!(
    pub struct TaskListView {
        pub id: TaskListId,
        pub name: String,
        pub kind: TaskListKind,
        pub completion_policy: ListCompletionPolicy,
        pub child_can_add: ChildCanAdd,
        pub dashboard_add: bool,
        pub calendar_default: CalendarDisplay,
        pub claim_ttl_minutes: Option<i32>,
        pub visible_on_dashboard: bool,
        pub color: Option<String>,
        pub sort: i32,
        pub version: i32,
    }
);

dto!(
    /// Every field is explicit; the UI fills it from `list_defaults(kind)`.
    pub struct TaskListInput {
        pub name: String,
        pub kind: TaskListKind,
        pub completion_policy: ListCompletionPolicy,
        pub child_can_add: ChildCanAdd,
        pub dashboard_add: bool,
        pub calendar_default: CalendarDisplay,
        pub claim_ttl_minutes: Option<i32>,
        pub visible_on_dashboard: bool,
        pub color: Option<String>,
        pub sort: i32,
    }
);

dto!(
    pub struct TaskListPatch {
        #[serde(default)]
        pub name: Option<String>,
        #[serde(default)]
        pub kind: Option<TaskListKind>,
        #[serde(default)]
        pub completion_policy: Option<ListCompletionPolicy>,
        #[serde(default)]
        pub child_can_add: Option<ChildCanAdd>,
        #[serde(default)]
        pub dashboard_add: Option<bool>,
        #[serde(default)]
        pub calendar_default: Option<CalendarDisplay>,
        /// Absent = keep, `null` = end of the local day.
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "number | null", optional)]
        #[schema(value_type = Option<i32>)]
        pub claim_ttl_minutes: Option<Option<i32>>,
        #[serde(default)]
        pub visible_on_dashboard: Option<bool>,
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub color: Option<Option<String>>,
        #[serde(default)]
        pub sort: Option<i32>,
    }
);

// ---- definitions ----

dto!(
    /// A season window, both ends `"MM-DD"`; may wrap the year end.
    pub struct Season {
        pub from: String,
        pub until: String,
    }
);

dto!(
    pub struct TaskDefinitionView {
        pub id: TaskDefinitionId,
        pub list: TaskListId,
        pub title: String,
        pub notes: Option<String>,
        pub steps_template: Vec<String>,
        pub schedule: Schedule,
        #[ts(type = "string")]
        pub start_date: NaiveDate,
        #[serde(with = "timefmt::hhmm_opt")]
        #[ts(type = "string | null")]
        #[schema(value_type = Option<String>, example = "18:00")]
        pub due_time: Option<NaiveTime>,
        pub lead_days: i32,
        pub on_missed: OnMissed,
        pub strategy: Strategy,
        pub candidates: Vec<MemberId>,
        pub advance_on: AdvanceOn,
        pub skip_advances: bool,
        /// Who the next occurrence goes to (rotation and fixed).
        pub next_assignee: Option<MemberId>,
        pub priority_flag: PriorityFlag,
        pub season: Option<Season>,
        pub season_condition: Option<String>,
        pub condition_ok: Option<bool>,
        pub last_done_at: Option<DateTime<Utc>>,
        pub next_due_at: Option<DateTime<Utc>>,
        pub paused: bool,
        pub version: i32,
    }
);

fn default_true() -> bool {
    true
}

dto!(
    pub struct TaskDefinitionInput {
        pub list: TaskListId,
        pub title: String,
        #[serde(default)]
        pub notes: Option<String>,
        #[serde(default)]
        pub steps_template: Vec<String>,
        pub schedule: Schedule,
        #[ts(type = "string")]
        pub start_date: NaiveDate,
        #[serde(default, with = "timefmt::hhmm_opt")]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>, example = "18:00")]
        pub due_time: Option<NaiveTime>,
        #[serde(default)]
        pub lead_days: i32,
        /// Defaults per schedule kind (D-038, D-049).
        #[serde(default)]
        pub on_missed: Option<OnMissed>,
        pub strategy: Strategy,
        #[serde(default)]
        pub candidates: Vec<MemberId>,
        #[serde(default)]
        pub advance_on: AdvanceOn,
        #[serde(default = "default_true")]
        pub skip_advances: bool,
        #[serde(default)]
        pub priority_flag: PriorityFlag,
        #[serde(default)]
        pub season: Option<Season>,
        #[serde(default)]
        pub season_condition: Option<String>,
        #[serde(default)]
        pub paused: bool,
    }
);

dto!(
    pub struct TaskDefinitionPatch {
        #[serde(default)]
        pub list: Option<TaskListId>,
        #[serde(default)]
        pub title: Option<String>,
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub notes: Option<Option<String>>,
        #[serde(default)]
        pub steps_template: Option<Vec<String>>,
        #[serde(default)]
        pub schedule: Option<Schedule>,
        #[serde(default)]
        #[ts(type = "string | null", optional)]
        pub start_date: Option<NaiveDate>,
        #[serde(
            default,
            with = "timefmt::dopt_hhmm",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub due_time: Option<Option<NaiveTime>>,
        #[serde(default)]
        pub lead_days: Option<i32>,
        #[serde(default)]
        pub on_missed: Option<OnMissed>,
        #[serde(default)]
        pub strategy: Option<Strategy>,
        #[serde(default)]
        pub candidates: Option<Vec<MemberId>>,
        #[serde(default)]
        pub advance_on: Option<AdvanceOn>,
        #[serde(default)]
        pub skip_advances: Option<bool>,
        #[serde(default)]
        pub priority_flag: Option<PriorityFlag>,
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(as = "Option<Season>", optional = nullable)]
        #[schema(value_type = Option<Season>)]
        pub season: Option<Option<Season>>,
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub season_condition: Option<Option<String>>,
        #[serde(default)]
        pub paused: Option<bool>,
    }
);

dto!(
    pub struct ValidateDefinitionRequest {
        #[serde(default)]
        pub rrule: Option<String>,
        #[serde(default)]
        pub season_condition: Option<String>,
        #[serde(default)]
        #[ts(type = "string | null", optional)]
        pub start_date: Option<NaiveDate>,
    }
);

dto!(
    pub struct DefinitionError {
        pub field: String,
        pub position: Option<i32>,
        pub message: String,
    }
);

dto!(
    pub struct ValidateDefinitionResponse {
        pub ok: bool,
        pub errors: Vec<DefinitionError>,
        /// At most 5.
        pub next_dates: Vec<DateTime<Utc>>,
    }
);

// ---- tasks ----

dto!(
    pub struct TaskStepView {
        pub id: TaskStepId,
        pub ord: i32,
        pub text: String,
        pub done_at: Option<DateTime<Utc>>,
        pub done_by: Option<MemberId>,
    }
);

dto!(
    pub struct TaskView {
        pub id: TaskId,
        pub list: TaskListId,
        pub list_name: String,
        pub definition: Option<TaskDefinitionId>,
        pub title: String,
        pub notes: Option<String>,
        pub due_at: Option<DateTime<Utc>>,
        pub visible_from: Option<DateTime<Utc>>,
        pub status: TaskStatus,
        pub origin: TaskOrigin,
        pub priority_flag: PriorityFlag,
        pub calendar_display: CalendarDisplay,
        pub assignees: Vec<MemberId>,
        /// A fixed assignee is away on the due date.
        pub assignee_away: bool,
        pub steps: Vec<TaskStepView>,
        pub claimed_by: Option<MemberId>,
        pub claimed_until: Option<DateTime<Utc>>,
        pub completed_at: Option<DateTime<Utc>>,
        pub completed_by: Option<MemberId>,
        /// Until when a reopen restores the definition.
        pub undo_until: Option<DateTime<Utc>>,
        pub score: f64,
        pub version: i32,
    }
);

dto!(
    pub struct NewTask {
        pub list: TaskListId,
        pub title: String,
        #[serde(default)]
        pub notes: Option<String>,
        #[serde(default)]
        pub due_at: Option<DateTime<Utc>>,
        #[serde(default)]
        pub assignees: Vec<MemberId>,
        #[serde(default)]
        pub steps: Vec<String>,
        #[serde(default)]
        pub priority_flag: Option<PriorityFlag>,
        #[serde(default)]
        pub calendar_display: Option<CalendarDisplay>,
        /// The acting member on a dashboard device.
        #[serde(default)]
        pub member: Option<MemberId>,
    }
);

dto!(
    pub struct TaskPatch {
        #[serde(default)]
        pub title: Option<String>,
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub notes: Option<Option<String>>,
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<DateTime<Utc>>)]
        pub due_at: Option<Option<DateTime<Utc>>>,
        #[serde(default)]
        pub assignees: Option<Vec<MemberId>>,
        #[serde(default)]
        pub priority_flag: Option<PriorityFlag>,
        #[serde(default)]
        pub calendar_display: Option<CalendarDisplay>,
        /// Replacing the steps keeps ticks for unchanged texts.
        #[serde(default)]
        pub steps: Option<Vec<String>>,
    }
);

dto!(
    pub struct CompleteTask {
        #[serde(default)]
        pub member: Option<MemberId>,
    }
);

dto!(
    pub struct StepTick {
        pub done: bool,
        #[serde(default)]
        pub member: Option<MemberId>,
    }
);

// ---- routines ----

dto!(
    pub struct RoutineStepView {
        pub id: RoutineStepId,
        pub ord: i32,
        pub text: String,
        #[serde(with = "timefmt::hhmm_opt")]
        #[ts(type = "string | null")]
        #[schema(value_type = Option<String>, example = "07:30")]
        pub target_time: Option<NaiveTime>,
    }
);

dto!(
    pub struct RoutineStepInput {
        pub text: String,
        #[serde(default, with = "timefmt::hhmm_opt")]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>, example = "07:30")]
        pub target_time: Option<NaiveTime>,
    }
);

dto!(
    pub struct RoutineView {
        pub id: RoutineId,
        pub name: String,
        pub members: Vec<MemberId>,
        pub steps: Vec<RoutineStepView>,
        #[serde(with = "timefmt::hhmm")]
        #[ts(type = "string")]
        #[schema(value_type = String, example = "06:45")]
        pub starts_at: NaiveTime,
        #[serde(with = "timefmt::hhmm")]
        #[ts(type = "string")]
        #[schema(value_type = String, example = "08:15")]
        pub done_by: NaiveTime,
        pub day_filter: DayFilter,
        pub school_calendar: Option<CalendarId>,
        pub no_school_match: Vec<String>,
        pub show_on_dashboard: bool,
        pub version: i32,
    }
);

dto!(
    pub struct RoutineInput {
        pub name: String,
        #[serde(default)]
        pub members: Vec<MemberId>,
        #[serde(default)]
        pub steps: Vec<RoutineStepInput>,
        #[serde(with = "timefmt::hhmm")]
        #[ts(type = "string")]
        #[schema(value_type = String, example = "06:45")]
        pub starts_at: NaiveTime,
        #[serde(with = "timefmt::hhmm")]
        #[ts(type = "string")]
        #[schema(value_type = String, example = "08:15")]
        pub done_by: NaiveTime,
        #[serde(default)]
        pub day_filter: DayFilter,
        #[serde(default)]
        pub school_calendar: Option<CalendarId>,
        /// Defaults to `no school, holiday, inset, closed`.
        #[serde(default)]
        pub no_school_match: Option<Vec<String>>,
        #[serde(default = "default_true")]
        pub show_on_dashboard: bool,
    }
);

dto!(
    pub struct RoutinePatch {
        #[serde(default)]
        pub name: Option<String>,
        #[serde(default)]
        pub members: Option<Vec<MemberId>>,
        #[serde(default)]
        pub steps: Option<Vec<RoutineStepInput>>,
        #[serde(default, with = "timefmt::hhmm_opt")]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub starts_at: Option<NaiveTime>,
        #[serde(default, with = "timefmt::hhmm_opt")]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub done_by: Option<NaiveTime>,
        #[serde(default)]
        pub day_filter: Option<DayFilter>,
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<CalendarId>)]
        pub school_calendar: Option<Option<CalendarId>>,
        #[serde(default)]
        pub no_school_match: Option<Vec<String>>,
        #[serde(default)]
        pub show_on_dashboard: Option<bool>,
    }
);

dto!(
    pub struct RoutineRunStepView {
        pub step: RoutineStepId,
        pub ord: i32,
        pub text: String,
        #[serde(with = "timefmt::hhmm_opt")]
        #[ts(type = "string | null")]
        #[schema(value_type = Option<String>, example = "07:30")]
        pub target_time: Option<NaiveTime>,
        pub done_at: Option<DateTime<Utc>>,
    }
);

dto!(
    pub struct RoutineRunView {
        pub id: RoutineRunId,
        pub routine: RoutineId,
        pub routine_name: String,
        pub member: MemberId,
        #[ts(type = "string")]
        pub run_date: NaiveDate,
        pub status: RunStatus,
        #[serde(with = "timefmt::hhmm")]
        #[ts(type = "string")]
        #[schema(value_type = String, example = "06:45")]
        pub starts_at: NaiveTime,
        #[serde(with = "timefmt::hhmm")]
        #[ts(type = "string")]
        #[schema(value_type = String, example = "08:15")]
        pub done_by: NaiveTime,
        pub steps: Vec<RoutineRunStepView>,
        pub done: i32,
        pub total: i32,
    }
);

// ---- absences ----

dto!(
    pub struct MemberAwayView {
        pub id: MemberAwayId,
        pub member: MemberId,
        #[ts(type = "string")]
        pub from: NaiveDate,
        #[ts(type = "string")]
        pub until: NaiveDate,
        pub note: Option<String>,
    }
);

dto!(
    pub struct NewAway {
        #[ts(type = "string")]
        pub from: NaiveDate,
        #[ts(type = "string")]
        pub until: NaiveDate,
        #[serde(default)]
        pub note: Option<String>,
    }
);

// ---- notification preferences ----

dto!(
    pub struct NotifyPrefView {
        pub member: MemberId,
        pub enabled_triggers: Vec<NotifyTrigger>,
        pub due_soon_minutes: i32,
        pub nudge_minutes: i32,
        #[serde(with = "timefmt::hhmm_opt")]
        #[ts(type = "string | null")]
        #[schema(value_type = Option<String>, example = "21:00")]
        pub quiet_from: Option<NaiveTime>,
        #[serde(with = "timefmt::hhmm_opt")]
        #[ts(type = "string | null")]
        #[schema(value_type = Option<String>, example = "07:00")]
        pub quiet_until: Option<NaiveTime>,
        pub copy_to_member: Option<MemberId>,
        /// Shown only to that member and to adults.
        pub topic: Option<String>,
        pub subscribe_url: Option<String>,
        /// False when the server has no `FA_NTFY_URL`.
        pub server_configured: bool,
        pub version: i32,
    }
);

dto!(
    pub struct NotifyPrefPatch {
        #[serde(default)]
        pub enabled_triggers: Option<Vec<NotifyTrigger>>,
        #[serde(default)]
        pub due_soon_minutes: Option<i32>,
        #[serde(default)]
        pub nudge_minutes: Option<i32>,
        /// Absent = keep, `null` = use the household default.
        #[serde(
            default,
            with = "timefmt::dopt_hhmm",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub quiet_from: Option<Option<NaiveTime>>,
        #[serde(
            default,
            with = "timefmt::dopt_hhmm",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<String>)]
        pub quiet_until: Option<Option<NaiveTime>>,
        #[serde(
            default,
            with = "timefmt::dopt",
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(type = "string | null", optional)]
        #[schema(value_type = Option<MemberId>)]
        pub copy_to_member: Option<Option<MemberId>>,
    }
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn times_serialise_as_hhmm() {
        let v = RoutineStepView {
            id: RoutineStepId::new(),
            ord: 0,
            text: "Teeth".into(),
            target_time: NaiveTime::from_hms_opt(7, 30, 0),
        };
        assert_eq!(
            serde_json::to_value(&v).unwrap()["target_time"],
            json!("07:30")
        );
        let i: RoutineStepInput =
            serde_json::from_value(json!({"text": "x", "target_time": "07:30:00"})).unwrap();
        assert_eq!(i.target_time, NaiveTime::from_hms_opt(7, 30, 0));
        assert!(
            serde_json::from_value::<RoutineStepInput>(json!({"text": "x", "target_time": "7pm"}))
                .is_err()
        );
    }

    #[test]
    fn patch_distinguishes_absent_and_null() {
        let p: TaskListPatch = serde_json::from_value(json!({})).unwrap();
        assert_eq!(p.claim_ttl_minutes, None);
        let p: TaskListPatch = serde_json::from_value(json!({"claim_ttl_minutes": null})).unwrap();
        assert_eq!(p.claim_ttl_minutes, Some(None));
        let p: TaskListPatch = serde_json::from_value(json!({"claim_ttl_minutes": 30})).unwrap();
        assert_eq!(p.claim_ttl_minutes, Some(Some(30)));
        let p: NotifyPrefPatch = serde_json::from_value(json!({"quiet_from": null})).unwrap();
        assert_eq!(p.quiet_from, Some(None));
        let p: NotifyPrefPatch = serde_json::from_value(json!({"quiet_from": "22:00"})).unwrap();
        assert_eq!(p.quiet_from, Some(NaiveTime::from_hms_opt(22, 0, 0)));
        assert_eq!(p.quiet_until, None);
    }

    #[test]
    fn schedule_is_tagged() {
        let s: Schedule =
            serde_json::from_value(json!({"kind": "after_done", "interval_days": 7})).unwrap();
        assert_eq!(s, Schedule::AfterDone { interval_days: 7 });
        assert_eq!(
            serde_json::to_value(Schedule::Rrule {
                rrule: "FREQ=DAILY".into()
            })
            .unwrap(),
            json!({"kind": "rrule", "rrule": "FREQ=DAILY"})
        );
    }

    #[test]
    fn definition_input_defaults() {
        let list = TaskListId::new();
        let i: TaskDefinitionInput = serde_json::from_value(json!({
            "list": list, "title": "Bins",
            "schedule": {"kind": "rrule", "rrule": "FREQ=WEEKLY;BYDAY=SA"},
            "start_date": "2026-01-03", "strategy": "fixed"
        }))
        .unwrap();
        assert!(i.skip_advances);
        assert_eq!(i.advance_on, AdvanceOn::AnyCompletion);
        assert_eq!(i.on_missed, None);
        assert!(i.candidates.is_empty());
    }

    #[test]
    fn enums_round_trip_db_text() {
        for v in TaskStatus::ALL {
            assert_eq!(v.as_str().parse::<TaskStatus>().unwrap(), *v);
            assert_eq!(serde_json::to_value(v).unwrap(), json!(v.as_str()));
        }
        for v in NotifyTrigger::ALL {
            assert_eq!(v.as_str().parse::<NotifyTrigger>().unwrap(), *v);
            assert_eq!(serde_json::to_value(v).unwrap(), json!(v.as_str()));
        }
        for v in DayFilter::ALL {
            assert_eq!(serde_json::to_value(v).unwrap(), json!(v.as_str()));
        }
        assert_eq!(ChildCanAdd::None.as_str(), "none");
        assert_eq!(CalendarDisplay::UntilDone.as_str(), "until_done");
        assert_eq!(OnMissed::RollOver.as_str(), "roll_over");
        assert_eq!(
            AdvanceOn::AssigneeCompletion.as_str(),
            "assignee_completion"
        );
        assert!("nope".parse::<Strategy>().is_err());
        assert_eq!(
            "adult_only".parse::<ListCompletionPolicy>().unwrap(),
            ListCompletionPolicy::AdultOnly
        );
    }
}

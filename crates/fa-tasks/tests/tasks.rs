use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use fa_domain::Schedule;
use fa_domain::tasks::due_instant;
use fa_domain::tasks_api::*;
use fa_domain::*;
use fa_store::*;
use fa_tasks::{
    MailTaskMode, MailTaskRequest, StatusFilter, TaskError, TaskQuery, TaskService, TaskViewMode,
    Viewer,
};
use sqlx::PgPool;

const LONDON: chrono_tz::Tz = chrono_tz::Europe::London;

fn tm(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}
fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}
fn t(y: i32, m: u32, day: u32, h: u32, min: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, day, h, min, 0).unwrap()
}

fn member(h: HouseholdId, name: &str, role: Role) -> Member {
    Member {
        id: MemberId::new(),
        household_id: h,
        display_name: name.into(),
        role,
        avatar: None,
        color: None,
        birthdate: None,
        grade: None,
        interests: vec![],
        hobbies: vec![],
        responsibilities: vec![],
        aliases: Vec::new(),
        gender: None,
        relevance_tags: Vec::new(),
        pin_hash: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

struct Env {
    pool: PgPool,
    store: Store,
    svc: TaskService,
    clock: Arc<Mutex<DateTime<Utc>>>,
    h: HouseholdId,
    adult: MemberId,
    kid1: MemberId,
    kid2: MemberId,
    list: TaskListId,
}

impl Env {
    fn at(&self, now: DateTime<Utc>) {
        *self.clock.lock().unwrap() = now;
    }

    async fn def(&self, f: impl FnOnce(&mut TaskDefinitionInput)) -> TaskDefinitionId {
        let mut i = TaskDefinitionInput {
            list: self.list,
            title: "Bins".into(),
            notes: None,
            steps_template: vec![],
            schedule: Schedule::AfterDone { interval_days: 7 },
            start_date: d(2026, 3, 1),
            due_time: None,
            lead_days: 0,
            on_missed: None,
            strategy: Strategy::Fixed,
            candidates: vec![self.kid1],
            advance_on: AdvanceOn::AnyCompletion,
            skip_advances: true,
            priority_flag: PriorityFlag::Normal,
            season: None,
            season_condition: None,
            paused: false,
        };
        f(&mut i);
        self.svc.create_definition(self.h, i).await.unwrap().id
    }

    async fn materialize(&self, now: DateTime<Utc>) -> usize {
        self.at(now);
        self.svc.materialize_all(now).await.unwrap()
    }

    async fn tasks(&self, status: StatusFilter) -> Vec<TaskView> {
        self.svc
            .list_tasks(
                self.h,
                &TaskQuery {
                    status: Some(status),
                    ..Default::default()
                },
                Viewer::default(),
            )
            .await
            .unwrap()
    }

    async fn definition(&self, id: TaskDefinitionId) -> TaskDefinition {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        TaskDefinitionRepo::get(&mut tx, id)
            .await
            .unwrap()
            .unwrap()
            .value
    }

    async fn count(&self, trigger: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM notification WHERE trigger = $1")
            .bind(trigger)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn prefs(&self, m: MemberId, triggers: &[&str]) {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        NotifyPrefRepo::upsert_for_member(
            &mut tx,
            &NotifyPref {
                id: uuid::Uuid::now_v7(),
                household_id: self.h,
                member_id: m,
                channel: "ntfy".into(),
                topic: format!("topic{}", m.as_uuid().simple()),
                triggers: triggers.iter().map(|s| s.to_string()).collect(),
                due_soon_minutes: 120,
                nudge_minutes: 10,
                quiet_from: None,
                quiet_until: None,
                copy_to_member_id: None,
                approval_mode: ApprovalMode::Batched,
                archived_at: None,
            },
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    async fn manual(&self, f: impl FnOnce(&mut NewTask)) -> TaskView {
        let mut n = NewTask {
            list: self.list,
            title: "Task".into(),
            notes: None,
            due_at: None,
            assignees: vec![],
            steps: vec![],
            priority_flag: None,
            calendar_display: None,
            member: None,
        };
        f(&mut n);
        self.svc
            .create_task(self.h, n, Some(self.adult), None)
            .await
            .unwrap()
    }
}

async fn env(pool: PgPool) -> Env {
    let store = Store::from_pool(pool.clone());
    let h = Household {
        id: HouseholdId::new(),
        name: "Test".into(),
        tz: "Europe/London".into(),
        location: None,
        meal_plan_horizon_weeks: 2,
        dashboard_completion_override: false,
        units: Units::Metric,
        calendar_past_days: 30,
        calendar_future_days: 400,
        weather_retention_days: 7,
        private_events_as_busy: true,
        now_list_size: 7,
        now_weights: fa_domain::tasks::NowWeights::default(),
        quiet_from: tm(21, 0),
        quiet_until: tm(7, 0),
        currency: fa_domain::DEFAULT_CURRENCY.to_string(),
        gender_terms: Default::default(),
        disabled_signals: Vec::new(),
        events: Default::default(),
        inbox: Default::default(),
        retention_mail: Default::default(),
        bill_lead_days: fa_domain::DEFAULT_BILL_LEAD_DAYS,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let (adult, kid1, kid2) = (
        member(h.id, "Alex", Role::Adult),
        member(h.id, "Sam", Role::Child),
        member(h.id, "Robin", Role::Child),
    );
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &h).await.unwrap();
    for m in [&adult, &kid1, &kid2] {
        MemberRepo::insert(&mut tx, m, None, None).await.unwrap();
    }
    tx.commit().await.unwrap();
    let clock = Arc::new(Mutex::new(t(2026, 3, 1, 8, 0)));
    let c = clock.clone();
    let svc = TaskService::with_clock(store.clone(), Arc::new(move || *c.lock().unwrap()));
    let list = svc
        .create_task_list(
            h.id,
            TaskListInput {
                name: "Chores".into(),
                kind: TaskListKind::Chores,
                completion_policy: ListCompletionPolicy::Assignee,
                child_can_add: ChildCanAdd::None,
                dashboard_add: false,
                calendar_default: CalendarDisplay::Hidden,
                claim_ttl_minutes: Some(30),
                visible_on_dashboard: true,
                color: None,
                sort: 0,
            },
        )
        .await
        .unwrap()
        .id;
    Env {
        pool,
        store,
        svc,
        clock,
        h: h.id,
        adult: adult.id,
        kid1: kid1.id,
        kid2: kid2.id,
        list,
    }
}

macro_rules! test {
    ($name:ident, |$e:ident| $body:block) => {
        #[sqlx::test(migrations = "../fa-store/migrations")]
        async fn $name(pool: PgPool) {
            let $e = env(pool).await;
            $body
        }
    };
}

fn weekly_sat() -> Schedule {
    Schedule::Rrule {
        rrule: "FREQ=WEEKLY;BYDAY=SA".into(),
    }
}

// ---- scheduling ----

test!(after_done_completion_and_skip_move_next_due, |e| {
    let id = e.def(|_| {}).await;
    assert_eq!(
        e.definition(id).await.next_due_at,
        Some(t(2026, 3, 1, 23, 59))
    );
    assert_eq!(e.materialize(t(2026, 3, 1, 9, 0)).await, 1);
    // roll_over: repeated ticks keep one occurrence.
    assert_eq!(e.materialize(t(2026, 3, 2, 9, 0)).await, 0);
    assert_eq!(e.materialize(t(2026, 3, 3, 9, 0)).await, 0);
    let open = e.tasks(StatusFilter::Open).await;
    assert_eq!(open.len(), 1);
    e.at(t(2026, 3, 3, 10, 0));
    let done = e
        .svc
        .complete(e.h, open[0].id, Some(e.kid1), None)
        .await
        .unwrap();
    assert_eq!(done.status, TaskStatus::Done);
    assert_eq!(done.undo_until, Some(t(2026, 3, 3, 10, 10)));
    let def = e.definition(id).await;
    assert_eq!(def.next_due_at, Some(t(2026, 3, 10, 23, 59)));
    assert_eq!(def.last_done_at, Some(t(2026, 3, 3, 10, 0)));
    assert_eq!(e.materialize(t(2026, 3, 9, 12, 0)).await, 0);
    assert_eq!(e.materialize(t(2026, 3, 10, 8, 0)).await, 1);
    let open = e.tasks(StatusFilter::Open).await;
    e.at(t(2026, 3, 12, 8, 0));
    e.svc.skip(e.h, open[0].id, Some(e.adult)).await.unwrap();
    let def = e.definition(id).await;
    assert_eq!(def.next_due_at, Some(t(2026, 3, 19, 23, 59)));
    // Only completion sets last_done_at.
    assert_eq!(def.last_done_at, Some(t(2026, 3, 3, 10, 0)));
});

test!(rrule_auto_skip_creates_one_per_week, |e| {
    e.at(t(2026, 1, 1, 8, 0));
    let id = e
        .def(|i| {
            i.schedule = weekly_sat();
            i.start_date = d(2026, 1, 1);
        })
        .await;
    assert_eq!(e.definition(id).await.on_missed, OnMissed::AutoSkip);
    assert_eq!(
        e.definition(id).await.next_due_at,
        Some(t(2026, 1, 3, 23, 59))
    );
    assert_eq!(e.materialize(t(2026, 1, 2, 12, 0)).await, 0);
    assert_eq!(e.materialize(t(2026, 1, 3, 8, 0)).await, 1);
    assert_eq!(e.materialize(t(2026, 1, 3, 8, 1)).await, 0);
    assert_eq!(
        e.definition(id).await.next_due_at,
        Some(t(2026, 1, 10, 23, 59))
    );
    assert_eq!(e.materialize(t(2026, 1, 10, 8, 0)).await, 1);
    assert_eq!(e.tasks(StatusFilter::Open).await.len(), 1);
    let skipped = e.tasks(StatusFilter::Skipped).await;
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].due_at, Some(t(2026, 1, 3, 23, 59)));
});

test!(rrule_roll_over_consumes_instances, |e| {
    e.at(t(2026, 1, 1, 8, 0));
    let id = e
        .def(|i| {
            i.schedule = weekly_sat();
            i.start_date = d(2026, 1, 1);
            i.on_missed = Some(OnMissed::RollOver);
        })
        .await;
    assert_eq!(e.materialize(t(2026, 1, 3, 8, 0)).await, 1);
    assert_eq!(e.materialize(t(2026, 1, 10, 8, 0)).await, 0);
    assert_eq!(
        e.definition(id).await.next_due_at,
        Some(t(2026, 1, 17, 23, 59))
    );
    assert_eq!(e.materialize(t(2026, 1, 17, 8, 0)).await, 0);
    assert_eq!(
        e.definition(id).await.next_due_at,
        Some(t(2026, 1, 24, 23, 59))
    );
    assert_eq!(e.tasks(StatusFilter::Any).await.len(), 1);
});

test!(rrule_pile_up_keeps_all, |e| {
    e.at(t(2026, 1, 1, 8, 0));
    e.def(|i| {
        i.schedule = weekly_sat();
        i.start_date = d(2026, 1, 1);
        i.on_missed = Some(OnMissed::PileUp);
    })
    .await;
    e.materialize(t(2026, 1, 3, 8, 0)).await;
    e.materialize(t(2026, 1, 10, 8, 0)).await;
    assert_eq!(e.tasks(StatusFilter::Open).await.len(), 2);
});

test!(rrule_dst_week_in_london, |e| {
    e.at(t(2026, 3, 21, 8, 0));
    let id = e
        .def(|i| {
            i.schedule = weekly_sat();
            i.start_date = d(2026, 3, 21);
            i.due_time = Some(tm(18, 0));
        })
        .await;
    assert_eq!(
        e.definition(id).await.next_due_at,
        Some(t(2026, 3, 21, 18, 0))
    );
    assert_eq!(e.materialize(t(2026, 3, 21, 12, 0)).await, 1);
    assert_eq!(e.materialize(t(2026, 3, 28, 12, 0)).await, 1);
    // 03-28 18:00 is GMT; 04-04 18:00 is BST (17:00 UTC).
    assert_eq!(
        e.definition(id).await.next_due_at,
        Some(t(2026, 4, 4, 17, 0))
    );
    let open = e.tasks(StatusFilter::Open).await;
    assert!(open.iter().any(|x| x.due_at == Some(t(2026, 3, 28, 18, 0))));
});

test!(lead_days_and_visible_from, |e| {
    e.at(t(2026, 5, 1, 8, 0));
    e.def(|i| {
        i.start_date = d(2026, 5, 10);
        i.due_time = Some(tm(18, 0));
        i.lead_days = 2;
    })
    .await;
    // visible from local 05-08 00:00 BST = 05-07 23:00 UTC.
    assert_eq!(e.materialize(t(2026, 5, 7, 22, 59)).await, 0);
    assert_eq!(e.materialize(t(2026, 5, 7, 23, 0)).await, 1);
    let open = e.tasks(StatusFilter::Open).await;
    assert_eq!(open[0].visible_from, Some(t(2026, 5, 7, 23, 0)));
    assert_eq!(open[0].due_at, Some(t(2026, 5, 10, 17, 0)));
});

test!(season_window_wrapping_year_end, |e| {
    e.at(t(2026, 6, 1, 8, 0));
    let id = e
        .def(|i| {
            i.start_date = d(2026, 6, 1);
            i.season = Some(Season {
                from: "11-01".into(),
                until: "02-28".into(),
            });
        })
        .await;
    let first = e.definition(id).await.next_due_at.unwrap();
    assert_eq!(first, due_instant(d(2026, 11, 1), None, LONDON));
    assert_eq!(e.materialize(t(2026, 10, 31, 12, 0)).await, 0);
    assert_eq!(e.materialize(t(2026, 11, 1, 8, 0)).await, 1);
    let bad = e
        .svc
        .create_definition(
            e.h,
            TaskDefinitionInput {
                list: e.list,
                title: "x".into(),
                notes: None,
                steps_template: vec![],
                schedule: Schedule::AfterDone { interval_days: 1 },
                start_date: d(2026, 1, 1),
                due_time: None,
                lead_days: 0,
                on_missed: None,
                strategy: Strategy::Fixed,
                candidates: vec![],
                advance_on: AdvanceOn::AnyCompletion,
                skip_advances: true,
                priority_flag: PriorityFlag::Normal,
                season: Some(Season {
                    from: "13-01".into(),
                    until: "02-28".into(),
                }),
                season_condition: None,
                paused: false,
            },
        )
        .await;
    assert!(matches!(bad, Err(TaskError::Invalid { .. })));
});

test!(condition_false_defers_and_stale_counts_as_true, |e| {
    let id = e.def(|_| {}).await;
    let now = t(2026, 3, 1, 9, 0);
    let set = |ok: bool, at: DateTime<Utc>| {
        let store = e.store.clone();
        let h = e.h;
        async move {
            let mut tx = store.tx_for(h).await.unwrap();
            TaskDefinitionRepo::set_condition_result(&mut tx, id, ok, at)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
    };
    set(false, now - Duration::hours(1)).await;
    assert_eq!(e.materialize(now).await, 0);
    set(false, now - Duration::hours(7)).await;
    assert_eq!(e.materialize(now).await, 1);
});

test!(invalid_rrule_is_rejected, |e| {
    let r = e
        .svc
        .create_definition(
            e.h,
            TaskDefinitionInput {
                list: e.list,
                title: "x".into(),
                notes: None,
                steps_template: vec![],
                schedule: Schedule::Rrule {
                    rrule: "FREQ=SOMETIMES".into(),
                },
                start_date: d(2026, 1, 1),
                due_time: None,
                lead_days: 0,
                on_missed: None,
                strategy: Strategy::Fixed,
                candidates: vec![],
                advance_on: AdvanceOn::AnyCompletion,
                skip_advances: true,
                priority_flag: PriorityFlag::Normal,
                season: None,
                season_condition: None,
                paused: false,
            },
        )
        .await;
    assert!(matches!(r, Err(TaskError::Invalid { ref field, .. }) if field == "rrule"));
});

// ---- strategies ----

async fn cursor_after(e: &Env, id: TaskDefinitionId) -> i32 {
    e.definition(id).await.rotation_cursor
}

test!(rotation_any_vs_assignee_completion, |e| {
    let any = e
        .def(|i| {
            i.title = "Any".into();
            i.strategy = Strategy::Rotation;
            i.candidates = vec![e.kid1, e.kid2, e.adult];
        })
        .await;
    let own = e
        .def(|i| {
            i.title = "Own".into();
            i.strategy = Strategy::Rotation;
            i.advance_on = AdvanceOn::AssigneeCompletion;
            i.candidates = vec![e.kid1, e.kid2, e.adult];
        })
        .await;
    assert_eq!(e.materialize(t(2026, 3, 1, 9, 0)).await, 2);
    let open = e.tasks(StatusFilter::Open).await;
    for task in &open {
        assert_eq!(task.assignees, vec![e.kid1]);
    }
    let by_title = |s: &str| open.iter().find(|x| x.title == s).unwrap().id;
    // Somebody else does both.
    e.svc
        .complete(e.h, by_title("Any"), Some(e.adult), None)
        .await
        .unwrap();
    e.svc
        .complete(e.h, by_title("Own"), Some(e.adult), None)
        .await
        .unwrap();
    assert_eq!(cursor_after(&e, any).await, 1);
    assert_eq!(cursor_after(&e, own).await, 0);
    let v = e.svc.get_definition(e.h, any).await.unwrap();
    assert_eq!(v.next_assignee, Some(e.kid2));
});

test!(rotation_skip_advances_and_absence, |e| {
    let adv = e
        .def(|i| {
            i.title = "Adv".into();
            i.strategy = Strategy::Rotation;
            i.candidates = vec![e.kid1, e.kid2, e.adult];
        })
        .await;
    let stay = e
        .def(|i| {
            i.title = "Stay".into();
            i.strategy = Strategy::Rotation;
            i.skip_advances = false;
            i.candidates = vec![e.kid1, e.kid2, e.adult];
        })
        .await;
    e.materialize(t(2026, 3, 1, 9, 0)).await;
    for task in e.tasks(StatusFilter::Open).await {
        e.svc.skip(e.h, task.id, Some(e.adult)).await.unwrap();
    }
    assert_eq!(cursor_after(&e, adv).await, 1);
    assert_eq!(cursor_after(&e, stay).await, 0);
    // kid2 is away on the next due date, so the adult takes the turn.
    e.svc
        .add_away(
            e.h,
            e.kid2,
            NewAway {
                from: d(2026, 3, 1),
                until: d(2026, 3, 20),
                note: None,
            },
        )
        .await
        .unwrap();
    let v = e.svc.get_definition(e.h, adv).await.unwrap();
    assert_eq!(v.next_assignee, Some(e.adult));
    assert!(
        e.svc
            .add_away(
                e.h,
                e.kid2,
                NewAway {
                    from: d(2026, 3, 5),
                    until: d(2026, 3, 4),
                    note: None
                }
            )
            .await
            .is_err()
    );
});

test!(pool_claims_and_expiry, |e| {
    e.def(|i| {
        i.strategy = Strategy::Pool;
        i.candidates = vec![e.kid1, e.kid2];
    })
    .await;
    e.materialize(t(2026, 3, 1, 9, 0)).await;
    let task = e.tasks(StatusFilter::Open).await.remove(0);
    assert_eq!(task.assignees.len(), 2);
    let now = t(2026, 3, 1, 9, 0);
    e.at(now);
    let c = e.svc.claim(e.h, task.id, e.kid1).await.unwrap();
    assert_eq!(c.claimed_by, Some(e.kid1));
    assert_eq!(c.claimed_until, Some(now + Duration::minutes(30)));
    assert!(matches!(
        e.svc.claim(e.h, task.id, e.kid2).await,
        Err(TaskError::ClaimedByOther)
    ));
    assert!(matches!(
        e.svc.release(e.h, task.id, e.kid2).await,
        Err(TaskError::ClaimedByOther)
    ));
    e.at(now + Duration::minutes(31));
    e.svc.tick_all(now + Duration::minutes(31)).await.unwrap();
    assert_eq!(e.tasks(StatusFilter::Open).await[0].claimed_by, None);
    assert_eq!(
        e.svc.claim(e.h, task.id, e.kid2).await.unwrap().claimed_by,
        Some(e.kid2)
    );
    let r = e.svc.release(e.h, task.id, e.kid2).await.unwrap();
    assert_eq!(r.claimed_by, None);
    // A non-pool task cannot be claimed.
    let manual = e.manual(|n| n.assignees = vec![e.kid1]).await;
    assert!(matches!(
        e.svc.claim(e.h, manual.id, e.kid1).await,
        Err(TaskError::NotPool)
    ));
});

test!(pool_claim_without_ttl_ends_with_the_local_day, |e| {
    e.svc
        .update_task_list(
            e.h,
            e.list,
            TaskListPatch {
                claim_ttl_minutes: Some(None),
                ..serde_json::from_value(serde_json::json!({})).unwrap()
            },
            1,
        )
        .await
        .unwrap();
    e.def(|i| {
        i.strategy = Strategy::Pool;
        i.candidates = vec![e.kid1];
    })
    .await;
    e.materialize(t(2026, 3, 1, 9, 0)).await;
    let task = e.tasks(StatusFilter::Open).await.remove(0);
    let c = e.svc.claim(e.h, task.id, e.kid1).await.unwrap();
    assert_eq!(c.claimed_until, Some(t(2026, 3, 1, 23, 59)));
    // A child outside the candidates cannot claim; an adult can.
    assert!(matches!(
        e.svc.claim(e.h, task.id, e.kid2).await,
        Err(TaskError::NotCandidate)
    ));
});

test!(fixed_with_away_member_is_flagged, |e| {
    e.def(|_| {}).await;
    e.svc
        .add_away(
            e.h,
            e.kid1,
            NewAway {
                from: d(2026, 2, 27),
                until: d(2026, 3, 3),
                note: Some("trip".into()),
            },
        )
        .await
        .unwrap();
    e.materialize(t(2026, 3, 1, 9, 0)).await;
    let task = e.tasks(StatusFilter::Open).await.remove(0);
    assert_eq!(task.assignees, vec![e.kid1]);
    assert!(task.assignee_away);
});

// ---- undo ----

test!(undo_within_ten_minutes_restores_the_definition, |e| {
    let id = e
        .def(|i| {
            i.lead_days = 10;
            i.strategy = Strategy::Rotation;
            i.candidates = vec![e.kid1, e.kid2];
        })
        .await;
    let t0 = t(2026, 3, 1, 9, 0);
    e.materialize(t0).await;
    let first = e.tasks(StatusFilter::Open).await.remove(0);
    let before = e.definition(id).await;
    e.at(t0);
    e.svc
        .complete(e.h, first.id, Some(e.kid1), None)
        .await
        .unwrap();
    // The follow-up appears (lead_days 10).
    assert_eq!(e.materialize(t0 + Duration::minutes(1)).await, 1);
    assert_eq!(e.tasks(StatusFilter::Open).await.len(), 1);
    e.at(t0 + Duration::minutes(5));
    let reopened = e.svc.reopen(e.h, first.id).await.unwrap();
    assert_eq!(reopened.status, TaskStatus::Open);
    let after = e.definition(id).await;
    assert_eq!(after.rotation_cursor, before.rotation_cursor);
    assert_eq!(after.last_done_at, before.last_done_at);
    assert_eq!(after.next_due_at, before.next_due_at);
    let open = e.tasks(StatusFilter::Open).await;
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].id, first.id);
});

test!(undo_after_ten_minutes_only_reopens, |e| {
    let id = e.def(|i| i.lead_days = 10).await;
    let t0 = t(2026, 3, 1, 9, 0);
    e.materialize(t0).await;
    let first = e.tasks(StatusFilter::Open).await.remove(0);
    e.at(t0);
    e.svc
        .complete(e.h, first.id, Some(e.kid1), None)
        .await
        .unwrap();
    e.materialize(t0 + Duration::minutes(1)).await;
    let moved = e.definition(id).await;
    e.at(t0 + Duration::minutes(11));
    e.svc.reopen(e.h, first.id).await.unwrap();
    let after = e.definition(id).await;
    assert_eq!(after.next_due_at, moved.next_due_at);
    assert_eq!(after.last_done_at, moved.last_done_at);
    assert_eq!(e.tasks(StatusFilter::Open).await.len(), 2);
});

test!(status_rules_are_typed_errors, |e| {
    let task = e.manual(|n| n.assignees = vec![e.kid1]).await;
    e.svc
        .complete(e.h, task.id, Some(e.kid1), None)
        .await
        .unwrap();
    assert!(matches!(
        e.svc.complete(e.h, task.id, Some(e.kid1), None).await,
        Err(TaskError::WrongStatus { .. })
    ));
    assert!(matches!(
        e.svc.skip(e.h, task.id, None).await,
        Err(TaskError::WrongStatus { .. })
    ));
    assert!(matches!(
        e.svc.accept(e.h, task.id).await,
        Err(TaskError::WrongStatus { .. })
    ));
    assert!(matches!(
        e.svc.reopen(e.h, TaskId::new()).await,
        Err(TaskError::NotFound)
    ));
});

test!(ticking_all_steps_does_not_complete, |e| {
    let task = e.manual(|n| n.steps = vec!["a".into(), "b".into()]).await;
    for s in &task.steps {
        let v = e
            .svc
            .tick_step(e.h, task.id, s.id, true, Some(e.kid1), None)
            .await
            .unwrap();
        assert_eq!(v.status, TaskStatus::Open);
    }
    let v = e.svc.get_task(e.h, task.id).await.unwrap();
    assert!(v.steps.iter().all(|s| s.done_at.is_some()));
});

// ---- Now list and calendar ----

test!(now_list_order_size_and_child_scope, |e| {
    sqlx::query("UPDATE household SET now_list_size = 3 WHERE id = $1")
        .bind(e.h.as_uuid())
        .execute(&e.pool)
        .await
        .unwrap();
    let now = t(2026, 3, 1, 12, 0);
    e.at(now);
    let a = e
        .manual(|n| {
            n.title = "A".into();
            n.due_at = Some(now + Duration::hours(12));
        })
        .await;
    let b = e
        .manual(|n| {
            n.title = "B".into();
            n.due_at = Some(now - Duration::hours(6));
        })
        .await;
    let c = e
        .manual(|n| {
            n.title = "C".into();
            n.priority_flag = Some(PriorityFlag::High);
        })
        .await;
    e.manual(|n| n.title = "D".into()).await;
    let q = TaskQuery {
        view: TaskViewMode::Now,
        ..Default::default()
    };
    let list = e.svc.list_tasks(e.h, &q, Viewer::default()).await.unwrap();
    assert_eq!(
        list.iter().map(|x| x.id).collect::<Vec<_>>(),
        vec![b.id, c.id, a.id]
    );
    // A child sees only their tasks and unassigned ones.
    let mine = e
        .manual(|n| {
            n.title = "Mine".into();
            n.assignees = vec![e.kid1];
        })
        .await;
    let theirs = e
        .manual(|n| {
            n.title = "Theirs".into();
            n.assignees = vec![e.kid2];
        })
        .await;
    let all = TaskQuery::default();
    let child = Viewer {
        member: Some(e.kid1),
        role: Some(Role::Child),
        device: false,
    };
    let ids: Vec<TaskId> = e
        .svc
        .list_tasks(e.h, &all, child)
        .await
        .unwrap()
        .iter()
        .map(|x| x.id)
        .collect();
    assert!(ids.contains(&mine.id) && ids.contains(&a.id));
    assert!(!ids.contains(&theirs.id));
    let mine_view = TaskQuery {
        view: TaskViewMode::Mine,
        ..Default::default()
    };
    let ids: Vec<TaskId> = e
        .svc
        .list_tasks(e.h, &mine_view, child)
        .await
        .unwrap()
        .iter()
        .map(|x| x.id)
        .collect();
    assert_eq!(ids, vec![mine.id]);
});

test!(calendar_range_rule, |e| {
    let now = t(2026, 3, 10, 12, 0);
    e.at(now);
    let due = Some(now + Duration::days(1));
    let mk = |title: &str, display: CalendarDisplay| {
        let title = title.to_owned();
        e.manual(move |n| {
            n.title = title;
            n.due_at = due;
            n.calendar_display = Some(display);
        })
    };
    let hidden = mk("hidden", CalendarDisplay::Hidden).await;
    let until_open = mk("until_open", CalendarDisplay::UntilDone).await;
    let until_done = mk("until_done", CalendarDisplay::UntilDone).await;
    let always_done = mk("always_done", CalendarDisplay::Always).await;
    for x in [&until_done, &always_done] {
        e.svc.complete(e.h, x.id, Some(e.kid1), None).await.unwrap();
    }
    let q = TaskQuery {
        from: Some(now),
        to: Some(now + Duration::days(3)),
        ..Default::default()
    };
    let ids: Vec<TaskId> = e
        .svc
        .list_tasks(e.h, &q, Viewer::default())
        .await
        .unwrap()
        .iter()
        .map(|x| x.id)
        .collect();
    assert!(ids.contains(&until_open.id));
    assert!(ids.contains(&always_done.id));
    assert!(!ids.contains(&until_done.id));
    assert!(!ids.contains(&hidden.id));
    let dash = e
        .svc
        .dashboard_tasks(e.h, now, now + Duration::days(3))
        .await
        .unwrap();
    let dids: Vec<TaskId> = dash.iter().map(|x| x.id).collect();
    assert!(
        dids.contains(&always_done.id)
            && dids.contains(&hidden.id)
            && !dids.contains(&until_done.id)
    );
});

// ---- routines ----

async fn routine(e: &Env, f: impl FnOnce(&mut RoutineInput)) -> RoutineView {
    let mut i = RoutineInput {
        name: "Morning".into(),
        members: vec![e.kid1],
        steps: vec![
            RoutineStepInput {
                text: "Teeth".into(),
                target_time: Some(tm(7, 30)),
            },
            RoutineStepInput {
                text: "Dress".into(),
                target_time: None,
            },
        ],
        starts_at: tm(6, 45),
        done_by: tm(8, 15),
        day_filter: DayFilter::Weekdays,
        school_calendar: None,
        no_school_match: None,
        show_on_dashboard: true,
    };
    f(&mut i);
    e.svc.create_routine(e.h, i).await.unwrap()
}

test!(routines_weekdays_done_and_skipped, |e| {
    routine(&e, |_| {}).await;
    // 2026-06-06 is a Saturday.
    assert_eq!(
        e.svc
            .routines_materialize_all(t(2026, 6, 6, 8, 0))
            .await
            .unwrap(),
        0
    );
    let mon = t(2026, 6, 1, 6, 0);
    assert_eq!(e.svc.routines_materialize_all(mon).await.unwrap(), 1);
    assert_eq!(e.svc.routines_materialize_all(mon).await.unwrap(), 0);
    e.at(mon);
    let run = e
        .svc
        .routines_today(e.h, Some(e.kid1))
        .await
        .unwrap()
        .remove(0);
    assert_eq!((run.done, run.total), (0, 2));
    let r = e
        .svc
        .tick_routine_step(e.h, run.id, run.steps[0].step, true, Some(e.kid1), None)
        .await
        .unwrap();
    assert_eq!((r.status, r.done), (RunStatus::Open, 1));
    let r = e
        .svc
        .tick_routine_step(e.h, run.id, run.steps[1].step, true, Some(e.kid1), None)
        .await
        .unwrap();
    assert_eq!(r.status, RunStatus::Done);
    let r = e
        .svc
        .tick_routine_step(e.h, run.id, run.steps[1].step, false, Some(e.kid1), None)
        .await
        .unwrap();
    assert_eq!(r.status, RunStatus::Open);
    // Tuesday: yesterday's open run is skipped and a new one made.
    let tue = t(2026, 6, 2, 6, 0);
    assert_eq!(e.svc.routines_materialize_all(tue).await.unwrap(), 1);
    let status: String = sqlx::query_scalar("SELECT status FROM routine_run WHERE id = $1")
        .bind(run.id.as_uuid())
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(status, "skipped");
    e.at(tue);
    assert!(matches!(
        e.svc
            .tick_routine_step(e.h, run.id, run.steps[0].step, true, None, None)
            .await,
        Err(TaskError::WrongStatus { .. })
    ));
});

test!(routines_school_days_skip_no_school, |e| {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let src = CalendarSource {
        id: CalendarSourceId::new(),
        household_id: e.h,
        kind: SourceKind::Ics,
        name: "School".into(),
        owner_member: Some(e.adult),
        oauth_account: None,
        url: None,
        username: None,
        secret_id: None,
        writable: false,
        poll_minutes: 15,
        state: SourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    CalendarSourceRepo::insert(&mut tx, &src).await.unwrap();
    let (cal, _) = CalendarRepo::upsert_discovered(&mut tx, src.id, "remote", "School", None)
        .await
        .unwrap();
    let cal = cal.value.id;
    let (from, to) = (d(2026, 6, 2), d(2026, 6, 3));
    EventRepo::upsert_synced(
        &mut tx,
        cal,
        &[NewOccurrence {
            remote_uid: "u1".into(),
            instance_key: String::new(),
            title: "No school - INSET day".into(),
            description: None,
            location: None,
            start_at: Utc.from_utc_datetime(&from.and_hms_opt(0, 0, 0).unwrap()),
            end_at: Utc.from_utc_datetime(&to.and_hms_opt(0, 0, 0).unwrap()),
            all_day: true,
            start_date: Some(from),
            end_date: Some(to),
            source_tz: None,
            is_private: false,
            cancelled: false,
            remote_etag: None,
            external_url: None,
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    routine(&e, |i| {
        i.day_filter = DayFilter::SchoolDays;
        i.school_calendar = Some(cal);
    })
    .await;
    assert_eq!(
        e.svc
            .routines_materialize_all(t(2026, 6, 1, 6, 0))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        e.svc
            .routines_materialize_all(t(2026, 6, 2, 6, 0))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        e.svc
            .routines_materialize_all(t(2026, 6, 3, 6, 0))
            .await
            .unwrap(),
        1
    );
});

// ---- producers ----

test!(assigned_notifications_skip_the_creator, |e| {
    e.manual(|n| n.assignees = vec![e.kid1, e.adult]).await;
    assert_eq!(e.count("assigned").await, 1);
    e.def(|_| {}).await;
    e.materialize(t(2026, 3, 1, 9, 0)).await;
    assert_eq!(e.count("assigned").await, 2);
    let title: String = sqlx::query_scalar(
        "SELECT title FROM notification WHERE trigger = 'assigned' ORDER BY created_at LIMIT 1",
    )
    .fetch_one(&e.pool)
    .await
    .unwrap();
    assert_eq!(title, "New task: Task");
});

test!(overdue_once_and_due_soon, |e| {
    e.prefs(e.kid1, &["overdue", "due_soon"]).await;
    let now = t(2026, 3, 1, 12, 0);
    e.at(now);
    e.manual(|n| {
        n.title = "Late".into();
        n.assignees = vec![e.kid1];
        n.due_at = Some(now - Duration::hours(1));
    })
    .await;
    e.manual(|n| {
        n.title = "Soon".into();
        n.assignees = vec![e.kid1];
        n.due_at = Some(now + Duration::minutes(60));
    })
    .await;
    e.manual(|n| {
        n.title = "Later".into();
        n.assignees = vec![e.kid1];
        n.due_at = Some(now + Duration::hours(5));
    })
    .await;
    e.svc.tick_all(now).await.unwrap();
    e.svc.tick_all(now + Duration::minutes(1)).await.unwrap();
    assert_eq!(e.count("overdue").await, 1);
    assert_eq!(e.count("due_soon").await, 1);
    let title: String =
        sqlx::query_scalar("SELECT title FROM notification WHERE trigger = 'overdue'")
            .fetch_one(&e.pool)
            .await
            .unwrap();
    assert_eq!(title, "Overdue: Late");
});

test!(routine_nudge, |e| {
    e.prefs(e.kid1, &["routine_nudge"]).await;
    routine(&e, |_| {}).await;
    // Monday 2026-06-01, BST: 08:15 local is 07:15 UTC; nudge window opens 08:05 local.
    e.svc
        .routines_materialize_all(t(2026, 6, 1, 5, 0))
        .await
        .unwrap();
    e.svc.tick_all(t(2026, 6, 1, 6, 59)).await.unwrap();
    assert_eq!(e.count("routine_nudge").await, 0);
    e.svc.tick_all(t(2026, 6, 1, 7, 6)).await.unwrap();
    e.svc.tick_all(t(2026, 6, 1, 7, 8)).await.unwrap();
    assert_eq!(e.count("routine_nudge").await, 1);
    let title: String =
        sqlx::query_scalar("SELECT title FROM notification WHERE trigger = 'routine_nudge'")
            .fetch_one(&e.pool)
            .await
            .unwrap();
    assert_eq!(title, "Routine: 2 steps left");
});

test!(digest_once_per_day_after_quiet_hours, |e| {
    e.prefs(e.kid1, &["digest"]).await;
    e.prefs(e.adult, &["digest"]).await;
    // 2026-03-02 is GMT: local equals UTC.
    let due = t(2026, 3, 2, 17, 0);
    e.at(t(2026, 3, 2, 5, 0));
    e.manual(|n| {
        n.title = "Homework".into();
        n.assignees = vec![e.kid1];
        n.due_at = Some(due);
    })
    .await;
    e.manual(|n| {
        n.title = "Tomorrow".into();
        n.assignees = vec![e.kid1];
        n.due_at = Some(due + Duration::days(1));
    })
    .await;
    e.svc.digest_all(t(2026, 3, 2, 6, 0)).await.unwrap();
    assert_eq!(e.count("digest").await, 0);
    e.svc.digest_all(t(2026, 3, 2, 7, 15)).await.unwrap();
    e.svc.digest_all(t(2026, 3, 2, 7, 30)).await.unwrap();
    assert_eq!(e.count("digest").await, 1);
    let (title, body): (String, String) =
        sqlx::query_as("SELECT title, body FROM notification WHERE trigger = 'digest'")
            .fetch_one(&e.pool)
            .await
            .unwrap();
    assert_eq!(title, "Good morning: 1 task today");
    assert_eq!(body, "Homework");
    // With everything done, the next day has nothing to list, so no digest.
    for task in e.tasks(StatusFilter::Open).await {
        e.svc
            .complete(e.h, task.id, Some(e.kid1), None)
            .await
            .unwrap();
    }
    e.svc.digest_all(t(2026, 3, 4, 8, 0)).await.unwrap();
    assert_eq!(e.count("digest").await, 1);
});

// ---- weather ----

async fn rule(e: &Env, def: Option<TaskDefinitionId>, mode: TaskMode) -> WeatherRuleId {
    let id = WeatherRuleId::new();
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    WeatherRuleRepo::insert(
        &mut tx,
        &WeatherRule {
            id,
            household_id: e.h,
            name: "Frost".into(),
            condition: "min(temp_c, today) < 0".into(),
            action: RuleAction::Task,
            severity: Severity::Info,
            message: "m".into(),
            task_definition: def.map(|d| serde_json::json!({ "definition": d })),
            task_mode: mode,
            status: RuleStatus::Active,
            suggested_by: SuggestedBy::User,
            archived_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    id
}

test!(from_weather_is_idempotent_and_suggests, |e| {
    let def = e.def(|i| i.due_time = Some(tm(9, 0))).await;
    let create = rule(&e, Some(def), TaskMode::Create).await;
    assert!(
        e.svc
            .from_weather(e.h, create, d(2026, 1, 5))
            .await
            .unwrap()
    );
    assert!(
        !e.svc
            .from_weather(e.h, create, d(2026, 1, 5))
            .await
            .unwrap()
    );
    assert!(
        e.svc
            .from_weather(e.h, create, d(2026, 1, 6))
            .await
            .unwrap()
    );
    let open = e.tasks(StatusFilter::Open).await;
    assert_eq!(open.len(), 2);
    assert!(
        open.iter()
            .all(|x| x.origin == TaskOrigin::Weather && x.definition.is_none())
    );
    assert!(open.iter().any(|x| x.due_at == Some(t(2026, 1, 5, 9, 0))));
    assert_eq!(open[0].assignees, vec![e.kid1]);

    let suggest = rule(&e, Some(def), TaskMode::Suggest).await;
    assert!(
        e.svc
            .from_weather(e.h, suggest, d(2026, 2, 1))
            .await
            .unwrap()
    );
    let sug = e.tasks(StatusFilter::Suggested).await;
    assert_eq!(sug.len(), 1);
    assert_eq!(e.tasks(StatusFilter::Open).await.len(), 2);
    let accepted = e.svc.accept(e.h, sug[0].id).await.unwrap();
    assert_eq!(accepted.status, TaskStatus::Open);

    let none = rule(&e, None, TaskMode::Create).await;
    assert!(!e.svc.from_weather(e.h, none, d(2026, 2, 1)).await.unwrap());
});

// ---- M3: email and bills ----

fn mail_req(e: &Env, msg: MessageId, title: &str, mode: MailTaskMode) -> MailTaskRequest {
    MailTaskRequest {
        household: Some(e.h),
        message: msg,
        kind: Some("action".into()),
        title: title.into(),
        due_at: Some(t(2026, 3, 10, 12, 0)),
        assignees: vec![e.adult, e.kid1],
        mode,
        external_url: Some("https://mail.example.com/m/1".into()),
    }
}

fn new_bill(e: &Env, due_on: Option<NaiveDate>, status: BillStatus) -> Bill {
    Bill {
        id: BillId::new(),
        household_id: e.h,
        message_id: None,
        org_id: None,
        payee: "Example Power".into(),
        amount_minor: 4250,
        currency: "USD".into(),
        due_on,
        account_hint: None,
        statement_period: None,
        status,
        confidence: 0.9,
        task_id: None,
        source_ref: serde_json::json!({}),
        dedupe_key: format!("bill:{}", BillId::new()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

async fn put_bill(e: &Env, b: &Bill) {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    BillRepo::insert_if_new(&mut tx, b).await.unwrap().unwrap();
    tx.commit().await.unwrap();
}

async fn get_bill(e: &Env, id: BillId) -> Bill {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    BillRepo::get(&mut tx, id).await.unwrap().unwrap().value
}

test!(from_mail_suggest_and_create, |e| {
    let msg = MessageId::new();
    let id = e
        .svc
        .from_mail(mail_req(&e, msg, "Sign the form", MailTaskMode::Suggest))
        .await
        .unwrap()
        .unwrap();
    let sug = e.tasks(StatusFilter::Suggested).await;
    assert_eq!(sug.len(), 1);
    assert_eq!(sug[0].id, id);
    assert_eq!(sug[0].origin, TaskOrigin::Email);
    assert_eq!(sug[0].list_name, "Email");
    assert_eq!(sug[0].due_at, Some(t(2026, 3, 10, 12, 0)));
    // Approval rows go to the adult only; no assigned rows yet.
    assert_eq!(e.count("approval_needed").await, 1);
    assert_eq!(e.count("assigned").await, 0);
    let src: serde_json::Value = sqlx::query_scalar("SELECT source_ref FROM task WHERE id = $1")
        .bind(id.as_uuid())
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(src["kind"], "email");
    assert_eq!(src["message_id"], msg.to_string());
    assert_eq!(src["external_url"], "https://mail.example.com/m/1");

    // A second message in create mode: open, with assignees and assigned rows.
    let id2 = e
        .svc
        .from_mail(mail_req(
            &e,
            MessageId::new(),
            "Reply to coach",
            MailTaskMode::Create,
        ))
        .await
        .unwrap()
        .unwrap();
    let open = e.tasks(StatusFilter::Open).await;
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].id, id2);
    assert_eq!(e.count("assigned").await, 2);
    assert_eq!(e.count("approval_needed").await, 1);
    // The Email list is reused.
    let lists: i64 = sqlx::query_scalar("SELECT count(*) FROM task_list WHERE kind = 'email'")
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(lists, 1);

    // Accepting the suggestion opens it.
    assert_eq!(
        e.svc.accept(e.h, id).await.unwrap().status,
        TaskStatus::Open
    );
});

test!(from_mail_is_idempotent, |e| {
    let msg = MessageId::new();
    assert!(
        e.svc
            .from_mail(mail_req(&e, msg, "Pack lunch", MailTaskMode::Suggest))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        e.svc
            .from_mail(mail_req(&e, msg, "Pack lunch", MailTaskMode::Suggest))
            .await
            .unwrap()
            .is_none()
    );
    // Another title on the same message is a different task.
    assert!(
        e.svc
            .from_mail(mail_req(&e, msg, "Bring money", MailTaskMode::Suggest))
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(e.tasks(StatusFilter::Suggested).await.len(), 2);
    assert_eq!(e.count("approval_needed").await, 2);
});

test!(from_bill_due_date_in_household_tz_and_noops, |e| {
    // Lead 3 days: due 2026-04-10 -> 2026-04-07 09:00 BST = 08:00 UTC.
    let b = new_bill(&e, Some(d(2026, 4, 10)), BillStatus::Open);
    put_bill(&e, &b).await;
    let id = e.svc.from_bill(Some(e.h), b.id).await.unwrap().unwrap();
    let open = e.tasks(StatusFilter::Open).await;
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].id, id);
    assert_eq!(open[0].title, "Pay Example Power");
    assert_eq!(open[0].list_name, "Bills");
    assert_eq!(open[0].due_at, Some(t(2026, 4, 7, 8, 0)));
    assert_eq!(open[0].assignees.len(), 1);
    assert_eq!(open[0].assignees[0], e.adult);
    assert_eq!(get_bill(&e, b.id).await.task_id, Some(id));

    // Idempotent: the task id is set.
    assert!(e.svc.from_bill(Some(e.h), b.id).await.unwrap().is_none());
    assert_eq!(e.tasks(StatusFilter::Open).await.len(), 1);

    // Not open: no task.
    let p = new_bill(&e, Some(d(2026, 4, 10)), BillStatus::Proposed);
    put_bill(&e, &p).await;
    assert!(e.svc.from_bill(Some(e.h), p.id).await.unwrap().is_none());
    assert!(get_bill(&e, p.id).await.task_id.is_none());

    // No due date: a task without a due instant; household found from the bill alone.
    let n = new_bill(&e, None, BillStatus::Open);
    put_bill(&e, &n).await;
    let nid = e.svc.from_bill(None, n.id).await.unwrap().unwrap();
    let all = e.tasks(StatusFilter::Open).await;
    let task = all.iter().find(|x| x.id == nid).unwrap();
    assert_eq!(task.due_at, None);

    // The Bills list is shared.
    let lists: i64 = sqlx::query_scalar("SELECT count(*) FROM task_list WHERE name = 'Bills'")
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(lists, 1);
});

test!(completing_and_reopening_pay_task_flips_bill, |e| {
    let b = new_bill(&e, Some(d(2026, 4, 10)), BillStatus::Open);
    put_bill(&e, &b).await;
    let id = e.svc.from_bill(Some(e.h), b.id).await.unwrap().unwrap();
    e.svc.complete(e.h, id, Some(e.adult), None).await.unwrap();
    assert_eq!(get_bill(&e, b.id).await.status, BillStatus::Paid);
    e.svc.reopen(e.h, id).await.unwrap();
    assert_eq!(get_bill(&e, b.id).await.status, BillStatus::Open);
    // A dismissed bill stays dismissed when its task is completed.
    {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        BillRepo::set_status(&mut tx, b.id, BillStatus::Dismissed)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    e.svc.complete(e.h, id, Some(e.adult), None).await.unwrap();
    assert_eq!(get_bill(&e, b.id).await.status, BillStatus::Dismissed);
});

test!(digest_mentions_items_waiting_for_approval, |e| {
    e.prefs(e.adult, &["digest"]).await;
    let now = t(2026, 3, 1, 8, 0);
    // Nothing at all: no digest.
    e.svc.digest(e.h, now).await.unwrap();
    assert_eq!(e.count("digest").await, 0);

    e.svc
        .from_mail(mail_req(
            &e,
            MessageId::new(),
            "Sign the form",
            MailTaskMode::Suggest,
        ))
        .await
        .unwrap();
    e.svc.digest(e.h, now).await.unwrap();
    assert_eq!(e.count("digest").await, 1);
    let body: String = sqlx::query_scalar("SELECT body FROM notification WHERE trigger = 'digest'")
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert!(body.contains("1 items waiting for approval"), "{body}");
});

test!(digest_skips_approval_line_when_mode_off, |e| {
    {
        let mut tx = e.store.tx_for(e.h).await.unwrap();
        let mut p = NotifyPref {
            id: uuid::Uuid::now_v7(),
            household_id: e.h,
            member_id: e.adult,
            channel: "ntfy".into(),
            topic: "topicoff".into(),
            triggers: vec!["digest".into()],
            due_soon_minutes: 120,
            nudge_minutes: 10,
            quiet_from: None,
            quiet_until: None,
            copy_to_member_id: None,
            approval_mode: ApprovalMode::Off,
            archived_at: None,
        };
        p.triggers = vec!["digest".into()];
        NotifyPrefRepo::upsert_for_member(&mut tx, &p, None)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    e.svc
        .from_mail(mail_req(
            &e,
            MessageId::new(),
            "Sign the form",
            MailTaskMode::Suggest,
        ))
        .await
        .unwrap();
    e.svc.digest(e.h, t(2026, 3, 1, 8, 0)).await.unwrap();
    assert_eq!(e.count("digest").await, 0);
});

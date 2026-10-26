//! M3: extracted events. Routing, the approval gate, human decisions, and writing to or deleting
//! from the provider. See arch/Ingest.md sections 8 and 10.

use chrono::{Duration, Utc};
use fa_connectors::{ConnectorError, EventWrite, caldav::CaldavClient, google::calendar, ics};
use fa_domain::mail_api::{EventApproval, EventProposalView};
use fa_domain::{
    CalendarId, DecisionPointKey, EventId, EventOrigin, EventStatus, HouseholdId, MemberId,
    NotifyTrigger, Subject,
};
use fa_pipeline::{Candidate, Evaluator, Pipeline};
use fa_store::{
    CalendarRepo, CalendarSourceRepo, ChangeBus, EventFull, EventRepo, JobRepo, NewJob,
    NotificationRepo, OAuthAccountRepo, OrgRepo, Tx,
};
use serde_json::json;

use crate::error::{CalendarError, Result, scrub};
use crate::remote::Creds;
use crate::service::{CalendarService, household_of, publish_calendar_updated, tz_of};

const SCOPE_EVENTS: &str = "https://www.googleapis.com/auth/calendar.events";
const SCOPE_FULL: &str = "https://www.googleapis.com/auth/calendar";
const RECONNECT: &str = "reconnect with calendar write access";
const SIMILARITY: f64 = 0.85;

fn words(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Same normalisation as `fa_rules::title_similarity` (fa-calendar does not depend on fa-rules).
fn title_similarity(a: &str, b: &str) -> f64 {
    strsim::jaro_winkler(&words(a), &words(b))
}

fn proposal_view(e: EventFull, from_label: Option<String>) -> EventProposalView {
    EventProposalView {
        id: e.event.id,
        title: e.event.title,
        start_at: e.event.start_at,
        end_at: e.event.end_at,
        all_day: e.event.all_day,
        location: e.event.location,
        confidence: e.event.confidence.unwrap_or(0.0),
        suggested_calendar: e.target_calendar_id,
        route_confidence: e.route_confidence,
        pending_cancel: e.pending_cancel,
        members: e.members,
        message: e.message_id,
        from_label,
        external_url: e
            .event
            .source_ref
            .as_ref()
            .and_then(|s| s.get("external_url"))
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        write_error: e.write_error,
        version: e.version,
    }
}

fn same_event(a: &EventFull, b: &EventFull) -> bool {
    let (x, y) = (&a.event, &b.event);
    if !x.dedupe_key.is_empty() && x.dedupe_key == y.dedupe_key {
        return true;
    }
    if x.remote_uid.is_some() && x.remote_uid == y.remote_uid {
        return true;
    }
    let close = match (x.all_day, y.all_day) {
        (true, true) => match (x.start_date, y.start_date) {
            (Some(p), Some(q)) => p == q,
            _ => x.start_at == y.start_at,
        },
        (false, false) => (x.start_at - y.start_at).abs() <= Duration::minutes(60),
        _ => false,
    };
    close && title_similarity(&x.title, &y.title) >= SIMILARITY
}

struct RouteEval(Vec<Candidate>);

impl Evaluator for RouteEval {
    fn candidates(&self, _: DecisionPointKey) -> Vec<Candidate> {
        self.0.clone()
    }
}

fn cand(id: CalendarId, confidence: f32, source: &str) -> Candidate {
    Candidate {
        value: json!(id),
        confidence,
        source: source.into(),
    }
}

async fn publish(tx: &mut Tx, h: HouseholdId, kind: &str) -> Result<()> {
    ChangeBus::publish(tx, h, kind, &json!({})).await?;
    Ok(())
}

async fn enqueue(
    tx: &mut Tx,
    kind: &str,
    h: HouseholdId,
    event: EventId,
    key: String,
) -> Result<()> {
    let mut job = NewJob::new(kind);
    job.household_id = Some(h);
    job.payload = json!({"household": h, "event": event});
    job.idempotency_key = Some(key);
    JobRepo::enqueue(tx, &job).await?;
    Ok(())
}

async fn enqueue_write(tx: &mut Tx, h: HouseholdId, event: EventId, delete: bool) -> Result<()> {
    let version = EventRepo::get_full(tx, event)
        .await?
        .map(|e| e.version)
        .unwrap_or_default();
    let kind = if delete {
        "calendar.delete"
    } else {
        "calendar.write"
    };
    enqueue(tx, kind, h, event, format!("{kind}:{event}:{version}")).await
}

async fn notify(tx: &mut Tx, event: &EventFull, title: &str, key: &str) -> Result<()> {
    NotificationRepo::insert_for_adults(
        tx,
        NotifyTrigger::ApprovalNeeded,
        "event",
        Some(event.event.id.as_uuid()),
        title,
        &event.event.title,
        Some("/inbox"),
        key,
    )
    .await?;
    Ok(())
}

/// A failure that retrying cannot fix is recorded and ends the job; others are retried.
fn classify(e: ConnectorError, secrets: &[&str]) -> (String, bool) {
    match e {
        ConnectorError::InsufficientScope => (RECONNECT.into(), true),
        ConnectorError::Unauthorized => (
            "the credentials were rejected; re-connect the source".into(),
            true,
        ),
        other => (scrub(&other.to_string(), secrets), false),
    }
}

/// What a write or delete needs, loaded in one transaction.
struct Target {
    event: EventFull,
    household: fa_domain::Household,
    calendar: fa_domain::Calendar,
    creds: Creds,
}

impl CalendarService {
    // ---- routing ----

    pub async fn route_event(&self, household: HouseholdId, id: EventId) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        let Some(ex) = EventRepo::get_full(&mut tx, id).await? else {
            return Ok(());
        };
        if ex.event.origin != EventOrigin::Extracted
            || ex.event.status != Some(EventStatus::Extracted)
        {
            return Ok(());
        }
        let hh = household_of(&mut tx, household).await?;
        let start = ex.event.start_at;
        let others: Vec<EventFull> = EventRepo::dedupe_candidates(
            &mut tx,
            household,
            start - Duration::days(1),
            start + Duration::days(1),
        )
        .await?
        .into_iter()
        .filter(|c| c.event.id != id)
        .collect();

        if ex.event.cancelled {
            let hit = others.iter().find(|c| {
                c.event.origin == EventOrigin::Extracted
                    && !c.event.cancelled
                    && matches!(
                        c.event.status,
                        Some(EventStatus::Proposed | EventStatus::Written)
                    )
                    && same_event(&ex, c)
            });
            if let Some(m) = hit {
                if let Some(msg) = ex.message_id {
                    EventRepo::link_message(&mut tx, m.event.id, msg).await?;
                }
                match m.event.status {
                    Some(EventStatus::Proposed) => {
                        EventRepo::set_status(&mut tx, m.event.id, EventStatus::Expired, None)
                            .await?;
                        publish(&mut tx, household, "inbox.updated").await?;
                    }
                    _ if hh.events.auto_write => {
                        enqueue_write(&mut tx, household, m.event.id, true).await?;
                    }
                    _ if !m.pending_cancel => {
                        EventRepo::set_pending_cancel(&mut tx, m.event.id, true).await?;
                        notify(
                            &mut tx,
                            m,
                            "Event cancelled by email",
                            &format!("approval:event:{}:cancel", m.event.id.as_uuid()),
                        )
                        .await?;
                        publish(&mut tx, household, "inbox.updated").await?;
                    }
                    _ => {}
                }
            }
            EventRepo::delete(&mut tx, id).await?;
            tx.commit().await?;
            return Ok(());
        }

        if let Some(m) = others
            .iter()
            .find(|c| !c.event.cancelled && same_event(&ex, c))
        {
            if let Some(msg) = ex.message_id {
                EventRepo::link_message(&mut tx, m.event.id, msg).await?;
            }
            EventRepo::delete(&mut tx, id).await?;
            tx.commit().await?;
            return Ok(());
        }

        // Route.
        let writable = CalendarRepo::writable_targets(&mut tx).await?;
        let mut cands = Vec::new();
        if let Some(org) = ex.org_id
            && let Some(o) = OrgRepo::get(&mut tx, org).await?
            && let Some(c) = o.value.default_calendar_id
            && writable.iter().any(|w| w.0 == c)
        {
            cands.push(cand(c, 0.95, "org.default_calendar"));
        }
        let member_defaults: Vec<CalendarId> = writable
            .iter()
            .filter(|w| w.2 && w.3.iter().any(|m| ex.members.contains(m)))
            .map(|w| w.0)
            .collect();
        if let [only] = member_defaults.as_slice() {
            cands.push(cand(*only, 0.9, "member.default_calendar"));
        }
        if let Some(w) = writable.iter().find(|w| w.2 && w.3.is_empty()) {
            cands.push(cand(w.0, 0.7, "household.default_calendar"));
        }
        Pipeline::ensure_defaults(&mut tx, household)
            .await
            .map_err(|e| CalendarError::Invalid(e.to_string()))?;
        // The best candidate pre-fills the proposal even when the pipeline did not accept it.
        let suggestion = cands
            .iter()
            .max_by(|a, b| a.confidence.total_cmp(&b.confidence))
            .and_then(|c| {
                serde_json::from_value::<CalendarId>(c.value.clone())
                    .ok()
                    .map(|id| (id, c.confidence))
            });
        let out = Pipeline::run(
            &mut tx,
            household,
            DecisionPointKey::EventCalendarRoute,
            Subject::event(id),
            &RouteEval(cands),
        )
        .await
        .map_err(|e| CalendarError::Invalid(e.to_string()))?;
        let routed = out.values.first().filter(|_| out.is_final).and_then(|c| {
            serde_json::from_value::<CalendarId>(c.value.clone())
                .ok()
                .filter(|id| writable.iter().any(|w| w.0 == *id))
                .map(|id| (id, c.confidence))
        });
        let shown = routed.or(suggestion);
        EventRepo::set_route(&mut tx, id, shown.map(|r| r.0), shown.map(|r| r.1)).await?;

        let auto = hh.events.auto_write
            && ex.event.confidence.unwrap_or(0.0) >= hh.events.extract_threshold
            && routed.is_some_and(|r| r.1 >= hh.events.route_threshold);
        if auto {
            EventRepo::set_status(&mut tx, id, EventStatus::Approved, None).await?;
            enqueue_write(&mut tx, household, id, false).await?;
        } else {
            EventRepo::set_status(&mut tx, id, EventStatus::Proposed, None).await?;
            notify(
                &mut tx,
                &ex,
                "Event suggested from email",
                &format!("approval:event:{}", id.as_uuid()),
            )
            .await?;
            publish(&mut tx, household, "inbox.updated").await?;
        }
        tx.commit().await?;
        Ok(())
    }

    // ---- proposals and decisions ----

    /// Events waiting for a decision, including approved ones whose write to the provider failed.
    pub async fn proposals(&self, household: HouseholdId) -> Result<Vec<EventProposalView>> {
        let mut tx = self.store.tx_for(household).await?;
        let mut rows: Vec<(EventFull, Option<String>)> = EventRepo::proposals(&mut tx)
            .await?
            .into_iter()
            .map(|p| (p.event, p.message_from))
            .collect();
        rows.sort_by_key(|(e, _)| (e.event.start_at, e.event.id.as_uuid()));
        Ok(rows
            .into_iter()
            .map(|(e, from)| proposal_view(e, from))
            .collect())
    }

    /// Approved events whose provider write failed: the inbox shows the error and lets the adult
    /// approve again (retry) or reject.
    pub async fn failed_writes(&self, household: HouseholdId) -> Result<Vec<EventProposalView>> {
        let mut tx = self.store.tx_for(household).await?;
        Ok(Self::failed(&mut tx)
            .await?
            .into_iter()
            .map(|(e, from)| proposal_view(e, from))
            .collect())
    }

    async fn failed(tx: &mut Tx) -> Result<Vec<(EventFull, Option<String>)>> {
        let mut out = Vec::new();
        for (id, from) in EventRepo::failed_writes(tx).await? {
            if let Some(e) = EventRepo::get_full(tx, id).await? {
                out.push((e, from));
            }
        }
        Ok(out)
    }

    async fn pending(tx: &mut Tx, id: EventId, cancel: bool) -> Result<EventFull> {
        let e = EventRepo::get_full(tx, id)
            .await?
            .ok_or(CalendarError::NotFound)?;
        // An approved event whose write failed may be approved again (retry) or rejected.
        let ok = e.event.origin == EventOrigin::Extracted
            && if cancel {
                e.event.status == Some(EventStatus::Written) && e.pending_cancel
            } else {
                e.event.status == Some(EventStatus::Proposed)
                    || (e.event.status == Some(EventStatus::Approved) && e.write_error.is_some())
            };
        if ok {
            Ok(e)
        } else {
            Err(CalendarError::State(
                "the event is not awaiting a decision".into(),
            ))
        }
    }

    pub async fn approve_event(
        &self,
        household: HouseholdId,
        id: EventId,
        a: EventApproval,
        by: Option<MemberId>,
    ) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        let e = Self::pending(&mut tx, id, false).await?;
        let hh = household_of(&mut tx, household).await?;
        let cal = a
            .calendar
            .or(e.target_calendar_id)
            .ok_or_else(|| CalendarError::Invalid("choose a calendar".into()))?;
        if !CalendarRepo::writable_targets(&mut tx)
            .await?
            .iter()
            .any(|w| w.0 == cal)
        {
            return Err(CalendarError::Invalid(
                "the calendar is not writable".into(),
            ));
        }
        let title = match a.title {
            Some(t) if !t.trim().is_empty() => t.trim().to_owned(),
            Some(_) => return Err(CalendarError::Invalid("the title is empty".into())),
            None => e.event.title.clone(),
        };
        let start_at = a.start_at.unwrap_or(e.event.start_at);
        let end_at = a.end_at.unwrap_or(e.event.end_at);
        if end_at < start_at {
            return Err(CalendarError::Invalid(
                "the event ends before it starts".into(),
            ));
        }
        let all_day = a.all_day.unwrap_or(e.event.all_day);
        let tz = tz_of(&hh);
        let (start_date, end_date) = if all_day {
            let s = start_at.with_timezone(&tz).date_naive();
            let en = end_at
                .with_timezone(&tz)
                .date_naive()
                .max(s + chrono::Days::new(1));
            (Some(s), Some(en))
        } else {
            (None, None)
        };
        EventRepo::apply_edit(
            &mut tx, id, &title, start_at, end_at, all_day, start_date, end_date,
        )
        .await?;
        if let Some(m) = &a.members {
            EventRepo::replace_members(&mut tx, household, id, m).await?;
        }
        EventRepo::set_route(&mut tx, id, Some(cal), e.route_confidence).await?;
        Pipeline::record_human(
            &mut tx,
            household,
            DecisionPointKey::EventCalendarRoute,
            Subject::event(id),
            json!(cal),
            by,
        )
        .await
        .map_err(|e| CalendarError::Invalid(e.to_string()))?;
        if e.write_error.is_some() {
            EventRepo::set_write_error(&mut tx, id, None).await?;
        }
        EventRepo::set_status(&mut tx, id, EventStatus::Approved, by).await?;
        enqueue_write(&mut tx, household, id, false).await?;
        publish(&mut tx, household, "inbox.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn reject_event(
        &self,
        household: HouseholdId,
        id: EventId,
        by: Option<MemberId>,
    ) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        Self::pending(&mut tx, id, false).await?;
        Pipeline::record_human(
            &mut tx,
            household,
            DecisionPointKey::EventCalendarRoute,
            Subject::event(id),
            json!("reject"),
            by,
        )
        .await
        .map_err(|e| CalendarError::Invalid(e.to_string()))?;
        EventRepo::set_status(&mut tx, id, EventStatus::Rejected, by).await?;
        publish(&mut tx, household, "inbox.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    /// Approves deleting an event we wrote.
    pub async fn approve_cancel(
        &self,
        household: HouseholdId,
        id: EventId,
        by: Option<MemberId>,
    ) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        Self::pending(&mut tx, id, true).await?;
        EventRepo::set_status(&mut tx, id, EventStatus::Written, by).await?;
        enqueue_write(&mut tx, household, id, true).await?;
        publish(&mut tx, household, "inbox.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    /// Keeps the event.
    pub async fn reject_cancel(
        &self,
        household: HouseholdId,
        id: EventId,
        by: Option<MemberId>,
    ) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        Self::pending(&mut tx, id, true).await?;
        EventRepo::set_pending_cancel(&mut tx, id, false).await?;
        EventRepo::set_status(&mut tx, id, EventStatus::Written, by).await?;
        publish(&mut tx, household, "inbox.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- provider writes ----

    async fn load_target(
        &self,
        household: HouseholdId,
        id: EventId,
        deleting: bool,
    ) -> Result<std::result::Result<Target, ()>> {
        let mut tx = self.store.tx_for(household).await?;
        let Some(event) = EventRepo::get_full(&mut tx, id).await? else {
            return Ok(Err(()));
        };
        let wanted = if deleting {
            EventStatus::Written
        } else {
            EventStatus::Approved
        };
        if event.event.origin != EventOrigin::Extracted || event.event.status != Some(wanted) {
            return Ok(Err(()));
        }
        let fail = |msg: &str| CalendarError::Invalid(msg.into());
        let result: Result<Target> = async {
            let cal_id = if deleting {
                event.event.calendar
            } else {
                event.target_calendar_id
            }
            .ok_or_else(|| fail("the event has no calendar"))?;
            let calendar = CalendarRepo::get(&mut tx, cal_id)
                .await?
                .ok_or_else(|| fail("the calendar is gone"))?
                .value;
            let src = CalendarSourceRepo::get(&mut tx, calendar.source)
                .await?
                .ok_or_else(|| fail("the source is gone"))?
                .value;
            if !src.writable || src.archived_at.is_some() {
                return Err(fail("the calendar is not writable"));
            }
            if let Some(acc) = src.oauth_account {
                let scopes = OAuthAccountRepo::get(&mut tx, acc)
                    .await?
                    .map(|a| a.value.scopes)
                    .unwrap_or_default();
                if !scopes.iter().any(|s| s == SCOPE_EVENTS || s == SCOPE_FULL) {
                    return Err(CalendarError::Remote(RECONNECT.into()));
                }
            }
            let creds = self.creds_of(&mut tx, &src).await?;
            if matches!(creds, Creds::Ics { .. }) {
                return Err(fail("the calendar is not writable"));
            }
            let household = household_of(&mut tx, household).await?;
            Ok(Target {
                event: event.clone(),
                household,
                calendar,
                creds,
            })
        }
        .await;
        match result {
            Ok(t) => Ok(Ok(t)),
            Err(e) => {
                // Permanent: record on the row and finish.
                EventRepo::set_write_error(&mut tx, id, Some(&scrub(&e.to_string(), &[]))).await?;
                tx.commit().await?;
                Ok(Err(()))
            }
        }
    }

    async fn record_failure(&self, household: HouseholdId, id: EventId, msg: &str) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        if EventRepo::get_full(&mut tx, id).await?.is_some() {
            EventRepo::set_write_error(&mut tx, id, Some(msg)).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn write_event(&self, household: HouseholdId, id: EventId) -> Result<()> {
        let Ok(t) = self.load_target(household, id, false).await? else {
            return Ok(());
        };
        let ev = &t.event.event;
        let w = EventWrite {
            summary: ev.title.clone(),
            description: ev.description.clone(),
            location: ev.location.clone(),
            start: ev.start_at,
            end: ev.end_at,
            all_day: ev.all_day,
            tz: t.household.tz.clone(),
        };
        let secrets = t.creds.secrets();
        let res: std::result::Result<_, ConnectorError> = match &t.creds {
            Creds::Google(acc) => {
                let token = self.remote.tokens.access_token(household, *acc).await?;
                calendar::insert_event(
                    &self.remote.http,
                    &self.remote.cfg.google_api_base,
                    &token,
                    &t.calendar.remote_id,
                    &w,
                )
                .await
                // The sync keys Google events by event id, so that is the stored remote uid.
                .map(|r| (r.id, r.etag, r.html_link, r.ical_uid))
            }
            Creds::Caldav { url, user, pass } => {
                let uid = format!("fa-{}@family-assistant", id.as_uuid());
                async {
                    let client = CaldavClient::new(self.remote.http.clone(), url, user, pass)?;
                    let body = ics::build_vevent(&uid, &w)?;
                    let etag = client
                        .put_event(&t.calendar.remote_id, &uid, &body, false, None)
                        .await?;
                    Ok((uid.clone(), etag, None, None))
                }
                .await
            }
            Creds::Ics { .. } => return Ok(()),
        };
        match res {
            Ok((uid, etag, link, ical_uid)) => {
                let mut tx = self.store.tx_for(household).await?;
                EventRepo::set_written(
                    &mut tx,
                    id,
                    t.calendar.id,
                    &uid,
                    etag.as_deref(),
                    link.as_deref(),
                )
                .await?;
                if let Some(u) = ical_uid {
                    EventRepo::merge_source_ref(&mut tx, id, &json!({"ical_uid": u})).await?;
                }
                publish_calendar_updated(&mut tx, &t.household, Utc::now()).await?;
                publish(&mut tx, household, "inbox.updated").await?;
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                let (msg, permanent) = classify(e, &secrets);
                self.record_failure(household, id, &msg).await?;
                if permanent {
                    Ok(())
                } else {
                    Err(CalendarError::Remote(msg))
                }
            }
        }
    }

    pub async fn delete_event(&self, household: HouseholdId, id: EventId) -> Result<()> {
        let Ok(t) = self.load_target(household, id, true).await? else {
            return Ok(());
        };
        let ev = &t.event.event;
        let secrets = t.creds.secrets();
        let res: std::result::Result<(), ConnectorError> = match &t.creds {
            Creds::Google(acc) => {
                let gid = ev.remote_uid.clone();
                let Some(gid) = gid else {
                    self.record_failure(household, id, "the provider event id is unknown")
                        .await?;
                    return Ok(());
                };
                let token = self.remote.tokens.access_token(household, *acc).await?;
                calendar::delete_event(
                    &self.remote.http,
                    &self.remote.cfg.google_api_base,
                    &token,
                    &t.calendar.remote_id,
                    &gid,
                )
                .await
            }
            Creds::Caldav { url, user, pass } => {
                let uid = ev
                    .remote_uid
                    .clone()
                    .unwrap_or_else(|| format!("fa-{}@family-assistant", id.as_uuid()));
                let href = format!("{}/{uid}.ics", t.calendar.remote_id.trim_end_matches('/'));
                async {
                    CaldavClient::new(self.remote.http.clone(), url, user, pass)?
                        .delete_event(&href, None)
                        .await
                }
                .await
            }
            Creds::Ics { .. } => return Ok(()),
        };
        match res {
            Ok(()) => {
                let mut tx = self.store.tx_for(household).await?;
                EventRepo::delete(&mut tx, id).await?;
                publish_calendar_updated(&mut tx, &t.household, Utc::now()).await?;
                publish(&mut tx, household, "inbox.updated").await?;
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                let (msg, permanent) = classify(e, &secrets);
                self.record_failure(household, id, &msg).await?;
                if permanent {
                    Ok(())
                } else {
                    Err(CalendarError::Remote(msg))
                }
            }
        }
    }
}

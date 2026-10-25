use std::collections::{HashMap, hash_map::Entry};

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use fa_domain::notify::{effective_quiet, in_quiet, quiet_end_after};
use fa_domain::{Household, MemberId, Notification, NotifyTrigger, Role};
use fa_store::{
    CalendarSourceRepo, HouseholdRepo, MemberRepo, NewNotification, NotificationRepo,
    NotifyPrefRepo,
};
use uuid::Uuid;

use crate::{NotifyError, NotifyService, NtfyMessage};

const BATCH_WINDOW_SECS: i64 = 60;
const MAX_ATTEMPTS: i32 = 5;
const PENDING_LIMIT: i64 = 500;

struct Group {
    topic: String,
    /// `(child display name, adult)` when the member's rows are copied.
    copy: Option<(String, MemberId)>,
    rows: Vec<Notification>,
}

fn tag(t: NotifyTrigger) -> &'static str {
    match t {
        NotifyTrigger::Assigned => "memo",
        NotifyTrigger::Overdue => "warning",
        NotifyTrigger::Digest => "sunrise",
        NotifyTrigger::DueSoon => "hourglass",
        NotifyTrigger::RoutineNudge => "alarm_clock",
        NotifyTrigger::SourceReauth => "key",
    }
}

impl NotifyService {
    /// One `notify.send` pass as of `now`, over every household.
    pub async fn run_send_at(&self, now: DateTime<Utc>) -> Result<(), NotifyError> {
        let mut tx = self.store.tx_system().await?;
        let households = HouseholdRepo::list(&mut tx).await?;
        tx.commit().await?;
        let mut first_err = None;
        for h in households {
            if let Err(e) = self.send_household(&h.value, now).await {
                tracing::warn!(error = %e, "notify.send failed for a household");
                first_err.get_or_insert(e);
            }
        }
        first_err.map_or(Ok(()), Err)
    }

    async fn send_household(&self, h: &Household, now: DateTime<Utc>) -> Result<(), NotifyError> {
        let tz: Tz = h.tz.parse().unwrap_or(Tz::UTC);
        let mut groups: Vec<(Uuid, Group)> = Vec::new();

        let mut tx = self.store.tx_for(h.id).await?;
        self.produce_reauth(&mut tx, h).await?;
        let rows = NotificationRepo::pending(&mut tx, now, PENDING_LIMIT).await?;
        let mut prefs = HashMap::new();
        for row in rows {
            let key = row.member_id.as_uuid();
            if let Entry::Vacant(e) = prefs.entry(key) {
                e.insert(NotifyPrefRepo::get_for_member(&mut tx, row.member_id).await?);
            }
            let Some(pref) = prefs.get(&key).and_then(|p| p.as_ref()).map(|p| &p.value) else {
                NotificationRepo::mark_dropped(&mut tx, row.id, "no preferences").await?;
                continue;
            };
            if !pref.triggers.iter().any(|t| t == row.trigger.as_str()) {
                NotificationRepo::mark_dropped(&mut tx, row.id, "trigger disabled").await?;
                continue;
            }
            let (from, until) = effective_quiet(
                pref.quiet_from,
                pref.quiet_until,
                h.quiet_from,
                h.quiet_until,
            );
            if in_quiet(now.with_timezone(&tz).time(), from, until) {
                let end = quiet_end_after(now, from, until, tz);
                NotificationRepo::defer(&mut tx, row.id, end).await?;
                continue;
            }
            if !self.configured() {
                NotificationRepo::mark_dropped(&mut tx, row.id, "not configured").await?;
                continue;
            }
            if row.created_at > now - Duration::seconds(BATCH_WINDOW_SECS) {
                continue;
            }
            if !groups.iter().any(|(k, _)| *k == key) {
                let copy = match pref.copy_to_member_id {
                    Some(adult) => {
                        let me = MemberRepo::get(&mut tx, row.member_id).await?;
                        let target = MemberRepo::get(&mut tx, adult).await?;
                        match (me, target) {
                            (Some(me), Some(t))
                                if me.value.role == Role::Child
                                    && t.value.role == Role::Adult
                                    && t.value.archived_at.is_none() =>
                            {
                                Some((me.value.display_name, adult))
                            }
                            _ => None,
                        }
                    }
                    None => None,
                };
                groups.push((
                    key,
                    Group {
                        topic: pref.topic.clone(),
                        copy,
                        rows: Vec::new(),
                    },
                ));
            }
            if let Some((_, g)) = groups.iter_mut().find(|(k, _)| *k == key) {
                g.rows.push(row);
            }
        }
        tx.commit().await?;

        for (_, g) in groups {
            self.deliver(h, g, now).await?;
        }
        Ok(())
    }

    async fn deliver(
        &self,
        h: &Household,
        g: Group,
        now: DateTime<Utc>,
    ) -> Result<(), NotifyError> {
        let msg = self.compose(&g.rows);
        let result = self.notifier.publish(&g.topic, &msg).await;
        let mut tx = self.store.tx_for(h.id).await?;
        match result {
            Ok(()) => {
                let ids: Vec<_> = g.rows.iter().map(|r| r.id).collect();
                NotificationRepo::mark_sent(&mut tx, &ids, now).await?;
                if let Some((name, adult)) = &g.copy {
                    for r in g.rows.iter().filter(|r| !r.dedupe_key.starts_with("copy:")) {
                        NotificationRepo::insert_dedupe(
                            &mut tx,
                            &NewNotification {
                                household_id: h.id,
                                member_id: *adult,
                                trigger: r.trigger,
                                subject_type: r.subject_type.clone(),
                                subject_id: r.subject_id,
                                title: format!("{name}: {}", r.title),
                                body: r.body.clone(),
                                click_path: r.click_path.clone(),
                                priority: r.priority,
                                dedupe_key: format!("copy:{}", r.id),
                                not_before: None,
                            },
                        )
                        .await?;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, rows = g.rows.len(), "ntfy publish failed");
                let err = e.to_string();
                for r in &g.rows {
                    let attempts = r.attempts + 1;
                    let next = now + Duration::minutes(1_i64 << attempts.min(20));
                    NotificationRepo::record_failure(
                        &mut tx,
                        r.id,
                        &err,
                        next,
                        attempts >= MAX_ATTEMPTS,
                    )
                    .await?;
                }
            }
        }
        tx.commit().await?;
        Ok(())
    }

    fn compose(&self, rows: &[Notification]) -> NtfyMessage {
        let top = rows
            .iter()
            .max_by_key(|r| (r.priority, std::cmp::Reverse(r.created_at)))
            .unwrap_or(&rows[0]);
        let priority = top.priority.clamp(1, 5) as u8;
        let tags = vec![tag(top.trigger).to_string()];
        if let [r] = rows {
            return NtfyMessage {
                title: r.title.clone(),
                body: r.body.clone(),
                priority,
                tags,
                click: r.click_path.as_deref().map(|p| self.click_url(p)),
            };
        }
        NtfyMessage {
            title: format!("{} updates", rows.len()),
            body: rows
                .iter()
                .map(|r| r.title.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            priority,
            tags,
            click: Some(self.click_url("/tasks")),
        }
    }

    /// A row for every adult for each calendar source that needs re-authorisation.
    async fn produce_reauth(
        &self,
        conn: &mut sqlx::PgConnection,
        h: &Household,
    ) -> Result<(), NotifyError> {
        let sources = CalendarSourceRepo::needing_reauth(conn).await?;
        if sources.is_empty() {
            return Ok(());
        }
        let adults = MemberRepo::adults(conn).await?;
        for (source, _owner, last) in sources {
            let stamp = last.map_or_else(|| "never".to_string(), |t| t.to_rfc3339());
            for adult in &adults {
                NotificationRepo::insert_dedupe(
                    conn,
                    &NewNotification {
                        household_id: h.id,
                        member_id: *adult,
                        trigger: NotifyTrigger::SourceReauth,
                        subject_type: "calendar_source".into(),
                        subject_id: Some(source.as_uuid()),
                        title: "A calendar needs to be reconnected".into(),
                        body: "Sign in again in Settings so it keeps syncing.".into(),
                        click_path: Some("/settings/calendars".into()),
                        priority: 3,
                        dedupe_key: format!("reauth:{source}:{stamp}:{adult}"),
                        not_before: None,
                    },
                )
                .await?;
            }
        }
        Ok(())
    }
}

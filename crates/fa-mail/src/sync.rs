//! Mail sync (arch/Ingest.md section 4): jobs `mail.sync_due` and `mail.sync{source}`.
//! Everything after the raw message is shared between the providers. Nothing here logs message
//! content, addresses or credentials: ids, counts and timings only.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use fa_domain::{
    AttachmentMeta, HouseholdId, MailProvider as ProviderKind, MailSource, MailSourceId,
    MailSourceState, MessageId, NotifyTrigger, SourceState,
};
use fa_jobs::{BoxFuture, JobContext, JobError, JobHandler};
use fa_store::{
    ChangeBus, JobRepo, MailFolderStateRepo, MailSourceRepo, MessageRepo, NewJob, NewMessage,
    NewNotification, NotificationRepo, OAuthAccountRepo, Tx, mail_defaults,
};

use crate::provider::{self, Delta, Fetched, MailProvider, RemoteRef};
use crate::sources::enqueue_sync_in;
use crate::{MailDeps, MailError, Result};

/// New messages fetched per job run; the rest is queued as a continuation.
pub const MAX_PER_RUN: usize = 2000;
const CHUNK: usize = 100;

pub(crate) fn job_err(e: MailError) -> JobError {
    match e {
        MailError::Store(s) => JobError::Store(s),
        other => JobError::Handler(other.to_string()),
    }
}

fn is_junk(labels: &[String]) -> bool {
    labels
        .iter()
        .any(|l| matches!(l.as_str(), "SPAM" | "TRASH" | "DRAFT"))
}

fn is_spam_or_trash(labels: &[String]) -> bool {
    labels
        .iter()
        .any(|l| matches!(l.as_str(), "SPAM" | "TRASH"))
}

pub struct SyncService {
    deps: Arc<MailDeps>,
}

impl SyncService {
    pub fn new(deps: Arc<MailDeps>) -> Self {
        Self { deps }
    }

    /// Enqueues `mail.sync{source}` for every due source. Returns how many jobs were added.
    pub async fn enqueue_due(&self, now: DateTime<Utc>) -> Result<usize> {
        let mut tx = self.deps.store.tx_system().await?;
        let due = MailSourceRepo::due_with_poll(&mut tx, now).await?;
        let mut n = 0;
        for (h, s, poll) in due {
            if enqueue_sync_in(&mut tx, h, s, poll, now).await? {
                n += 1;
            }
        }
        tx.commit().await?;
        Ok(n)
    }

    /// One sync run of a source. The source's state and `last_error` are always updated:
    /// auth failures pause it as `needs_reauth` (and notify the owners), transient errors are
    /// returned so the job retries.
    pub async fn sync_source(&self, household: HouseholdId, id: MailSourceId) -> Result<()> {
        let src = {
            let mut tx = self.deps.store.tx_for(household).await?;
            mail_defaults::ensure(&mut tx, household).await?;
            let src = MailSourceRepo::get(&mut tx, id).await?.map(|v| v.value);
            tx.commit().await?;
            src
        };
        let Some(src) = src else { return Ok(()) };
        if src.archived_at.is_some() || src.state != MailSourceState::Ok {
            return Ok(());
        }
        match self.sync_inner(&src).await {
            Ok(()) => Ok(()),
            Err(MailError::NeedsReauth) => {
                self.mark_reauth(&src).await?;
                Ok(())
            }
            Err(e) => {
                let permanent = matches!(e, MailError::Invalid(_));
                let state = if permanent {
                    MailSourceState::Error
                } else {
                    MailSourceState::Ok
                };
                if let Ok(mut tx) = self.deps.store.tx_for(household).await {
                    let r = MailSourceRepo::set_sync_state(
                        &mut tx,
                        id,
                        None,
                        state,
                        Some(&e.to_string()),
                        None,
                    )
                    .await;
                    if r.is_ok() {
                        let _ = publish(&mut tx, household, id).await;
                        let _ = tx.commit().await;
                    }
                }
                tracing::warn!(source = %id, "mail sync failed");
                if permanent { Ok(()) } else { Err(e) }
            }
        }
    }

    async fn sync_inner(&self, src: &MailSource) -> Result<()> {
        let (src, mut p) = provider::open(&self.deps, src.household_id, src.id, None).await?;
        let r = self.run(&src, p.as_mut()).await;
        p.close().await;
        r
    }

    async fn run(&self, src: &MailSource, p: &mut dyn MailProvider) -> Result<()> {
        let started = std::time::Instant::now();
        let h = src.household_id;
        let now = Utc::now();
        let since = (now - Duration::days(src.backfill_days.into())).date_naive();
        let query = src.query.as_deref();

        let mut delta = Delta::default();
        let refs: Vec<RemoteRef> = if p.has_cursor() {
            let mut d = p.incremental(since, query).await?;
            if d.reset {
                let mut tx = self.deps.store.tx_for(h).await?;
                MailSourceRepo::clear_history_id(&mut tx, src.id).await?;
                tx.commit().await?;
                p.initial(since, query).await?
            } else {
                let added = std::mem::take(&mut d.added);
                delta = d;
                added
            }
        } else {
            p.initial(since, query).await?
        };

        // Skip what is stored already; stop at the per-run cap.
        let existing = {
            let ids: Vec<String> = refs.iter().map(|r| r.id.clone()).collect();
            let mut tx = self.deps.store.tx_for(h).await?;
            MessageRepo::existing_remote_ids(&mut tx, src.id, &ids).await?
        };
        let mut todo: Vec<RemoteRef> = Vec::new();
        let mut cut = refs.len();
        let mut partial = false;
        for (i, r) in refs.iter().enumerate() {
            if existing.contains(&r.id) {
                continue;
            }
            if todo.len() == MAX_PER_RUN {
                cut = i;
                partial = true;
                break;
            }
            todo.push(r.clone());
        }

        let mut inserted: Vec<MessageId> = Vec::new();
        for chunk in todo.chunks(CHUNK) {
            let fetched = p.fetch_many(chunk).await?;
            let dedupe = chunk.iter().any(|r| r.refetch);
            let mut tx = self.deps.store.tx_for(h).await?;
            for f in fetched {
                if let Some(id) = ingest(&mut tx, src, f, dedupe, now).await? {
                    inserted.push(id);
                }
            }
            tx.commit().await?;
        }
        self.enqueue_process(h, &inserted, None).await?;

        let (label_changes, recategorise) = self.apply_labels(src, &delta).await?;
        self.enqueue_process(h, &recategorise, Some("email.category"))
            .await?;

        let st = p.state(partial.then(|| &refs[..cut]));
        let mut tx = self.deps.store.tx_for(h).await?;
        for f in &st.folders {
            MailFolderStateRepo::upsert(&mut tx, f).await?;
        }
        if let Some(ext) = p.gmail_ext()
            && ext != src.gmail_ext
        {
            MailSourceRepo::set_gmail_ext(&mut tx, src.id, ext).await?;
        }
        MailSourceRepo::set_sync_state(
            &mut tx,
            src.id,
            st.history_id.as_deref(),
            MailSourceState::Ok,
            None,
            (!partial).then_some(now),
        )
        .await?;
        if !inserted.is_empty()
            || label_changes > 0
            || src.last_error.is_some()
            || src.last_synced_at.is_none()
        {
            publish(&mut tx, h, src.id).await?;
        }
        tx.commit().await?;

        if partial {
            let mut tx = self.deps.store.tx_system().await?;
            let mut job = NewJob::new("mail.sync");
            job.household_id = Some(h);
            job.payload = serde_json::json!({"household": h, "source": src.id});
            job.idempotency_key = Some(format!("mail.sync:{}:cont:{}", src.id, now.timestamp()));
            JobRepo::enqueue(&mut tx, &job).await?;
            tx.commit().await?;
        }
        tracing::info!(
            source = %src.id,
            new = inserted.len(),
            label_changes,
            more = partial,
            ms = started.elapsed().as_millis() as u64,
            "mail sync"
        );
        Ok(())
    }

    /// Applies label changes to stored messages. Returns the number changed and the messages
    /// that moved to Spam or Trash.
    async fn apply_labels(&self, src: &MailSource, d: &Delta) -> Result<(usize, Vec<MessageId>)> {
        if d.labels_added.is_empty() && d.labels_removed.is_empty() && d.labels_set.is_empty() {
            return Ok((0, vec![]));
        }
        let mut remote: Vec<String> = d
            .labels_added
            .iter()
            .chain(&d.labels_removed)
            .chain(&d.labels_set)
            .map(|(id, _)| id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        remote.sort();
        let mut tx = self.deps.store.tx_for(src.household_id).await?;
        let rows = MessageRepo::label_state(&mut tx, src.id, &remote).await?;
        let mut map: HashMap<String, (MessageId, Vec<String>, Vec<String>)> = rows
            .into_iter()
            .map(|(id, r, l)| (r, (id, l.clone(), l)))
            .collect();
        for (r, add) in &d.labels_added {
            if let Some(e) = map.get_mut(r) {
                for l in add {
                    if !e.1.contains(l) {
                        e.1.push(l.clone());
                    }
                }
            }
        }
        for (r, rem) in &d.labels_removed {
            if let Some(e) = map.get_mut(r) {
                e.1.retain(|l| !rem.contains(l));
            }
        }
        for (r, set) in &d.labels_set {
            if let Some(e) = map.get_mut(r) {
                e.1 = set.clone();
            }
        }
        let mut changed = 0;
        let mut moved = Vec::new();
        for (id, new, old) in map.values() {
            let (mut a, mut b) = (new.clone(), old.clone());
            a.sort();
            b.sort();
            if a == b {
                continue;
            }
            MessageRepo::set_labels(&mut tx, *id, new).await?;
            changed += 1;
            if is_spam_or_trash(new) && !is_spam_or_trash(old) {
                moved.push(*id);
            }
        }
        if changed > 0 {
            publish(&mut tx, src.household_id, src.id).await?;
        }
        tx.commit().await?;
        Ok((changed, moved))
    }

    /// Queues `mail.process{message}`. With `only` the job re-runs one decision point, under a
    /// key that allows repeats.
    async fn enqueue_process(
        &self,
        household: HouseholdId,
        messages: &[MessageId],
        only: Option<&str>,
    ) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }
        let stamp = Utc::now().timestamp();
        let mut tx = self.deps.store.tx_system().await?;
        for m in messages {
            let mut job = NewJob::new("mail.process");
            job.household_id = Some(household);
            job.payload = match only {
                Some(o) => serde_json::json!({"household": household, "message": m, "only": o}),
                None => serde_json::json!({"household": household, "message": m}),
            };
            job.idempotency_key = Some(match only {
                Some(o) => format!("mail.process:{m}:{o}:{stamp}"),
                None => format!("mail.process:{m}"),
            });
            JobRepo::enqueue(&mut tx, &job).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// `needs_reauth`: pause, mark the Google account, tell the owners.
    async fn mark_reauth(&self, src: &MailSource) -> Result<()> {
        let h = src.household_id;
        let msg = "the account needs to be reconnected";
        let mut tx = self.deps.store.tx_for(h).await?;
        MailSourceRepo::set_sync_state(
            &mut tx,
            src.id,
            None,
            MailSourceState::NeedsReauth,
            Some(msg),
            None,
        )
        .await?;
        if src.provider == ProviderKind::Gmail
            && let Some(acc) = src.oauth_account_id
        {
            OAuthAccountRepo::set_state(&mut tx, acc, SourceState::NeedsReauth, Some(msg)).await?;
        }
        let stamp = src
            .last_synced_at
            .map_or_else(|| "never".to_owned(), |t| t.to_rfc3339());
        let key = format!("reauth:mail:{}:{stamp}", src.id);
        let owners = MailSourceRepo::owners(&mut tx, src.id).await?;
        let (title, body, path) = (
            "A mailbox needs to be reconnected",
            "Sign in again in Settings so it keeps syncing.",
            "/settings/mail",
        );
        if owners.is_empty() {
            NotificationRepo::insert_for_adults(
                &mut tx,
                NotifyTrigger::SourceReauth,
                "mail_source",
                Some(src.id.as_uuid()),
                title,
                body,
                Some(path),
                &key,
            )
            .await?;
        }
        for o in owners {
            NotificationRepo::insert_dedupe(
                &mut tx,
                &NewNotification {
                    household_id: h,
                    member_id: o,
                    trigger: NotifyTrigger::SourceReauth,
                    subject_type: "mail_source".into(),
                    subject_id: Some(src.id.as_uuid()),
                    title: title.into(),
                    body: body.into(),
                    click_path: Some(path.into()),
                    priority: 3,
                    dedupe_key: format!("{key}:{o}"),
                    not_before: None,
                },
            )
            .await?;
        }
        publish(&mut tx, h, src.id).await?;
        tx.commit().await?;
        tracing::warn!(source = %src.id, "mail source needs reauth");
        Ok(())
    }
}

async fn publish(tx: &mut Tx, h: HouseholdId, id: MailSourceId) -> Result<()> {
    ChangeBus::publish(tx, h, "mail.updated", &serde_json::json!({"source": id})).await?;
    Ok(())
}

/// Parses and stores one fetched message as `pending`. `None` when skipped.
async fn ingest(
    tx: &mut Tx,
    src: &MailSource,
    f: Fetched,
    dedupe_rfc: bool,
    now: DateTime<Utc>,
) -> Result<Option<MessageId>> {
    if is_junk(&f.labels) {
        return Ok(None);
    }
    let parsed = match fa_rules::parse(&f.raw) {
        Ok(p) => p,
        Err(_) => {
            tracing::warn!(source = %src.id, "unparseable message skipped");
            return Ok(None);
        }
    };
    if dedupe_rfc
        && let Some(m) = &parsed.message_id
        && MessageRepo::exists_rfc822(tx, src.id, m).await?
    {
        return Ok(None);
    }
    let snippet: String = parsed
        .body_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect();
    let attachments: Vec<AttachmentMeta> = parsed
        .attachments
        .iter()
        .map(|a| AttachmentMeta {
            part: a.part.clone(),
            filename: a.filename.clone(),
            mime: a.mime.clone(),
            size: a.size as i64,
            ics: a.ics.clone(),
            pdf_text: None,
        })
        .collect();
    let new = NewMessage {
        household_id: src.household_id,
        source_id: src.id,
        remote_id: f.remote_id,
        folder: f.folder,
        thread_id: f.thread_id,
        rfc822_id: parsed.message_id.clone(),
        received_at: f.received_at.or(parsed.date).unwrap_or(now),
        from_addr: parsed.from_addr.clone(),
        from_name: parsed.from_name.clone(),
        to_addrs: parsed.to.clone(),
        cc_addrs: parsed.cc.clone(),
        subject: parsed.subject.clone(),
        gmail_labels: f.labels,
        has_attachments: !attachments.is_empty(),
        body_text: Some(parsed.body_text.clone()),
        body_html: parsed.body_html.clone(),
        snippet: Some(snippet).filter(|s| !s.is_empty()),
        headers: serde_json::to_value(&parsed.headers).ok(),
        attachments: serde_json::to_value(&attachments).ok(),
    };
    Ok(MessageRepo::insert_parsed(tx, &new).await?)
}

// -------------------------------------------------------------------- jobs

pub(crate) fn job_payload_id<T: FromStr>(
    payload: &serde_json::Value,
    key: &str,
) -> Result<T, JobError> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| JobError::Handler(format!("payload lacks a valid `{key}`")))
}

struct SyncDue(Arc<SyncService>);
struct Sync(Arc<SyncService>);

impl JobHandler for SyncDue {
    fn kind(&self) -> &'static str {
        "mail.sync_due"
    }
    fn run<'a>(
        &'a self,
        _: &'a JobContext,
        _: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            self.0.enqueue_due(Utc::now()).await.map_err(job_err)?;
            Ok(())
        })
    }
}

impl JobHandler for Sync {
    fn kind(&self) -> &'static str {
        "mail.sync"
    }
    fn run<'a>(
        &'a self,
        _: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let h: HouseholdId = job_payload_id(&payload, "household")?;
            let s: MailSourceId = job_payload_id(&payload, "source")?;
            self.0.sync_source(h, s).await.map_err(job_err)
        })
    }
}

/// `mail.sync_due`, `mail.sync`, `mail.auto_actions` and `mail.retention`.
pub fn handlers(deps: &Arc<MailDeps>) -> Vec<Arc<dyn JobHandler>> {
    let svc = Arc::new(SyncService::new(deps.clone()));
    let mut h: Vec<Arc<dyn JobHandler>> = vec![Arc::new(SyncDue(svc.clone())), Arc::new(Sync(svc))];
    h.extend(crate::actions::handlers(deps));
    h.extend(crate::retention::handlers(deps));
    h
}

pub fn default_schedules() -> Vec<(&'static str, &'static str)> {
    vec![
        ("mail.sync_due", "* * * * *"),
        ("mail.retention", "30 3 * * *"),
    ]
}

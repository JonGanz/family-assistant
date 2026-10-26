//! Mail sources: CRUD, the IMAP connection test and sync enqueueing.
//! Visibility and ownership checks are done by fa-api through authz; lists are filtered by the
//! viewer here.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use fa_connectors::ImapClient;
use fa_domain::mail_api::{
    ImapTestRequest, ImapTestResult, MailSourcePatch, MailSourceView, NewMailSource,
};
use fa_domain::{
    HouseholdId, MailProvider, MailSource, MailSourceId, MailSourceState, MailVisibility, MemberId,
    SecretKind,
};
use fa_store::{
    ChangeBus, JobRepo, MailSourceRepo, NewJob, OAuthAccountRepo, SecretRepo, Store, Tx, Versioned,
};

use crate::provider::{SECRET_OWNER, conn_err, security_for};
use crate::{MailDeps, MailError, Result};

pub const GMAIL_READONLY: &str = "https://www.googleapis.com/auth/gmail.readonly";
pub const GMAIL_MODIFY: &str = "https://www.googleapis.com/auth/gmail.modify";

const RECONNECT: &str = "reconnect Google with Gmail access";

pub struct SourceService {
    deps: Arc<MailDeps>,
}

fn check_range(name: &str, v: i32, lo: i32, hi: i32) -> Result<i32> {
    if (lo..=hi).contains(&v) {
        Ok(v)
    } else {
        Err(MailError::Invalid(format!("{name} must be {lo} to {hi}")))
    }
}

fn clean_query(q: Option<String>) -> Option<String> {
    q.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

fn port_of(p: i32) -> Result<u16> {
    u16::try_from(p)
        .ok()
        .filter(|p| *p > 0)
        .ok_or_else(|| MailError::Invalid("the IMAP port is invalid".into()))
}

/// Queues `mail.sync{source}`, one job per source and poll slot.
pub(crate) async fn enqueue_sync_in(
    tx: &mut Tx,
    household: HouseholdId,
    source: MailSourceId,
    poll_minutes: i32,
    now: DateTime<Utc>,
) -> Result<bool> {
    let slot = now.timestamp() / 60 / i64::from(poll_minutes.max(1));
    let mut job = NewJob::new("mail.sync");
    job.household_id = Some(household);
    job.payload = serde_json::json!({"household": household, "source": source});
    job.idempotency_key = Some(format!("mail.sync:{source}:{slot}"));
    Ok(JobRepo::enqueue(tx, &job).await?.is_some())
}

async fn publish(tx: &mut Tx, h: HouseholdId, id: MailSourceId) -> Result<()> {
    ChangeBus::publish(tx, h, "mail.updated", &serde_json::json!({"source": id})).await?;
    Ok(())
}

impl SourceService {
    pub fn new(deps: Arc<MailDeps>) -> Self {
        Self { deps }
    }

    fn store(&self) -> &Store {
        &self.deps.store
    }

    fn view(
        v: &Versioned<MailSource>,
        owners: &[MemberId],
        count: i64,
        viewer: MemberId,
    ) -> MailSourceView {
        let s = &v.value;
        MailSourceView {
            id: s.id,
            provider: s.provider,
            name: s.name.clone(),
            address: s.address.clone(),
            owners: owners.to_vec(),
            visibility: s.visibility,
            read_only: s.read_only,
            backfill_days: s.backfill_days,
            query: s.query.clone(),
            poll_minutes: s.poll_minutes,
            folders: s.folders.clone(),
            archive_folder: s.archive_folder.clone(),
            gmail_ext: s.gmail_ext,
            imap_host: s.imap_host.clone(),
            imap_port: s.imap_port,
            imap_username: s.imap_username.clone(),
            oauth_account: s.oauth_account_id,
            state: s.state,
            last_synced_at: s.last_synced_at,
            last_error: s.last_error.clone(),
            message_count: count,
            can_manage: owners.contains(&viewer),
            version: v.version,
        }
    }

    async fn view_of(tx: &mut Tx, id: MailSourceId, viewer: MemberId) -> Result<MailSourceView> {
        let v = MailSourceRepo::get(tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        let owners = MailSourceRepo::owners(tx, id).await?;
        let count = MailSourceRepo::message_counts(tx)
            .await?
            .get(&id)
            .copied()
            .unwrap_or(0);
        Ok(Self::view(&v, &owners, count, viewer))
    }

    /// The sources the viewer owns or that are visible to the household.
    pub async fn list(
        &self,
        household: HouseholdId,
        viewer: MemberId,
    ) -> Result<Vec<MailSourceView>> {
        let mut tx = self.store().tx_for(household).await?;
        let readable = MailSourceRepo::readable_by(&mut tx, viewer).await?;
        let counts = MailSourceRepo::message_counts(&mut tx).await?;
        Ok(MailSourceRepo::list(&mut tx, false)
            .await?
            .into_iter()
            .filter(|(s, _)| readable.contains(&s.value.id))
            .map(|(s, owners)| {
                let n = counts.get(&s.value.id).copied().unwrap_or(0);
                Self::view(&s, &owners, n, viewer)
            })
            .collect())
    }

    /// Whether the Google account grants what the source needs.
    async fn check_scopes(
        &self,
        tx: &mut Tx,
        account: fa_domain::OAuthAccountId,
        by: Option<MemberId>,
        read_only: bool,
    ) -> Result<fa_domain::OAuthAccount> {
        let acc = OAuthAccountRepo::get(tx, account)
            .await?
            .ok_or(MailError::NotFound)?
            .value;
        if by.is_some_and(|m| m != acc.owner_member) {
            return Err(MailError::Forbidden);
        }
        let has = |s: &str| acc.scopes.iter().any(|x| x == s);
        if !(has(GMAIL_READONLY) || has(GMAIL_MODIFY)) || (!read_only && !has(GMAIL_MODIFY)) {
            return Err(MailError::Invalid(RECONNECT.into()));
        }
        Ok(acc)
    }

    pub async fn create(
        &self,
        household: HouseholdId,
        new: NewMailSource,
        by: MemberId,
    ) -> Result<MailSourceView> {
        let name = new.name.trim().to_owned();
        if name.is_empty() {
            return Err(MailError::Invalid("the name is empty".into()));
        }
        let backfill = check_range("backfill_days", new.backfill_days.unwrap_or(30), 1, 365)?;
        let poll = check_range("poll_minutes", new.poll_minutes.unwrap_or(5), 1, 1440)?;
        let read_only = new.read_only.unwrap_or(true);
        let owners = if new.owners.is_empty() {
            vec![by]
        } else {
            new.owners.clone()
        };
        let id = MailSourceId::new();
        let now = Utc::now();
        let mut src = MailSource {
            id,
            household_id: household,
            provider: new.provider,
            name,
            address: String::new(),
            oauth_account_id: None,
            imap_host: None,
            imap_port: None,
            imap_username: None,
            secret_id: None,
            folders: vec!["INBOX".into()],
            archive_folder: None,
            gmail_ext: false,
            read_only,
            visibility: new.visibility.unwrap_or(MailVisibility::Owners),
            backfill_days: backfill,
            query: clean_query(new.query),
            poll_minutes: poll,
            history_id: None,
            state: MailSourceState::Ok,
            last_synced_at: None,
            last_error: None,
            archived_at: None,
            created_at: now,
            updated_at: now,
        };
        let mut password: Option<String> = None;
        let mut tx = self.store().tx_for(household).await?;
        match new.provider {
            MailProvider::Gmail => {
                let account = new.oauth_account.ok_or_else(|| {
                    MailError::Invalid("a Gmail source needs a Google account".into())
                })?;
                let acc = self
                    .check_scopes(&mut tx, account, Some(by), read_only)
                    .await?;
                src.oauth_account_id = Some(account);
                src.address = new
                    .address
                    .clone()
                    .filter(|a| !a.trim().is_empty())
                    .unwrap_or(acc.email);
            }
            MailProvider::Imap => {
                let imap = new
                    .imap
                    .ok_or_else(|| MailError::Invalid("IMAP settings are required".into()))?;
                let pw = imap
                    .password
                    .clone()
                    .filter(|p| !p.is_empty())
                    .ok_or_else(|| MailError::Invalid("the IMAP password is required".into()))?;
                port_of(imap.port)?;
                // Drop the transaction while talking to the server.
                drop(tx);
                let t = self
                    .test_imap(ImapTestRequest {
                        host: imap.host.clone(),
                        port: imap.port,
                        username: imap.username.clone(),
                        password: pw.clone(),
                    })
                    .await;
                if !t.ok {
                    return Err(MailError::Invalid(
                        t.error.unwrap_or_else(|| "the IMAP test failed".into()),
                    ));
                }
                tx = self.store().tx_for(household).await?;
                src.address = new
                    .address
                    .clone()
                    .filter(|a| !a.trim().is_empty())
                    .unwrap_or_else(|| imap.username.clone());
                src.imap_host = Some(imap.host.trim().to_owned());
                src.imap_port = Some(imap.port);
                src.imap_username = Some(imap.username.clone());
                src.gmail_ext = t.gmail_ext;
                if let Some(f) = imap.folders.filter(|f| !f.is_empty()) {
                    src.folders = f;
                }
                src.archive_folder = imap.archive_folder.filter(|f| !f.trim().is_empty());
                password = Some(pw);
            }
        }
        MailSourceRepo::insert(&mut tx, &src, &owners).await?;
        if let Some(pw) = password {
            let secret = SecretRepo::put(
                &mut tx,
                &self.deps.key,
                household,
                SECRET_OWNER,
                id.as_uuid(),
                SecretKind::Other,
                pw.as_bytes(),
                Some(by),
            )
            .await?;
            MailSourceRepo::set_secret(&mut tx, id, secret).await?;
        }
        publish(&mut tx, household, id).await?;
        let view = Self::view_of(&mut tx, id, by).await?;
        tx.commit().await?;
        self.enqueue_sync(household, id).await?;
        Ok(view)
    }

    pub async fn update(
        &self,
        household: HouseholdId,
        viewer: MemberId,
        id: MailSourceId,
        patch: &MailSourcePatch,
        expected_version: i32,
    ) -> Result<MailSourceView> {
        let mut tx = self.store().tx_for(household).await?;
        let cur = MailSourceRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        let mut s = cur.value.clone();
        if let Some(n) = &patch.name {
            if n.trim().is_empty() {
                return Err(MailError::Invalid("the name is empty".into()));
            }
            s.name = n.trim().to_owned();
        }
        if let Some(v) = patch.visibility {
            s.visibility = v;
        }
        if let Some(ro) = patch.read_only {
            if !ro && let Some(acc) = s.oauth_account_id {
                self.check_scopes(&mut tx, acc, None, false).await?;
            }
            s.read_only = ro;
        }
        if let Some(d) = patch.backfill_days {
            s.backfill_days = check_range("backfill_days", d, 1, 365)?;
        }
        if let Some(p) = patch.poll_minutes {
            s.poll_minutes = check_range("poll_minutes", p, 1, 1440)?;
        }
        if let Some(q) = &patch.query {
            s.query = clean_query(q.clone());
        }
        let mut creds_changed = false;
        let mut resumed = false;
        if let Some(st) = patch.state {
            match st {
                MailSourceState::Paused => s.state = st,
                MailSourceState::Ok => {
                    resumed = s.state != MailSourceState::Ok;
                    s.state = st;
                }
                _ => {
                    return Err(MailError::Invalid(
                        "only `paused` and `ok` can be set".into(),
                    ));
                }
            }
        }
        if let Some(imap) = &patch.imap {
            if s.provider != MailProvider::Imap {
                return Err(MailError::Invalid("not an IMAP source".into()));
            }
            port_of(imap.port)?;
            creds_changed = s.imap_host.as_deref() != Some(imap.host.trim())
                || s.imap_port != Some(imap.port)
                || s.imap_username.as_deref() != Some(imap.username.as_str());
            s.imap_host = Some(imap.host.trim().to_owned());
            s.imap_port = Some(imap.port);
            s.imap_username = Some(imap.username.clone());
            if let Some(f) = imap.folders.clone().filter(|f| !f.is_empty()) {
                s.folders = f;
            }
            if let Some(a) = &imap.archive_folder {
                s.archive_folder = Some(a.trim().to_owned()).filter(|a| !a.is_empty());
            }
            if let Some(pw) = imap.password.as_ref().filter(|p| !p.is_empty()) {
                let secret = SecretRepo::put(
                    &mut tx,
                    &self.deps.key,
                    household,
                    SECRET_OWNER,
                    id.as_uuid(),
                    SecretKind::Other,
                    pw.as_bytes(),
                    Some(viewer),
                )
                .await?;
                s.secret_id = Some(secret);
                creds_changed = true;
            }
        }
        if creds_changed && s.state == MailSourceState::NeedsReauth {
            s.state = MailSourceState::Ok;
            resumed = true;
        }
        MailSourceRepo::update(&mut tx, &s, expected_version).await?;
        if let Some(o) = &patch.owners {
            if o.is_empty() {
                return Err(MailError::Invalid("a source needs an owner".into()));
            }
            MailSourceRepo::set_owners(&mut tx, id, o).await?;
        }
        if resumed || creds_changed {
            MailSourceRepo::set_sync_state(&mut tx, id, None, MailSourceState::Ok, None, None)
                .await?;
        }
        publish(&mut tx, household, id).await?;
        let view = Self::view_of(&mut tx, id, viewer).await?;
        tx.commit().await?;
        if resumed || creds_changed {
            self.enqueue_sync(household, id).await?;
        }
        Ok(view)
    }

    pub async fn archive(&self, household: HouseholdId, id: MailSourceId) -> Result<()> {
        let mut tx = self.store().tx_for(household).await?;
        MailSourceRepo::archive(&mut tx, id).await?;
        publish(&mut tx, household, id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Logs in, lists the folders and detects the Gmail extensions. Never stores anything.
    pub async fn test_imap(&self, req: ImapTestRequest) -> ImapTestResult {
        let fail = |e: String| ImapTestResult {
            ok: false,
            gmail_ext: false,
            folders: vec![],
            error: Some(e),
        };
        let port = match port_of(req.port) {
            Ok(p) => p,
            Err(e) => return fail(e.to_string()),
        };
        let cfg = fa_connectors::ImapConfig {
            host: req.host.trim().to_owned(),
            port,
            security: security_for(port, self.deps.cfg.allow_plain_imap),
            username: req.username,
            password: req.password,
            insecure_tls: self.deps.cfg.imap_tls_insecure,
        };
        let run = async {
            let mut c = ImapClient::connect(cfg).await.map_err(conn_err)?;
            let caps = c.capabilities().await.map_err(conn_err)?;
            let folders = c.list_folders().await.map_err(conn_err)?;
            let _ = c.logout().await;
            Ok::<_, MailError>((caps, folders))
        };
        match run.await {
            Ok((caps, folders)) => ImapTestResult {
                ok: true,
                gmail_ext: caps.gmail_ext,
                folders: folders.into_iter().map(|f| f.name).collect(),
                error: None,
            },
            Err(MailError::NeedsReauth) => fail("the server refused the login".into()),
            Err(e) => fail(e.to_string()),
        }
    }

    /// Queues `mail.sync{source}` on request ("Sync now", creating or resuming a source). The key
    /// is per five seconds, not per poll slot: a slot key would swallow the request whenever a
    /// scheduled sync already ran in the last `poll_minutes`. Returns whether a job was added.
    pub async fn enqueue_sync(&self, household: HouseholdId, id: MailSourceId) -> Result<bool> {
        {
            let mut tx = self.store().tx_for(household).await?;
            MailSourceRepo::get(&mut tx, id)
                .await?
                .ok_or(MailError::NotFound)?;
        }
        let mut job = NewJob::new("mail.sync");
        job.household_id = Some(household);
        job.payload = serde_json::json!({"household": household, "source": id});
        job.idempotency_key = Some(format!(
            "mail.sync:{id}:manual:{}",
            Utc::now().timestamp() / 5
        ));
        let mut tx = self.store().tx_system().await?;
        let added = JobRepo::enqueue(&mut tx, &job).await?.is_some();
        tx.commit().await?;
        Ok(added)
    }
}

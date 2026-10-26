//! Mail providers: one interface over the Gmail API and IMAP (arch/Ingest.md section 4).
//! Nothing here logs message content, addresses, tokens or credentials.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use fa_connectors::google::gmail;
use fa_connectors::{Caps, ConnectorError, Http, ImapClient, ImapConfig, ImapSecurity, SpecialUse};
use fa_domain::{
    AccessTokenProvider, HouseholdId, MailProvider as ProviderKind, MailSource, MailSourceId,
    OAuthAccountId, SecretKind, TokenError,
};
use fa_store::{MailFolderState, MailFolderStateRepo, MailSourceRepo, SecretRepo};

use crate::{MailDeps, MailError, Result};

/// Mail secrets (the IMAP app password) are stored under this owner type.
pub const SECRET_OWNER: &str = "mail_source";

const TOKEN_TTL: Duration = Duration::from_secs(120);
const GMAIL_PARALLEL: usize = 4;

/// A message found by a listing, to be fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRef {
    /// Gmail id. For IMAP a placeholder `uidvalidity:uid` (the final id of a Gmail-extended
    /// server is only known after the fetch).
    pub id: String,
    pub thread_id: Option<String>,
    pub folder: Option<String>,
    pub uid: Option<u32>,
    /// Found again after a UIDVALIDITY reset: may duplicate a stored message.
    pub refetch: bool,
}

#[derive(Debug, Default)]
pub struct Delta {
    pub added: Vec<RemoteRef>,
    /// `(remote id, labels added)`.
    pub labels_added: Vec<(String, Vec<String>)>,
    pub labels_removed: Vec<(String, Vec<String>)>,
    /// `(remote id, the complete label set)`.
    pub labels_set: Vec<(String, Vec<String>)>,
    /// The cursor expired (Gmail 404): forget it and run `initial`.
    pub reset: bool,
}

#[derive(Debug, Clone)]
pub struct Fetched {
    pub remote_id: String,
    pub thread_id: Option<String>,
    pub folder: Option<String>,
    pub labels: Vec<String>,
    pub raw: Vec<u8>,
    pub received_at: Option<DateTime<Utc>>,
}

/// Where a stored message lives at the provider.
#[derive(Debug, Clone)]
pub struct Target {
    pub remote_id: String,
    pub folder: Option<String>,
    pub rfc822_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Archive,
    Unarchive,
    MarkRead,
    MarkUnread,
    Label(String),
    Unlabel(String),
}

/// An IMAP move changes the message's UID and folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocated {
    pub remote_id: String,
    pub folder: String,
}

/// The cursor to store after a sync. Empty fields mean "keep what is stored".
#[derive(Debug, Default)]
pub struct ProviderState {
    pub history_id: Option<String>,
    pub folders: Vec<MailFolderState>,
}

pub fn conn_err(e: ConnectorError) -> MailError {
    match e {
        ConnectorError::Unauthorized
        | ConnectorError::AuthFailed
        | ConnectorError::InsufficientScope => MailError::NeedsReauth,
        other => MailError::Connector(other.to_string()),
    }
}

fn token_err(e: TokenError) -> MailError {
    match e {
        TokenError::NeedsReauth => MailError::NeedsReauth,
        TokenError::Other(m) => MailError::Connector(m),
    }
}

#[async_trait]
pub trait MailProvider: Send {
    /// A cursor from an earlier sync exists, so `incremental` applies.
    fn has_cursor(&self) -> bool;
    /// Lists the backfill window, ignoring any stored cursor.
    async fn initial(&mut self, since: NaiveDate, query: Option<&str>) -> Result<Vec<RemoteRef>>;
    async fn incremental(&mut self, since: NaiveDate, query: Option<&str>) -> Result<Delta>;
    /// Fetches messages (Gmail: up to 4 in flight). Messages deleted meanwhile are skipped.
    async fn fetch_many(&mut self, refs: &[RemoteRef]) -> Result<Vec<Fetched>>;
    async fn fetch(&mut self, r: &RemoteRef) -> Result<Fetched> {
        self.fetch_many(std::slice::from_ref(r))
            .await?
            .pop()
            .ok_or(MailError::NotFound)
    }
    /// The raw RFC 822 text of a stored message (for on-demand extraction such as PDF bills).
    async fn fetch_raw(&mut self, t: &Target) -> Result<Vec<u8>>;
    async fn modify(&mut self, t: &Target, op: &Op) -> Result<Option<Relocated>>;
    /// The cursor after a sync. `processed` is the prefix of the listing that was handled when
    /// the run stopped early (`None` = everything).
    fn state(&self, processed: Option<&[RemoteRef]>) -> ProviderState;
    /// IMAP: whether the server speaks the Gmail extensions.
    fn gmail_ext(&self) -> Option<bool> {
        None
    }
    async fn close(&mut self) {}
}

// ---------------------------------------------------------------- Gmail API

pub type LabelCache = Arc<Mutex<HashMap<String, String>>>;

pub struct GmailProvider {
    http: Http,
    base: String,
    tokens: Arc<dyn AccessTokenProvider>,
    household: HouseholdId,
    account: OAuthAccountId,
    history_id: Option<String>,
    new_history: Option<String>,
    token: Option<(String, Instant)>,
    labels: LabelCache,
}

impl GmailProvider {
    pub fn new(deps: &MailDeps, src: &MailSource, labels: LabelCache) -> Result<Self> {
        Ok(Self {
            http: deps.http.clone(),
            base: deps.cfg.gmail_api_base.clone(),
            tokens: deps.tokens.clone(),
            household: src.household_id,
            account: src
                .oauth_account_id
                .ok_or_else(|| MailError::Invalid("the source has no Google account".into()))?,
            history_id: src.history_id.clone(),
            new_history: None,
            token: None,
            labels,
        })
    }

    async fn token(&mut self) -> Result<String> {
        if let Some((t, at)) = &self.token
            && at.elapsed() < TOKEN_TTL
        {
            return Ok(t.clone());
        }
        let t = self
            .tokens
            .access_token(self.household, self.account)
            .await
            .map_err(token_err)?;
        self.token = Some((t.clone(), Instant::now()));
        Ok(t)
    }

    fn cached(&self, name: &str) -> Option<String> {
        self.labels.lock().ok().and_then(|m| m.get(name).cloned())
    }

    fn remember(&self, labels: impl IntoIterator<Item = (String, String)>) {
        if let Ok(mut m) = self.labels.lock() {
            m.extend(labels);
        }
    }

    /// The id of a user label, creating it when `create`.
    async fn label_id(&mut self, name: &str, create: bool) -> Result<Option<String>> {
        if let Some(id) = self.cached(name) {
            return Ok(Some(id));
        }
        let tok = self.token().await?;
        let all = gmail::list_labels(&self.http, &self.base, &tok)
            .await
            .map_err(conn_err)?;
        self.remember(all.into_iter().map(|l| (l.name, l.id)));
        if let Some(id) = self.cached(name) {
            return Ok(Some(id));
        }
        if !create {
            return Ok(None);
        }
        let l = gmail::create_label(&self.http, &self.base, &tok, name)
            .await
            .map_err(conn_err)?;
        self.remember([(l.name, l.id.clone())]);
        Ok(Some(l.id))
    }
}

#[async_trait]
impl MailProvider for GmailProvider {
    fn has_cursor(&self) -> bool {
        self.history_id.is_some()
    }

    async fn initial(&mut self, since: NaiveDate, query: Option<&str>) -> Result<Vec<RemoteRef>> {
        let tok = self.token().await?;
        // The cursor is read before listing so nothing arriving meanwhile is missed.
        let profile = gmail::get_profile(&self.http, &self.base, &tok)
            .await
            .map_err(conn_err)?;
        self.new_history = Some(profile.history_id);
        let q = format!(
            "after:{} {}",
            since.format("%Y/%m/%d"),
            query.unwrap_or_default()
        );
        let mut out = Vec::new();
        let mut page: Option<String> = None;
        loop {
            let (refs, next) =
                gmail::list_messages(&self.http, &self.base, &tok, q.trim(), page.as_deref())
                    .await
                    .map_err(conn_err)?;
            out.extend(refs.into_iter().map(|m| RemoteRef {
                id: m.id,
                thread_id: Some(m.thread_id).filter(|t| !t.is_empty()),
                folder: None,
                uid: None,
                refetch: false,
            }));
            match next {
                Some(n) => page = Some(n),
                None => break,
            }
        }
        Ok(out)
    }

    async fn incremental(&mut self, _: NaiveDate, _: Option<&str>) -> Result<Delta> {
        let Some(start) = self.history_id.clone() else {
            return Ok(Delta {
                reset: true,
                ..Default::default()
            });
        };
        let tok = self.token().await?;
        let mut delta = Delta::default();
        let mut seen = std::collections::HashSet::new();
        let mut latest = None;
        let mut page: Option<String> = None;
        loop {
            let h = match gmail::list_history(&self.http, &self.base, &tok, &start, page.as_deref())
                .await
            {
                Ok(h) => h,
                Err(ConnectorError::HistoryExpired) => {
                    self.history_id = None;
                    return Ok(Delta {
                        reset: true,
                        ..Default::default()
                    });
                }
                Err(e) => return Err(conn_err(e)),
            };
            for m in h.added {
                if seen.insert(m.id.clone()) {
                    delta.added.push(RemoteRef {
                        id: m.id,
                        thread_id: Some(m.thread_id).filter(|t| !t.is_empty()),
                        folder: None,
                        uid: None,
                        refetch: false,
                    });
                }
            }
            delta.labels_added.extend(h.labels_added);
            delta.labels_removed.extend(h.labels_removed);
            if h.history_id.is_some() {
                latest = h.history_id;
            }
            match h.next_page_token {
                Some(n) => page = Some(n),
                None => break,
            }
        }
        self.new_history = latest.or(Some(start));
        Ok(delta)
    }

    async fn fetch_many(&mut self, refs: &[RemoteRef]) -> Result<Vec<Fetched>> {
        let mut out = Vec::with_capacity(refs.len());
        for chunk in refs.chunks(GMAIL_PARALLEL) {
            let tok = self.token().await?;
            let mut set = tokio::task::JoinSet::new();
            for r in chunk {
                let (http, base, tok, id) = (
                    self.http.clone(),
                    self.base.clone(),
                    tok.clone(),
                    r.id.clone(),
                );
                set.spawn(async move { gmail::get_raw(&http, &base, &tok, &id).await });
            }
            while let Some(res) = set.join_next().await {
                match res.map_err(|e| MailError::Connector(format!("fetch task: {e}")))? {
                    Ok(m) => out.push(Fetched {
                        remote_id: m.id,
                        thread_id: Some(m.thread_id).filter(|t| !t.is_empty()),
                        folder: None,
                        labels: m.label_ids,
                        raw: m.raw,
                        received_at: Some(m.internal_date),
                    }),
                    // Deleted between listing and fetching.
                    Err(ConnectorError::Http(404)) => {}
                    Err(e) => return Err(conn_err(e)),
                }
            }
        }
        Ok(out)
    }

    async fn fetch_raw(&mut self, t: &Target) -> Result<Vec<u8>> {
        let tok = self.token().await?;
        Ok(gmail::get_raw(&self.http, &self.base, &tok, &t.remote_id)
            .await
            .map_err(conn_err)?
            .raw)
    }

    async fn modify(&mut self, t: &Target, op: &Op) -> Result<Option<Relocated>> {
        let (add, remove): (Vec<String>, Vec<String>) = match op {
            Op::Archive => (vec![], vec!["INBOX".into()]),
            Op::Unarchive => (vec!["INBOX".into()], vec![]),
            Op::MarkRead => (vec![], vec!["UNREAD".into()]),
            Op::MarkUnread => (vec!["UNREAD".into()], vec![]),
            Op::Label(n) => match self.label_id(n, true).await? {
                Some(id) => (vec![id], vec![]),
                None => return Ok(None),
            },
            Op::Unlabel(n) => match self.label_id(n, false).await? {
                Some(id) => (vec![], vec![id]),
                None => return Ok(None),
            },
        };
        let tok = self.token().await?;
        gmail::modify(&self.http, &self.base, &tok, &t.remote_id, &add, &remove)
            .await
            .map_err(conn_err)?;
        Ok(None)
    }

    fn state(&self, processed: Option<&[RemoteRef]>) -> ProviderState {
        ProviderState {
            history_id: if processed.is_none() {
                self.new_history.clone()
            } else {
                None
            },
            folders: vec![],
        }
    }
}

// --------------------------------------------------------------------- IMAP

pub struct ImapProvider {
    client: ImapClient,
    source: MailSourceId,
    folders: Vec<String>,
    archive_folder: Option<String>,
    ext: bool,
    states: HashMap<String, MailFolderState>,
    new_states: Vec<MailFolderState>,
    listed: HashMap<String, Vec<u32>>,
    selected: Option<String>,
    caps: Caps,
}

fn gm_hex(id: u64) -> String {
    format!("{id:x}")
}

/// `\Inbox` -> `INBOX`, so Gmail-IMAP labels read like Gmail API label ids.
fn normalise_label(l: &str) -> String {
    match l.strip_prefix('\\') {
        Some(rest) => rest.to_ascii_uppercase(),
        None => l.to_owned(),
    }
}

/// IMAP keyword for a label on servers without the Gmail extensions.
fn keyword(label: &str) -> String {
    let k: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("FA_{}", k.trim_start_matches('_'))
}

pub fn imap_config(deps: &MailDeps, src: &MailSource, password: String) -> Result<ImapConfig> {
    let port = src
        .imap_port
        .and_then(|p| u16::try_from(p).ok())
        .ok_or_else(|| MailError::Invalid("bad IMAP port".into()))?;
    Ok(ImapConfig {
        host: src
            .imap_host
            .clone()
            .ok_or_else(|| MailError::Invalid("no IMAP host".into()))?,
        port,
        security: security_for(port, deps.cfg.allow_plain_imap),
        username: src
            .imap_username
            .clone()
            .ok_or_else(|| MailError::Invalid("no IMAP username".into()))?,
        password,
        insecure_tls: deps.cfg.imap_tls_insecure,
    })
}

/// Port 993 is TLS from the first byte; elsewhere STARTTLS, or no encryption when the process
/// allows it (`FA_ALLOW_PLAIN_IMAP`, tests only).
pub fn security_for(port: u16, allow_plain: bool) -> ImapSecurity {
    if port == 993 {
        ImapSecurity::Implicit
    } else if allow_plain {
        ImapSecurity::Plain
    } else {
        ImapSecurity::StartTls
    }
}

impl ImapProvider {
    pub async fn connect(
        cfg: ImapConfig,
        src: &MailSource,
        states: Vec<MailFolderState>,
    ) -> Result<Self> {
        let mut client = ImapClient::connect(cfg).await.map_err(conn_err)?;
        let caps = client.capabilities().await.map_err(conn_err)?;
        let existing = client.list_folders().await.map_err(conn_err)?;
        let mut folders: Vec<String> = src
            .folders
            .iter()
            .filter(|f| {
                existing.iter().all(|e| {
                    e.name != **f
                        || !matches!(e.special_use, Some(SpecialUse::Junk | SpecialUse::Trash))
                })
            })
            .cloned()
            .collect();
        if folders.is_empty() {
            folders.push("INBOX".into());
        }
        let archive_folder = src.archive_folder.clone().or_else(|| {
            existing
                .iter()
                .find(|f| f.special_use == Some(SpecialUse::Archive))
                .map(|f| f.name.clone())
        });
        Ok(Self {
            client,
            source: src.id,
            folders,
            archive_folder,
            ext: caps.gmail_ext,
            states: states.into_iter().map(|s| (s.folder.clone(), s)).collect(),
            new_states: Vec::new(),
            listed: HashMap::new(),
            selected: None,
            caps,
        })
    }

    async fn select(&mut self, folder: &str) -> Result<fa_connectors::FolderStatus> {
        let st = self.client.select(folder).await.map_err(conn_err)?;
        self.selected = Some(folder.to_owned());
        Ok(st)
    }

    async fn scan(&mut self, since: NaiveDate, query: Option<&str>) -> Result<Vec<RemoteRef>> {
        self.new_states.clear();
        self.listed.clear();
        let gm = if self.ext {
            query.filter(|q| !q.trim().is_empty())
        } else {
            None
        };
        let mut out = Vec::new();
        for f in self.folders.clone() {
            let st = self.select(&f).await?;
            let new = MailFolderState {
                source_id: self.source,
                folder: f.clone(),
                uidvalidity: i64::from(st.uidvalidity),
                uidnext: i64::from(st.uidnext),
                highestmodseq: st.highestmodseq.and_then(|m| i64::try_from(m).ok()),
            };
            let prior = self.states.get(&f).cloned();
            let (min_uid, refetch) = match &prior {
                Some(p) if p.uidvalidity == new.uidvalidity => {
                    if p.uidnext >= new.uidnext {
                        self.new_states.push(new);
                        self.listed.insert(f, Vec::new());
                        continue;
                    }
                    (u32::try_from(p.uidnext).unwrap_or(0), false)
                }
                Some(_) => (0, true),
                None => (0, false),
            };
            let uids: Vec<u32> = self
                .client
                .uid_search_since(since, gm)
                .await
                .map_err(conn_err)?
                .into_iter()
                .filter(|u| *u >= min_uid)
                .collect();
            for u in &uids {
                out.push(RemoteRef {
                    id: format!("{}:{u}", new.uidvalidity),
                    thread_id: None,
                    folder: Some(f.clone()),
                    uid: Some(*u),
                    refetch,
                });
            }
            self.listed.insert(f, uids);
            self.new_states.push(new);
        }
        Ok(out)
    }

    /// `(folder, uid)` of a stored message.
    async fn locate(&mut self, t: &Target) -> Result<(String, u32)> {
        let folder = t
            .folder
            .clone()
            .or_else(|| self.folders.first().cloned())
            .unwrap_or_else(|| "INBOX".into());
        let st = self.select(&folder).await?;
        if self.ext {
            let id = t
                .rfc822_id
                .as_deref()
                .ok_or_else(|| MailError::Invalid("the message has no Message-ID".into()))?;
            let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap_or_default();
            let uids = self
                .client
                .uid_search_since(epoch, Some(&format!("rfc822msgid:{id}")))
                .await
                .map_err(conn_err)?;
            let uid = uids.first().copied().ok_or(MailError::NotFound)?;
            return Ok((folder, uid));
        }
        let (uv, uid) = t
            .remote_id
            .split_once(':')
            .and_then(|(v, u)| Some((v.parse::<u32>().ok()?, u.parse::<u32>().ok()?)))
            .ok_or_else(|| MailError::Invalid("unexpected message id".into()))?;
        if uv != st.uidvalidity {
            return Err(MailError::Connector(
                "the folder was reset; the message cannot be found".into(),
            ));
        }
        Ok((folder, uid))
    }

    /// Moves a message between folders and finds where it landed.
    async fn relocate(
        &mut self,
        uid: u32,
        from: &str,
        dest: &str,
        rfc822: Option<&str>,
    ) -> Result<Option<Relocated>> {
        let before = self.select(dest).await?;
        self.select(from).await?;
        self.client.uid_move(uid, dest).await.map_err(conn_err)?;
        let after = self.select(dest).await?;
        let moved = self
            .client
            .uid_fetch_range(before.uidnext, false)
            .await
            .map_err(conn_err)?;
        let hit = moved
            .iter()
            .find(|m| match rfc822 {
                Some(id) => m
                    .raw
                    .windows(id.len())
                    .any(|w| w.eq_ignore_ascii_case(id.as_bytes())),
                None => true,
            })
            .map(|m| Relocated {
                remote_id: format!("{}:{}", after.uidvalidity, m.uid),
                folder: dest.to_owned(),
            });
        Ok(hit)
    }
}

#[async_trait]
impl MailProvider for ImapProvider {
    fn has_cursor(&self) -> bool {
        !self.states.is_empty()
    }

    async fn initial(&mut self, since: NaiveDate, query: Option<&str>) -> Result<Vec<RemoteRef>> {
        self.states.clear();
        self.scan(since, query).await
    }

    async fn incremental(&mut self, since: NaiveDate, query: Option<&str>) -> Result<Delta> {
        Ok(Delta {
            added: self.scan(since, query).await?,
            ..Default::default()
        })
    }

    async fn fetch_many(&mut self, refs: &[RemoteRef]) -> Result<Vec<Fetched>> {
        let mut out = Vec::with_capacity(refs.len());
        let mut i = 0;
        while i < refs.len() {
            let folder = refs[i].folder.clone().unwrap_or_else(|| "INBOX".into());
            let mut uids = Vec::new();
            while i < refs.len() && refs[i].folder.as_deref().unwrap_or("INBOX") == folder {
                uids.extend(refs[i].uid);
                i += 1;
            }
            let st = self.select(&folder).await?;
            if let Some(n) = self.new_states.iter().find(|s| s.folder == folder)
                && n.uidvalidity != i64::from(st.uidvalidity)
            {
                return Err(MailError::Connector(
                    "UIDVALIDITY changed during the sync".into(),
                ));
            }
            for m in self
                .client
                .uid_fetch(&uids, self.ext)
                .await
                .map_err(conn_err)?
            {
                let mut labels: Vec<String> = if self.ext {
                    m.gm_labels.iter().map(|l| normalise_label(l)).collect()
                } else {
                    Vec::new()
                };
                if !m.flags.iter().any(|f| f.eq_ignore_ascii_case("\\Seen")) {
                    labels.push("UNREAD".into());
                }
                if m.flags.iter().any(|f| f.eq_ignore_ascii_case("\\Flagged")) {
                    labels.push("STARRED".into());
                }
                let (remote_id, thread_id) = match (self.ext, m.gm_msgid) {
                    (true, Some(id)) => (gm_hex(id), m.gm_thrid.map(gm_hex)),
                    _ => (format!("{}:{}", st.uidvalidity, m.uid), None),
                };
                out.push(Fetched {
                    remote_id,
                    thread_id,
                    folder: Some(folder.clone()),
                    labels,
                    raw: m.raw,
                    received_at: m.internal_date,
                });
            }
        }
        Ok(out)
    }

    async fn fetch_raw(&mut self, t: &Target) -> Result<Vec<u8>> {
        let (folder, uid) = self.locate(t).await?;
        self.select(&folder).await?;
        self.client
            .uid_fetch(&[uid], false)
            .await
            .map_err(conn_err)?
            .pop()
            .map(|m| m.raw)
            .ok_or(MailError::NotFound)
    }

    async fn modify(&mut self, t: &Target, op: &Op) -> Result<Option<Relocated>> {
        let (folder, uid) = self.locate(t).await?;
        match op {
            Op::MarkRead | Op::MarkUnread => {
                let add = *op == Op::MarkRead;
                self.client
                    .uid_store_flags(uid, add, &["\\Seen"])
                    .await
                    .map_err(conn_err)?;
            }
            Op::Label(n) | Op::Unlabel(n) => {
                let add = matches!(op, Op::Label(_));
                if self.ext {
                    self.client
                        .gm_labels(uid, add, &[n.as_str()])
                        .await
                        .map_err(conn_err)?;
                } else {
                    let kw = keyword(n);
                    self.client
                        .uid_store_flags(uid, add, &[kw.as_str()])
                        .await
                        .map_err(conn_err)?;
                }
            }
            Op::Archive | Op::Unarchive if self.ext => {
                self.client
                    .gm_labels(uid, *op == Op::Unarchive, &["\\Inbox"])
                    .await
                    .map_err(conn_err)?;
            }
            Op::Archive => {
                let dest = self.archive_folder.clone().ok_or_else(|| {
                    MailError::Invalid("no archive folder is configured for this mailbox".into())
                })?;
                return self
                    .relocate(uid, &folder, &dest, t.rfc822_id.as_deref())
                    .await;
            }
            Op::Unarchive => {
                let dest = self
                    .folders
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "INBOX".into());
                return self
                    .relocate(uid, &folder, &dest, t.rfc822_id.as_deref())
                    .await;
            }
        }
        Ok(None)
    }

    fn state(&self, processed: Option<&[RemoteRef]>) -> ProviderState {
        let mut folders = Vec::new();
        for s in &self.new_states {
            let Some(done) = processed else {
                folders.push(s.clone());
                continue;
            };
            let listed = self.listed.get(&s.folder).map(Vec::as_slice).unwrap_or(&[]);
            let handled: Vec<u32> = done
                .iter()
                .filter(|r| r.folder.as_deref() == Some(s.folder.as_str()))
                .filter_map(|r| r.uid)
                .collect();
            if listed.iter().all(|u| handled.contains(u)) {
                folders.push(s.clone());
            } else if let Some(max) = handled.iter().max() {
                let mut part = s.clone();
                part.uidnext = i64::from(*max) + 1;
                folders.push(part);
            }
        }
        ProviderState {
            history_id: None,
            folders,
        }
    }

    fn gmail_ext(&self) -> Option<bool> {
        Some(self.caps.gmail_ext)
    }

    async fn close(&mut self) {
        let _ = self.client.logout().await;
    }
}

// -------------------------------------------------------------------- open

/// Loads a source and opens its provider. `labels` is the Gmail label cache to share.
pub async fn open(
    deps: &MailDeps,
    household: HouseholdId,
    id: MailSourceId,
    labels: Option<LabelCache>,
) -> Result<(MailSource, Box<dyn MailProvider>)> {
    let mut tx = deps.store.tx_for(household).await?;
    let src = MailSourceRepo::get(&mut tx, id)
        .await?
        .ok_or(MailError::NotFound)?
        .value;
    let provider: Box<dyn MailProvider> = match src.provider {
        ProviderKind::Gmail => {
            Box::new(GmailProvider::new(deps, &src, labels.unwrap_or_default())?)
        }
        ProviderKind::Imap => {
            let pw = SecretRepo::get(
                &mut tx,
                &deps.key,
                household,
                SECRET_OWNER,
                id.as_uuid(),
                SecretKind::Other,
            )
            .await?
            .and_then(|b| String::from_utf8(b).ok())
            .ok_or_else(|| MailError::Invalid("the stored IMAP password is missing".into()))?;
            let states = MailFolderStateRepo::list(&mut tx, id).await?;
            drop(tx);
            let cfg = imap_config(deps, &src, pw)?;
            Box::new(ImapProvider::connect(cfg, &src, states).await?)
        }
    };
    Ok((src, provider))
}

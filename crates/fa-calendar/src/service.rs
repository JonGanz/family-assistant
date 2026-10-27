use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use fa_connectors::{Http, RemoteCalendar, Window, ics};
use fa_domain::api::{
    CalendarPatch, CalendarView, EventView, RemoteCalendar as RemoteCalendarDto, SourcePatch,
    SourceTestResult, SourceView,
};
use fa_domain::calendar::{CalendarMeta, EventMeta, HouseholdFlags, Viewer, mask, view};
use fa_domain::{
    AccessTokenProvider, Calendar, CalendarId, CalendarSource, CalendarSourceId, EventId,
    Household, HouseholdId, MemberId, SecretKind, SourceKind, SourceState,
};
use fa_store::{
    CalendarRepo, CalendarSourceRepo, ChangeBus, EventRepo, HouseholdRepo, JobRepo, MasterKey,
    NewJob, NewOccurrence, OAuthAccountRepo, SecretRepo, Store, StoreError, Tx, Versioned,
};

use crate::error::{CalendarError, Result, scrub};
use crate::remote::{Creds, Fetched, Prior, Remote, Span};
use crate::{ConnectorConfig, MAX_RANGE_DAYS};

const SECRET_OWNER: &str = "calendar_source";

pub struct CalendarService {
    pub(crate) store: Store,
    pub(crate) key: Arc<MasterKey>,
    pub(crate) remote: Remote,
}

/// Everything one sync run needs, loaded in a single short transaction.
struct Loaded {
    source: Versioned<CalendarSource>,
    household: Household,
    creds: Creds,
}

pub fn mask_url(url: &str) -> Option<String> {
    let u = url::Url::parse(&ics::normalize_url(url)).ok()?;
    Some(format!("{}://{}/…", u.scheme(), u.host_str()?))
}

pub fn window_of(h: &Household, now: DateTime<Utc>) -> Window {
    Window {
        from: now - Duration::days(h.calendar_past_days.into()),
        to: now + Duration::days(h.calendar_future_days.into()),
    }
}

pub(crate) fn tz_of(h: &Household) -> Tz {
    h.tz.parse().unwrap_or(Tz::UTC)
}

/// `calendar.updated{from, to}` over the household's whole window.
pub(crate) async fn publish_calendar_updated(
    tx: &mut Tx,
    h: &Household,
    now: DateTime<Utc>,
) -> Result<()> {
    let w = window_of(h, now);
    ChangeBus::publish(
        tx,
        h.id,
        "calendar.updated",
        &serde_json::json!({"from": w.from, "to": w.to}),
    )
    .await?;
    Ok(())
}

async fn publish_source_updated(
    tx: &mut Tx,
    h: HouseholdId,
    id: CalendarSourceId,
    state: SourceState,
) -> Result<()> {
    ChangeBus::publish(
        tx,
        h,
        "source.updated",
        &serde_json::json!({"id": id, "state": state}),
    )
    .await?;
    Ok(())
}

pub(crate) async fn household_of(tx: &mut Tx, id: HouseholdId) -> Result<Household> {
    Ok(HouseholdRepo::get(tx, id)
        .await?
        .ok_or(CalendarError::NotFound)?
        .value)
}

impl CalendarService {
    pub fn new(
        store: Store,
        key: MasterKey,
        cfg: ConnectorConfig,
        tokens: Arc<dyn AccessTokenProvider>,
    ) -> Result<Self> {
        let http = Http::with_policy(cfg.allow_http, cfg.allow_private_hosts)?;
        Ok(Self {
            store,
            key: Arc::new(key),
            remote: Remote { http, cfg, tokens },
        })
    }

    async fn secret_text(&self, tx: &mut Tx, src: &CalendarSource) -> Result<String> {
        let bytes = SecretRepo::get(
            tx,
            &self.key,
            src.household_id,
            SECRET_OWNER,
            src.id.as_uuid(),
            SecretKind::Other,
        )
        .await?
        .ok_or_else(|| CalendarError::Remote("the stored credential is missing".into()))?;
        String::from_utf8(bytes)
            .map_err(|_| CalendarError::Store(StoreError::Corrupt("secret is not utf-8".into())))
    }

    pub(crate) async fn creds_of(&self, tx: &mut Tx, src: &CalendarSource) -> Result<Creds> {
        Ok(match src.kind {
            SourceKind::Google => Creds::Google(src.oauth_account.ok_or_else(|| {
                CalendarError::Store(StoreError::Corrupt("google source without account".into()))
            })?),
            SourceKind::Caldav => Creds::Caldav {
                url: src.url.clone().unwrap_or_default(),
                user: src.username.clone().unwrap_or_default(),
                pass: self.secret_text(tx, src).await?,
            },
            SourceKind::Ics => Creds::Ics {
                url: self.secret_text(tx, src).await?,
            },
        })
    }

    async fn put_secret(&self, tx: &mut Tx, src: &CalendarSource, text: &str) -> Result<()> {
        let id = SecretRepo::put(
            tx,
            &self.key,
            src.household_id,
            SECRET_OWNER,
            src.id.as_uuid(),
            SecretKind::Other,
            text.as_bytes(),
            src.owner_member,
        )
        .await?;
        CalendarSourceRepo::set_secret(tx, src.id, id).await?;
        Ok(())
    }

    // ---- views ----

    fn source_view(v: &Versioned<CalendarSource>, ics_url: Option<&str>) -> SourceView {
        let s = &v.value;
        SourceView {
            id: s.id,
            kind: s.kind,
            name: s.name.clone(),
            owner: s.owner_member,
            masked_url: match s.kind {
                SourceKind::Caldav => s.url.as_deref().and_then(mask_url),
                SourceKind::Ics => ics_url.and_then(mask_url),
                SourceKind::Google => None,
            },
            username: s.username.clone(),
            state: s.state,
            last_synced_at: s.last_synced_at,
            last_error: s.last_error.clone(),
            poll_minutes: s.poll_minutes,
            version: v.version,
        }
    }

    async fn view_of(&self, tx: &mut Tx, v: &Versioned<CalendarSource>) -> Result<SourceView> {
        let url = if v.value.kind == SourceKind::Ics && v.value.secret_id.is_some() {
            Some(self.secret_text(tx, &v.value).await?)
        } else {
            None
        };
        Ok(Self::source_view(v, url.as_deref()))
    }

    async fn calendar_view(tx: &mut Tx, c: &Versioned<Calendar>) -> Result<CalendarView> {
        Ok(CalendarView {
            id: c.value.id,
            source: c.value.source,
            name: c.value.name.clone(),
            color: c.value.color.clone(),
            enabled: c.value.enabled,
            members: CalendarRepo::members(tx, c.value.id).await?,
            version: c.version,
        })
    }

    pub async fn list_sources(&self, household: HouseholdId) -> Result<Vec<SourceView>> {
        let mut tx = self.store.tx_for(household).await?;
        let mut out = Vec::new();
        for s in CalendarSourceRepo::list(&mut tx, false).await? {
            out.push(self.view_of(&mut tx, &s).await?);
        }
        Ok(out)
    }

    pub async fn list_calendars(&self, household: HouseholdId) -> Result<Vec<CalendarView>> {
        let mut tx = self.store.tx_for(household).await?;
        let mut out = Vec::new();
        for c in CalendarRepo::list(&mut tx, None).await? {
            out.push(Self::calendar_view(&mut tx, &c).await?);
        }
        Ok(out)
    }

    /// Ids of the household's calendar sources the app may write to.
    pub async fn writable_sources(
        &self,
        household: HouseholdId,
    ) -> Result<std::collections::HashSet<CalendarSourceId>> {
        let mut tx = self.store.tx_for(household).await?;
        Ok(CalendarSourceRepo::list(&mut tx, false)
            .await?
            .into_iter()
            .filter(|s| s.value.writable)
            .map(|s| s.value.id)
            .collect())
    }

    /// Ids of the calendars marked as default targets of extracted events.
    pub async fn default_calendars(
        &self,
        household: HouseholdId,
    ) -> Result<std::collections::HashSet<CalendarId>> {
        let mut tx = self.store.tx_for(household).await?;
        Ok(CalendarRepo::list(&mut tx, None)
            .await?
            .into_iter()
            .filter(|c| c.value.is_default_target)
            .map(|c| c.value.id)
            .collect())
    }

    /// Lets the app write events to the source. Google sources need the account to have granted
    /// calendar write access; ICS feeds cannot be written.
    pub async fn set_source_writable(
        &self,
        household: HouseholdId,
        id: CalendarSourceId,
        writable: bool,
    ) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        let src = CalendarSourceRepo::get(&mut tx, id)
            .await?
            .ok_or(CalendarError::NotFound)?
            .value;
        if writable {
            match src.kind {
                SourceKind::Ics => {
                    return Err(CalendarError::Invalid(
                        "an ICS feed cannot be written to".into(),
                    ));
                }
                SourceKind::Google => {
                    let granted = match src.oauth_account {
                        Some(a) => OAuthAccountRepo::get(&mut tx, a).await?.is_some_and(|a| {
                            a.value.scopes.iter().any(|s| {
                                s == "https://www.googleapis.com/auth/calendar.events"
                                    || s == "https://www.googleapis.com/auth/calendar"
                            })
                        }),
                        None => false,
                    };
                    if !granted {
                        return Err(CalendarError::Invalid(
                            "reconnect with calendar write access".into(),
                        ));
                    }
                }
                SourceKind::Caldav => {}
            }
        }
        CalendarSourceRepo::set_writable(&mut tx, id, writable).await?;
        publish_source_updated(&mut tx, household, id, src.state).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Marks the calendar as a default target for extracted events.
    pub async fn set_default_target(
        &self,
        household: HouseholdId,
        id: CalendarId,
        default_target: bool,
    ) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        CalendarRepo::set_default_target(&mut tx, id, default_target).await?;
        let h = household_of(&mut tx, household).await?;
        publish_calendar_updated(&mut tx, &h, Utc::now()).await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- sources ----

    fn creds_of_new(&self, new: &fa_domain::api::NewSource) -> Result<Creds> {
        use fa_domain::api::NewSource as N;
        Ok(match new {
            N::Google { oauth_account, .. } => Creds::Google(*oauth_account),
            N::Caldav {
                url,
                username,
                password,
                ..
            } => {
                self.remote.http.parse_url(url)?;
                Creds::Caldav {
                    url: url.trim().to_owned(),
                    user: username.clone(),
                    pass: password.clone(),
                }
            }
            N::Ics { url, .. } => {
                let url = ics::normalize_url(url.trim());
                self.remote.http.parse_url(&url)?;
                Creds::Ics { url }
            }
        })
    }

    async fn span_for(&self, household: HouseholdId) -> Result<(Household, Span)> {
        let mut tx = self.store.tx_for(household).await?;
        let h = household_of(&mut tx, household).await?;
        let span = Span {
            window: window_of(&h, Utc::now()),
            tz: tz_of(&h),
        };
        Ok((h, span))
    }

    /// Dry run: connects, lists calendars and samples up to 3 upcoming events. Never writes.
    pub async fn test_source(
        &self,
        household: HouseholdId,
        new: &fa_domain::api::NewSource,
    ) -> SourceTestResult {
        let fail = |e: CalendarError| SourceTestResult {
            ok: false,
            calendars: vec![],
            sample_events: vec![],
            error: Some(e.to_string()),
        };
        let run = async {
            let creds = self.creds_of_new(new)?;
            let (_, span) = self.span_for(household).await?;
            let name = new_name(new);
            let cals = self.remote.discover(household, &creds, name).await?;
            let mut sample = Vec::new();
            if let Some(first) = cals.first() {
                // A failed sample must not hide a working connection (the Google sample needs a
                // calendar the account may not read, for instance).
                if let Ok(s) = self.remote.sample(household, &creds, first, &span).await {
                    sample = s;
                }
            }
            Ok::<_, CalendarError>((cals, sample))
        };
        match run.await {
            Ok((cals, sample)) => SourceTestResult {
                ok: true,
                calendars: cals.iter().map(remote_dto).collect(),
                sample_events: sample
                    .iter()
                    .map(|o| sample_view(o, cals.first().and_then(|c| c.color.clone())))
                    .collect(),
                error: None,
            },
            Err(e) => fail(e),
        }
    }

    /// Stores the source and its credential, discovers synchronously and inserts the calendars
    /// disabled (an ICS source's only calendar is enabled and a first sync is queued).
    pub async fn create_source(
        &self,
        household: HouseholdId,
        owner: Option<MemberId>,
        new: fa_domain::api::NewSource,
    ) -> Result<(SourceView, Vec<CalendarView>)> {
        use fa_domain::api::NewSource as N;
        let creds = self.creds_of_new(&new)?;
        let (kind, name, own, account, url, user, poll) = match &new {
            N::Google {
                name,
                owner,
                oauth_account,
            } => (
                SourceKind::Google,
                name,
                *owner,
                Some(*oauth_account),
                None,
                None,
                15,
            ),
            N::Caldav {
                name,
                owner,
                url,
                username,
                ..
            } => (
                SourceKind::Caldav,
                name,
                *owner,
                None,
                Some(url.trim().to_owned()),
                Some(username.clone()),
                15,
            ),
            N::Ics { name, owner, .. } => (SourceKind::Ics, name, *owner, None, None, None, 60),
        };
        if name.trim().is_empty() {
            return Err(CalendarError::Invalid("the name is empty".into()));
        }
        if let Creds::Google(acc) = &creds {
            let mut tx = self.store.tx_for(household).await?;
            OAuthAccountRepo::get(&mut tx, *acc)
                .await?
                .ok_or(CalendarError::NotFound)?;
        }
        let remote_cals = self.remote.discover(household, &creds, name).await?;
        if kind == SourceKind::Ics {
            // Make sure the feed is reachable and parses before it is stored.
            let (_, span) = self.span_for(household).await?;
            let cal = &remote_cals[0];
            self.remote.sample(household, &creds, cal, &span).await?;
        }
        let now = Utc::now();
        let src = CalendarSource {
            id: CalendarSourceId::new(),
            household_id: household,
            kind,
            name: name.trim().to_owned(),
            owner_member: owner.or(own),
            oauth_account: account,
            url,
            username: user,
            secret_id: None,
            writable: false,
            poll_minutes: poll,
            state: SourceState::Ok,
            last_synced_at: None,
            last_error: None,
            archived_at: None,
            created_at: now,
            updated_at: now,
        };
        let mut tx = self.store.tx_for(household).await?;
        CalendarSourceRepo::insert(&mut tx, &src).await?;
        match &creds {
            Creds::Caldav { pass, .. } => self.put_secret(&mut tx, &src, pass).await?,
            Creds::Ics { url } => self.put_secret(&mut tx, &src, url).await?,
            Creds::Google(_) => {}
        }
        let mut cal_views = Vec::new();
        for rc in &remote_cals {
            let (c, created) = CalendarRepo::upsert_discovered(
                &mut tx,
                src.id,
                &rc.remote_id,
                &rc.name,
                rc.color.as_deref(),
            )
            .await?;
            let mut c = c;
            if created && kind == SourceKind::Ics {
                let patch = CalendarPatch {
                    enabled: Some(true),
                    ..Default::default()
                };
                c.version = CalendarRepo::update(&mut tx, c.value.id, &patch, c.version).await?;
                c.value.enabled = true;
            }
            cal_views.push(Self::calendar_view(&mut tx, &c).await?);
        }
        let v = CalendarSourceRepo::get(&mut tx, src.id)
            .await?
            .ok_or(CalendarError::NotFound)?;
        let view = self.view_of(&mut tx, &v).await?;
        publish_source_updated(&mut tx, household, src.id, SourceState::Ok).await?;
        tx.commit().await?;
        if kind == SourceKind::Ics {
            self.enqueue_sync(household, src.id).await?;
        }
        Ok((view, cal_views))
    }

    /// A credential change resets the state to `ok` and queues a sync.
    pub async fn update_source(
        &self,
        household: HouseholdId,
        id: CalendarSourceId,
        patch: &SourcePatch,
        expected_version: i32,
    ) -> Result<SourceView> {
        let mut tx = self.store.tx_for(household).await?;
        let cur = CalendarSourceRepo::get(&mut tx, id)
            .await?
            .ok_or(CalendarError::NotFound)?;
        CalendarSourceRepo::update(&mut tx, id, patch, expected_version).await?;
        let mut creds_changed = false;
        if let Some(c) = &patch.credentials {
            match cur.value.kind {
                SourceKind::Caldav => {
                    creds_changed = c.url.is_some() || c.username.is_some() || c.password.is_some();
                    if let Some(p) = &c.password {
                        self.put_secret(&mut tx, &cur.value, p).await?;
                    }
                }
                SourceKind::Ics => {
                    if let Some(u) = &c.url {
                        let u = ics::normalize_url(u.trim());
                        self.remote.http.parse_url(&u)?;
                        self.put_secret(&mut tx, &cur.value, &u).await?;
                        creds_changed = true;
                    }
                }
                SourceKind::Google => {}
            }
        }
        if creds_changed {
            CalendarSourceRepo::set_sync_result(&mut tx, id, SourceState::Ok, None, None).await?;
        }
        let v = CalendarSourceRepo::get(&mut tx, id)
            .await?
            .ok_or(CalendarError::NotFound)?;
        let view = self.view_of(&mut tx, &v).await?;
        publish_source_updated(&mut tx, household, id, v.value.state).await?;
        tx.commit().await?;
        if creds_changed {
            self.enqueue_sync(household, id).await?;
        }
        Ok(view)
    }

    pub async fn archive_source(&self, household: HouseholdId, id: CalendarSourceId) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        CalendarSourceRepo::archive(&mut tx, id).await?;
        let h = household_of(&mut tx, household).await?;
        publish_calendar_updated(&mut tx, &h, Utc::now()).await?;
        publish_source_updated(&mut tx, household, id, SourceState::Ok).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Queues `calendar.sync{source}`; one job per source and minute. Returns whether one was added.
    pub async fn enqueue_sync(&self, household: HouseholdId, id: CalendarSourceId) -> Result<bool> {
        let mut tx = self.store.tx_system().await?;
        let r = enqueue_sync_in(&mut tx, household, id, Utc::now()).await?;
        tx.commit().await?;
        Ok(r)
    }

    pub async fn update_calendar(
        &self,
        household: HouseholdId,
        id: CalendarId,
        patch: &CalendarPatch,
        expected_version: i32,
    ) -> Result<CalendarView> {
        let mut tx = self.store.tx_for(household).await?;
        let before = CalendarRepo::get(&mut tx, id)
            .await?
            .ok_or(CalendarError::NotFound)?;
        CalendarRepo::update(&mut tx, id, patch, expected_version).await?;
        if let Some(m) = &patch.members {
            CalendarRepo::set_members(&mut tx, id, m).await?;
        }
        let after = CalendarRepo::get(&mut tx, id)
            .await?
            .ok_or(CalendarError::NotFound)?;
        let h = household_of(&mut tx, household).await?;
        publish_calendar_updated(&mut tx, &h, Utc::now()).await?;
        let view = Self::calendar_view(&mut tx, &after).await?;
        tx.commit().await?;
        if after.value.enabled && !before.value.enabled {
            self.enqueue_sync(household, before.value.source).await?;
        }
        Ok(view)
    }

    // ---- queries ----

    /// Events overlapping `[from, to)` as the viewer may see them (D-036).
    pub async fn events(
        &self,
        household: HouseholdId,
        viewer: Viewer,
        flags: HouseholdFlags,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        member: Option<MemberId>,
    ) -> Result<Vec<EventView>> {
        if to < from {
            return Err(CalendarError::Invalid("`to` is before `from`".into()));
        }
        if to - from > Duration::days(MAX_RANGE_DAYS) {
            return Err(CalendarError::RangeTooLarge(MAX_RANGE_DAYS));
        }
        let mut tx = self.store.tx_for(household).await?;
        let rows = EventRepo::range(&mut tx, from, to, None).await?;
        let mut out = Vec::new();
        for r in rows {
            let ids: Vec<MemberId> = r.members.iter().map(|(m, _)| *m).collect();
            if let Viewer::ChildUser(me) = viewer
                && !(ids.is_empty() || ids.contains(&me))
            {
                continue;
            }
            if let Some(m) = member
                && !ids.contains(&m)
            {
                continue;
            }
            let visibility = view(
                &EventMeta {
                    is_private: r.event.is_private,
                },
                &CalendarMeta {
                    members: &r.members,
                    owner_role: r.owner_role,
                },
                &viewer,
                &flags,
            );
            let e = r.event;
            let external_url = e
                .source_ref
                .as_ref()
                .and_then(|s| s.get("external_url"))
                .and_then(|u| u.as_str())
                .map(str::to_owned);
            out.push(mask(EventView {
                id: e.id,
                calendar: e.calendar,
                title: e.title,
                location: e.location,
                description: e.description,
                start_at: e.start_at,
                end_at: e.end_at,
                all_day: e.all_day,
                start_date: e.start_date,
                end_date: e.end_date,
                members: ids,
                color: r.color,
                visibility,
                external_url,
            }));
        }
        Ok(out)
    }

    // ---- sync ----

    async fn load(&self, household: HouseholdId, id: CalendarSourceId) -> Result<Option<Loaded>> {
        let mut tx = self.store.tx_for(household).await?;
        let Some(source) = CalendarSourceRepo::get(&mut tx, id).await? else {
            return Ok(None);
        };
        if source.value.archived_at.is_some() {
            return Ok(None);
        }
        let h = household_of(&mut tx, household).await?;
        let creds = self.creds_of(&mut tx, &source.value).await?;
        Ok(Some(Loaded {
            source,
            household: h,
            creds,
        }))
    }

    /// One sync run of a source (arch/Feeds.md section 3.3). The source's state and `last_error`
    /// are always updated; transient errors are returned so the job retries.
    pub async fn sync_source(&self, household: HouseholdId, id: CalendarSourceId) -> Result<()> {
        let Some(loaded) = self.load(household, id).await? else {
            return Ok(());
        };
        let secrets: Vec<String> = loaded
            .creds
            .secrets()
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let secret_refs: Vec<&str> = secrets.iter().map(String::as_str).collect();
        let res = self.sync_inner(&loaded).await;
        let (state, last_error, synced, ret) = match res {
            Ok(warnings) => (
                SourceState::Ok,
                (!warnings.is_empty()).then(|| warnings.join("; ")),
                Some(Utc::now()),
                Ok(()),
            ),
            Err(CalendarError::Unauthorized) => (
                SourceState::NeedsReauth,
                Some("the credentials were rejected; re-connect the source".to_owned()),
                None,
                Ok(()),
            ),
            Err(e) => {
                let msg = scrub(&e.to_string(), &secret_refs);
                (SourceState::Error, Some(msg), None, Err(e))
            }
        };
        let mut tx = self.store.tx_for(household).await?;
        CalendarSourceRepo::set_sync_result(&mut tx, id, state, last_error.as_deref(), synced)
            .await?;
        if state == SourceState::NeedsReauth
            && let Some(acc) = loaded.source.value.oauth_account
        {
            OAuthAccountRepo::set_state(
                &mut tx,
                acc,
                SourceState::NeedsReauth,
                last_error.as_deref(),
            )
            .await?;
        }
        publish_source_updated(&mut tx, household, id, state).await?;
        tx.commit().await?;
        ret.map_err(|e| match e {
            CalendarError::Remote(m) => CalendarError::Remote(scrub(&m, &secret_refs)),
            other => other,
        })
    }

    async fn sync_inner(&self, l: &Loaded) -> Result<Vec<String>> {
        let src = &l.source.value;
        let hh = src.household_id;
        let now = Utc::now();
        let span = Span {
            window: window_of(&l.household, now),
            tz: tz_of(&l.household),
        };
        let mut warnings: Vec<String> = Vec::new();

        // 1. Discover: network first, then one short transaction.
        let discovered = self.remote.discover(hh, &l.creds, &src.name).await?;
        let calendars = {
            let mut tx = self.store.tx_for(hh).await?;
            for rc in &discovered {
                let (c, created) = CalendarRepo::upsert_discovered(
                    &mut tx,
                    src.id,
                    &rc.remote_id,
                    &rc.name,
                    rc.color.as_deref(),
                )
                .await?;
                if created && src.kind == SourceKind::Ics {
                    let p = CalendarPatch {
                        enabled: Some(true),
                        ..Default::default()
                    };
                    CalendarRepo::update(&mut tx, c.value.id, &p, c.version).await?;
                }
            }
            let seen: Vec<String> = discovered.iter().map(|c| c.remote_id.clone()).collect();
            let gone = CalendarRepo::archive_missing(&mut tx, src.id, &seen).await?;
            for cal in &gone {
                EventRepo::delete_calendar_events(&mut tx, *cal).await?;
            }
            if !gone.is_empty() {
                publish_calendar_updated(&mut tx, &l.household, now).await?;
            }
            let cals = CalendarRepo::list(&mut tx, Some(src.id)).await?;
            tx.commit().await?;
            cals
        };

        // 2. Fetch and apply each enabled calendar.
        let mut first_err: Option<CalendarError> = None;
        for cal in calendars.iter().filter(|c| c.value.enabled) {
            let Some(rc) = discovered
                .iter()
                .find(|r| r.remote_id == cal.value.remote_id)
            else {
                continue;
            };
            match self.sync_calendar(l, &cal.value, rc, &span, now).await {
                Ok(w) => warnings.extend(w),
                Err(e) => {
                    tracing::warn!(source = %src.id, calendar = %cal.value.id, error = %scrub(&e.to_string(), &[]), "calendar sync failed");
                    if matches!(e, CalendarError::Unauthorized) || first_err.is_none() {
                        first_err = Some(e);
                    }
                }
            }
        }
        if let Some(e) = first_err {
            return Err(e);
        }
        warnings.sort();
        warnings.dedup();
        Ok(warnings)
    }

    async fn sync_calendar(
        &self,
        l: &Loaded,
        cal: &Calendar,
        rc: &RemoteCalendar,
        span: &Span,
        now: DateTime<Utc>,
    ) -> Result<Vec<String>> {
        let hh = l.source.value.household_id;
        let prior = Prior {
            sync_state: cal.sync_state.clone(),
            last_full_sync_at: cal.last_full_sync_at,
        };
        let f: Fetched = self
            .remote
            .fetch(hh, &l.creds, rc, &prior, span, now)
            .await?;
        if f.skip {
            return Ok(vec![]);
        }
        let mut tx = self.store.tx_for(hh).await?;
        let mut changed = 0u64;
        if f.reset {
            changed += EventRepo::delete_calendar_events(&mut tx, cal.id).await?;
        }
        changed += EventRepo::upsert_synced(&mut tx, cal.id, &f.occurrences).await?;
        if f.full {
            let seen: Vec<(String, String)> = f
                .occurrences
                .iter()
                .map(|o| (o.remote_uid.clone(), o.instance_key.clone()))
                .collect();
            changed += EventRepo::delete_unseen_in_window(
                &mut tx,
                cal.id,
                span.window.from,
                span.window.to,
                &seen,
            )
            .await?;
        }
        for (uid, key) in &f.deletes {
            changed += EventRepo::delete_by_uid(&mut tx, cal.id, uid, Some(key)).await?;
        }
        // An incremental run that returned no token keeps the old one.
        let state = f.new_state.as_deref().or(if f.full {
            None
        } else {
            prior.sync_state.as_deref()
        });
        CalendarRepo::set_sync_state(&mut tx, cal.id, state, f.full.then_some(now)).await?;
        if changed > 0 {
            ChangeBus::publish(
                &mut tx,
                hh,
                "calendar.updated",
                &serde_json::json!({"from": span.window.from, "to": span.window.to}),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(f.warnings)
    }
}

pub(crate) async fn enqueue_sync_in(
    tx: &mut Tx,
    household: HouseholdId,
    id: CalendarSourceId,
    now: DateTime<Utc>,
) -> Result<bool> {
    let mut job = NewJob::new("calendar.sync");
    job.household_id = Some(household);
    job.payload = serde_json::json!({"household": household, "source": id});
    job.idempotency_key = Some(format!("calendar.sync:{id}:{}", now.timestamp() / 60));
    Ok(JobRepo::enqueue(tx, &job).await?.is_some())
}

fn new_name(n: &fa_domain::api::NewSource) -> &str {
    use fa_domain::api::NewSource as N;
    match n {
        N::Google { name, .. } | N::Caldav { name, .. } | N::Ics { name, .. } => name,
    }
}

fn remote_dto(c: &RemoteCalendar) -> RemoteCalendarDto {
    RemoteCalendarDto {
        remote_id: c.remote_id.clone(),
        name: c.name.clone(),
        color: c.color.clone(),
    }
}

fn sample_view(o: &NewOccurrence, color: Option<String>) -> EventView {
    EventView {
        id: EventId::new(),
        calendar: None,
        title: o.title.clone(),
        location: o.location.clone(),
        description: o.description.clone(),
        start_at: o.start_at,
        end_at: o.end_at,
        all_day: o.all_day,
        start_date: o.start_date,
        end_date: o.end_date,
        members: vec![],
        color,
        visibility: fa_domain::Visibility::Full,
        external_url: None,
    }
}

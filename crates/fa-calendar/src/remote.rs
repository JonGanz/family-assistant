//! Talking to the three kinds of remotes and mapping what comes back to `NewOccurrence`.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use fa_connectors::{
    ConnectorError, EventTime, Http, ParsedComponent, RemoteCalendar, RemoteEvent, Window,
    caldav::CaldavClient,
    google::calendar::{self, EventQuery},
    ics::{self, IcsFetch},
};
use fa_domain::{AccessTokenProvider, HouseholdId, OAuthAccountId};
use fa_store::NewOccurrence;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::expand::{ExpandOptions, expand, local_to_utc};

/// Endpoints and transport options of the outbound connectors.
#[derive(Debug, Clone)]
pub struct ConnectorConfig {
    /// Allows plain `http://` feeds (LAN CalDAV servers and tests).
    pub allow_http: bool,
    /// Allows loopback, LAN and other private addresses (C-05).
    pub allow_private_hosts: bool,
    /// Base URL of the Google APIs (`https://www.googleapis.com`).
    pub google_api_base: String,
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            allow_http: false,
            allow_private_hosts: false,
            google_api_base: fa_connectors::google::DEFAULT_API_BASE.to_owned(),
        }
    }
}

#[derive(Clone)]
pub enum Creds {
    Google(OAuthAccountId),
    Caldav {
        url: String,
        user: String,
        pass: String,
    },
    Ics {
        url: String,
    },
}

impl Creds {
    pub fn secrets(&self) -> Vec<&str> {
        match self {
            Creds::Google(_) => vec![],
            Creds::Caldav { pass, url, .. } => vec![pass.as_str(), url.as_str()],
            Creds::Ics { url } => vec![url.as_str()],
        }
    }
}

pub const ICS_CALENDAR_ID: &str = "ics";

/// How long a calendar may go without a full fetch even if the remote reports no change (the expansion
/// window moves forward with time).
pub const FULL_REFRESH: Duration = Duration::hours(24);

#[derive(Clone)]
pub struct Remote {
    pub http: Http,
    pub cfg: ConnectorConfig,
    pub tokens: Arc<dyn AccessTokenProvider>,
}

/// What one calendar fetch produced.
#[derive(Debug, Default)]
pub struct Fetched {
    /// Nothing changed remotely.
    pub skip: bool,
    pub occurrences: Vec<NewOccurrence>,
    /// Incremental deletions `(uid, instance_key)`.
    pub deletes: Vec<(String, String)>,
    /// The occurrences are the complete picture of the window (so unseen rows are deleted).
    pub full: bool,
    /// Drop every synced event of the calendar first (Google `410 Gone`).
    pub reset: bool,
    pub new_state: Option<String>,
    pub warnings: Vec<String>,
}

/// What the previous run left on the calendar row.
#[derive(Debug, Clone, Default)]
pub struct Prior {
    pub sync_state: Option<String>,
    pub last_full_sync_at: Option<DateTime<Utc>>,
}

impl Prior {
    fn fresh(&self, now: DateTime<Utc>) -> bool {
        self.last_full_sync_at
            .is_some_and(|t| now - t < FULL_REFRESH)
    }
}

#[derive(Serialize, Deserialize, Default)]
struct IcsState {
    etag: Option<String>,
    last_modified: Option<String>,
}

pub struct Span {
    pub window: Window,
    pub tz: Tz,
}

impl Span {
    fn opts(&self) -> ExpandOptions {
        ExpandOptions {
            household_tz: self.tz,
            from: self.window.from,
            to: self.window.to,
        }
    }
}

impl Remote {
    async fn token(&self, household: HouseholdId, account: OAuthAccountId) -> Result<String> {
        Ok(self.tokens.access_token(household, account).await?)
    }

    fn caldav(&self, url: &str, user: &str, pass: &str) -> Result<CaldavClient> {
        Ok(CaldavClient::new(self.http.clone(), url, user, pass)?)
    }

    pub async fn discover(
        &self,
        household: HouseholdId,
        creds: &Creds,
        source_name: &str,
    ) -> Result<Vec<RemoteCalendar>> {
        Ok(match creds {
            Creds::Google(acc) => {
                let t = self.token(household, *acc).await?;
                calendar::list_calendars(&self.http, &self.cfg.google_api_base, &t).await?
            }
            Creds::Caldav { url, user, pass } => self.caldav(url, user, pass)?.discover().await?,
            Creds::Ics { .. } => vec![RemoteCalendar {
                remote_id: ICS_CALENDAR_ID.into(),
                name: source_name.to_owned(),
                ctag: None,
            }],
        })
    }

    pub async fn fetch(
        &self,
        household: HouseholdId,
        creds: &Creds,
        remote: &RemoteCalendar,
        prior: &Prior,
        span: &Span,
        now: DateTime<Utc>,
    ) -> Result<Fetched> {
        match creds {
            Creds::Google(acc) => {
                self.fetch_google(household, *acc, remote, prior, span, now)
                    .await
            }
            Creds::Caldav { url, user, pass } => {
                let ctag = remote.ctag.clone();
                if ctag.is_some() && prior.sync_state == ctag && prior.fresh(now) {
                    return Ok(Fetched {
                        skip: true,
                        ..Default::default()
                    });
                }
                let blobs = self
                    .caldav(url, user, pass)?
                    .fetch(remote, &span.window)
                    .await?;
                let mut warnings = Vec::new();
                let mut comps: Vec<ParsedComponent> = Vec::new();
                for b in blobs {
                    match ics::parse(&b) {
                        Ok(c) => comps.extend(c),
                        Err(_) => warnings.push("one calendar object could not be parsed".into()),
                    }
                }
                let mut f = expand_full(&comps, span);
                f.warnings.extend(warnings);
                f.new_state = ctag;
                Ok(f)
            }
            Creds::Ics { url } => {
                let st: IcsState = if prior.fresh(now) {
                    prior
                        .sync_state
                        .as_deref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default()
                } else {
                    IcsState::default()
                };
                match ics::fetch(
                    &self.http,
                    url,
                    st.etag.as_deref(),
                    st.last_modified.as_deref(),
                )
                .await?
                {
                    IcsFetch::NotModified => Ok(Fetched {
                        skip: true,
                        ..Default::default()
                    }),
                    IcsFetch::Fetched(f) => {
                        let comps = ics::parse(&f.body)?;
                        let mut out = expand_full(&comps, span);
                        out.new_state = serde_json::to_string(&IcsState {
                            etag: f.etag,
                            last_modified: f.last_modified,
                        })
                        .ok();
                        Ok(out)
                    }
                }
            }
        }
    }

    async fn fetch_google(
        &self,
        household: HouseholdId,
        account: OAuthAccountId,
        remote: &RemoteCalendar,
        prior: &Prior,
        span: &Span,
        now: DateTime<Utc>,
    ) -> Result<Fetched> {
        let token = self.token(household, account).await?;
        let base = &self.cfg.google_api_base;
        let mut reset = false;
        if let (Some(st), true) = (&prior.sync_state, prior.fresh(now)) {
            match calendar::list_events(
                &self.http,
                base,
                &token,
                &remote.remote_id,
                &EventQuery::SyncToken(st.clone()),
            )
            .await
            {
                Ok(page) => {
                    let mut f = Fetched {
                        new_state: page.next_sync_token.or_else(|| Some(st.clone())),
                        ..Default::default()
                    };
                    for ev in page.items {
                        if ev.cancelled {
                            f.deletes.push((ev.uid, ev.instance_key));
                        } else {
                            let o = google_occurrence(ev, span.tz);
                            if o.end_at > span.window.from && o.start_at < span.window.to {
                                f.occurrences.push(o);
                            }
                        }
                    }
                    return Ok(f);
                }
                Err(ConnectorError::Gone) => reset = true,
                Err(e) => return Err(e.into()),
            }
        }
        let page = calendar::list_events(
            &self.http,
            base,
            &token,
            &remote.remote_id,
            &EventQuery::Window(span.window),
        )
        .await?;
        Ok(Fetched {
            occurrences: page
                .items
                .into_iter()
                .filter(|e| !e.cancelled)
                .map(|e| google_occurrence(e, span.tz))
                .collect(),
            full: true,
            reset,
            new_state: page.next_sync_token,
            ..Default::default()
        })
    }

    /// A few upcoming events of the first calendar, for "test connection".
    pub async fn sample(
        &self,
        household: HouseholdId,
        creds: &Creds,
        remote: &RemoteCalendar,
        span: &Span,
    ) -> Result<Vec<NewOccurrence>> {
        let mut f = self
            .fetch(
                household,
                creds,
                remote,
                &Prior::default(),
                span,
                Utc::now(),
            )
            .await?;
        f.occurrences.sort_by_key(|o| o.start_at);
        f.occurrences.truncate(3);
        Ok(f.occurrences)
    }
}

fn expand_full(comps: &[ParsedComponent], span: &Span) -> Fetched {
    let e = expand(comps, &span.opts());
    Fetched {
        occurrences: e.occurrences,
        warnings: e.warnings,
        full: true,
        ..Default::default()
    }
}

pub fn google_occurrence(ev: RemoteEvent, tz: Tz) -> NewOccurrence {
    let (start_at, end_at, all_day, start_date, end_date) = match (ev.start, ev.end) {
        (EventTime::Date(s), end) => {
            let e = match end {
                EventTime::Date(e) if e > s => e,
                _ => s + Duration::days(1),
            };
            (
                local_to_utc(s.and_time(chrono::NaiveTime::MIN), tz),
                local_to_utc(e.and_time(chrono::NaiveTime::MIN), tz),
                true,
                Some(s),
                Some(e),
            )
        }
        (EventTime::DateTime(s), EventTime::DateTime(e)) => (s, e.max(s), false, None, None),
        (EventTime::DateTime(s), EventTime::Date(_)) => (s, s, false, None, None),
    };
    NewOccurrence {
        remote_uid: ev.uid,
        instance_key: ev.instance_key,
        title: ev.title,
        description: ev.description,
        location: ev.location,
        start_at,
        end_at,
        all_day,
        start_date,
        end_date,
        source_tz: None,
        is_private: ev.private,
        cancelled: false,
        remote_etag: ev.etag,
        external_url: ev.external_url,
    }
}

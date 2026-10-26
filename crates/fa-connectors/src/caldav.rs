use std::collections::HashMap;

use chrono::{DateTime, Utc};
use quick_xml::{Reader, events::Event};
use reqwest::Method;
use url::Url;

use crate::{ConnectorError, Http, RemoteCalendar, Window, error::Result};

const PROPFIND_PRINCIPAL: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:"><d:prop><d:current-user-principal/></d:prop></d:propfind>"#;

const PROPFIND_HOME: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><c:calendar-home-set/></d:prop></d:propfind>"#;

const PROPFIND_COLLECTIONS: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/" xmlns:ic="http://apple.com/ns/ical/"><d:prop><d:resourcetype/><d:displayname/><ic:calendar-color/><cs:getctag/></d:prop></d:propfind>"#;

const PROPFIND_CTAG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:" xmlns:cs="http://calendarserver.org/ns/"><d:prop><cs:getctag/></d:prop></d:propfind>"#;

/// One property of a multistatus response entry.
#[derive(Debug, Default, Clone)]
pub struct Prop {
    pub text: String,
    pub hrefs: Vec<String>,
    pub children: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct DavResponse {
    pub href: String,
    /// Properties from `200 OK` propstats, keyed by local name.
    pub props: HashMap<String, Prop>,
}

/// Parses a WebDAV multistatus body. Namespace prefixes are ignored (local names only).
pub fn parse_multistatus(xml: &str) -> Result<Vec<DavResponse>> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<String> = Vec::new();
    let mut out = Vec::new();
    let mut cur = DavResponse::default();
    let mut propstat: HashMap<String, Prop> = HashMap::new();
    let mut status_ok = false;

    fn name(local: &str) -> String {
        local.to_owned()
    }

    loop {
        let ev = reader
            .read_event()
            .map_err(|e| ConnectorError::Parse(format!("xml: {e}")))?;
        let push_text = |stack: &Vec<String>,
                         s: &str,
                         propstat: &mut HashMap<String, Prop>,
                         cur: &mut DavResponse,
                         status_ok: &mut bool| {
            let Some(last) = stack.last() else { return };
            if stack.len() == 3 && last == "href" {
                cur.href.push_str(s);
            } else if stack.len() == 4 && last == "status" {
                *status_ok = s.contains(" 200");
            } else if stack.len() >= 5
                && let Some(p) = propstat.get_mut(&stack[4])
            {
                if last == "href"
                    && let Some(h) = p.hrefs.last_mut()
                {
                    h.push_str(s);
                }
                p.text.push_str(s);
            }
        };
        match ev {
            Event::Start(e) => {
                let n = name(e.local_name().as_ref());
                stack.push(n.clone());
                match stack.len() {
                    2 if n == "response" => cur = DavResponse::default(),
                    3 if n == "propstat" => {
                        propstat.clear();
                        status_ok = false;
                    }
                    5 => {
                        propstat.insert(n, Prop::default());
                    }
                    l if l > 5 => {
                        if let Some(p) = propstat.get_mut(&stack[4]) {
                            if n == "href" {
                                p.hrefs.push(String::new());
                            }
                            p.children.push(n);
                        }
                    }
                    _ => {}
                }
            }
            Event::Empty(e) => {
                let n = name(e.local_name().as_ref());
                match stack.len() {
                    4 => {
                        propstat.insert(n, Prop::default());
                    }
                    l if l >= 5 => {
                        if let Some(p) = propstat.get_mut(&stack[4]) {
                            p.children.push(n);
                        }
                    }
                    _ => {}
                }
            }
            Event::End(_) => {
                let n = stack.pop().unwrap_or_default();
                match (stack.len(), n.as_str()) {
                    (2, "propstat") => {
                        if status_ok {
                            cur.props.extend(propstat.drain());
                        }
                        propstat.clear();
                    }
                    (1, "response") => out.push(std::mem::take(&mut cur)),
                    _ => {}
                }
            }
            Event::Text(t) => push_text(&stack, &t, &mut propstat, &mut cur, &mut status_ok),
            Event::CData(t) => push_text(&stack, &t, &mut propstat, &mut cur, &mut status_ok),
            Event::GeneralRef(r) => {
                let s = match r.resolve_char_ref() {
                    Ok(Some(c)) => c.to_string(),
                    _ => match &*r {
                        "lt" => "<".into(),
                        "gt" => ">".into(),
                        "amp" => "&".into(),
                        "quot" => "\"".into(),
                        "apos" => "'".into(),
                        _ => String::new(),
                    },
                };
                push_text(&stack, &s, &mut propstat, &mut cur, &mut status_ok);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out)
}

fn ical_time(t: DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

fn report_body(w: &Window) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><d:getetag/><c:calendar-data/></d:prop><c:filter><c:comp-filter name="VCALENDAR"><c:comp-filter name="VEVENT"><c:time-range start="{}" end="{}"/></c:comp-filter></c:comp-filter></c:filter></c:calendar-query>"#,
        ical_time(w.from),
        ical_time(w.to)
    )
}

#[derive(Clone)]
pub struct CaldavClient {
    http: Http,
    base: Url,
    user: String,
    pass: String,
}

impl CaldavClient {
    pub fn new(http: Http, base: &str, user: &str, pass: &str) -> Result<Self> {
        Ok(Self {
            base: http.parse_url(base)?,
            http,
            user: user.to_owned(),
            pass: pass.to_owned(),
        })
    }

    async fn dav(
        &self,
        method: &str,
        url: &Url,
        depth: &str,
        body: String,
    ) -> Result<(Url, String)> {
        let m = Method::from_bytes(method.as_bytes())
            .map_err(|e| ConnectorError::Parse(e.to_string()))?;
        let rb = self
            .http
            .request(m, url)?
            .basic_auth(&self.user, Some(&self.pass))
            .header("Depth", depth)
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(body);
        let resp = self.http.send(rb).await?;
        let final_url = resp.url().clone();
        let bytes = self.http.ok_body(resp).await?;
        Ok((final_url, String::from_utf8_lossy(&bytes).into_owned()))
    }

    fn resolve(&self, from: &Url, href: &str) -> Result<Url> {
        let u = from
            .join(href)
            .map_err(|e| ConnectorError::Parse(format!("bad href: {e}")))?;
        self.http.check_url(&u)?;
        Ok(u)
    }

    /// Stored id: the path (and query) when on the client's origin, else the absolute URL.
    fn remote_id(&self, u: &Url) -> String {
        if u.origin() == self.base.origin() {
            match u.query() {
                Some(q) => format!("{}?{q}", u.path()),
                None => u.path().to_owned(),
            }
        } else {
            u.to_string()
        }
    }

    pub async fn discover(&self) -> Result<Vec<RemoteCalendar>> {
        // 1. Start point: /.well-known/caldav if it answers, else the base URL.
        let well_known = self
            .base
            .join("/.well-known/caldav")
            .map_err(|e| ConnectorError::Parse(e.to_string()))?;
        let (start, body) = match self
            .dav("PROPFIND", &well_known, "0", PROPFIND_PRINCIPAL.into())
            .await
        {
            Ok(r) => r,
            Err(ConnectorError::Unauthorized) => return Err(ConnectorError::Unauthorized),
            Err(_) => {
                self.dav("PROPFIND", &self.base, "0", PROPFIND_PRINCIPAL.into())
                    .await?
            }
        };
        // 2. current-user-principal (or the start itself).
        let principal = parse_multistatus(&body)?
            .into_iter()
            .find_map(|r| {
                r.props
                    .get("current-user-principal")
                    .and_then(|p| p.hrefs.first().cloned())
            })
            .map(|h| self.resolve(&start, &h))
            .transpose()?
            .unwrap_or(start);
        // 3. calendar-home-set.
        let (purl, body) = self
            .dav("PROPFIND", &principal, "0", PROPFIND_HOME.into())
            .await?;
        let home = parse_multistatus(&body)?
            .into_iter()
            .find_map(|r| {
                r.props
                    .get("calendar-home-set")
                    .and_then(|p| p.hrefs.first().cloned())
            })
            .map(|h| self.resolve(&purl, &h))
            .transpose()?
            .unwrap_or(purl);
        // 4. Depth-1 listing of calendar collections.
        let (hurl, body) = self
            .dav("PROPFIND", &home, "1", PROPFIND_COLLECTIONS.into())
            .await?;
        let mut out = Vec::new();
        for r in parse_multistatus(&body)? {
            let is_cal = r
                .props
                .get("resourcetype")
                .is_some_and(|p| p.children.iter().any(|c| c == "calendar"));
            if !is_cal {
                continue;
            }
            let url = self.resolve(&hurl, &r.href)?;
            let text = |k: &str| {
                r.props
                    .get(k)
                    .map(|p| p.text.trim().to_owned())
                    .filter(|s| !s.is_empty())
            };
            out.push(RemoteCalendar {
                name: text("displayname").unwrap_or_else(|| {
                    r.href
                        .trim_end_matches('/')
                        .rsplit('/')
                        .next()
                        .unwrap_or("")
                        .to_owned()
                }),
                color: text("calendar-color"),
                ctag: text("getctag"),
                remote_id: self.remote_id(&url),
            });
        }
        Ok(out)
    }

    pub async fn ctag(&self, cal: &RemoteCalendar) -> Result<Option<String>> {
        let url = self.resolve(&self.base, &cal.remote_id)?;
        let (_, body) = self
            .dav("PROPFIND", &url, "0", PROPFIND_CTAG.into())
            .await?;
        Ok(parse_multistatus(&body)?.into_iter().find_map(|r| {
            r.props
                .get("getctag")
                .map(|p| p.text.trim().to_owned())
                .filter(|s| !s.is_empty())
        }))
    }

    /// Raw VCALENDAR blobs for events overlapping the window.
    pub async fn fetch(&self, cal: &RemoteCalendar, window: &Window) -> Result<Vec<String>> {
        let url = self.resolve(&self.base, &cal.remote_id)?;
        let (_, body) = self.dav("REPORT", &url, "1", report_body(window)).await?;
        Ok(parse_multistatus(&body)?
            .into_iter()
            .filter_map(|r| r.props.get("calendar-data").map(|p| p.text.clone()))
            .filter(|t| !t.trim().is_empty())
            .collect())
    }

    /// Collection URL with a trailing slash; `calendar_url` is a stored remote id or absolute URL.
    fn collection(&self, calendar_url: &str) -> Result<Url> {
        let mut u = self.resolve(&self.base, calendar_url)?;
        if !u.path().ends_with('/') {
            let p = format!("{}/", u.path());
            u.set_path(&p);
        }
        Ok(u)
    }

    /// `PUT {calendar}/{uid}.ics`. Returns the new ETag when the server sends one.
    /// A failed precondition (`If-None-Match: *` on an existing event, stale `If-Match`) is
    /// `Http(412)`.
    pub async fn put_event(
        &self,
        calendar_url: &str,
        uid: &str,
        ics_text: &str,
        if_none_match: bool,
        if_match: Option<&str>,
    ) -> Result<Option<String>> {
        let mut url = self.collection(calendar_url)?;
        url.path_segments_mut()
            .map_err(|_| ConnectorError::Parse("bad calendar url".into()))?
            .pop_if_empty()
            .push(&format!("{uid}.ics"));
        let mut rb = self
            .http
            .request(Method::PUT, &url)?
            .basic_auth(&self.user, Some(&self.pass))
            .header("Content-Type", "text/calendar; charset=utf-8")
            .body(ics_text.to_owned());
        if if_none_match {
            rb = rb.header("If-None-Match", "*");
        }
        if let Some(m) = if_match {
            rb = rb.header("If-Match", m);
        }
        let resp = self.http.send(rb).await?;
        Http::check_status(resp.status())?;
        Ok(resp
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned))
    }

    /// `DELETE` of an event resource; an already-missing event (404/410) counts as done.
    pub async fn delete_event(&self, href: &str, etag: Option<&str>) -> Result<()> {
        let url = self.resolve(&self.base, href)?;
        let mut rb = self
            .http
            .request(Method::DELETE, &url)?
            .basic_auth(&self.user, Some(&self.pass));
        if let Some(e) = etag {
            rb = rb.header("If-Match", e);
        }
        let resp = self.http.send(rb).await?;
        if matches!(resp.status().as_u16(), 404 | 410) {
            return Ok(());
        }
        Http::check_status(resp.status())
    }
}

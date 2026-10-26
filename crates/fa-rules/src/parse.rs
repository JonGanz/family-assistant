//! RFC 822 parsing into the pieces the pipeline stores and the rules read.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use mail_parser::{MessageParser, PartKind};

use crate::types::AttachmentFacts;
use crate::{Error, Result};

/// PDFs larger than this are not kept in memory (Ingest.md 4.8).
pub const MAX_PDF_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ParsedAttachment {
    pub part: String,
    pub filename: Option<String>,
    pub mime: String,
    pub size: usize,
    /// Whole text of a `text/calendar` part or `.ics` file.
    pub ics: Option<String>,
    /// PDF bytes up to 5 MB, for transient bill extraction. Never stored.
    pub pdf: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedMessage {
    /// Lower-cased.
    pub from_addr: String,
    pub from_name: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub date: Option<DateTime<Utc>>,
    pub message_id: Option<String>,
    pub in_reply_to: Vec<String>,
    pub references: Vec<String>,
    pub body_text: String,
    /// Sanitised: scripts (other than JSON-LD), frames and plugins removed. Markup is kept
    /// for parsing only; the app never renders it as HTML.
    pub body_html: Option<String>,
    /// Lower-case names: `list-id`, `list-unsubscribe`, `precedence`, `auto-submitted`,
    /// `reply-to`, `in-reply-to`.
    pub headers: HashMap<String, String>,
    pub attachments: Vec<ParsedAttachment>,
}

impl ParsedMessage {
    pub fn attachment_facts(&self) -> Vec<AttachmentFacts> {
        self.attachments
            .iter()
            .map(|a| AttachmentFacts {
                part: a.part.clone(),
                filename: a.filename.clone(),
                mime: a.mime.clone(),
                size: a.size,
                ics: a.ics.clone(),
            })
            .collect()
    }
}

const KEPT_HEADERS: [&str; 6] = [
    "List-Id",
    "List-Unsubscribe",
    "Precedence",
    "Auto-Submitted",
    "Reply-To",
    "In-Reply-To",
];

pub fn parse(raw: &[u8]) -> Result<ParsedMessage> {
    if raw.iter().all(u8::is_ascii_whitespace) {
        return Err(Error::Parse);
    }
    let msg = MessageParser::new().parse(raw).ok_or(Error::Parse)?;
    let mut out = ParsedMessage::default();

    if let Some(m) = msg.from().and_then(|a| a.first()) {
        out.from_addr = m
            .address()
            .map(fa_domain::normalise_address)
            .unwrap_or_default();
        out.from_name = m
            .name()
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty());
    }
    let addrs = |list: Option<mail_parser::AddressList<'_>>| -> Vec<String> {
        list.map(|l| {
            l.mailboxes()
                .filter_map(|m| m.address())
                .map(fa_domain::normalise_address)
                .filter(|a| !a.is_empty())
                .collect()
        })
        .unwrap_or_default()
    };
    out.to = addrs(msg.to());
    out.cc = addrs(msg.cc());
    out.subject = msg.subject().unwrap_or_default().trim().to_owned();
    out.date = msg
        .date()
        .and_then(|d| DateTime::from_timestamp(d.to_timestamp(), 0));
    out.message_id = msg.message_id().map(str::to_owned);
    out.in_reply_to = msg
        .in_reply_to()
        .map(|l| l.iter().map(str::to_owned).collect())
        .unwrap_or_default();
    out.references = msg
        .references()
        .map(|l| l.iter().map(str::to_owned).collect())
        .unwrap_or_default();

    for name in KEPT_HEADERS {
        if let Some(h) = msg.headers().get(name) {
            let v = String::from_utf8_lossy(h.raw_value());
            let v = crate::text::collapse_ws(&v);
            if !v.is_empty() {
                out.headers.insert(name.to_lowercase(), v);
            }
        }
    }

    let mut text: Option<String> = None;
    let mut html: Option<String> = None;
    for part in msg.text_body() {
        match part.kind() {
            PartKind::Text if text.is_none() && !is_calendar(&part) => {
                text = part.text().map(|t| t.into_owned());
            }
            PartKind::Html if html.is_none() => {
                html = part.html_utf8().map(|t| t.into_owned());
            }
            _ => {}
        }
    }
    for part in msg.html_body() {
        if html.is_none()
            && let PartKind::Html = part.kind()
        {
            html = part.html_utf8().map(|t| t.into_owned());
        }
    }
    let html = html
        .map(|h| sanitise_html(&h))
        .filter(|h| !h.trim().is_empty());
    out.body_text = match text.filter(|t| !t.trim().is_empty()) {
        Some(t) => t,
        None => html.as_deref().map(html_to_text).unwrap_or_default(),
    };
    out.body_html = html;

    for part in msg.parts() {
        if matches!(part.kind(), PartKind::Multipart | PartKind::Message(_)) {
            continue;
        }
        let mime = part
            .content_type()
            .map(|c| format!("{}/{}", c.ctype(), c.subtype().unwrap_or("octet-stream")))
            .unwrap_or_else(|| "text/plain".into())
            .to_lowercase();
        let filename = part.attachment_name().map(str::to_owned);
        let fname_lc = filename.as_deref().unwrap_or_default().to_lowercase();
        let is_ics =
            mime == "text/calendar" || mime == "application/ics" || fname_lc.ends_with(".ics");
        let is_pdf = mime == "application/pdf" || fname_lc.ends_with(".pdf");
        if !(part.is_attachment() || is_ics || is_pdf) {
            continue;
        }
        let size = part.decoded_len();
        out.attachments.push(ParsedAttachment {
            part: part.id().to_string(),
            filename,
            ics: is_ics.then(|| String::from_utf8_lossy(&part.decoded()).into_owned()),
            pdf: (is_pdf && size <= MAX_PDF_BYTES).then(|| part.decoded().into_owned()),
            mime,
            size,
        });
    }
    Ok(out)
}

fn is_calendar(part: &mail_parser::MessagePart<'_>) -> bool {
    part.is_content_type("text", "calendar")
}

/// Plain text of an HTML body at width 100, without link footnotes.
pub(crate) fn html_to_text(html: &str) -> String {
    html2text::config::plain_no_decorate()
        .string_from_read(html.as_bytes(), 100)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// Removes `<script>` (except JSON-LD), `<iframe>`, `<object>`, `<embed>` blocks.
pub(crate) fn sanitise_html(html: &str) -> String {
    let mut s = html.to_owned();
    for tag in ["script", "iframe", "object", "embed"] {
        s = strip_blocks(&s, tag);
    }
    s
}

fn strip_blocks(html: &str, tag: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}");
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    while let Some(rel) = lower[i..].find(&open) {
        let start = i + rel;
        let after = lower[start + open.len()..].chars().next();
        if after.is_some_and(|c| c.is_ascii_alphanumeric()) {
            out.push_str(&html[i..start + open.len()]);
            i = start + open.len();
            continue;
        }
        let Some(tag_end) = lower[start..].find('>').map(|e| start + e + 1) else {
            out.push_str(&html[i..start]);
            i = html.len();
            break;
        };
        let keep = tag == "script" && lower[start..tag_end].contains("ld+json");
        let end = match lower[tag_end..].find(&close) {
            Some(c) => {
                let c = tag_end + c;
                lower[c..].find('>').map_or(html.len(), |e| c + e + 1)
            }
            None => html.len(),
        };
        out.push_str(&html[i..start]);
        if keep {
            out.push_str(&html[start..end]);
        }
        i = end;
    }
    out.push_str(&html[i..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_scripts_but_keeps_json_ld() {
        let h = r#"<p>a</p><script>alert(1)</script><SCRIPT type="application/ld+json">{"a":1}</SCRIPT><iframe src=x></iframe><p>b</p>"#;
        let s = sanitise_html(h);
        assert!(!s.contains("alert"));
        assert!(!s.contains("iframe"));
        assert!(s.contains(r#"{"a":1}"#));
        assert!(s.contains("<p>b</p>"));
    }

    #[test]
    fn simple_message() {
        let raw = b"From: Ann Teacher <Ann@Example.edu>\r\nTo: pat@example.com\r\nSubject: Hello\r\nMessage-ID: <1@example.edu>\r\nList-Unsubscribe: <mailto:x@example.edu>\r\nDate: Tue, 28 Oct 2026 09:00:00 +0000\r\n\r\nHi there\r\n";
        let m = parse(raw).unwrap();
        assert_eq!(m.from_addr, "ann@example.edu");
        assert_eq!(m.from_name.as_deref(), Some("Ann Teacher"));
        assert_eq!(m.to, vec!["pat@example.com"]);
        assert_eq!(m.subject, "Hello");
        assert!(m.headers.contains_key("list-unsubscribe"));
        assert!(m.body_text.contains("Hi there"));
        assert!(m.date.is_some());
    }

    #[test]
    fn broken_inputs_never_panic() {
        let cases: Vec<Vec<u8>> = vec![
            vec![],
            b"\r\n\r\n".to_vec(),
            b"garbage without headers".to_vec(),
            vec![0xff, 0xfe, 0x00, 0x01, 0x80, 0x81],
            b"From: <<<>>>\r\nTo: ,,,;;;\r\nSubject: =?utf-8?q?=ZZ?=\r\n\r\nbody".to_vec(),
            b"Content-Type: multipart/mixed; boundary=\"x\"\r\n\r\n--x\r\nContent-Type: text/calendar\r\n\r\nBEGIN:VCALENDAR\r\n--x".to_vec(),
            b"Content-Type: multipart/alternative; boundary=\r\n\r\n--\r\n".to_vec(),
            b"Content-Type: text/html\r\n\r\n<p><b>unclosed <script>".to_vec(),
            b"Date: not a date\r\nFrom: a\r\n\r\n".to_vec(),
            "Subject: ".repeat(10_000).into_bytes(),
        ];
        for c in cases {
            let _ = parse(&c);
        }
        let mut deep = b"Content-Type: multipart/mixed; boundary=b\r\n\r\n".to_vec();
        for _ in 0..500 {
            deep.extend_from_slice(b"--b\r\nContent-Type: multipart/mixed; boundary=b\r\n\r\n");
        }
        let _ = parse(&deep);
    }
}

//! Gmail REST client (`gmail/v1`). Functions take `(&Http, base, access_token, …)`.
//! Never logs message content, ids of people, or tokens.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::Deserialize;
use serde_json::json;

use super::{api_url, ok_body, parse_json};
use crate::{ConnectorError, Http, error::Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub email: String,
    pub history_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MsgRef {
    pub id: String,
    #[serde(rename = "threadId", default)]
    pub thread_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryPage {
    pub added: Vec<MsgRef>,
    /// `(message id, labels added)`.
    pub labels_added: Vec<(String, Vec<String>)>,
    pub labels_removed: Vec<(String, Vec<String>)>,
    pub deleted: Vec<String>,
    pub history_id: Option<String>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMessage {
    pub id: String,
    pub thread_id: String,
    pub label_ids: Vec<String>,
    pub internal_date: DateTime<Utc>,
    pub raw: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Label {
    pub id: String,
    pub name: String,
}

fn endpoint(base: &str, tail: &[&str]) -> Result<url::Url> {
    let mut seg = vec!["gmail", "v1", "users", "me"];
    seg.extend_from_slice(tail);
    api_url(base, &seg)
}

async fn get(http: &Http, url: &url::Url, access: &str) -> Result<bytes::Bytes> {
    let resp = http.send(http.get(url)?.bearer_auth(access)).await?;
    ok_body(http, resp).await
}

fn b64(s: &str) -> Result<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(s.trim_end_matches('='))
        .map_err(|e| ConnectorError::Parse(format!("base64: {e}")))
}

pub async fn get_profile(http: &Http, base: &str, access: &str) -> Result<Profile> {
    #[derive(Deserialize)]
    struct P {
        #[serde(rename = "emailAddress")]
        email: String,
        #[serde(rename = "historyId")]
        history_id: String,
    }
    let p: P = parse_json(&get(http, &endpoint(base, &["profile"])?, access).await?)?;
    Ok(Profile {
        email: p.email,
        history_id: p.history_id,
    })
}

/// One page of `messages.list`; pass the returned token to get the next page.
pub async fn list_messages(
    http: &Http,
    base: &str,
    access: &str,
    q: &str,
    page_token: Option<&str>,
) -> Result<(Vec<MsgRef>, Option<String>)> {
    #[derive(Deserialize)]
    struct L {
        #[serde(default)]
        messages: Vec<MsgRef>,
        #[serde(rename = "nextPageToken")]
        next: Option<String>,
    }
    let mut url = endpoint(base, &["messages"])?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("maxResults", "500");
        if !q.is_empty() {
            qp.append_pair("q", q);
        }
        if let Some(t) = page_token {
            qp.append_pair("pageToken", t);
        }
    }
    let l: L = parse_json(&get(http, &url, access).await?)?;
    Ok((l.messages, l.next))
}

/// One page of `history.list`. A 404 means the start id is too old: `HistoryExpired`.
pub async fn list_history(
    http: &Http,
    base: &str,
    access: &str,
    start_history_id: &str,
    page_token: Option<&str>,
) -> Result<HistoryPage> {
    #[derive(Deserialize)]
    struct MsgWrap {
        message: MsgRef,
    }
    #[derive(Deserialize)]
    struct LabelWrap {
        message: MsgRef,
        #[serde(rename = "labelIds", default)]
        label_ids: Vec<String>,
    }
    #[derive(Deserialize, Default)]
    struct Rec {
        #[serde(rename = "messagesAdded", default)]
        added: Vec<MsgWrap>,
        #[serde(rename = "messagesDeleted", default)]
        deleted: Vec<MsgWrap>,
        #[serde(rename = "labelsAdded", default)]
        labels_added: Vec<LabelWrap>,
        #[serde(rename = "labelsRemoved", default)]
        labels_removed: Vec<LabelWrap>,
    }
    #[derive(Deserialize)]
    struct H {
        #[serde(default)]
        history: Vec<Rec>,
        #[serde(rename = "historyId")]
        history_id: Option<String>,
        #[serde(rename = "nextPageToken")]
        next: Option<String>,
    }
    let mut url = endpoint(base, &["history"])?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("startHistoryId", start_history_id);
        qp.append_pair("maxResults", "500");
        for t in [
            "messageAdded",
            "messageDeleted",
            "labelAdded",
            "labelRemoved",
        ] {
            qp.append_pair("historyTypes", t);
        }
        if let Some(t) = page_token {
            qp.append_pair("pageToken", t);
        }
    }
    let resp = http.send(http.get(&url)?.bearer_auth(access)).await?;
    if resp.status().as_u16() == 404 {
        return Err(ConnectorError::HistoryExpired);
    }
    let h: H = parse_json(&ok_body(http, resp).await?)?;
    let mut out = HistoryPage {
        history_id: h.history_id,
        next_page_token: h.next,
        ..Default::default()
    };
    for r in h.history {
        out.added.extend(r.added.into_iter().map(|m| m.message));
        out.deleted
            .extend(r.deleted.into_iter().map(|m| m.message.id));
        out.labels_added.extend(
            r.labels_added
                .into_iter()
                .map(|l| (l.message.id, l.label_ids)),
        );
        out.labels_removed.extend(
            r.labels_removed
                .into_iter()
                .map(|l| (l.message.id, l.label_ids)),
        );
    }
    Ok(out)
}

/// `messages.get?format=raw`.
pub async fn get_raw(http: &Http, base: &str, access: &str, id: &str) -> Result<RawMessage> {
    #[derive(Deserialize)]
    struct M {
        id: String,
        #[serde(rename = "threadId", default)]
        thread_id: String,
        #[serde(rename = "labelIds", default)]
        label_ids: Vec<String>,
        #[serde(rename = "internalDate")]
        internal_date: Option<String>,
        raw: String,
    }
    let mut url = endpoint(base, &["messages", id])?;
    url.query_pairs_mut().append_pair("format", "raw");
    let m: M = parse_json(&get(http, &url, access).await?)?;
    let ms: i64 = m
        .internal_date
        .as_deref()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ConnectorError::Parse("missing internalDate".into()))?;
    Ok(RawMessage {
        internal_date: DateTime::from_timestamp_millis(ms)
            .ok_or_else(|| ConnectorError::Parse("bad internalDate".into()))?,
        raw: b64(&m.raw)?,
        id: m.id,
        thread_id: m.thread_id,
        label_ids: m.label_ids,
    })
}

pub async fn get_attachment(
    http: &Http,
    base: &str,
    access: &str,
    message: &str,
    attachment: &str,
) -> Result<Vec<u8>> {
    #[derive(Deserialize)]
    struct A {
        data: String,
    }
    let url = endpoint(base, &["messages", message, "attachments", attachment])?;
    let a: A = parse_json(&get(http, &url, access).await?)?;
    b64(&a.data)
}

/// `messages.modify`: add and remove label ids (needs `gmail.modify`).
pub async fn modify(
    http: &Http,
    base: &str,
    access: &str,
    id: &str,
    add: &[String],
    remove: &[String],
) -> Result<()> {
    let url = endpoint(base, &["messages", id, "modify"])?;
    let rb = http
        .request(Method::POST, &url)?
        .bearer_auth(access)
        .json(&json!({ "addLabelIds": add, "removeLabelIds": remove }));
    let resp = http.send(rb).await?;
    ok_body(http, resp).await.map(|_| ())
}

pub async fn list_labels(http: &Http, base: &str, access: &str) -> Result<Vec<Label>> {
    #[derive(Deserialize)]
    struct L {
        #[serde(default)]
        labels: Vec<Label>,
    }
    let l: L = parse_json(&get(http, &endpoint(base, &["labels"])?, access).await?)?;
    Ok(l.labels)
}

pub async fn create_label(http: &Http, base: &str, access: &str, name: &str) -> Result<Label> {
    let url = endpoint(base, &["labels"])?;
    let rb = http
        .request(Method::POST, &url)?
        .bearer_auth(access)
        .json(&json!({
            "name": name,
            "labelListVisibility": "labelShow",
            "messageListVisibility": "show",
        }));
    let resp = http.send(rb).await?;
    parse_json(&ok_body(http, resp).await?)
}

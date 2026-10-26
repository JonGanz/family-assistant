//! Suggested tasks from action phrases and contact questions (Ingest.md 7, D-060), and
//! contact suggestions (D-056).

use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use fa_domain::entities::Role;
use fa_domain::{MemberId, OrgId};
use regex::Regex;

use crate::dates::find_dates;
use crate::signals::{grade_mentions, grade_or_class_hit, match_org, name_hit};
use crate::text::{WordSet, collapse_ws, sentence_around, truncate_chars};
use crate::types::{Context, MessageFacts};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Action,
    Reply,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedAction {
    pub kind: ActionKind,
    pub title: String,
    pub due_at: Option<DateTime<Utc>>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactHint {
    pub org: OrgId,
    pub email: String,
    pub name: Option<String>,
    pub member: MemberId,
}

struct Phrase {
    text: &'static str,
    kind: ActionKind,
    strong: bool,
}

const PHRASES: [Phrase; 9] = [
    Phrase {
        text: "please sign",
        kind: ActionKind::Action,
        strong: true,
    },
    Phrase {
        text: "permission slip",
        kind: ActionKind::Action,
        strong: true,
    },
    Phrase {
        text: "return by",
        kind: ActionKind::Action,
        strong: true,
    },
    Phrase {
        text: "rsvp by",
        kind: ActionKind::Action,
        strong: true,
    },
    Phrase {
        text: "rsvp",
        kind: ActionKind::Action,
        strong: false,
    },
    Phrase {
        text: "reply by",
        kind: ActionKind::Reply,
        strong: true,
    },
    Phrase {
        text: "please reply",
        kind: ActionKind::Reply,
        strong: false,
    },
    Phrase {
        text: "let me know by",
        kind: ActionKind::Reply,
        strong: false,
    },
    Phrase {
        text: "please return",
        kind: ActionKind::Action,
        strong: true,
    },
];

const MAX_ACTIONS: usize = 5;

static SECOND_PERSON: LazyLock<WordSet> =
    LazyLock::new(|| WordSet::new(&["you", "your", "you're", "yours", "please"]));
static NAME_FROM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\w]+").expect("static regex"));

fn due_in(sentence: &str, f: &MessageFacts<'_>, ctx: &Context) -> Option<DateTime<Utc>> {
    find_dates(sentence, f.received_at, ctx.tz, false)
        .into_iter()
        .next()
        .map(|p| fa_domain::tasks::due_instant(p.date, p.time, ctx.tz))
}

pub fn extract_actions(f: &MessageFacts<'_>, ctx: &Context) -> Vec<ExtractedAction> {
    let text = f.text();
    let lower = text.to_lowercase();
    let mut out: Vec<ExtractedAction> = Vec::new();
    let mut taken: Vec<(usize, usize)> = Vec::new();

    for ph in &PHRASES {
        for (pos, _) in lower.match_indices(ph.text) {
            let end = pos + ph.text.len();
            if !text.is_char_boundary(pos) || !text.is_char_boundary(end) {
                continue;
            }
            // Whole phrase only ("rsvp" inside another word is not one).
            let before = lower[..pos].chars().next_back();
            let after = lower[end..].chars().next();
            if before.is_some_and(char::is_alphanumeric) || after.is_some_and(char::is_alphanumeric)
            {
                continue;
            }
            let sentence = sentence_around(&text, pos, end);
            let s_start = sentence.as_ptr() as usize - text.as_ptr() as usize;
            let range = (s_start, s_start + sentence.len());
            if taken.contains(&range) {
                continue;
            }
            taken.push(range);
            let due_at = due_in(sentence, f, ctx);
            let (with_due, without_due) = if ph.strong { (0.8, 0.7) } else { (0.7, 0.6) };
            out.push(ExtractedAction {
                kind: ph.kind,
                title: truncate_chars(sentence, 80),
                due_at,
                confidence: if due_at.is_some() {
                    with_due
                } else {
                    without_due
                },
            });
            if out.len() >= MAX_ACTIONS {
                return out;
            }
        }
    }

    // A direct question from an organisation contact becomes a reply task.
    if match_org(f, ctx).is_some_and(|m| m.via_contact)
        && let Some(q) = question(&text)
    {
        let title = truncate_chars(q, 80);
        if !out.iter().any(|a| a.title == title) {
            out.push(ExtractedAction {
                kind: ActionKind::Reply,
                title,
                due_at: None,
                confidence: 0.6,
            });
        }
    }
    out
}

fn question(text: &str) -> Option<&str> {
    let mut from = 0;
    while let Some(rel) = text[from..].find('?') {
        let q = from + rel;
        let s = sentence_around(text, q, q + 1);
        if SECOND_PERSON.is_match(s) {
            return Some(s);
        }
        from = q + 1;
    }
    None
}

/// An unknown sender at a known org's domain who mentions a child (D-056).
pub fn contact_hint(f: &MessageFacts<'_>, ctx: &Context) -> Option<ContactHint> {
    let m = match_org(f, ctx)?;
    if m.via_contact {
        return None;
    }
    let from = fa_domain::normalise_address(f.from_addr);
    // A sender listed explicitly on the org is known already.
    if m.org
        .senders
        .iter()
        .any(|s| fa_domain::normalise_address(s) == from)
    {
        return None;
    }
    let text = f.text();
    let mentions = grade_mentions(&text);
    let children = || ctx.members.iter().filter(|c| c.role == Role::Child);
    let member = children().find(|c| name_hit(c, &text)).or_else(|| {
        children().find(|c| {
            c.enrollments.iter().any(|e| e.org == m.org.id)
                && grade_or_class_hit(c, Some(m.org.id), &text, &mentions)
        })
    })?;
    let name = f
        .from_name
        .map(collapse_ws)
        .filter(|n| !n.is_empty() && !NAME_FROM.replace_all(n, "").is_empty());
    Some(ContactHint {
        org: m.org.id,
        email: from,
        name,
        member: member.id,
    })
}

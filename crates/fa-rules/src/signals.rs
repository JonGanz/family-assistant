//! Built-in signals (Ingest.md 6.2) and rule evaluation per decision point.

use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{Duration, NaiveTime};
use fa_domain::entities::Role;
use fa_domain::{Candidate, DecisionPointKey, Gender, MemberId, OrgId, OrgKind};
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};

use crate::dates::find_dates;
use crate::rules::first_match;
use crate::text::{WordSet, collapse_ws, contains_any_word, contains_word};
use crate::types::{Context, MemberCtx, MessageFacts, OrgCtx};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalDef {
    pub key: &'static str,
    pub decision_point: DecisionPointKey,
    pub description: &'static str,
    pub confidence: f32,
}

const fn sig(
    key: &'static str,
    decision_point: DecisionPointKey,
    description: &'static str,
    confidence: f32,
) -> SignalDef {
    SignalDef {
        key,
        decision_point,
        description,
        confidence,
    }
}

use DecisionPointKey::{EmailCategory, EmailIsBill, EmailMemberLink, EmailTimeSensitive};

static SIGNALS: [SignalDef; 15] = [
    sig("gmail_spam", EmailCategory, "Gmail marked it as spam", 0.99),
    sig(
        "gmail_promotions",
        EmailCategory,
        "Gmail's Promotions tab",
        0.85,
    ),
    sig("gmail_social", EmailCategory, "Gmail's Social tab", 0.85),
    sig(
        "org_contact",
        EmailCategory,
        "Sender is a contact of an organisation: its kind",
        0.95,
    ),
    sig(
        "org_sender",
        EmailCategory,
        "Sender's address or domain belongs to an organisation: its kind",
        0.9,
    ),
    sig(
        "bulk_newsletter",
        EmailCategory,
        "List-Unsubscribe or bulk precedence from an unknown sender: newsletters",
        0.8,
    ),
    sig(
        "payment_received",
        EmailIsBill,
        "Payment received, thank you for your payment, autopay scheduled: not a bill",
        0.85,
    ),
    sig(
        "biller_org",
        EmailIsBill,
        "Sender is a biller organisation: a bill",
        0.9,
    ),
    sig(
        "bill_phrases",
        EmailIsBill,
        "Bill phrases together with a currency amount: a bill",
        0.8,
    ),
    sig(
        "contact_enrolled",
        EmailMemberLink,
        "Sender is a contact enrolled with a child: that child",
        0.95,
    ),
    sig(
        "org_child_name",
        EmailMemberLink,
        "Known organisation and a child's name or alias in the text: that child",
        0.9,
    ),
    sig(
        "org_grade_class",
        EmailMemberLink,
        "Known organisation and a grade or class phrase matching an enrollment",
        0.85,
    ),
    sig(
        "org_tag_gender",
        EmailMemberLink,
        "Known organisation and a child's tag or a gender term for the child",
        0.8,
    ),
    sig(
        "org_all_children",
        EmailMemberLink,
        "Known organisation with no child signal: every child enrolled there (falls through to a human)",
        0.6,
    ),
    sig(
        "urgent_dated",
        EmailTimeSensitive,
        "Urgency phrase and a date within 48 hours",
        0.85,
    ),
];

pub fn signals() -> &'static [SignalDef] {
    &SIGNALS
}

fn def(key: &str) -> &'static SignalDef {
    SIGNALS.iter().find(|s| s.key == key).unwrap_or(&SIGNALS[0])
}

fn cand(key: &str, value: Value) -> Candidate {
    Candidate {
        value,
        confidence: def(key).confidence,
        source: key.to_owned(),
    }
}

pub(crate) struct OrgMatch<'c> {
    pub org: &'c OrgCtx,
    pub via_contact: bool,
}

/// The org a sender belongs to: a listed contact beats an explicit sender, which beats a
/// domain match.
pub(crate) fn match_org<'c>(f: &MessageFacts<'_>, ctx: &'c Context) -> Option<OrgMatch<'c>> {
    let from = fa_domain::normalise_address(f.from_addr);
    let by_contact = ctx.orgs.iter().find(|o| {
        o.contacts
            .iter()
            .any(|c| fa_domain::normalise_address(&c.email) == from)
    });
    if let Some(org) = by_contact {
        return Some(OrgMatch {
            org,
            via_contact: true,
        });
    }
    let by_sender = ctx.orgs.iter().find(|o| {
        o.senders
            .iter()
            .any(|s| fa_domain::normalise_address(s) == from)
    });
    let domain = fa_domain::domain_of(&from);
    let found = by_sender.or_else(|| {
        let d = domain?;
        ctx.orgs
            .iter()
            .find(|o| o.domains.iter().any(|od| fa_domain::domain_matches(d, od)))
    });
    found.map(|org| OrgMatch {
        org,
        via_contact: false,
    })
}

fn kind_category(k: OrgKind) -> Option<&'static str> {
    match k {
        OrgKind::School => Some("school"),
        OrgKind::Daycare => Some("daycare"),
        OrgKind::Club => Some("activities"),
        OrgKind::Biller => Some("bills"),
        OrgKind::Other => None,
    }
}

fn category_exists(ctx: &Context, key: &str) -> bool {
    ctx.categories.iter().any(|(k, _)| k == key)
}

// ---- grades ----

fn re(p: &str) -> Regex {
    RegexBuilder::new(p)
        .case_insensitive(true)
        .build()
        .expect("static regex")
}

static GRADE_RANGE: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\b(?:grades?|years?|yrs?)\s*(\d{1,2})\s*(?:-|–|—|to|through|thru)\s*(\d{1,2})\b")
});
static GRADE_SINGLE: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?:grade|year|yr)\s*(\d{1,2}|k)\b"));
static GRADE_ORDINAL: LazyLock<Regex> =
    LazyLock::new(|| re(r"\b(\d{1,2})(?:st|nd|rd|th)[\s-]*(?:grade|graders?|year)\b"));
static PREK: LazyLock<Regex> = LazyLock::new(|| re(r"\bpre[\s-]?k(?:indergarten)?\b"));
static KINDER: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?:kindergarten|kinder)\b"));

/// Pre-K is -1, kindergarten 0.
pub(crate) fn grade_ord(s: &str) -> Option<i32> {
    let s = s.trim().to_lowercase();
    let s = s
        .trim_start_matches("grade")
        .trim_start_matches("year")
        .trim();
    match s {
        "pre-k" | "prek" | "pre k" | "pk" | "pre-kindergarten" | "prekindergarten" => Some(-1),
        "k" | "kindergarten" | "kinder" => Some(0),
        _ => {
            let digits = s.trim_end_matches(|c: char| c.is_alphabetic());
            let n: i32 = digits.parse().ok()?;
            (0..=13).contains(&n).then_some(n)
        }
    }
}

/// Inclusive grade ranges mentioned in `text`.
pub(crate) fn grade_mentions(text: &str) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    let mut taken: Vec<std::ops::Range<usize>> = Vec::new();
    for c in GRADE_RANGE.captures_iter(text) {
        let (Some(w), Some(a), Some(b)) = (c.get(0), c.get(1), c.get(2)) else {
            continue;
        };
        if let (Ok(a), Ok(b)) = (a.as_str().parse::<i32>(), b.as_str().parse::<i32>())
            && a <= b
        {
            out.push((a, b));
            taken.push(w.range());
        }
    }
    let free = |taken: &[std::ops::Range<usize>], r: &std::ops::Range<usize>| {
        !taken.iter().any(|t| t.start < r.end && r.start < t.end)
    };
    for c in GRADE_SINGLE.captures_iter(text) {
        let (Some(w), Some(g)) = (c.get(0), c.get(1)) else {
            continue;
        };
        if free(&taken, &w.range())
            && let Some(n) = grade_ord(g.as_str())
        {
            out.push((n, n));
        }
    }
    for c in GRADE_ORDINAL.captures_iter(text) {
        if let Some(n) = c.get(1).and_then(|g| g.as_str().parse::<i32>().ok()) {
            out.push((n, n));
        }
    }
    let prek: Vec<_> = PREK.find_iter(text).map(|m| m.range()).collect();
    if !prek.is_empty() {
        out.push((-1, -1));
    }
    for m in KINDER.find_iter(text) {
        if !prek
            .iter()
            .any(|p| p.start <= m.start() && m.start() < p.end)
        {
            out.push((0, 0));
        }
    }
    out
}

/// Grade ordinals for a member: the member's own, plus those of enrollments (at `org` when
/// given).
pub(crate) fn member_grades(m: &MemberCtx, org: Option<OrgId>) -> Vec<i32> {
    let mut v: Vec<i32> = m
        .enrollments
        .iter()
        .filter(|e| org.is_none_or(|o| e.org == o))
        .filter_map(|e| e.grade.as_deref().and_then(grade_ord))
        .collect();
    if org.is_none() || v.is_empty() {
        v.extend(m.grade.as_deref().and_then(grade_ord));
    }
    v
}

pub(crate) fn grade_or_class_hit(
    m: &MemberCtx,
    org: Option<OrgId>,
    text: &str,
    mentions: &[(i32, i32)],
) -> bool {
    let grades = member_grades(m, org);
    if grades
        .iter()
        .any(|g| mentions.iter().any(|(lo, hi)| lo <= g && g <= hi))
    {
        return true;
    }
    m.enrollments
        .iter()
        .filter(|e| org.is_none_or(|o| e.org == o))
        .filter_map(|e| e.class_name.as_deref())
        .any(|c| !c.trim().is_empty() && contains_word(text, c))
}

pub(crate) fn name_hit(m: &MemberCtx, text: &str) -> bool {
    let mut names: Vec<&str> = vec![m.name.as_str()];
    names.extend(m.aliases.iter().map(String::as_str));
    if let Some(first) = m.name.split_whitespace().next()
        && first.chars().count() >= 3
    {
        names.push(first);
    }
    contains_any_word(text, &names)
}

pub(crate) fn tag_hit(m: &MemberCtx, text: &str) -> bool {
    contains_any_word(text, &m.tags)
}

pub(crate) fn gender_terms(ctx: &Context, g: Gender) -> &[String] {
    match g {
        Gender::Female => &ctx.gender_terms.female,
        Gender::Male => &ctx.gender_terms.male,
        Gender::Other => &[],
    }
}

pub(crate) fn gender_hit(m: &MemberCtx, ctx: &Context, text: &str) -> bool {
    m.gender
        .is_some_and(|g| contains_any_word(text, gender_terms(ctx, g)))
}

// ---- bills ----

static AMOUNT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"[$£€¥]\s?\d[\d,.]*|\b(?:USD|EUR|GBP|JPY|CAD|AUD|NZD|CHF|MXN|INR|CNY|SEK|NOK|DKK)\s?\d[\d,.]*|\d[\d,.]*\s?(?:USD|EUR|GBP|JPY|CAD|AUD|NZD|CHF|MXN|INR|CNY|SEK|NOK|DKK)\b",
    )
});

pub(crate) fn has_amount(text: &str) -> bool {
    AMOUNT.is_match(text)
}

// ---- evaluation ----

pub fn evaluate(dp: DecisionPointKey, f: &MessageFacts<'_>, ctx: &Context) -> Vec<Candidate> {
    let org = match_org(f, ctx);
    let mut out = Vec::new();

    if let Some(rule) = first_match(dp, f, ctx, org.as_ref().map(|m| m.org)) {
        out.extend(rule_candidates(dp, rule, ctx));
    }

    let enabled = |key: &str| !ctx.disabled_signals.contains(key);
    let mut built: Vec<Candidate> = match dp {
        EmailCategory => category_signals(f, ctx, org.as_ref(), &enabled),
        EmailIsBill => bill_signals(f, org.as_ref(), &enabled),
        EmailMemberLink => member_signals(f, ctx, org.as_ref(), &enabled),
        EmailTimeSensitive => time_signals(f, ctx, &enabled),
        DecisionPointKey::EventCalendarRoute => Vec::new(),
    };
    built.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.extend(built);
    out
}

fn rule_candidates(
    dp: DecisionPointKey,
    rule: &fa_domain::MailRule,
    ctx: &Context,
) -> Vec<Candidate> {
    let source = rule.id.to_string();
    let one = |value: Value| Candidate {
        value,
        confidence: rule.confidence,
        source: source.clone(),
    };
    match dp {
        EmailCategory => match rule.value.as_str() {
            Some(k) if category_exists(ctx, k) => vec![one(rule.value.clone())],
            _ => vec![],
        },
        EmailTimeSensitive | EmailIsBill => {
            if rule.value.is_boolean() {
                vec![one(rule.value.clone())]
            } else {
                vec![]
            }
        }
        EmailMemberLink => match &rule.value {
            Value::String(_) => vec![one(rule.value.clone())],
            Value::Array(a) => a
                .iter()
                .filter(|v| v.is_string())
                .cloned()
                .map(one)
                .collect(),
            _ => vec![],
        },
        DecisionPointKey::EventCalendarRoute => vec![one(rule.value.clone())],
    }
}

fn category_signals(
    f: &MessageFacts<'_>,
    ctx: &Context,
    org: Option<&OrgMatch<'_>>,
    enabled: &dyn Fn(&str) -> bool,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    let has_label = |l: &str| f.gmail_labels.iter().any(|x| x.eq_ignore_ascii_case(l));
    let mut push = |key: &str, cat: &str| {
        if enabled(key) && category_exists(ctx, cat) {
            out.push(cand(key, json!(cat)));
        }
    };
    if has_label("SPAM") {
        push("gmail_spam", "junk");
    }
    if has_label("CATEGORY_PROMOTIONS") {
        push("gmail_promotions", "promotions");
    }
    if has_label("CATEGORY_SOCIAL") {
        push("gmail_social", "social");
    }
    match org {
        Some(m) => {
            if let Some(cat) = kind_category(m.org.kind) {
                push(
                    if m.via_contact {
                        "org_contact"
                    } else {
                        "org_sender"
                    },
                    cat,
                );
            }
        }
        None => {
            let bulk = f
                .headers
                .keys()
                .any(|k| k.eq_ignore_ascii_case("list-unsubscribe"))
                || f.headers.iter().any(|(k, v)| {
                    k.eq_ignore_ascii_case("precedence") && v.to_lowercase().contains("bulk")
                });
            if bulk {
                push("bulk_newsletter", "newsletters");
            }
        }
    }
    out
}

fn bill_signals(
    f: &MessageFacts<'_>,
    org: Option<&OrgMatch<'_>>,
    enabled: &dyn Fn(&str) -> bool,
) -> Vec<Candidate> {
    let text = collapse_ws(&f.text());
    let mut out = Vec::new();
    let received = contains_any_word(
        &text,
        &[
            "payment received",
            "we received your payment",
            "thank you for your payment",
            "autopay scheduled",
        ],
    );
    if received {
        if enabled("payment_received") {
            out.push(cand("payment_received", json!(false)));
        }
        // The more specific "payment received" wins over a biller's usual mail.
        return out;
    }
    if org.is_some_and(|m| m.org.kind == OrgKind::Biller) && enabled("biller_org") {
        out.push(cand("biller_org", json!(true)));
    }
    if enabled("bill_phrases")
        && contains_any_word(
            &text,
            &[
                "amount due",
                "statement is ready",
                "payment due",
                "balance due",
                "total due",
            ],
        )
        && has_amount(&text)
    {
        out.push(cand("bill_phrases", json!(true)));
    }
    out
}

fn member_signals(
    f: &MessageFacts<'_>,
    ctx: &Context,
    org: Option<&OrgMatch<'_>>,
    enabled: &dyn Fn(&str) -> bool,
) -> Vec<Candidate> {
    let text = f.text();
    let from = fa_domain::normalise_address(f.from_addr);
    let mut best: HashMap<MemberId, Candidate> = HashMap::new();
    fn put(best: &mut HashMap<MemberId, Candidate>, m: &MemberCtx, key: &str) {
        let c = cand(key, json!(m.id.to_string()));
        match best.get(&m.id) {
            Some(old) if old.confidence >= c.confidence => {}
            _ => {
                best.insert(m.id, c);
            }
        }
    }
    let children = || ctx.members.iter().filter(|m| m.role == Role::Child);

    if enabled("contact_enrolled") {
        for m in children() {
            if m.enrollments.iter().any(|e| {
                e.contacts
                    .iter()
                    .any(|c| fa_domain::normalise_address(c) == from)
            }) {
                put(&mut best, m, "contact_enrolled");
            }
        }
    }
    if let Some(om) = org {
        let oid = om.org.id;
        let enrolled = |m: &MemberCtx| m.enrollments.iter().any(|e| e.org == oid);
        let mentions = grade_mentions(&text);
        let mut any_child_signal = false;
        for m in children() {
            if enabled("org_child_name") && name_hit(m, &text) {
                put(&mut best, m, "org_child_name");
                any_child_signal = true;
            }
            if enabled("org_grade_class")
                && enrolled(m)
                && grade_or_class_hit(m, Some(oid), &text, &mentions)
            {
                put(&mut best, m, "org_grade_class");
                any_child_signal = true;
            }
            if enabled("org_tag_gender")
                && enrolled(m)
                && (tag_hit(m, &text) || gender_hit(m, ctx, &text))
            {
                put(&mut best, m, "org_tag_gender");
                any_child_signal = true;
            }
        }
        if enabled("org_all_children") && !any_child_signal && best.is_empty() {
            for m in children().filter(|m| enrolled(m)) {
                put(&mut best, m, "org_all_children");
            }
        }
    }
    best.into_values().collect()
}

static URGENT: LazyLock<WordSet> = LazyLock::new(|| {
    WordSet::new(&[
        "today",
        "tomorrow",
        "cancelled",
        "canceled",
        "closed",
        "early dismissal",
        "action required",
    ])
});

fn time_signals(
    f: &MessageFacts<'_>,
    ctx: &Context,
    enabled: &dyn Fn(&str) -> bool,
) -> Vec<Candidate> {
    if !enabled("urgent_dated") {
        return vec![];
    }
    let text = collapse_ws(&f.text());
    if !URGENT.is_match(&text) {
        return vec![];
    }
    let from = f.received_at;
    let to = from + Duration::hours(48);
    let near = find_dates(&f.text(), f.received_at, ctx.tz, false)
        .into_iter()
        .any(|p| {
            let start = fa_domain::tasks::local_to_utc(
                p.date.and_time(p.time.unwrap_or(NaiveTime::MIN)),
                ctx.tz,
            );
            let end = match p.time {
                Some(_) => start,
                None => start + Duration::hours(24),
            };
            end >= from && start <= to
        });
    if near {
        vec![cand("urgent_dated", json!(true))]
    } else {
        vec![]
    }
}

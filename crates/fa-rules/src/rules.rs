//! User rule conditions (Ingest.md 6.1).

use fa_domain::{DecisionPointKey, MailRule, RuleCondition, RuleField, RuleOp};
use regex::RegexBuilder;

use crate::text::{WordSet, collapse_ws};
use crate::types::{Context, MessageFacts, OrgCtx};

pub const MAX_REGEX_LEN: usize = 256;
const REGEX_SIZE_LIMIT: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuleError {
    #[error("a rule needs at least one condition")]
    NoConditions,
    #[error("condition {index}: {reason}")]
    Condition { index: usize, reason: String },
}

fn bad(index: usize, reason: &str) -> RuleError {
    RuleError::Condition {
        index,
        reason: reason.to_owned(),
    }
}

pub(crate) fn compile(pattern: &str) -> Result<regex::Regex, String> {
    if pattern.chars().count() > MAX_REGEX_LEN {
        return Err(format!("regex longer than {MAX_REGEX_LEN} characters"));
    }
    RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .build()
        .map_err(|e| format!("invalid regex: {e}"))
}

/// Reports the first bad condition with its index.
pub fn validate_rule(conditions: &[RuleCondition]) -> Result<(), RuleError> {
    if conditions.is_empty() {
        return Err(RuleError::NoConditions);
    }
    for (i, c) in conditions.iter().enumerate() {
        if c.field == RuleField::Header && c.header.as_deref().is_none_or(|h| h.trim().is_empty()) {
            return Err(bad(i, "header conditions need a header name"));
        }
        if c.op == RuleOp::Exists {
            continue;
        }
        if c.values.iter().all(|v| v.trim().is_empty()) {
            return Err(bad(i, "needs at least one value"));
        }
        if c.op == RuleOp::Matches {
            for v in &c.values {
                compile(v).map_err(|e| bad(i, &e))?;
            }
        }
    }
    Ok(())
}

fn haystacks(c: &RuleCondition, f: &MessageFacts<'_>, org: Option<&OrgCtx>) -> Vec<String> {
    let one = |s: &str| vec![s.to_owned()];
    match c.field {
        RuleField::FromAddr => one(f.from_addr),
        RuleField::FromDomain => fa_domain::domain_of(f.from_addr)
            .map(|d| vec![d.to_owned()])
            .unwrap_or_default(),
        RuleField::To => f.to.iter().chain(f.cc).cloned().collect(),
        RuleField::Subject => one(f.subject),
        RuleField::Body => one(f.body_text),
        RuleField::GmailLabel => f.gmail_labels.to_vec(),
        RuleField::Header => {
            let want = c.header.as_deref().unwrap_or_default().to_lowercase();
            f.headers
                .iter()
                .filter(|(k, _)| k.to_lowercase() == want)
                .map(|(_, v)| v.clone())
                .collect()
        }
        RuleField::HasAttachmentType => f
            .attachments
            .iter()
            .flat_map(|a| {
                let ext = a
                    .filename
                    .as_deref()
                    .and_then(|n| n.rsplit_once('.'))
                    .map(|(_, e)| e.to_lowercase());
                let sub = a.mime.split_once('/').map(|(_, s)| s.to_owned());
                [Some(a.mime.clone()), sub, ext]
            })
            .flatten()
            .collect(),
        RuleField::OrgKind => org
            .map(|o| vec![o.kind.as_str().to_owned()])
            .unwrap_or_default(),
        RuleField::Org => org
            .map(|o| vec![o.name.clone(), o.id.to_string()])
            .unwrap_or_default(),
        RuleField::Source => one(&f.source.to_string()),
    }
}

fn norm(s: &str) -> String {
    collapse_ws(s).to_lowercase()
}

fn condition_matches(c: &RuleCondition, f: &MessageFacts<'_>, org: Option<&OrgCtx>) -> bool {
    let hay = haystacks(c, f, org);
    let values: Vec<&String> = c.values.iter().filter(|v| !v.trim().is_empty()).collect();
    match c.op {
        RuleOp::Exists => hay.iter().any(|h| !h.trim().is_empty()),
        RuleOp::Is | RuleOp::In => hay.iter().any(|h| {
            values.iter().any(|v| {
                if c.field == RuleField::FromDomain {
                    fa_domain::domain_matches(h, v)
                } else if c.field == RuleField::FromAddr || c.field == RuleField::To {
                    fa_domain::normalise_address(h) == fa_domain::normalise_address(v)
                } else {
                    norm(h) == norm(v)
                }
            })
        }),
        RuleOp::ContainsAny => {
            let set = WordSet::new(&values);
            hay.iter().any(|h| set.is_match(h))
        }
        RuleOp::Matches => values
            .iter()
            .filter_map(|v| compile(v).ok())
            .any(|re| hay.iter().any(|h| re.is_match(h))),
    }
}

pub(crate) fn rule_matches(r: &MailRule, f: &MessageFacts<'_>, org: Option<&OrgCtx>) -> bool {
    !r.conditions.is_empty() && r.conditions.iter().all(|c| condition_matches(c, f, org))
}

/// The first matching enabled rule for `dp` by `priority` (lowest first).
pub(crate) fn first_match<'c>(
    dp: DecisionPointKey,
    f: &MessageFacts<'_>,
    ctx: &'c Context,
    org: Option<&OrgCtx>,
) -> Option<&'c MailRule> {
    let mut rules: Vec<&MailRule> = ctx
        .rules
        .iter()
        .filter(|r| r.enabled && r.archived_at.is_none() && r.decision_point == dp)
        .collect();
    rules.sort_by_key(|r| r.priority);
    rules.into_iter().find(|r| rule_matches(r, f, org))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cond(field: RuleField, op: RuleOp, values: &[&str]) -> RuleCondition {
        RuleCondition {
            field,
            op,
            header: None,
            values: values.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn validation() {
        assert_eq!(validate_rule(&[]), Err(RuleError::NoConditions));
        let ok = cond(RuleField::Subject, RuleOp::Matches, &["^Re:"]);
        let bad_re = cond(RuleField::Body, RuleOp::Matches, &["("]);
        let long = cond(RuleField::Body, RuleOp::Matches, &[&"a".repeat(257)]);
        let empty = cond(RuleField::Body, RuleOp::Is, &[]);
        let hdr = cond(RuleField::Header, RuleOp::Exists, &[]);
        assert!(validate_rule(std::slice::from_ref(&ok)).is_ok());
        for (i, c) in [bad_re, long, empty, hdr].into_iter().enumerate() {
            match validate_rule(&[ok.clone(), c]) {
                Err(RuleError::Condition { index, .. }) => assert_eq!(index, 1, "case {i}"),
                other => panic!("case {i}: {other:?}"),
            }
        }
        let mut hdr_ok = cond(RuleField::Header, RuleOp::Exists, &[]);
        hdr_ok.header = Some("List-Id".into());
        assert!(validate_rule(&[hdr_ok]).is_ok());
    }

    #[test]
    fn big_regex_rejected_by_size_limit() {
        let p = "(a{1000}){1000}";
        assert!(compile(p).is_err());
    }
}

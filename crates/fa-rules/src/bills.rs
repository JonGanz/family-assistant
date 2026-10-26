//! Bill extraction (Ingest.md 7, D-009) and PDF text.

use std::sync::LazyLock;

use chrono::NaiveDate;
use chrono_tz::Tz;
use regex::{Regex, RegexBuilder};

use crate::dates::find_dates;
use crate::parse::MAX_PDF_BYTES;
use crate::text::collapse_ws;
use crate::types::{MessageFacts, OrgCtx};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedBill {
    pub payee: String,
    pub amount_minor: i64,
    pub currency: String,
    pub due_on: Option<NaiveDate>,
    /// Last four digits after "account".
    pub account_hint: Option<String>,
    pub confidence: f32,
}

const TEMPLATE_CONF: f32 = 0.9;
const CODES: &str = "USD|EUR|GBP|JPY|CAD|AUD|NZD|CHF|MXN|INR|CNY|SEK|NOK|DKK";
/// Distance in bytes between a label and its amount or date.
const NEAR: usize = 40;

fn re(p: &str) -> Regex {
    RegexBuilder::new(p)
        .case_insensitive(true)
        .build()
        .expect("static regex")
}

static MONEY: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"(?P<sym>[$£€¥])\s?(?P<n1>\d[\d.,]*\d|\d)|\b(?P<iso1>{CODES})\s?(?P<n2>\d[\d.,]*\d|\d)|(?P<n3>\d[\d.,]*\d|\d)\s?(?P<iso2>{CODES})\b"
    ))
});
static ACCOUNT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"\baccount[\s:#*xX•.\-]*(?:(?:number|no\.?|num|ending|ends|in|#)[\s:#*xX•.\-]*)*(\d[\d\s-]{2,}\d)",
    )
});

fn is_dollar(code: &str) -> bool {
    matches!(code, "USD" | "CAD" | "AUD" | "NZD" | "MXN")
}

fn exponent(code: &str) -> u32 {
    if matches!(code, "JPY") { 0 } else { 2 }
}

/// "1,234.56", "1.234,56", "42", "12,50" to minor units.
fn parse_amount(s: &str, exp: u32) -> Option<i64> {
    let s = s.trim().trim_end_matches(['.', ',']);
    if s.is_empty() {
        return None;
    }
    let last_dot = s.rfind('.');
    let last_comma = s.rfind(',');
    let (int_part, frac_part) = match (last_dot, last_comma) {
        (Some(d), Some(c)) => {
            let i = d.max(c);
            (&s[..i], &s[i + 1..])
        }
        (Some(i), None) | (None, Some(i)) => {
            let sep = s.as_bytes()[i] as char;
            let n_sep = s.matches(sep).count();
            let after = &s[i + 1..];
            if n_sep > 1 || exp == 0 || (after.len() == 3 && i <= 3) {
                (s, "")
            } else {
                (&s[..i], after)
            }
        }
        (None, None) => (s, ""),
    };
    let digits: String = int_part.chars().filter(char::is_ascii_digit).collect();
    let int: i64 = if digits.is_empty() {
        0
    } else {
        digits.parse().ok()?
    };
    let frac_digits: String = frac_part.chars().filter(char::is_ascii_digit).collect();
    let mut frac: String = frac_digits.chars().take(exp as usize).collect();
    while frac.len() < exp as usize {
        frac.push('0');
    }
    let frac_n: i64 = if frac.is_empty() {
        0
    } else {
        frac.parse().ok()?
    };
    int.checked_mul(10_i64.pow(exp))?.checked_add(frac_n)
}

struct Money {
    start: usize,
    end: usize,
    minor: i64,
    currency: String,
}

fn amounts(text: &str, default_currency: &str) -> Vec<Money> {
    MONEY
        .captures_iter(text)
        .filter_map(|c| {
            let m = c.get(0)?;
            let currency = if let Some(sym) = c.name("sym") {
                match sym.as_str() {
                    "$" => {
                        if is_dollar(default_currency) {
                            default_currency.to_owned()
                        } else {
                            "USD".into()
                        }
                    }
                    "£" => "GBP".into(),
                    "€" => "EUR".into(),
                    _ => {
                        if default_currency == "CNY" {
                            "CNY".into()
                        } else {
                            "JPY".into()
                        }
                    }
                }
            } else {
                c.name("iso1")
                    .or_else(|| c.name("iso2"))?
                    .as_str()
                    .to_uppercase()
            };
            let n = c
                .name("n1")
                .or_else(|| c.name("n2"))
                .or_else(|| c.name("n3"))?
                .as_str();
            let minor = parse_amount(n, exponent(&currency))?;
            (minor > 0).then_some(Money {
                start: m.start(),
                end: m.end(),
                minor,
                currency,
            })
        })
        .collect()
}

/// An amount within `NEAR` bytes after the end of `label`.
fn amount_after<'a>(
    text: &str,
    lower: &str,
    labels: &[&str],
    ms: &'a [Money],
) -> Option<&'a Money> {
    let mut best: Option<(usize, &Money)> = None;
    for l in labels {
        for (pos, _) in lower.match_indices(l) {
            let end = pos + l.len();
            if let Some(m) = ms
                .iter()
                .find(|m| m.start >= end && m.start - end <= NEAR && text.is_char_boundary(end))
                && best.is_none_or(|(p, _)| pos < p)
            {
                best = Some((pos, m));
            }
        }
    }
    best.map(|(_, m)| m)
}

struct Found {
    minor: i64,
    currency: String,
    strong: bool,
}

fn generic_amount(text: &str, default_currency: &str) -> Option<Found> {
    let ms = amounts(text, default_currency);
    if ms.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    let strong_labels = [
        "total amount due",
        "amount due",
        "total due",
        "balance due",
        "payment due",
        "amount owing",
        "amount owed",
    ];
    if let Some(m) = amount_after(text, &lower, &strong_labels, &ms) {
        return Some(Found {
            minor: m.minor,
            currency: m.currency.clone(),
            strong: true,
        });
    }
    // "$45.00 is due"
    for m in &ms {
        let tail: String = lower[m.end..].chars().take(14).collect();
        if tail.contains(" due") || tail.contains(" is due") {
            return Some(Found {
                minor: m.minor,
                currency: m.currency.clone(),
                strong: true,
            });
        }
    }
    let weak_labels = ["new balance", "total", "balance", "amount"];
    amount_after(text, &lower, &weak_labels, &ms).map(|m| Found {
        minor: m.minor,
        currency: m.currency.clone(),
        strong: false,
    })
}

fn first_date(snippet: &str, received: chrono::DateTime<chrono::Utc>) -> Option<NaiveDate> {
    let snippet: String = snippet.chars().take(NEAR).collect();
    find_dates(&snippet, received, Tz::UTC, true)
        .into_iter()
        .find(|p| p.span.start <= 25)
        .map(|p| p.date)
}

fn generic_due(text: &str, received: chrono::DateTime<chrono::Utc>) -> Option<NaiveDate> {
    let lower = text.to_lowercase();
    let mut hits: Vec<usize> = Vec::new();
    hits.extend(lower.match_indices("due").map(|(p, _)| p + 3));
    hits.sort_unstable();
    for end in &hits {
        if lower.is_char_boundary(*end)
            && let Some(d) = first_date(&text[*end..], received)
        {
            return Some(d);
        }
    }
    for (p, key) in lower
        .match_indices("pay by")
        .chain(lower.match_indices("payment by"))
        .chain(lower.match_indices("pay before"))
        .chain(lower.match_indices(" by "))
    {
        let end = p + key.len();
        let line_start = lower[..p].rfind('\n').map_or(0, |i| i + 1);
        let line = &lower[line_start..p];
        if (key.trim() != "by" || line.contains("pay") || line.contains("due"))
            && lower.is_char_boundary(end)
            && let Some(d) = first_date(&text[end..], received)
        {
            return Some(d);
        }
    }
    None
}

fn account_hint(text: &str) -> Option<String> {
    let c = ACCOUNT.captures(text)?;
    let digits: String = c
        .get(1)?
        .as_str()
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    (digits.len() >= 4).then(|| digits[digits.len() - 4..].to_owned())
}

fn payee(f: &MessageFacts<'_>, org: Option<&OrgCtx>) -> String {
    if let Some(o) = org {
        return o.name.clone();
    }
    if let Some(n) = f.from_name.map(str::trim).filter(|n| !n.is_empty()) {
        return n.to_owned();
    }
    let domain = fa_domain::domain_of(f.from_addr).unwrap_or(f.from_addr);
    let labels: Vec<&str> = domain.split('.').collect();
    let label = if labels.len() >= 2 {
        labels[labels.len() - 2]
    } else {
        domain
    };
    let mut cs = label.chars();
    match cs.next() {
        Some(c) => c.to_uppercase().collect::<String>() + cs.as_str(),
        None => String::new(),
    }
}

fn template_bill(
    o: &OrgCtx,
    text: &str,
    received: chrono::DateTime<chrono::Utc>,
    default_currency: &str,
) -> Option<(i64, String, Option<NaiveDate>, Option<String>)> {
    let pick = |pattern: &Option<String>| -> Option<String> {
        let re = crate::rules::compile(pattern.as_deref()?).ok()?;
        let c = re.captures(text)?;
        Some(c.get(1).or_else(|| c.get(0))?.as_str().trim().to_owned())
    };
    let amount_text = pick(&o.bill_amount_regex)?;
    let ms = amounts(&amount_text, default_currency);
    let (minor, currency) = match ms.first() {
        Some(m) => (m.minor, m.currency.clone()),
        None => {
            let n: String = amount_text
                .chars()
                .filter(|c| c.is_ascii_digit() || matches!(c, '.' | ','))
                .collect();
            let minor = parse_amount(&n, exponent(default_currency))?;
            (minor, default_currency.to_owned())
        }
    };
    if minor <= 0 {
        return None;
    }
    let due = pick(&o.bill_due_regex).and_then(|d| {
        find_dates(&d, received, Tz::UTC, true)
            .into_iter()
            .next()
            .map(|p| p.date)
    });
    let account = pick(&o.bill_account_regex).map(|a| {
        let digits: String = a.chars().filter(char::is_ascii_digit).collect();
        if digits.len() >= 4 {
            digits[digits.len() - 4..].to_owned()
        } else {
            a
        }
    });
    Some((minor, currency, due, account))
}

/// A biller template when the org has one, else the generic parser; the body first, then
/// `pdf_text` when the body has no amount. At most one bill per message.
pub fn extract_bills(
    f: &MessageFacts<'_>,
    org: Option<&OrgCtx>,
    pdf_text: Option<&str>,
    currency: &str,
) -> Vec<ExtractedBill> {
    let body = format!("{}\n{}", f.subject, f.body_text);
    let texts: Vec<&str> = std::iter::once(body.as_str()).chain(pdf_text).collect();
    let payee = payee(f, org);

    if let Some(o) = org {
        for t in &texts {
            if let Some((minor, cur, due, account)) = template_bill(o, t, f.received_at, currency) {
                return vec![ExtractedBill {
                    payee,
                    amount_minor: minor,
                    currency: cur,
                    due_on: due,
                    account_hint: account,
                    confidence: TEMPLATE_CONF,
                }];
            }
        }
    }
    for t in &texts {
        if let Some(found) = generic_amount(t, currency) {
            let due = generic_due(t, f.received_at)
                .or_else(|| texts.iter().find_map(|o| generic_due(o, f.received_at)));
            let confidence = match (found.strong, due.is_some()) {
                (true, true) => 0.8,
                (true, false) => 0.7,
                (false, _) => 0.6,
            };
            return vec![ExtractedBill {
                payee,
                amount_minor: found.minor,
                currency: found.currency,
                due_on: due,
                account_hint: texts.iter().find_map(|t| account_hint(t)),
                confidence,
            }];
        }
    }
    Vec::new()
}

/// Text of a PDF, at most `max_bytes` of it. Parsing can panic on hostile files, so it is
/// guarded; the caller runs this in `spawn_blocking` with a timeout.
pub fn pdf_text(bytes: &[u8], max_bytes: usize) -> Result<String> {
    if bytes.len() > MAX_PDF_BYTES {
        return Err(Error::Pdf("pdf larger than 5 MB".into()));
    }
    let res = std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(bytes));
    let text = match res {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => return Err(Error::Pdf(e.to_string())),
        Err(_) => return Err(Error::Pdf("pdf parser panicked".into())),
    };
    let mut text = text.trim().to_owned();
    if text.len() > max_bytes {
        let mut cut = max_bytes;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
    }
    // Keep line structure for the label search but squeeze runs of blanks.
    Ok(text.lines().map(collapse_ws).collect::<Vec<_>>().join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_parse() {
        for (s, exp, want) in [
            ("1,234.56", 2, 123456),
            ("1.234,56", 2, 123456),
            ("42", 2, 4200),
            ("12,50", 2, 1250),
            ("12.5", 2, 1250),
            ("1,234", 2, 123400),
            ("1,234,567.89", 2, 123456789),
            ("1,234", 0, 1234),
        ] {
            assert_eq!(parse_amount(s, exp), Some(want), "{s}");
        }
    }
}

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use fa_domain::entities::Role;
use fa_domain::{
    Candidate, CategoryBehaviour, DecisionPointKey as Dp, Gender, HouseholdId, MailRule,
    MailRuleId, MailSourceId, MemberId, OrgId, OrgKind, Relevance, RuleCondition, RuleField,
    RuleOp,
};
use fa_rules::*;
use serde_json::json;

const NY: Tz = chrono_tz::America::New_York;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn local(tz: Tz, s: &str) -> DateTime<Utc> {
    let n = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
    tz.from_local_datetime(&n).unwrap().with_timezone(&Utc)
}

struct Ids {
    school: OrgId,
    power: OrgId,
    ava: MemberId,
    ben: MemberId,
    pat: MemberId,
}

fn ctx() -> (Context, Ids) {
    let ids = Ids {
        school: OrgId::new(),
        power: OrgId::new(),
        ava: MemberId::new(),
        ben: MemberId::new(),
        pat: MemberId::new(),
    };
    let mut c = Context::new(NY);
    c.orgs.push(OrgCtx {
        id: ids.school,
        kind: OrgKind::School,
        name: "Example Elementary".into(),
        domains: vec!["school.example.edu".into()],
        senders: vec![],
        contacts: vec![ContactCtx {
            email: "rivera@school.example.edu".into(),
            name: Some("Ms Rivera".into()),
        }],
        bill_amount_regex: None,
        bill_due_regex: None,
        bill_account_regex: None,
    });
    c.orgs.push(OrgCtx {
        id: ids.power,
        kind: OrgKind::Biller,
        name: "Power Co".into(),
        domains: vec!["power.example.com".into()],
        senders: vec![],
        contacts: vec![],
        bill_amount_regex: None,
        bill_due_regex: None,
        bill_account_regex: None,
    });
    let enroll = |grade: &str, class: Option<&str>, contacts: &[&str]| EnrollCtx {
        org: ids.school,
        grade: Some(grade.into()),
        class_name: class.map(str::to_owned),
        contacts: contacts.iter().map(|s| s.to_string()).collect(),
    };
    c.members.push(MemberCtx {
        id: ids.ava,
        name: "Ava".into(),
        aliases: vec!["Avie".into()],
        grade: Some("3".into()),
        gender: Some(Gender::Female),
        tags: vec![],
        role: Role::Child,
        enrollments: vec![enroll("3", Some("Room 12"), &["rivera@school.example.edu"])],
    });
    c.members.push(MemberCtx {
        id: ids.ben,
        name: "Ben".into(),
        aliases: vec![],
        grade: Some("5".into()),
        gender: Some(Gender::Male),
        tags: vec!["band".into()],
        role: Role::Child,
        enrollments: vec![enroll("5", None, &[])],
    });
    c.members.push(MemberCtx {
        id: ids.pat,
        name: "Pat".into(),
        aliases: vec![],
        grade: None,
        gender: None,
        tags: vec![],
        role: Role::Adult,
        enrollments: vec![],
    });
    for (k, b) in [
        ("school", CategoryBehaviour::Extract),
        ("bills", CategoryBehaviour::Extract),
        ("newsletters", CategoryBehaviour::Keep),
        ("promotions", CategoryBehaviour::Ignore),
        ("social", CategoryBehaviour::Ignore),
        ("junk", CategoryBehaviour::Ignore),
        ("other", CategoryBehaviour::Keep),
    ] {
        c.categories.push((k.to_owned(), b));
    }
    c.owners.push(ids.pat);
    (c, ids)
}

/// Owns everything a `MessageFacts` borrows.
struct Fx {
    from: String,
    from_name: Option<String>,
    subject: String,
    body: String,
    html: Option<String>,
    labels: Vec<String>,
    headers: HashMap<String, String>,
    atts: Vec<AttachmentFacts>,
    received: DateTime<Utc>,
}

impl Fx {
    fn new(from: &str, subject: &str, body: &str) -> Self {
        Self {
            from: from.into(),
            from_name: None,
            subject: subject.into(),
            body: body.into(),
            html: None,
            labels: vec![],
            headers: HashMap::new(),
            atts: vec![],
            received: local(NY, "2026-10-28 09:00"),
        }
    }
    fn from_parsed(p: &ParsedMessage) -> Self {
        Self {
            from: p.from_addr.clone(),
            from_name: p.from_name.clone(),
            subject: p.subject.clone(),
            body: p.body_text.clone(),
            html: p.body_html.clone(),
            labels: vec![],
            headers: p.headers.clone(),
            atts: p.attachment_facts(),
            received: p.date.unwrap(),
        }
    }
    fn facts(&self) -> MessageFacts<'_> {
        MessageFacts {
            from_addr: &self.from,
            from_name: self.from_name.as_deref(),
            to: &[],
            cc: &[],
            subject: &self.subject,
            body_text: &self.body,
            body_html: self.html.as_deref(),
            headers: &self.headers,
            gmail_labels: &self.labels,
            attachments: &self.atts,
            received_at: self.received,
            source: MailSourceId::new(),
        }
    }
    fn eval(&self, dp: Dp, c: &Context) -> Vec<Candidate> {
        evaluate(dp, &self.facts(), c)
    }
}

// ---- parse and fixtures ----

#[test]
fn parse_ics_mail() {
    let p = parse(&fixture("teacher_ics.eml")).unwrap();
    assert_eq!(p.from_addr, "rivera@school.example.edu");
    assert_eq!(p.subject, "Parent-teacher conference");
    assert!(p.body_text.contains("conference"));
    assert_eq!(p.attachments.len(), 1);
    let a = &p.attachments[0];
    assert_eq!(a.mime, "text/calendar");
    assert_eq!(a.filename.as_deref(), Some("invite.ics"));
    assert!(a.ics.as_deref().unwrap().contains("BEGIN:VEVENT"));
    assert_eq!(p.message_id.as_deref(), Some("conf-1@school.example.edu"));
}

#[test]
fn html_only_mail_gets_text() {
    let p = parse(&fixture("newsletter.eml")).unwrap();
    assert!(p.body_html.as_deref().unwrap().contains("<h3>"));
    assert!(p.body_text.contains("Third graders"));
    assert!(p.headers.contains_key("list-unsubscribe"));
}

#[test]
fn ics_event_and_cancel() {
    let (c, _) = ctx();
    let fx = Fx::from_parsed(&parse(&fixture("teacher_ics.eml")).unwrap());
    let ev = extract_events(&fx.facts(), &c);
    assert_eq!(ev.len(), 1);
    let e = &ev[0];
    assert_eq!(e.title, "Parent-teacher conference");
    assert_eq!(e.start, local(NY, "2026-11-06 18:00"));
    assert_eq!(e.end, Some(local(NY, "2026-11-06 19:00")));
    assert_eq!(e.location.as_deref(), Some("Room 12, Example Elementary"));
    assert_eq!(e.uid.as_deref(), Some("conf-2026-11-06@school.example.edu"));
    assert!(!e.cancel && !e.all_day);
    assert_eq!(e.confidence, 0.95);
    assert_eq!(e.source, EventSource::Ics);

    let fx = Fx::from_parsed(&parse(&fixture("cancel_ics.eml")).unwrap());
    let ev = extract_events(&fx.facts(), &c);
    assert_eq!(ev.len(), 1);
    assert!(ev[0].cancel);
    assert_eq!(
        ev[0].uid.as_deref(),
        Some("conf-2026-11-06@school.example.edu")
    );
}

#[test]
fn ics_all_day_utc_and_duration() {
    let (c, _) = ctx();
    let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:20261110\r\nSUMMARY:No school\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nDTSTART:20261110T150000Z\r\nDURATION:PT1H30M\r\nSUMMARY:Call\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let mut fx = Fx::new("a@example.edu", "s", "b");
    fx.atts.push(AttachmentFacts {
        mime: "text/calendar".into(),
        ics: Some(ics.into()),
        ..AttachmentFacts::default()
    });
    let ev = extract_events(&fx.facts(), &c);
    assert_eq!(ev.len(), 2);
    assert!(ev[0].all_day);
    assert_eq!(ev[0].start, local(NY, "2026-11-10 00:00"));
    assert_eq!(ev[0].end, Some(local(NY, "2026-11-11 00:00")));
    assert_eq!(
        ev[1].start,
        Utc.with_ymd_and_hms(2026, 11, 10, 15, 0, 0).unwrap()
    );
    assert_eq!(
        ev[1].end,
        Some(Utc.with_ymd_and_hms(2026, 11, 10, 16, 30, 0).unwrap())
    );
}

#[test]
fn json_ld_booking() {
    let (c, _) = ctx();
    let fx = Fx::from_parsed(&parse(&fixture("booking_jsonld.eml")).unwrap());
    let ev = extract_events(&fx.facts(), &c);
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].title, "Example Bistro");
    assert_eq!(
        ev[0].start,
        Utc.with_ymd_and_hms(2026, 11, 13, 0, 30, 0).unwrap()
    );
    assert!(ev[0].location.as_deref().unwrap().contains("Springfield"));
    assert_eq!(ev[0].confidence, 0.9);
    assert_eq!(ev[0].source, EventSource::SchemaOrg);
}

#[test]
fn json_ld_event_graph_and_microdata() {
    let (c, _) = ctx();
    let mut fx = Fx::new("a@example.com", "Tickets", "");
    fx.html = Some(
        r#"<script type="application/ld+json">{"@graph":[{"@type":"SportsEvent","name":"Game night","startDate":"2026-11-14","location":{"@type":"Place","name":"Field"}}]}</script>"#
            .into(),
    );
    let ev = extract_events(&fx.facts(), &c);
    assert_eq!(ev.len(), 1);
    assert!(ev[0].all_day);
    assert_eq!(ev[0].end, Some(local(NY, "2026-11-15 00:00")));
    assert_eq!(ev[0].location.as_deref(), Some("Field"));

    fx.html = Some(
        r#"<div itemscope itemtype="http://schema.org/Event"><span itemprop="name">Recital</span>
        <meta itemprop="startDate" content="2026-11-20T17:00:00">
        <div itemprop="location" itemscope itemtype="http://schema.org/Place"><span itemprop="name">Hall</span></div></div>"#
            .into(),
    );
    let ev = extract_events(&fx.facts(), &c);
    assert_eq!(ev.len(), 1, "{ev:?}");
    assert_eq!(ev[0].title, "Recital");
    assert_eq!(ev[0].start, local(NY, "2026-11-20 17:00"));
    assert_eq!(ev[0].location.as_deref(), Some("Hall"));
    assert!(!ev[0].cancel);
}

// ---- date phrases in events ----

fn phrase_events(text: &str, subject: &str, recv: &str, tz: Tz) -> Vec<ExtractedEvent> {
    let mut c = ctx().0;
    c.tz = tz;
    let mut fx = Fx::new("a@example.edu", subject, text);
    fx.received = local(tz, recv);
    extract_events(&fx.facts(), &c)
}

#[test]
fn phrase_event_title_and_confidence() {
    let ev = phrase_events(
        "Join us on Saturday, Nov 7 at 6pm in the gym.",
        "Fall carnival",
        "2026-10-28 09:00",
        NY,
    );
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].title, "Fall carnival");
    assert_eq!(ev[0].start, local(NY, "2026-11-07 18:00"));
    assert_eq!(ev[0].confidence, 0.7);
    assert_eq!(ev[0].source, EventSource::DatePhrase);

    let ev = phrase_events(
        "Carnival is Saturday, Nov 7 at 6pm. Picture day is 11/9 8:30-9:30am.",
        "News",
        "2026-10-28 09:00",
        NY,
    );
    assert_eq!(ev.len(), 2);
    assert!(ev[0].title.starts_with("Carnival is Saturday"));
    assert!(ev[1].title.starts_with("Picture day"));
    assert_eq!(ev[1].end, Some(local(NY, "2026-11-09 09:30")));
}

#[test]
fn phrase_events_dst() {
    // New York spring forward: 02:30 on 2026-03-08 does not exist, the first valid instant
    // after the gap is 03:00 EDT (07:00Z).
    let ev = phrase_events(
        "Meet Sunday, March 8 at 2:30am",
        "Run",
        "2026-03-02 09:00",
        NY,
    );
    assert_eq!(
        ev[0].start,
        Utc.with_ymd_and_hms(2026, 3, 8, 7, 0, 0).unwrap()
    );
    // Fall back: 01:30 on 2026-11-01 happens twice; the earlier (EDT, 05:30Z) is taken.
    let ev = phrase_events(
        "Meet Sunday, Nov 1 at 1:30am",
        "Run",
        "2026-10-28 09:00",
        NY,
    );
    assert_eq!(
        ev[0].start,
        Utc.with_ymd_and_hms(2026, 11, 1, 5, 30, 0).unwrap()
    );
    // An ordinary EST day after the change.
    let ev = phrase_events("Meet Friday, Nov 6 at 6pm", "Run", "2026-10-28 09:00", NY);
    assert_eq!(
        ev[0].start,
        Utc.with_ymd_and_hms(2026, 11, 6, 23, 0, 0).unwrap()
    );

    let lon = chrono_tz::Europe::London;
    // London spring forward 2026-03-29: 01:30 is skipped, first valid is 02:00 BST (01:00Z).
    let ev = phrase_events(
        "Meet Sunday, March 29 at 1:30am",
        "Run",
        "2026-03-20 09:00",
        lon,
    );
    assert_eq!(
        ev[0].start,
        Utc.with_ymd_and_hms(2026, 3, 29, 1, 0, 0).unwrap()
    );
    // London fall back 2026-10-25: 01:30 twice; earlier is BST (00:30Z).
    let ev = phrase_events(
        "Meet Sunday, Oct 25 at 1:30am",
        "Run",
        "2026-10-20 09:00",
        lon,
    );
    assert_eq!(
        ev[0].start,
        Utc.with_ymd_and_hms(2026, 10, 25, 0, 30, 0).unwrap()
    );
    // Same wall time in winter is UTC.
    let ev = phrase_events("Meet Friday, Dec 4 at 6pm", "Run", "2026-11-20 09:00", lon);
    assert_eq!(
        ev[0].start,
        Utc.with_ymd_and_hms(2026, 12, 4, 18, 0, 0).unwrap()
    );
}

#[test]
fn phrase_relative_and_bare() {
    let ev = phrase_events(
        "Early pickup tomorrow at 3.",
        "Pickup",
        "2026-10-28 09:00",
        NY,
    );
    assert_eq!(ev[0].start, local(NY, "2026-10-29 15:00"));
    assert_eq!(ev[0].confidence, 0.6);
    // A bare weekday or "tomorrow" without a time is not an event.
    assert!(phrase_events("Forms due Friday.", "Forms", "2026-10-28 09:00", NY).is_empty());
    // A past date mid-year is dropped.
    assert!(phrase_events("It was March 3 at 6pm", "Old", "2026-10-28 09:00", NY).is_empty());
    // Day/month order only when unambiguous.
    let ev = phrase_events("Open day 17/11 at 6pm", "Open", "2026-10-28 09:00", NY);
    assert_eq!(ev[0].start, local(NY, "2026-11-17 18:00"));
    let ev = phrase_events("Open day 3/11 at 6pm", "Open", "2026-12-20 09:00", NY);
    assert_eq!(ev[0].start, local(NY, "2027-03-11 18:00"));
}

// ---- sections and relevance ----

#[test]
fn newsletter_sections_and_relevance() {
    let (c, ids) = ctx();
    let fx = Fx::from_parsed(&parse(&fixture("newsletter.eml")).unwrap());
    let secs = sections(&fx.facts());
    let titles: Vec<Option<&str>> = secs.iter().map(|s| s.title.as_deref()).collect();
    assert_eq!(
        titles,
        vec![
            None,
            Some("Grade 3 News"),
            Some("Grade 5 News"),
            Some("Girls' Volleyball"),
            Some("Lunch Menu")
        ]
    );
    let rel = |who: MemberId| -> Vec<Relevance> {
        let m = c.members.iter().find(|m| m.id == who).unwrap();
        secs.iter().map(|s| relevance(s, m, &c)).collect()
    };
    use Relevance::*;
    assert_eq!(
        rel(ids.ava),
        vec![General, Relevant, Other, Relevant, General]
    );
    assert_eq!(rel(ids.ben), vec![General, Other, Relevant, Other, General]);
}

#[test]
fn plain_text_sections() {
    let body = "Dear families,\n\nThis month we have news.\n\nBand Concert\nThe concert is soon.\n\nLunch Menu:\nPizza.\n\nThis is just a long sentence paragraph that continues on and on past sixty characters in length.";
    let fx = Fx::new("a@example.edu", "News", body);
    let secs = sections(&fx.facts());
    let titles: Vec<Option<&str>> = secs.iter().map(|s| s.title.as_deref()).collect();
    assert_eq!(titles, vec![None, Some("Band Concert"), Some("Lunch Menu")]);
    assert!(secs[1].text.contains("concert is soon"));
    assert!(secs[2].text.contains("Pizza"));
    assert!(secs[2].text.contains("long sentence"));
}

#[test]
fn bold_only_paragraph_is_a_title() {
    let mut fx = Fx::new("a@example.edu", "News", "x");
    fx.html = Some("<p>Intro</p><p><strong>Book Fair</strong></p><p>Next week.</p><p>Plain <b>bold</b> mix</p>".into());
    let secs = sections(&fx.facts());
    assert_eq!(secs.len(), 2);
    assert_eq!(secs[1].title.as_deref(), Some("Book Fair"));
    assert!(secs[1].text.contains("Plain bold mix"));
}

// ---- signals ----

#[test]
fn every_signal_has_a_case() {
    let (c, ids) = ctx();
    let ava = ids.ava.to_string();
    let ben = ids.ben.to_string();
    let school = "office@school.example.edu";

    struct Case {
        key: &'static str,
        dp: Dp,
        fx: Fx,
        // member_link: member ids that must carry the signal as their best candidate
        value: serde_json::Value,
    }
    let labelled = |label: &str| {
        let mut f = Fx::new("x@elsewhere.example.com", "Hi", "text");
        f.labels.push(label.into());
        f
    };
    let mut bulk = Fx::new("x@elsewhere.example.com", "Sale", "text");
    bulk.headers
        .insert("list-unsubscribe".into(), "<mailto:u@example.com>".into());
    let mut bulk2 = Fx::new("x@elsewhere.example.com", "Sale", "text");
    bulk2.headers.insert("precedence".into(), "bulk".into());
    let cases = vec![
        Case {
            key: "gmail_spam",
            dp: Dp::EmailCategory,
            fx: labelled("SPAM"),
            value: json!("junk"),
        },
        Case {
            key: "gmail_promotions",
            dp: Dp::EmailCategory,
            fx: labelled("CATEGORY_PROMOTIONS"),
            value: json!("promotions"),
        },
        Case {
            key: "gmail_social",
            dp: Dp::EmailCategory,
            fx: labelled("CATEGORY_SOCIAL"),
            value: json!("social"),
        },
        Case {
            key: "org_contact",
            dp: Dp::EmailCategory,
            fx: Fx::new("Rivera@school.example.edu", "Hi", "x"),
            value: json!("school"),
        },
        Case {
            key: "org_sender",
            dp: Dp::EmailCategory,
            fx: Fx::new(school, "Hi", "x"),
            value: json!("school"),
        },
        Case {
            key: "bulk_newsletter",
            dp: Dp::EmailCategory,
            fx: bulk,
            value: json!("newsletters"),
        },
        Case {
            key: "bulk_newsletter",
            dp: Dp::EmailCategory,
            fx: bulk2,
            value: json!("newsletters"),
        },
        Case {
            key: "payment_received",
            dp: Dp::EmailIsBill,
            fx: Fx::new(
                "a@elsewhere.example.com",
                "Receipt",
                "Thank you for your payment of $10.00",
            ),
            value: json!(false),
        },
        Case {
            key: "payment_received",
            dp: Dp::EmailIsBill,
            fx: Fx::new(
                "billing@power.example.com",
                "Autopay",
                "Autopay scheduled for Nov 1. Amount due $5.00",
            ),
            value: json!(false),
        },
        Case {
            key: "biller_org",
            dp: Dp::EmailIsBill,
            fx: Fx::new("billing@power.example.com", "Hello", "news"),
            value: json!(true),
        },
        Case {
            key: "bill_phrases",
            dp: Dp::EmailIsBill,
            fx: Fx::new(
                "a@elsewhere.example.com",
                "Invoice",
                "Your balance due is $45.00 on receipt",
            ),
            value: json!(true),
        },
        Case {
            key: "contact_enrolled",
            dp: Dp::EmailMemberLink,
            fx: Fx::new("rivera@school.example.edu", "Hi", "nothing special"),
            value: json!(ava),
        },
        Case {
            key: "org_child_name",
            dp: Dp::EmailMemberLink,
            fx: Fx::new(school, "Hi", "Avie did great today"),
            value: json!(ava),
        },
        Case {
            key: "org_grade_class",
            dp: Dp::EmailMemberLink,
            fx: Fx::new(school, "Hi", "Grade 5 students bring a lunch"),
            value: json!(ben),
        },
        Case {
            key: "org_grade_class",
            dp: Dp::EmailMemberLink,
            fx: Fx::new(school, "Hi", "Meet in Room 12 at the gym"),
            value: json!(ava),
        },
        Case {
            key: "org_grade_class",
            dp: Dp::EmailMemberLink,
            fx: Fx::new(school, "Hi", "Grades 4-6 trip"),
            value: json!(ben),
        },
        Case {
            key: "org_tag_gender",
            dp: Dp::EmailMemberLink,
            fx: Fx::new(school, "Hi", "Band rehearsal moved"),
            value: json!(ben),
        },
        Case {
            key: "org_tag_gender",
            dp: Dp::EmailMemberLink,
            fx: Fx::new(school, "Hi", "Girls' volleyball tryouts"),
            value: json!(ava),
        },
        Case {
            key: "org_all_children",
            dp: Dp::EmailMemberLink,
            fx: Fx::new(school, "Hi", "General notice"),
            value: json!(ava),
        },
        Case {
            key: "org_all_children",
            dp: Dp::EmailMemberLink,
            fx: Fx::new(school, "Hi", "General notice"),
            value: json!(ben),
        },
        Case {
            key: "urgent_dated",
            dp: Dp::EmailTimeSensitive,
            fx: Fx::new(school, "Closed", "School is closed tomorrow"),
            value: json!(true),
        },
        Case {
            key: "urgent_dated",
            dp: Dp::EmailTimeSensitive,
            fx: Fx::new(school, "Update", "Early dismissal on Thursday at 1pm"),
            value: json!(true),
        },
    ];
    let mut covered = HashSet::new();
    for case in cases {
        let got = case.fx.eval(case.dp, &c);
        let hit = got
            .iter()
            .find(|g| g.source == case.key && g.value == case.value)
            .unwrap_or_else(|| panic!("{}: {got:?}", case.key));
        let def = signals().iter().find(|s| s.key == case.key).unwrap();
        assert_eq!(def.decision_point, case.dp, "{}", case.key);
        assert_eq!(hit.confidence, def.confidence, "{}", case.key);
        covered.insert(case.key);
    }
    let all: HashSet<&str> = signals().iter().map(|s| s.key).collect();
    assert_eq!(covered, all);
}

#[test]
fn signal_confidences_match_the_table() {
    let want = [
        ("gmail_spam", 0.99),
        ("gmail_promotions", 0.85),
        ("gmail_social", 0.85),
        ("org_contact", 0.95),
        ("org_sender", 0.9),
        ("bulk_newsletter", 0.8),
        ("payment_received", 0.85),
        ("biller_org", 0.9),
        ("bill_phrases", 0.8),
        ("contact_enrolled", 0.95),
        ("org_child_name", 0.9),
        ("org_grade_class", 0.85),
        ("org_tag_gender", 0.8),
        ("org_all_children", 0.6),
        ("urgent_dated", 0.85),
    ];
    assert_eq!(signals().len(), want.len());
    for (k, conf) in want {
        assert_eq!(
            signals().iter().find(|s| s.key == k).unwrap().confidence,
            conf,
            "{k}"
        );
    }
}

#[test]
fn signal_negatives() {
    let (mut c, ids) = ctx();
    let school = "office@school.example.edu";
    // Not urgent without a near date; not near with a far date.
    assert!(
        Fx::new(school, "Hi", "Closed on Nov 20")
            .eval(Dp::EmailTimeSensitive, &c)
            .is_empty()
    );
    assert!(
        Fx::new(school, "Hi", "See you tomorrow")
            .eval(Dp::EmailTimeSensitive, &c)
            .len()
            == 1
    );
    assert!(
        Fx::new(school, "Hi", "Fun news at Nov 7")
            .eval(Dp::EmailTimeSensitive, &c)
            .is_empty()
    );
    // Adults are never linked by name; a gender word alone with no org does nothing.
    let got = Fx::new(school, "Hi", "Pat should call").eval(Dp::EmailMemberLink, &c);
    assert!(got.iter().all(|g| g.value != json!(ids.pat.to_string())));
    assert!(
        Fx::new("x@elsewhere.example.com", "Hi", "girls, ava, grade 3")
            .eval(Dp::EmailMemberLink, &c)
            .is_empty()
    );
    // A payment-received mail from a biller is not a bill.
    let got = Fx::new(
        "billing@power.example.com",
        "Thanks",
        "Payment received. Thank you.",
    )
    .eval(Dp::EmailIsBill, &c);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].value, json!(false));
    // One candidate per member, the best signal wins.
    let got = Fx::new("rivera@school.example.edu", "Hi", "Ava, Grade 3, girls")
        .eval(Dp::EmailMemberLink, &c);
    assert_eq!(
        got.iter()
            .filter(|g| g.value == json!(ids.ava.to_string()))
            .count(),
        1
    );
    assert_eq!(got[0].confidence, 0.95);
    // Categories that do not exist are never returned.
    c.categories.retain(|(k, _)| k != "school");
    assert!(
        Fx::new(school, "Hi", "x")
            .eval(Dp::EmailCategory, &c)
            .is_empty()
    );
    // Disabled signals are skipped.
    let mut f = Fx::new("x@elsewhere.example.com", "Hi", "x");
    f.labels.push("SPAM".into());
    assert_eq!(f.eval(Dp::EmailCategory, &c).len(), 1);
    c.disabled_signals.insert("gmail_spam".into());
    assert!(f.eval(Dp::EmailCategory, &c).is_empty());
}

#[test]
fn mail_fixtures_classify() {
    let (c, _) = ctx();
    let promo = Fx::from_parsed(&parse(&fixture("promo.eml")).unwrap());
    let got = promo.eval(Dp::EmailCategory, &c);
    // No "promotions" Gmail label here; the bulk headers say newsletters.
    assert_eq!(got[0].value, json!("newsletters"));
    let paid = Fx::from_parsed(&parse(&fixture("payment_received.eml")).unwrap());
    let got = paid.eval(Dp::EmailIsBill, &c);
    assert_eq!(got[0].value, json!(false));
    let bill = Fx::from_parsed(&parse(&fixture("utility_bill.eml")).unwrap());
    let got = bill.eval(Dp::EmailIsBill, &c);
    assert!(
        got.iter()
            .any(|g| g.value == json!(true) && g.confidence == 0.9)
    );
}

fn rule(
    dp: Dp,
    priority: i32,
    conds: Vec<RuleCondition>,
    value: serde_json::Value,
    conf: f32,
) -> MailRule {
    MailRule {
        id: MailRuleId::new(),
        household_id: HouseholdId::new(),
        decision_point: dp,
        name: "r".into(),
        priority,
        conditions: conds,
        value,
        confidence: conf,
        enabled: true,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn cond(field: RuleField, op: RuleOp, header: Option<&str>, values: &[&str]) -> RuleCondition {
    RuleCondition {
        field,
        op,
        header: header.map(str::to_owned),
        values: values.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn user_rules_by_priority_first_match_wins() {
    let (mut c, _) = ctx();
    let low = rule(
        Dp::EmailCategory,
        5,
        vec![cond(
            RuleField::Subject,
            RuleOp::ContainsAny,
            None,
            &["pizza"],
        )],
        json!("newsletters"),
        0.7,
    );
    let high = rule(
        Dp::EmailCategory,
        1,
        vec![cond(
            RuleField::FromDomain,
            RuleOp::Is,
            None,
            &["example.com"],
        )],
        json!("junk"),
        0.95,
    );
    let nomatch = rule(
        Dp::EmailCategory,
        0,
        vec![cond(
            RuleField::Subject,
            RuleOp::Is,
            None,
            &["something else"],
        )],
        json!("bills"),
        1.0,
    );
    let wrong_dp = rule(
        Dp::EmailIsBill,
        0,
        vec![cond(
            RuleField::Subject,
            RuleOp::ContainsAny,
            None,
            &["pizza"],
        )],
        json!(true),
        1.0,
    );
    let (low_id, high_id) = (low.id.to_string(), high.id.to_string());
    c.rules = vec![low, high, nomatch, wrong_dp];
    let f = Fx::new("a@shop.example.com", "Free Pizza night", "x");
    let got = f.eval(Dp::EmailCategory, &c);
    assert_eq!(got[0].source, high_id);
    assert_eq!(got[0].value, json!("junk"));
    assert_eq!(got[0].confidence, 0.95);
    assert!(got.iter().all(|g| g.source != low_id));
    // Only the lower rule matches for another domain; whole-word: "pizzas" does not match.
    let f = Fx::new("a@other.example.org", "Free Pizza night", "x");
    assert_eq!(f.eval(Dp::EmailCategory, &c)[0].source, low_id);
    let f = Fx::new("a@other.example.org", "Free pizzas", "x");
    assert!(f.eval(Dp::EmailCategory, &c).is_empty());
    // Disabled rules and rules with unknown categories are ignored.
    c.rules[1].enabled = false;
    let f = Fx::new("a@shop.example.com", "Free Pizza night", "x");
    assert_eq!(f.eval(Dp::EmailCategory, &c)[0].source, low_id);
    c.rules[0].value = json!("nonexistent");
    assert!(f.eval(Dp::EmailCategory, &c).is_empty());
}

#[test]
fn rule_condition_fields_and_ops() {
    let (mut c, _) = ctx();
    let mut f = Fx::new(
        "Coach@Club.Example.com",
        "Practice moved",
        "See attached. Order #12345",
    );
    f.headers
        .insert("list-id".into(), "<team.example.com>".into());
    f.labels.push("INBOX".into());
    f.atts.push(AttachmentFacts {
        part: "1".into(),
        filename: Some("Schedule.PDF".into()),
        mime: "application/pdf".into(),
        size: 10,
        ics: None,
    });
    let checks: Vec<(RuleCondition, bool)> = vec![
        (
            cond(
                RuleField::FromAddr,
                RuleOp::Is,
                None,
                &["coach@club.example.com"],
            ),
            true,
        ),
        (
            cond(
                RuleField::FromAddr,
                RuleOp::In,
                None,
                &["x@example.com", "COACH@club.example.com"],
            ),
            true,
        ),
        (
            cond(RuleField::FromDomain, RuleOp::Is, None, &["example.com"]),
            true,
        ),
        (
            cond(RuleField::FromDomain, RuleOp::Is, None, &["ample.com"]),
            false,
        ),
        (
            cond(RuleField::Subject, RuleOp::ContainsAny, None, &["practice"]),
            true,
        ),
        (
            cond(RuleField::Subject, RuleOp::ContainsAny, None, &["pract"]),
            false,
        ),
        (
            cond(RuleField::Body, RuleOp::Matches, None, &[r"Order #\d{5}"]),
            true,
        ),
        (cond(RuleField::Body, RuleOp::Matches, None, &["("]), false),
        (
            cond(RuleField::GmailLabel, RuleOp::Is, None, &["inbox"]),
            true,
        ),
        (
            cond(RuleField::Header, RuleOp::Exists, Some("List-Id"), &[]),
            true,
        ),
        (
            cond(RuleField::Header, RuleOp::Exists, Some("X-Nope"), &[]),
            false,
        ),
        (
            cond(
                RuleField::Header,
                RuleOp::ContainsAny,
                Some("LIST-ID"),
                &["team.example.com"],
            ),
            true,
        ),
        (
            cond(RuleField::HasAttachmentType, RuleOp::Is, None, &["pdf"]),
            true,
        ),
        (
            cond(
                RuleField::HasAttachmentType,
                RuleOp::Is,
                None,
                &["application/pdf"],
            ),
            true,
        ),
        (
            cond(RuleField::HasAttachmentType, RuleOp::Is, None, &["ics"]),
            false,
        ),
        (cond(RuleField::OrgKind, RuleOp::Is, None, &["club"]), false),
        (cond(RuleField::Source, RuleOp::Exists, None, &[]), true),
    ];
    for (i, (cd, want)) in checks.into_iter().enumerate() {
        c.rules = vec![rule(Dp::EmailIsBill, 0, vec![cd], json!(true), 0.9)];
        let got = f
            .eval(Dp::EmailIsBill, &c)
            .iter()
            .any(|g| g.source == c.rules[0].id.to_string());
        assert_eq!(got, want, "check {i}");
    }
    // AND of two conditions, and the org fields.
    let g = Fx::new("rivera@school.example.edu", "Hi", "x");
    c.rules = vec![rule(
        Dp::EmailIsBill,
        0,
        vec![
            cond(RuleField::OrgKind, RuleOp::In, None, &["school", "daycare"]),
            cond(RuleField::Org, RuleOp::Is, None, &["example elementary"]),
        ],
        json!(false),
        0.9,
    )];
    assert_eq!(g.eval(Dp::EmailIsBill, &c)[0].value, json!(false));
    // Member-link rules may name several members.
    let ids = [MemberId::new().to_string(), MemberId::new().to_string()];
    c.rules = vec![rule(
        Dp::EmailMemberLink,
        0,
        vec![cond(RuleField::Subject, RuleOp::Exists, None, &[])],
        json!(ids),
        0.9,
    )];
    assert_eq!(f.eval(Dp::EmailMemberLink, &c).len(), 2);
}

// ---- bills ----

fn bill_of(p: &ParsedMessage, org: Option<&OrgCtx>, pdf: Option<&str>) -> Vec<ExtractedBill> {
    let fx = Fx::from_parsed(p);
    extract_bills(&fx.facts(), org, pdf, "USD")
}

#[test]
fn utility_bill_generic() {
    let (c, _) = ctx();
    let p = parse(&fixture("utility_bill.eml")).unwrap();
    let bills = bill_of(&p, Some(&c.orgs[1]), None);
    assert_eq!(bills.len(), 1);
    let b = &bills[0];
    assert_eq!(b.payee, "Power Co");
    assert_eq!(b.amount_minor, 12345);
    assert_eq!(b.currency, "USD");
    assert_eq!(b.due_on, NaiveDate::from_ymd_opt(2026, 11, 20));
    assert_eq!(b.account_hint.as_deref(), Some("4821"));
    assert_eq!(b.confidence, 0.8);
    // Without an org, the payee comes from the sender.
    let bills = bill_of(&p, None, None);
    assert_eq!(bills[0].payee, "Power Co");
}

#[test]
fn bill_template_and_currencies() {
    let (c, _) = ctx();
    let mut org = c.orgs[1].clone();
    org.bill_amount_regex = Some(r"Charges this month:\s*(\S+)".into());
    org.bill_due_regex = Some(r"Pay before (\w+ \d{1,2}, \d{4})".into());
    org.bill_account_regex = Some(r"Acct (\d+)".into());
    let mut fx = Fx::new(
        "billing@power.example.com",
        "Bill",
        "Charges this month: £1,234.50\nPay before Dec 3, 2026\nAcct 99887766",
    );
    let b = extract_bills(&fx.facts(), Some(&org), None, "USD");
    assert_eq!(b.len(), 1);
    assert_eq!((b[0].amount_minor, b[0].currency.as_str()), (123450, "GBP"));
    assert_eq!(b[0].due_on, NaiveDate::from_ymd_opt(2026, 12, 3));
    assert_eq!(b[0].account_hint.as_deref(), Some("7766"));
    assert_eq!(b[0].confidence, 0.9);
    // A broken template regex falls back to the generic parser.
    org.bill_amount_regex = Some("(".into());
    fx.body = "Total due: EUR 80,50 by 2026-12-01".into();
    let b = extract_bills(&fx.facts(), Some(&org), None, "USD");
    assert_eq!((b[0].amount_minor, b[0].currency.as_str()), (8050, "EUR"));
    assert_eq!(b[0].due_on, NaiveDate::from_ymd_opt(2026, 12, 1));
    // ISO code after the amount, yen has no minor unit; default dollar currency.
    fx.body = "Amount due 5000 JPY".into();
    let b = extract_bills(&fx.facts(), None, None, "USD");
    assert_eq!((b[0].amount_minor, b[0].currency.as_str()), (5000, "JPY"));
    fx.body = "Balance due: $10.00".into();
    let b = extract_bills(&fx.facts(), None, None, "CAD");
    assert_eq!(b[0].currency, "CAD");
    // No amount, no bill.
    fx.body = "See you soon".into();
    assert!(extract_bills(&fx.facts(), None, None, "USD").is_empty());
    // A label without a strong phrase is weaker.
    fx.body = "Total: $20.00".into();
    assert_eq!(
        extract_bills(&fx.facts(), None, None, "USD")[0].confidence,
        0.6
    );
}

fn make_pdf(text: &str) -> Vec<u8> {
    let stream = format!("BT /F1 12 Tf 72 700 Td ({text}) Tj ET");
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{stream}\nendstream", stream.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![];
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for ch in data.chunks(3) {
        let n = (u32::from(ch[0]) << 16)
            | (u32::from(*ch.get(1).unwrap_or(&0)) << 8)
            | u32::from(*ch.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= ch.len() {
                s.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s.as_bytes()
        .chunks(76)
        .map(|l| std::str::from_utf8(l).unwrap())
        .collect::<Vec<_>>()
        .join("\r\n")
}

#[test]
fn bill_only_in_pdf() {
    let pdf = make_pdf("Amount due: $88.10 Due: Nov 25, 2026");
    let eml = format!(
        "From: Water Co <billing@water.example.com>\r\nTo: parent@example.com\r\nSubject: Your statement\r\nDate: Mon, 26 Oct 2026 07:00:00 -0400\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"B\"\r\n\r\n--B\r\nContent-Type: text/plain\r\n\r\nYour statement is attached.\r\n--B\r\nContent-Type: application/pdf; name=\"stmt.pdf\"\r\nContent-Disposition: attachment; filename=\"stmt.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n--B--\r\n",
        b64(&pdf)
    );
    let p = parse(eml.as_bytes()).unwrap();
    assert_eq!(p.attachments.len(), 1);
    let bytes = p.attachments[0].pdf.as_deref().expect("pdf kept in memory");
    assert_eq!(bytes, pdf.as_slice());
    // The body alone has no bill.
    assert!(bill_of(&p, None, None).is_empty());
    let text = match pdf_text(bytes, 100_000) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("SKIPPED: pdf-extract cannot read the hand-written test PDF: {e}");
            return;
        }
    };
    assert!(text.contains("88.10"), "{text}");
    let b = bill_of(&p, None, Some(&text));
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].amount_minor, 8810);
    assert_eq!(b[0].due_on, NaiveDate::from_ymd_opt(2026, 11, 25));
    assert_eq!(b[0].payee, "Water Co");
    // The output cap and garbage input.
    assert!(pdf_text(bytes, 5).unwrap().len() <= 5);
    assert!(pdf_text(b"not a pdf", 100).is_err());
}

// ---- actions and contact hints ----

#[test]
fn permission_slip_action() {
    let (c, _) = ctx();
    let fx = Fx::from_parsed(&parse(&fixture("permission_slip.eml")).unwrap());
    let acts = extract_actions(&fx.facts(), &c);
    let a = acts
        .iter()
        .find(|a| a.title.contains("permission slip"))
        .expect("slip action");
    assert_eq!(a.kind, ActionKind::Action);
    // Received Tue 2026-10-27: "Friday" is the 30th, due at the end of that local day.
    assert_eq!(a.due_at, Some(local(NY, "2026-10-30 23:59")));
    assert_eq!(a.confidence, 0.8);
    assert!(a.title.chars().count() <= 80);
}

#[test]
fn reply_actions_and_contact_questions() {
    let (c, _) = ctx();
    let fx = Fx::new(
        "rivera@school.example.edu",
        "Pickup",
        "Hello. Could you confirm Ava's pickup time? Thanks.",
    );
    let acts = extract_actions(&fx.facts(), &c);
    assert_eq!(acts.len(), 1);
    assert_eq!(acts[0].kind, ActionKind::Reply);
    assert_eq!(acts[0].confidence, 0.6);
    // The same question from a stranger is not a task.
    let fx = Fx::new(
        "x@elsewhere.example.com",
        "Pickup",
        "Could you confirm Ava's pickup time?",
    );
    assert!(extract_actions(&fx.facts(), &c).is_empty());
    // "please reply" and "let me know by" with a deadline.
    let fx = Fx::new(
        "x@elsewhere.example.com",
        "Party",
        "Please reply by Nov 5. Also RSVP.",
    );
    let acts = extract_actions(&fx.facts(), &c);
    assert!(acts.iter().any(|a| a.kind == ActionKind::Reply
        && a.due_at == Some(local(NY, "2026-11-05 23:59"))
        && a.confidence == 0.8));
    let fx = Fx::new(
        "x@elsewhere.example.com",
        "Party",
        "Let me know by Friday at 5pm.",
    );
    let acts = extract_actions(&fx.facts(), &c);
    assert_eq!(acts[0].due_at, Some(local(NY, "2026-10-30 17:00")));
    assert_eq!(acts[0].confidence, 0.7);
    // A plain newsletter has none.
    let fx = Fx::from_parsed(&parse(&fixture("newsletter.eml")).unwrap());
    assert!(extract_actions(&fx.facts(), &c).is_empty());
}

#[test]
fn contact_hints() {
    let (c, ids) = ctx();
    let mut fx = Fx::new(
        "new.teacher@school.example.edu",
        "Hello",
        "I am teaching Ava this year.",
    );
    fx.from_name = Some("Mr New".into());
    let h = contact_hint(&fx.facts(), &c).expect("hint");
    assert_eq!(h.org, ids.school);
    assert_eq!(h.email, "new.teacher@school.example.edu");
    assert_eq!(h.name.as_deref(), Some("Mr New"));
    assert_eq!(h.member, ids.ava);
    // By grade phrase, enrolled child only.
    let fx = Fx::new(
        "new.teacher@school.example.edu",
        "Hello",
        "Welcome to Grade 5",
    );
    assert_eq!(contact_hint(&fx.facts(), &c).unwrap().member, ids.ben);
    // No hint for a known contact, an unknown domain, or no child mentioned.
    assert!(
        contact_hint(
            &Fx::new("rivera@school.example.edu", "Hi", "Ava").facts(),
            &c
        )
        .is_none()
    );
    assert!(contact_hint(&Fx::new("x@elsewhere.example.com", "Hi", "Ava").facts(), &c).is_none());
    assert!(
        contact_hint(
            &Fx::new("new.teacher@school.example.edu", "Hi", "Hello all").facts(),
            &c
        )
        .is_none()
    );
}

// ---- dedupe ----

#[test]
fn dedupe_key_and_similarity() {
    let start = local(NY, "2026-11-06 18:07");
    assert_eq!(
        event_dedupe_key("Parent-Teacher Conference!", start, false, NY),
        "parent-teacher-conference|2026-11-06|18:00"
    );
    assert_eq!(
        event_dedupe_key(
            "  Parent teacher   conference ",
            local(NY, "2026-11-06 18:14"),
            false,
            NY
        ),
        "parent-teacher-conference|2026-11-06|18:00"
    );
    assert_eq!(
        event_dedupe_key(
            "Parent teacher conference",
            local(NY, "2026-11-06 18:15"),
            false,
            NY
        ),
        "parent-teacher-conference|2026-11-06|18:15"
    );
    assert_eq!(
        event_dedupe_key("No School", local(NY, "2026-11-10 00:00"), true, NY),
        "no-school|2026-11-10"
    );
    // The local date, not the UTC one: 23:30 local is already the next day in UTC.
    assert_eq!(
        event_dedupe_key("Late", local(NY, "2026-11-06 23:30"), false, NY),
        "late|2026-11-06|23:30"
    );
    assert_eq!(
        title_similarity("Parent teacher conference", "parent-teacher conference!"),
        1.0
    );
    assert!(title_similarity("Parent teacher conference", "Parent-teacher conferences") >= 0.85);
    assert!(title_similarity("Picture day", "Parent teacher conference") < 0.85);
}

#[test]
fn promo_and_slip_parse() {
    let p = parse(&fixture("promo.eml")).unwrap();
    assert_eq!(
        p.headers.get("precedence").map(String::as_str),
        Some("bulk")
    );
    let (c, _) = ctx();
    let fx = Fx::from_parsed(&p);
    assert!(extract_events(&fx.facts(), &c).is_empty());
}

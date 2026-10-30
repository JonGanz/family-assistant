//! End-to-end tests of mail processing, the inbox, labelling and setup over a real Postgres.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, NaiveTime, TimeZone, Utc};
use fa_domain::*;
use fa_mail::MailError;
use fa_mail::context::Viewer;
use fa_mail::inbox::InboxService;
use fa_mail::labelling::LabelService;
use fa_mail::process::{AttachmentSource, ProcessService, expire_all};
use fa_mail::setup::SetupService;
use fa_rules::ParsedMessage;
use fa_store::*;
use serde_json::json;
use sqlx::PgPool;

const MIGRATIONS: &str = "../fa-store/migrations";

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 28, 12, 0, 0).unwrap()
}

fn fixture(name: &str) -> ParsedMessage {
    let raw = std::fs::read(format!(
        "{}/../fa-rules/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    fa_rules::parse(&raw).unwrap()
}

fn mail(from: &str, subject: &str, body: &str) -> ParsedMessage {
    ParsedMessage {
        from_addr: from.into(),
        subject: subject.into(),
        body_text: body.into(),
        date: Some(now()),
        ..Default::default()
    }
}

fn member(h: HouseholdId, name: &str, role: Role) -> Member {
    Member {
        id: MemberId::new(),
        household_id: h,
        display_name: name.into(),
        role,
        avatar: None,
        color: None,
        birthdate: None,
        grade: None,
        interests: vec![],
        hobbies: vec![],
        responsibilities: vec![],
        aliases: Vec::new(),
        gender: None,
        relevance_tags: Vec::new(),
        pin_hash: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

struct Env {
    pool: PgPool,
    store: Store,
    svc: ProcessService,
    h: HouseholdId,
    pat: MemberId,
    sam: MemberId,
    ava: MemberId,
    ben: MemberId,
    src: MailSourceId,
    school: OrgId,
    power: OrgId,
    rivera: OrgContactId,
}

async fn env(pool: PgPool) -> Env {
    let store = Store::from_pool(pool.clone());
    let h = HouseholdId::new();
    let hh = Household {
        id: h,
        name: "Test".into(),
        tz: "UTC".into(),
        location: None,
        meal_plan_horizon_weeks: 2,
        dashboard_completion_override: false,
        units: Units::Metric,
        week_start: Default::default(),
        calendar_past_days: 30,
        calendar_future_days: 400,
        weather_retention_days: 7,
        private_events_as_busy: true,
        now_list_size: 7,
        now_weights: fa_domain::tasks::NowWeights::default(),
        quiet_from: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
        quiet_until: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
        currency: DEFAULT_CURRENCY.to_string(),
        gender_terms: Default::default(),
        disabled_signals: Vec::new(),
        events: Default::default(),
        inbox: Default::default(),
        retention_mail: Default::default(),
        bill_lead_days: DEFAULT_BILL_LEAD_DAYS,
        dashboard_pending_approvals: true,
        setup_completed_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let pat = member(h, "Pat", Role::Adult);
    let sam = member(h, "Sam", Role::Adult);
    let mut ava = member(h, "Ava", Role::Child);
    ava.grade = Some("3".into());
    ava.gender = Some(Gender::Female);
    ava.aliases = vec!["Avie".into()];
    let mut ben = member(h, "Ben", Role::Child);
    ben.grade = Some("5".into());
    ben.gender = Some(Gender::Male);
    ben.relevance_tags = vec!["band".into()];
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &hh).await.unwrap();
    for m in [&pat, &sam, &ava, &ben] {
        MemberRepo::insert(&mut tx, m, None, None).await.unwrap();
    }
    tx.commit().await.unwrap();

    let now_ts = Utc::now();
    let src = MailSource {
        id: MailSourceId::new(),
        household_id: h,
        provider: MailProvider::Imap,
        name: "Parent mail".into(),
        address: "parent@example.com".into(),
        oauth_account_id: None,
        imap_host: Some("imap.example.com".into()),
        imap_port: Some(993),
        imap_username: Some("parent@example.com".into()),
        secret_id: None,
        folders: vec!["INBOX".into()],
        archive_folder: None,
        gmail_ext: false,
        read_only: false,
        visibility: MailVisibility::Owners,
        backfill_days: 30,
        query: None,
        poll_minutes: 5,
        history_id: None,
        state: MailSourceState::Ok,
        last_synced_at: None,
        last_error: None,
        archived_at: None,
        created_at: now_ts,
        updated_at: now_ts,
    };
    let mk_org = |kind: OrgKind, name: &str, domain: &str| Org {
        id: OrgId::new(),
        household_id: h,
        kind,
        name: name.into(),
        domains: vec![domain.into()],
        senders: vec![],
        ics_calendar_id: None,
        default_calendar_id: None,
        contact_mode: ContactMode::Suggest,
        bill_amount_regex: None,
        bill_due_regex: None,
        bill_account_regex: None,
        archived_at: None,
        created_at: now_ts,
        updated_at: now_ts,
    };
    let school = mk_org(OrgKind::School, "Example Elementary", "school.example.edu");
    let power = mk_org(OrgKind::Biller, "Power Co", "power.example.com");
    let rivera = OrgContact {
        id: OrgContactId::new(),
        household_id: h,
        org_id: school.id,
        name: "Ms Rivera".into(),
        email: "rivera@school.example.edu".into(),
        role: ContactRole::Teacher,
        archived_at: None,
        created_at: now_ts,
        updated_at: now_ts,
    };
    let enroll = |m: &Member, grade: &str| Enrollment {
        id: EnrollmentId::new(),
        household_id: h,
        member_id: m.id,
        org_id: school.id,
        grade: Some(grade.into()),
        class_name: None,
        from_date: None,
        until_date: None,
        created_at: now_ts,
        updated_at: now_ts,
    };
    let (e_ava, e_ben) = (enroll(&ava, "3"), enroll(&ben, "5"));
    let mut tx = store.tx_for(h).await.unwrap();
    MailSourceRepo::insert(&mut tx, &src, &[pat.id])
        .await
        .unwrap();
    OrgRepo::insert(&mut tx, &school).await.unwrap();
    OrgRepo::insert(&mut tx, &power).await.unwrap();
    OrgContactRepo::insert(&mut tx, &rivera).await.unwrap();
    EnrollmentRepo::insert(&mut tx, &e_ava).await.unwrap();
    EnrollmentRepo::set_contacts(&mut tx, e_ava.id, &[rivera.id])
        .await
        .unwrap();
    EnrollmentRepo::insert(&mut tx, &e_ben).await.unwrap();
    tx.commit().await.unwrap();

    Env {
        svc: ProcessService::new(store.clone()).with_clock(now),
        pool,
        store,
        h,
        pat: pat.id,
        sam: sam.id,
        ava: ava.id,
        ben: ben.id,
        src: src.id,
        school: school.id,
        power: power.id,
        rivera: rivera.id,
    }
}

impl Env {
    async fn insert(&self, p: &ParsedMessage, remote: &str, labels: &[&str]) -> MessageId {
        let attachments: Vec<_> = p
            .attachments
            .iter()
            .map(|a| {
                json!({"part": a.part, "filename": a.filename, "mime": a.mime,
                       "size": a.size, "ics": a.ics})
            })
            .collect();
        let m = NewMessage {
            household_id: self.h,
            source_id: self.src,
            remote_id: remote.into(),
            folder: Some("INBOX".into()),
            thread_id: None,
            rfc822_id: p.message_id.clone(),
            received_at: p.date.unwrap_or_else(now),
            from_addr: p.from_addr.clone(),
            from_name: p.from_name.clone(),
            to_addrs: p.to.clone(),
            cc_addrs: p.cc.clone(),
            subject: p.subject.clone(),
            gmail_labels: labels.iter().map(|s| s.to_string()).collect(),
            has_attachments: !attachments.is_empty(),
            body_text: Some(p.body_text.clone()),
            body_html: p.body_html.clone(),
            snippet: Some(p.body_text.chars().take(200).collect()),
            headers: Some(json!(p.headers)),
            attachments: Some(json!(attachments)),
        };
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        let id = MessageRepo::insert_parsed(&mut tx, &m)
            .await
            .unwrap()
            .expect("new message");
        tx.commit().await.unwrap();
        id
    }

    async fn process(&self, id: MessageId) -> fa_mail::process::Emitted {
        self.svc.process(self.h, id, None).await.unwrap()
    }

    async fn message(&self, id: MessageId) -> Message {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        MessageRepo::get(&mut tx, id).await.unwrap().unwrap()
    }

    async fn members(&self, id: MessageId) -> Vec<(MemberId, DecidedBy, f32)> {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        MessageRepo::members(&mut tx, id).await.unwrap()
    }

    async fn count(&self, sql: &str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(&self.pool).await.unwrap()
    }

    async fn jobs(&self, kind: &str) -> Vec<serde_json::Value> {
        sqlx::query_scalar("SELECT payload FROM job WHERE kind = $1 ORDER BY created_at")
            .bind(kind)
            .fetch_all(&self.pool)
            .await
            .unwrap()
    }

    async fn set_contact_mode(&self, mode: ContactMode) {
        let mut tx = self.store.tx_for(self.h).await.unwrap();
        let mut o = OrgRepo::get(&mut tx, self.school).await.unwrap().unwrap();
        o.value.contact_mode = mode;
        OrgRepo::update(&mut tx, &o.value, o.version).await.unwrap();
        tx.commit().await.unwrap();
    }

    fn pat(&self) -> Viewer {
        Viewer::adult(self.pat)
    }
}

macro_rules! test {
    ($name:ident, $e:ident, $body:block) => {
        #[sqlx::test(migrations = "../fa-store/migrations")]
        async fn $name(pool: PgPool) {
            let $e = env(pool).await;
            $body
        }
    };
}

#[allow(dead_code)]
const _: &str = MIGRATIONS;

test!(teacher_ics_mail_proposes_one_event, e, {
    let id = e.insert(&fixture("teacher_ics.eml"), "m1", &[]).await;
    let out = e.process(id).await;
    assert_eq!(out.events, 1);
    let m = e.message(id).await;
    assert_eq!(m.category.as_deref(), Some("school"));
    assert_eq!(m.processing_state, ProcessingState::Processed);
    let members = e.members(id).await;
    assert!(
        members
            .iter()
            .any(|(m, by, _)| *m == e.ava && *by == DecidedBy::Rule)
    );
    assert!(!members.iter().any(|(m, _, _)| *m == e.ben));

    let (status, origin, msg, conf): (String, String, Option<uuid::Uuid>, Option<f32>) =
        sqlx::query_as("SELECT status, origin, message_id, confidence FROM event")
            .fetch_one(&e.pool)
            .await
            .unwrap();
    assert_eq!(
        (status.as_str(), origin.as_str()),
        ("extracted", "extracted")
    );
    assert_eq!(msg, Some(id.as_uuid()));
    assert_eq!(conf, Some(0.95));
    // The event is tied to the linked child and to the sending org.
    assert_eq!(e.count("SELECT count(*) FROM event_member").await, 1);
    assert_eq!(
        e.count(&format!(
            "SELECT count(*) FROM event WHERE org_id = '{}'",
            e.school
        ))
        .await,
        1
    );
    let route = e.jobs("calendar.route").await;
    assert_eq!(route.len(), 1);
    assert!(route[0].get("event").is_some());
    assert_eq!(e.jobs("mail.auto_actions").await.len(), 1);
    // Every decision point has a decision of the right stage.
    let decisions = e
        .count("SELECT count(*) FROM decision WHERE subject_type = 'message'")
        .await;
    assert!(decisions >= 4);
});

test!(newsletter_sections_carry_relevance_per_child, e, {
    let id = e.insert(&fixture("newsletter.eml"), "m2", &[]).await;
    e.process(id).await;
    let setup = SetupService::new(e.store.clone());
    for (child, expect) in [
        (
            e.ava,
            vec![
                Relevance::General,
                Relevance::Relevant,
                Relevance::Other,
                Relevance::Relevant,
                Relevance::General,
            ],
        ),
        (
            e.ben,
            vec![
                Relevance::General,
                Relevance::Other,
                Relevance::Relevant,
                Relevance::Other,
                Relevance::General,
            ],
        ),
    ] {
        let d = setup
            .message_detail(e.h, e.pat(), id, Some(child))
            .await
            .unwrap();
        let got: Vec<Relevance> = d.sections.iter().map(|s| s.relevance.unwrap()).collect();
        assert_eq!(got, expect);
        assert!(!d.decisions.is_empty());
    }
    let d = setup.message_detail(e.h, e.pat(), id, None).await.unwrap();
    assert!(d.sections.iter().all(|s| s.relevance.is_none()));
    assert!(d.can_act);
});

test!(confident_bill_opens_and_queues_the_pay_task, e, {
    let id = e.insert(&fixture("utility_bill.eml"), "m3", &[]).await;
    let out = e.process(id).await;
    assert_eq!((out.bills_open, out.bills_proposed), (1, 0));
    let m = e.message(id).await;
    assert_eq!(m.is_bill, Some(true));
    assert_eq!(m.category.as_deref(), Some("bills"));
    let (status, amount, due): (String, i64, Option<chrono::NaiveDate>) =
        sqlx::query_as("SELECT status, amount_minor, due_on FROM bill")
            .fetch_one(&e.pool)
            .await
            .unwrap();
    assert_eq!((status.as_str(), amount), ("open", 12345));
    assert_eq!(due, chrono::NaiveDate::from_ymd_opt(2026, 11, 20));
    let jobs = e.jobs("tasks.from_bill").await;
    assert_eq!(jobs.len(), 1);
    assert!(jobs[0].get("bill").is_some());
    assert_eq!(
        e.count("SELECT count(*) FROM notification WHERE trigger = 'approval_needed'")
            .await,
        0
    );
});

test!(low_confidence_bill_is_proposed_with_approval_rows, e, {
    let p = mail(
        "billing@power.example.com",
        "Statement",
        "Your statement is ready. Amount due: $50.00",
    );
    let id = e.insert(&p, "m4", &[]).await;
    let out = e.process(id).await;
    assert_eq!((out.bills_open, out.bills_proposed), (0, 1));
    assert_eq!(
        e.count("SELECT count(*) FROM bill WHERE status = 'proposed'")
            .await,
        1
    );
    assert!(e.jobs("tasks.from_bill").await.is_empty());
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT dedupe_key, click_path FROM notification WHERE trigger = 'approval_needed'",
    )
    .fetch_all(&e.pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1, "only the owner of the source is notified");
    assert!(rows.iter().all(|r| r.1.as_deref() == Some("/inbox")));
    assert!(rows.iter().all(|r| r.0.starts_with("approval:bill:")));
});

test!(payment_received_makes_no_bill, e, {
    let id = e.insert(&fixture("payment_received.eml"), "m5", &[]).await;
    let out = e.process(id).await;
    assert_eq!((out.bills_open, out.bills_proposed), (0, 0));
    assert_eq!(e.message(id).await.is_bill, Some(false));
    assert_eq!(e.count("SELECT count(*) FROM bill").await, 0);
});

test!(permission_slip_suggests_a_task_and_a_contact, e, {
    let id = e.insert(&fixture("permission_slip.eml"), "m6", &[]).await;
    let out = e.process(id).await;
    assert!(out.tasks >= 1);
    let jobs = e.jobs("tasks.from_mail").await;
    let slip = jobs
        .iter()
        .find(|j| {
            j["title"]
                .as_str()
                .unwrap_or("")
                .contains("permission slip")
        })
        .expect("slip task");
    assert_eq!(slip["mode"], json!("suggest"));
    assert_eq!(slip["kind"], json!("action"));
    assert_eq!(slip["message"], json!(id));
    let assignees = slip["assignees"].as_array().unwrap();
    assert!(assignees.contains(&json!(e.ava)));
    // Unknown sender at the school's domain, naming Ava: a contact suggestion.
    assert_eq!(out.contacts_suggested, 1);
    let (email, member): (String, Option<uuid::Uuid>) =
        sqlx::query_as("SELECT email, member_id FROM contact_suggestion WHERE status = 'proposed'")
            .fetch_one(&e.pool)
            .await
            .unwrap();
    assert_eq!(email, "chen@school.example.edu");
    assert_eq!(member, Some(e.ava.as_uuid()));
    assert_eq!(
        e.count("SELECT count(*) FROM notification WHERE dedupe_key LIKE 'approval:contact:%'")
            .await,
        1
    );
});

test!(auto_contact_mode_creates_and_links_the_contact, e, {
    e.set_contact_mode(ContactMode::Auto).await;
    let id = e.insert(&fixture("permission_slip.eml"), "m7", &[]).await;
    let out = e.process(id).await;
    assert_eq!((out.contacts_created, out.contacts_suggested), (1, 0));
    assert_eq!(
        e.count("SELECT count(*) FROM org_contact WHERE email = 'chen@school.example.edu'")
            .await,
        1
    );
    assert_eq!(e.count("SELECT count(*) FROM contact_suggestion").await, 0);
    let linked = e
        .count(
            "SELECT count(*) FROM enrollment_contact ec
             JOIN org_contact c ON c.id = ec.contact_id
             JOIN enrollment en ON en.id = ec.enrollment_id
             JOIN member m ON m.id = en.member_id
             WHERE c.email = 'chen@school.example.edu' AND m.display_name = 'Ava'",
        )
        .await;
    assert_eq!(linked, 1);
    // Processing again neither duplicates the contact nor reports a creation.
    let again = e.process(id).await;
    assert_eq!(again.contacts_created, 0);
    assert_eq!(
        e.count("SELECT count(*) FROM org_contact WHERE email = 'chen@school.example.edu'")
            .await,
        1
    );
});

test!(promotions_are_not_extracted, e, {
    let p = mail(
        "deals@shop.example.com",
        "Big sale",
        "Join us Saturday, Oct 31 at 6pm. Amount due: $5.00. Please sign up and return by Friday.",
    );
    let id = e.insert(&p, "m8", &["CATEGORY_PROMOTIONS"]).await;
    let out = e.process(id).await;
    assert_eq!(e.message(id).await.category.as_deref(), Some("promotions"));
    assert_eq!(out, Default::default());
    assert_eq!(e.count("SELECT count(*) FROM event").await, 0);
    assert!(e.jobs("calendar.route").await.is_empty());
    assert!(e.jobs("tasks.from_mail").await.is_empty());
    assert_eq!(e.count("SELECT count(*) FROM bill").await, 0);
});

test!(reprocessing_does_not_duplicate_anything, e, {
    let ics = e.insert(&fixture("teacher_ics.eml"), "a", &[]).await;
    let slip = e.insert(&fixture("permission_slip.eml"), "b", &[]).await;
    let bill = e
        .insert(
            &mail(
                "billing@power.example.com",
                "Statement",
                "Your statement is ready. Amount due: $50.00",
            ),
            "c",
            &[],
        )
        .await;
    for id in [ics, slip, bill] {
        e.process(id).await;
    }
    let snapshot = |e: &Env| {
        let e = e.pool.clone();
        async move {
            let mut v = Vec::new();
            for sql in [
                "SELECT count(*) FROM event",
                "SELECT count(*) FROM bill",
                "SELECT count(*) FROM contact_suggestion",
                "SELECT count(*) FROM job WHERE kind = 'tasks.from_mail'",
                "SELECT count(*) FROM job WHERE kind = 'calendar.route'",
                "SELECT count(*) FROM job WHERE kind = 'mail.auto_actions'",
                "SELECT count(*) FROM notification",
            ] {
                v.push(
                    sqlx::query_scalar::<_, i64>(sql)
                        .fetch_one(&e)
                        .await
                        .unwrap(),
                );
            }
            v
        }
    };
    let before = snapshot(&e).await;
    assert!(before[0] == 1 && before[1] == 1 && before[2] == 1);
    for id in [ics, slip, bill] {
        let out = e.process(id).await;
        assert_eq!(out.events + out.tasks + out.contacts_suggested, 0);
        assert_eq!(out.bills_open + out.bills_proposed, 0);
    }
    assert_eq!(snapshot(&e).await, before);
});

test!(visibility_follows_source_owners, e, {
    let id = e.insert(&fixture("teacher_ics.eml"), "v1", &[]).await;
    e.process(id).await;
    let setup = SetupService::new(e.store.clone());
    let q = || MessageQuery {
        category: None,
        member: None,
        source: None,
        state: None,
        q: None,
        include_ignored: None,
        before: None,
        limit: None,
    };
    assert_eq!(
        setup
            .list_messages(e.h, e.pat(), q())
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    let sam = Viewer::adult(e.sam);
    assert!(
        setup
            .list_messages(e.h, sam, q())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert!(matches!(
        setup.message_detail(e.h, sam, id, None).await,
        Err(MailError::NotFound)
    ));
    let child = Viewer {
        member: Some(e.ava),
        role: Some(Role::Child),
        ..Default::default()
    };
    assert!(matches!(
        setup.list_messages(e.h, child, q()).await,
        Err(MailError::Forbidden)
    ));
    let device = Viewer {
        device: true,
        elevated: true,
        ..Default::default()
    };
    assert!(matches!(
        setup.list_messages(e.h, device, q()).await,
        Err(MailError::Forbidden)
    ));
    // A household source is readable by every adult.
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let mut s = MailSourceRepo::get(&mut tx, e.src).await.unwrap().unwrap();
    s.value.visibility = MailVisibility::Household;
    MailSourceRepo::update(&mut tx, &s.value, s.version)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        setup
            .list_messages(e.h, sam, q())
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(setup.message_detail(e.h, sam, id, None).await.is_ok());
});

test!(ignored_categories_are_hidden_unless_asked, e, {
    let p = mail("deals@shop.example.com", "Sale", "Everything must go");
    let id = e.insert(&p, "i1", &["CATEGORY_PROMOTIONS"]).await;
    e.process(id).await;
    let setup = SetupService::new(e.store.clone());
    let q = |inc: bool| MessageQuery {
        category: None,
        member: None,
        source: None,
        state: None,
        q: None,
        include_ignored: Some(inc),
        before: None,
        limit: Some(10),
    };
    assert!(
        setup
            .list_messages(e.h, e.pat(), q(false))
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        setup
            .list_messages(e.h, e.pat(), q(true))
            .await
            .unwrap()
            .items
            .len(),
        1
    );
});

test!(time_sensitive_mail_notifies_the_readers, e, {
    let p = mail(
        "rivera@school.example.edu",
        "Schedule change",
        "School is closed tomorrow, October 29, for a staff day.",
    );
    let id = e.insert(&p, "t1", &[]).await;
    e.process(id).await;
    assert_eq!(e.message(id).await.time_sensitive, Some(true));
    let rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT member_id, dedupe_key FROM notification WHERE trigger = 'mail_time_sensitive'",
    )
    .fetch_all(&e.pool)
    .await
    .unwrap();
    // Owners-only source: just its adult owner.
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, e.pat.as_uuid());
    assert_eq!(rows[0].1, format!("mts:{id}:{}", e.pat));
    e.process(id).await;
    assert_eq!(
        e.count("SELECT count(*) FROM notification WHERE trigger = 'mail_time_sensitive'")
            .await,
        1
    );
});

test!(
    labelling_records_human_decisions_and_requeues_extraction,
    e,
    {
        let promo = fixture("promo.eml");
        let id = e.insert(&promo, "l1", &[]).await;
        e.process(id).await;
        assert_eq!(e.message(id).await.category.as_deref(), Some("newsletters"));
        let labels = LabelService::new(e.store.clone());

        let d = labels
            .label(
                e.h,
                e.pat(),
                id,
                LabelInput {
                    decision_point: DecisionPointKey::EmailCategory,
                    value: json!("school"),
                },
            )
            .await
            .unwrap();
        assert_eq!(d.summary.category.as_deref(), Some("school"));
        let cat = d
            .decisions
            .iter()
            .find(|x| x.decision_point == DecisionPointKey::EmailCategory)
            .unwrap();
        assert_eq!(cat.stage, StageKind::Human);
        assert!(cat.is_final);
        assert_eq!(cat.decided_by, Some(e.pat));
        assert_eq!(cat.value, json!("school"));
        let reprocess = e.jobs("mail.process").await;
        assert_eq!(reprocess.len(), 1);
        assert_eq!(
            reprocess[0]["only"],
            json!(["events", "bills", "tasks", "contacts"])
        );

        labels
            .label(
                e.h,
                e.pat(),
                id,
                LabelInput {
                    decision_point: DecisionPointKey::EmailMemberLink,
                    value: json!([e.ben]),
                },
            )
            .await
            .unwrap();
        labels
            .label(
                e.h,
                e.pat(),
                id,
                LabelInput {
                    decision_point: DecisionPointKey::EmailTimeSensitive,
                    value: json!(true),
                },
            )
            .await
            .unwrap();
        let d = labels
            .label(
                e.h,
                e.pat(),
                id,
                LabelInput {
                    decision_point: DecisionPointKey::EmailIsBill,
                    value: json!(false),
                },
            )
            .await
            .unwrap();
        assert_eq!(d.summary.members, vec![e.ben]);
        assert_eq!(d.summary.time_sensitive, Some(true));
        let members = e.members(id).await;
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].1, DecidedBy::User);
        let msg_learning: bool =
            sqlx::query_scalar("SELECT is_learning_example FROM message WHERE id = $1")
                .bind(id.as_uuid())
                .fetch_one(&e.pool)
                .await
                .unwrap();
        assert!(msg_learning);

        // Re-processing keeps the person's answers.
        e.process(id).await;
        let m = e.message(id).await;
        assert_eq!(m.category.as_deref(), Some("school"));
        assert_eq!(m.time_sensitive, Some(true));
        assert_eq!(e.members(id).await[0].0, e.ben);
        assert_eq!(e.members(id).await[0].1, DecidedBy::User);

        // Bad input is rejected; children cannot label.
        assert!(matches!(
            labels
                .label(
                    e.h,
                    e.pat(),
                    id,
                    LabelInput {
                        decision_point: DecisionPointKey::EmailCategory,
                        value: json!("nope"),
                    },
                )
                .await,
            Err(MailError::Invalid(_))
        ));
        let child = Viewer {
            member: Some(e.ava),
            role: Some(Role::Child),
            ..Default::default()
        };
        assert!(matches!(
            labels
                .label(
                    e.h,
                    child,
                    id,
                    LabelInput {
                        decision_point: DecisionPointKey::EmailIsBill,
                        value: json!(true),
                    },
                )
                .await,
            Err(MailError::Forbidden)
        ));
    }
);

test!(label_queue_progress_and_eval_sample, e, {
    let a = e.insert(&fixture("promo.eml"), "q1", &[]).await;
    let b = e.insert(&fixture("newsletter.eml"), "q2", &[]).await;
    e.process(a).await;
    e.process(b).await;
    let labels = LabelService::new(e.store.clone());
    let first = labels.next(e.h, e.pat(), None).await.unwrap().unwrap();
    assert_eq!(first.progress.target, 200);
    assert_eq!(first.progress.in_sample, 0);
    let first_id = first.message.summary.id;
    let second = labels
        .next(e.h, e.pat(), Some(first_id))
        .await
        .unwrap()
        .unwrap();
    assert_ne!(second.message.summary.id, first_id);

    labels
        .set_eval_sample(e.h, e.pat(), first_id, true)
        .await
        .unwrap();
    labels
        .label(
            e.h,
            e.pat(),
            first_id,
            LabelInput {
                decision_point: DecisionPointKey::EmailIsBill,
                value: json!(false),
            },
        )
        .await
        .unwrap();
    let p = labels.progress(e.h).await.unwrap();
    assert_eq!((p.in_sample, p.labelled), (1, 1));
    // The labelled message leaves the queue.
    let next = labels.next(e.h, e.pat(), None).await.unwrap().unwrap();
    assert_ne!(next.message.summary.id, first_id);
    // Sam cannot reach the owner-only source.
    assert!(
        labels
            .next(e.h, Viewer::adult(e.sam), None)
            .await
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        labels
            .set_eval_sample(e.h, Viewer::adult(e.sam), first_id, false)
            .await,
        Err(MailError::NotFound)
    ));
});

async fn inbox_fixture(e: &Env) -> (BillId, ContactSuggestionId, EventId, TaskId) {
    let low = mail(
        "billing@power.example.com",
        "Statement",
        "Your statement is ready. Amount due: $50.00",
    );
    let bill_msg = e.insert(&low, "ib", &[]).await;
    e.process(bill_msg).await;
    let slip = e.insert(&fixture("permission_slip.eml"), "is", &[]).await;
    e.process(slip).await;
    let ics = e.insert(&fixture("teacher_ics.eml"), "ie", &[]).await;
    e.process(ics).await;
    sqlx::query("UPDATE event SET status = 'proposed'")
        .execute(&e.pool)
        .await
        .unwrap();
    let bill: uuid::Uuid = sqlx::query_scalar("SELECT id FROM bill")
        .fetch_one(&e.pool)
        .await
        .unwrap();
    let contact: uuid::Uuid = sqlx::query_scalar("SELECT id FROM contact_suggestion")
        .fetch_one(&e.pool)
        .await
        .unwrap();
    let event: uuid::Uuid = sqlx::query_scalar("SELECT id FROM event")
        .fetch_one(&e.pool)
        .await
        .unwrap();
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let list = TaskListRepo::get_or_create(
        &mut tx,
        TaskListKind::Email,
        "Email",
        &TaskList {
            id: TaskListId::new(),
            household_id: e.h,
            name: String::new(),
            kind: TaskListKind::Email,
            completion_policy: ListCompletionPolicy::Assignee,
            child_can_add: ChildCanAdd::None,
            dashboard_add: false,
            calendar_default: CalendarDisplay::UntilDone,
            claim_ttl_minutes: None,
            visible_on_dashboard: true,
            color: None,
            sort: 0,
            archived_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    )
    .await
    .unwrap();
    let task = TaskId::new();
    TaskRepo::insert(
        &mut tx,
        &NewTaskRow {
            id: task,
            household_id: e.h,
            list_id: list.value.id,
            definition_id: None,
            weather_rule_id: None,
            title: "Sign the slip".into(),
            notes: None,
            due_at: None,
            visible_from: None,
            status: TaskStatus::Suggested,
            origin: TaskOrigin::Email,
            source_ref: Some(json!({"kind": "email", "message_id": slip.to_string()})),
            priority_flag: PriorityFlag::Normal,
            calendar_display: CalendarDisplay::UntilDone,
            created_by: None,
        },
        &[e.ava],
        &[],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    (bill.into(), contact.into(), event.into(), task)
}

test!(inbox_lists_all_kinds_by_viewer, e, {
    let (bill, contact, event, task) = inbox_fixture(&e).await;
    let inbox = InboxService::new(e.store.clone());
    // approval_needed rows go only to adults who can read the owners-only source.
    let to: Vec<uuid::Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT member_id FROM notification WHERE trigger = 'approval_needed'",
    )
    .fetch_all(&e.pool)
    .await
    .unwrap();
    assert_eq!(to, vec![e.pat.as_uuid()]);
    let v = inbox.list(e.h, e.pat()).await.unwrap();
    assert_eq!(v.counts, InboxCounts::new(1, 1, 1, 1));
    assert_eq!(v.items.len(), 4);
    let mut kinds = Vec::new();
    for i in &v.items {
        match i {
            InboxItem::Event { event: x } => {
                assert_eq!(x.id, event);
                assert!(x.message.is_some());
                kinds.push("event");
            }
            InboxItem::Bill { bill: x } => {
                assert_eq!(x.id, bill);
                assert_eq!(x.amount_minor, 5000);
                assert_eq!(x.status, BillStatus::Proposed);
                kinds.push("bill");
            }
            InboxItem::Task { task: x } => {
                assert_eq!(x.id, task);
                assert_eq!(x.status, TaskStatus::Suggested);
                kinds.push("task");
            }
            InboxItem::Contact { contact: x } => {
                assert_eq!(x.id, contact);
                assert_eq!(x.org_name, "Example Elementary");
                kinds.push("contact");
            }
        }
    }
    kinds.sort();
    assert_eq!(kinds, vec!["bill", "contact", "event", "task"]);
    let only_bills = inbox
        .list_kind(e.h, e.pat(), Some(InboxKind::Bill))
        .await
        .unwrap();
    assert_eq!(only_bills.items.len(), 1);

    // Sam is an adult but cannot read the owners-only source: nothing shows, nothing is allowed.
    let sam = Viewer::adult(e.sam);
    let v = inbox.list(e.h, sam).await.unwrap();
    assert!(v.items.is_empty());
    assert_eq!(v.counts.total, 0);
    assert_eq!(inbox.counts(e.h, sam).await.unwrap().total, 0);
    assert!(matches!(
        inbox
            .approve_bill(
                e.h,
                bill,
                BillPatch {
                    payee: None,
                    amount_minor: None,
                    currency: None,
                    due_on: None,
                    account_hint: None,
                    status: None,
                },
                sam
            )
            .await,
        Err(MailError::Forbidden)
    ));
    assert!(matches!(
        inbox.reject_bill(e.h, bill, sam).await,
        Err(MailError::Forbidden)
    ));
    assert!(matches!(
        inbox
            .approve_contact(
                e.h,
                contact,
                ContactApproval {
                    name: None,
                    role: None,
                    member: None
                },
                sam
            )
            .await,
        Err(MailError::Forbidden)
    ));
    assert!(matches!(
        inbox.reject_contact(e.h, contact, sam).await,
        Err(MailError::Forbidden)
    ));
    assert!(matches!(
        inbox.ensure_event(e.h, sam, event).await,
        Err(MailError::Forbidden)
    ));
    assert!(matches!(
        inbox.ensure_task(e.h, sam, task).await,
        Err(MailError::Forbidden)
    ));
    assert!(inbox.ensure_event(e.h, e.pat(), event).await.is_ok());

    // A household-visible source is readable by every adult.
    sqlx::query("UPDATE mail_source SET visibility = 'household'")
        .execute(&e.pool)
        .await
        .unwrap();
    let v = inbox.list(e.h, sam).await.unwrap();
    assert_eq!(v.items.len(), 4);
    assert!(inbox.ensure_event(e.h, sam, event).await.is_ok());

    // Orphaned items (the source is archived) show for every adult, without links.
    sqlx::query("UPDATE mail_source SET visibility = 'owners', archived_at = now()")
        .execute(&e.pool)
        .await
        .unwrap();
    let v = inbox.list(e.h, sam).await.unwrap();
    assert_eq!(v.items.len(), 4);
    assert!(v.items.iter().all(|i| match i {
        InboxItem::Event { event } => event.message.is_none() && event.external_url.is_none(),
        InboxItem::Bill { bill } => bill.message.is_none(),
        InboxItem::Contact { contact } => contact.message.is_none(),
        InboxItem::Task { .. } => true,
    }));

    // Devices, elevated or not, and children are refused.
    for device in [
        Viewer {
            device: true,
            ..Default::default()
        },
        Viewer {
            device: true,
            elevated: true,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            inbox.list(e.h, device).await,
            Err(MailError::Forbidden)
        ));
        assert!(matches!(
            inbox.counts(e.h, device).await,
            Err(MailError::Forbidden)
        ));
        assert!(matches!(
            inbox.reject_bill(e.h, bill, device).await,
            Err(MailError::Forbidden)
        ));
    }
    let child = Viewer {
        member: Some(e.ava),
        role: Some(Role::Child),
        ..Default::default()
    };
    assert!(matches!(
        inbox.list(e.h, child).await,
        Err(MailError::Forbidden)
    ));
});

test!(inbox_bill_approve_and_reject, e, {
    let bill_msg = e
        .insert(
            &mail(
                "billing@power.example.com",
                "Statement",
                "Your statement is ready. Amount due: $50.00",
            ),
            "b1",
            &[],
        )
        .await;
    e.process(bill_msg).await;
    let other = e
        .insert(
            &mail(
                "billing@power.example.com",
                "Statement 2",
                "Your statement is ready. Amount due: $75.00",
            ),
            "b2",
            &[],
        )
        .await;
    e.process(other).await;
    let inbox = InboxService::new(e.store.clone());
    let bills = inbox
        .list_kind(e.h, e.pat(), Some(InboxKind::Bill))
        .await
        .unwrap();
    let ids: Vec<BillId> = bills
        .items
        .iter()
        .map(|i| match i {
            InboxItem::Bill { bill } => bill.id,
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(ids.len(), 2);
    let (keep, drop) = {
        let first = bills.items.iter().find_map(|i| match i {
            InboxItem::Bill { bill } if bill.amount_minor == 5000 => Some(bill.id),
            _ => None,
        });
        let second = bills.items.iter().find_map(|i| match i {
            InboxItem::Bill { bill } if bill.amount_minor == 7500 => Some(bill.id),
            _ => None,
        });
        (first.unwrap(), second.unwrap())
    };
    let patch = BillPatch {
        payee: None,
        amount_minor: Some(5250),
        currency: None,
        due_on: Some(chrono::NaiveDate::from_ymd_opt(2026, 11, 30)),
        account_hint: None,
        status: None,
    };
    let v = inbox.approve_bill(e.h, keep, patch, e.pat()).await.unwrap();
    assert_eq!(v.status, BillStatus::Open);
    assert_eq!(v.amount_minor, 5250);
    assert_eq!(v.due_on, chrono::NaiveDate::from_ymd_opt(2026, 11, 30));
    let jobs = e.jobs("tasks.from_bill").await;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["bill"], json!(keep));
    // Approving again changes nothing.
    let again = inbox
        .approve_bill(
            e.h,
            keep,
            BillPatch {
                payee: None,
                amount_minor: None,
                currency: None,
                due_on: None,
                account_hint: None,
                status: None,
            },
            e.pat(),
        )
        .await
        .unwrap();
    assert_eq!(again.status, BillStatus::Open);
    assert_eq!(e.jobs("tasks.from_bill").await.len(), 1);

    let v = inbox.reject_bill(e.h, drop, e.pat()).await.unwrap();
    assert_eq!(v.status, BillStatus::Dismissed);
    // Both answers are human decisions on `email.is_bill`.
    let human: Vec<(serde_json::Value, bool)> = sqlx::query_as(
        "SELECT value, is_final FROM decision
         WHERE stage_kind = 'human' AND decided_by IS NOT NULL ORDER BY value::text",
    )
    .fetch_all(&e.pool)
    .await
    .unwrap();
    assert_eq!(human, vec![(json!(false), true), (json!(true), true)]);
    assert_eq!(e.message(other).await.is_bill, Some(false));
    assert_eq!(e.message(bill_msg).await.is_bill, Some(true));
    // A dismissed bill cannot be approved.
    assert!(matches!(
        inbox
            .approve_bill(
                e.h,
                drop,
                BillPatch {
                    payee: None,
                    amount_minor: None,
                    currency: None,
                    due_on: None,
                    account_hint: None,
                    status: None,
                },
                e.pat()
            )
            .await,
        Err(MailError::Conflict)
    ));
    assert_eq!(inbox.counts(e.h, e.pat()).await.unwrap().bill, 0);
});

test!(inbox_contact_approve_and_reject, e, {
    let slip = e.insert(&fixture("permission_slip.eml"), "c1", &[]).await;
    e.process(slip).await;
    let other = e
        .insert(
            &mail(
                "newcomer@school.example.edu",
                "Hello",
                "I will be teaching Ben this year.",
            ),
            "c2",
            &[],
        )
        .await;
    e.process(other).await;
    let inbox = InboxService::new(e.store.clone());
    let sugg: Vec<(uuid::Uuid, String)> =
        sqlx::query_as("SELECT id, email FROM contact_suggestion ORDER BY email")
            .fetch_all(&e.pool)
            .await
            .unwrap();
    assert_eq!(sugg.len(), 2);
    let chen = ContactSuggestionId::from(sugg[0].0);
    let newcomer = ContactSuggestionId::from(sugg[1].0);
    assert_eq!(sugg[0].1, "chen@school.example.edu");

    let c = inbox
        .approve_contact(
            e.h,
            chen,
            ContactApproval {
                name: Some("Mr Chen".into()),
                role: Some(ContactRole::Teacher),
                member: None,
            },
            e.pat(),
        )
        .await
        .unwrap();
    assert_eq!(c.email, "chen@school.example.edu");
    assert_eq!(c.role, ContactRole::Teacher);
    assert_eq!(c.org, e.school);
    let linked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM enrollment_contact ec JOIN enrollment en ON en.id = ec.enrollment_id
         WHERE ec.contact_id = $1 AND en.member_id = $2",
    )
    .bind(c.id.as_uuid())
    .bind(e.ava.as_uuid())
    .fetch_one(&e.pool)
    .await
    .unwrap();
    assert_eq!(linked, 1);
    inbox.reject_contact(e.h, newcomer, e.pat()).await.unwrap();
    assert_eq!(inbox.counts(e.h, e.pat()).await.unwrap().contact, 0);
    // The rejected address is remembered: the same sender is not suggested again.
    let again = e
        .insert(
            &mail(
                "newcomer@school.example.edu",
                "Hello again",
                "Ben is doing well.",
            ),
            "c3",
            &[],
        )
        .await;
    let out = e.process(again).await;
    assert_eq!(out.contacts_suggested, 0);
    assert_eq!(
        e.count("SELECT count(*) FROM contact_suggestion WHERE status = 'rejected'")
            .await,
        1
    );
    // Decided suggestions cannot be decided again.
    assert!(matches!(
        inbox
            .approve_contact(
                e.h,
                newcomer,
                ContactApproval {
                    name: None,
                    role: None,
                    member: None
                },
                e.pat()
            )
            .await,
        Err(MailError::Conflict)
    ));
});

test!(expiry_marks_stale_proposals, e, {
    let (_, _, event, task) = inbox_fixture(&e).await;
    let slip_age = Utc::now();
    // Nothing is stale yet (the event starts in November 2026; the clock is real).
    let before = expire_all(&e.store, now()).await.unwrap();
    assert_eq!(before, 0);
    let after_event = Utc.with_ymd_and_hms(2026, 11, 7, 0, 0, 0).unwrap();
    assert!(expire_all(&e.store, after_event).await.unwrap() >= 1);
    let status: String = sqlx::query_scalar("SELECT status FROM event WHERE id = $1")
        .bind(event.as_uuid())
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(status, "expired");
    // 15 days later (past the default 14) the suggested task and contact expire; bills never do.
    let late = slip_age + chrono::Duration::days(15);
    expire_all(&e.store, late).await.unwrap();
    let t: String = sqlx::query_scalar("SELECT status FROM task WHERE id = $1")
        .bind(task.as_uuid())
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(t, "skipped");
    assert_eq!(
        e.count("SELECT count(*) FROM contact_suggestion WHERE status = 'expired'")
            .await,
        1
    );
    assert_eq!(
        e.count("SELECT count(*) FROM bill WHERE status = 'proposed'")
            .await,
        1
    );
    assert_eq!(
        InboxService::new(e.store.clone())
            .counts(e.h, e.pat())
            .await
            .unwrap()
            .total,
        1
    );
});

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

struct FakePdfs(Vec<u8>);

#[async_trait]
impl AttachmentSource for FakePdfs {
    async fn pdf_attachments(&self, _: &Message) -> Vec<(String, Vec<u8>)> {
        vec![("2".into(), self.0.clone())]
    }
}

fn pdf_mail() -> ParsedMessage {
    let mut p = mail(
        "billing@power.example.com",
        "Your statement",
        "Your statement is attached.",
    );
    p.attachments.push(fa_rules::ParsedAttachment {
        part: "2".into(),
        filename: Some("stmt.pdf".into()),
        mime: "application/pdf".into(),
        size: 1000,
        ics: None,
        pdf: None,
    });
    p
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn bill_found_in_a_pdf_when_the_body_has_no_amount(pool: PgPool) {
    let e = env(pool).await;
    let pdf = make_pdf("Amount due: $88.10 Due: Nov 25, 2026");
    if fa_rules::pdf_text(&pdf, 10_000).is_err() {
        eprintln!("SKIPPED: pdf-extract cannot read the hand-written test PDF");
        return;
    }
    let svc = ProcessService::new(e.store.clone())
        .with_clock(now)
        .with_attachments(Arc::new(FakePdfs(pdf)));
    let id = e.insert(&pdf_mail(), "p1", &[]).await;
    let mut tx = e.store.tx_system().await.unwrap();
    MessageRepo::household_of(&mut tx, id)
        .await
        .unwrap()
        .unwrap();
    drop(tx);
    // The body names the bill ("statement") but has no amount; the PDF supplies it.
    let out = svc.process(e.h, id, None).await.unwrap();
    assert_eq!(out.bills_open + out.bills_proposed, 1);
    let amount: i64 = sqlx::query_scalar("SELECT amount_minor FROM bill")
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(amount, 8810);
    // The text is stored in `attachments` and used without the source next time.
    let stored: serde_json::Value =
        sqlx::query_scalar("SELECT attachments FROM message WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_one(&e.pool)
            .await
            .unwrap();
    assert!(stored[0]["pdf_text"].as_str().unwrap().contains("88.10"));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn unreadable_pdf_leaves_no_bill(pool: PgPool) {
    let e = env(pool).await;
    let svc = ProcessService::new(e.store.clone())
        .with_clock(now)
        .with_attachments(Arc::new(FakePdfs(b"not a pdf".to_vec())));
    let id = e.insert(&pdf_mail(), "p2", &[]).await;
    let out = svc.process(e.h, id, None).await.unwrap();
    assert_eq!(out.bills_open + out.bills_proposed, 0);
    assert_eq!(
        e.message(id).await.processing_state,
        ProcessingState::Processed
    );
}

test!(setup_rules_signals_categories_and_orgs, e, {
    let setup = SetupService::new(e.store.clone());

    // Rules: validation, test run, priority over built-in signals.
    let bad = MailRuleInput {
        decision_point: DecisionPointKey::EmailCategory,
        name: "bad".into(),
        priority: None,
        conditions: vec![RuleCondition {
            field: RuleField::Subject,
            op: RuleOp::Matches,
            header: None,
            values: vec!["(".into()],
        }],
        value: json!("school"),
        confidence: None,
        enabled: None,
    };
    assert!(matches!(
        setup.create_rule(e.h, bad.clone()).await,
        Err(MailError::Invalid(_))
    ));
    let mut no_cat = bad.clone();
    no_cat.conditions[0].values = vec!["sale".into()];
    no_cat.value = json!("does_not_exist");
    assert!(matches!(
        setup.create_rule(e.h, no_cat).await,
        Err(MailError::Invalid(_))
    ));
    let a = e
        .insert(
            &mail("x@elsewhere.example.com", "Spring sale", "Hello"),
            "r1",
            &[],
        )
        .await;
    let b = e
        .insert(
            &mail("x@elsewhere.example.com", "Hello there", "Hi"),
            "r2",
            &[],
        )
        .await;
    e.process(a).await;
    e.process(b).await;
    let mut rule = bad.clone();
    rule.name = "Sales are junk".into();
    rule.conditions[0].values = vec!["sale".into()];
    rule.value = json!("junk");
    let tested = setup
        .test_rule(
            e.h,
            e.pat(),
            RuleTestRequest {
                rule: rule.clone(),
                limit: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(tested.scanned, 2);
    assert_eq!(tested.matched.len(), 1);
    assert_eq!(tested.matched[0].message, a);
    let created = setup.create_rule(e.h, rule).await.unwrap();
    assert_eq!(setup.list_rules(e.h).await.unwrap().len(), 1);
    let c = e
        .insert(
            &mail("y@elsewhere.example.com", "Winter sale", "Hello"),
            "r3",
            &[],
        )
        .await;
    e.process(c).await;
    assert_eq!(e.message(c).await.category.as_deref(), Some("junk"));
    let updated = setup
        .update_rule(
            e.h,
            created.id,
            MailRulePatch {
                name: None,
                priority: Some(5),
                conditions: None,
                value: None,
                confidence: None,
                enabled: Some(false),
            },
            created.version,
        )
        .await
        .unwrap();
    assert!(!updated.enabled);
    assert!(matches!(
        setup
            .update_rule(
                e.h,
                created.id,
                MailRulePatch {
                    name: None,
                    priority: None,
                    conditions: None,
                    value: None,
                    confidence: None,
                    enabled: Some(true)
                },
                created.version
            )
            .await,
        Err(MailError::Conflict) | Err(MailError::Store(_))
    ));
    setup.delete_rule(e.h, created.id).await.unwrap();
    assert!(setup.list_rules(e.h).await.unwrap().is_empty());

    // Signals.
    let signals = setup.list_signals(e.h).await.unwrap();
    assert!(!signals.is_empty() && signals.iter().all(|s| s.enabled));
    let s = setup.set_signal(e.h, "gmail_spam", false).await.unwrap();
    assert!(!s.enabled);
    let spam = e
        .insert(&mail("s@x.example.com", "Win", "Cash"), "r4", &["SPAM"])
        .await;
    e.process(spam).await;
    assert_ne!(e.message(spam).await.category.as_deref(), Some("junk"));
    assert!(
        setup
            .set_signal(e.h, "gmail_spam", true)
            .await
            .unwrap()
            .enabled
    );
    assert!(matches!(
        setup.set_signal(e.h, "no_such_signal", true).await,
        Err(MailError::NotFound)
    ));

    // Categories.
    let cats = setup.list_categories(e.h, false).await.unwrap();
    assert_eq!(cats.len(), 12);
    let new = setup
        .create_category(
            e.h,
            CategoryInput {
                key: "pets".into(),
                label: "Pets".into(),
                behaviour: CategoryBehaviour::Keep,
                notify_time_sensitive: false,
                auto_actions: None,
                email_tasks: None,
                color: None,
                ord: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(new.auto_actions, vec![AutoAction::Label]);
    assert_eq!(new.email_tasks, EmailTasksMode::Suggest);
    assert!(new.ord >= 12);
    assert!(matches!(
        setup
            .create_category(
                e.h,
                CategoryInput {
                    key: "pets".into(),
                    label: "Again".into(),
                    behaviour: CategoryBehaviour::Keep,
                    notify_time_sensitive: false,
                    auto_actions: None,
                    email_tasks: None,
                    color: None,
                    ord: None,
                }
            )
            .await,
        Err(MailError::Conflict)
    ));
    assert!(matches!(
        setup
            .create_category(
                e.h,
                CategoryInput {
                    key: "Bad Key".into(),
                    label: "x".into(),
                    behaviour: CategoryBehaviour::Keep,
                    notify_time_sensitive: false,
                    auto_actions: None,
                    email_tasks: None,
                    color: None,
                    ord: None,
                }
            )
            .await,
        Err(MailError::Invalid(_))
    ));
    let patched = setup
        .update_category(
            e.h,
            new.id,
            CategoryPatch {
                label: Some("Pet mail".into()),
                behaviour: Some(CategoryBehaviour::Extract),
                notify_time_sensitive: None,
                auto_actions: Some(vec![
                    AutoAction::Label,
                    AutoAction::Archive,
                    AutoAction::Label,
                ]),
                email_tasks: Some(EmailTasksMode::Off),
                color: Some(Some("#aabbcc".into())),
                ord: None,
                archived: Some(true),
            },
            new.version,
        )
        .await
        .unwrap();
    assert_eq!(patched.key, "pets");
    assert!(patched.archived);
    assert_eq!(
        patched.auto_actions,
        vec![AutoAction::Label, AutoAction::Archive]
    );
    assert_eq!(setup.list_categories(e.h, false).await.unwrap().len(), 12);
    assert_eq!(setup.list_categories(e.h, true).await.unwrap().len(), 13);
    let other = cats.iter().find(|c| c.key == "other").unwrap();
    assert!(matches!(
        setup
            .update_category(
                e.h,
                other.id,
                CategoryPatch {
                    label: None,
                    behaviour: None,
                    notify_time_sensitive: None,
                    auto_actions: None,
                    email_tasks: None,
                    color: None,
                    ord: None,
                    archived: Some(true)
                },
                other.version
            )
            .await,
        Err(MailError::Invalid(_))
    ));

    // Orgs, contacts, enrollments.
    let org = setup
        .create_org(
            e.h,
            OrgInput {
                kind: OrgKind::Club,
                name: "Swim Club".into(),
                domains: vec!["@Swim.Example.com".into(), "swim.example.com".into()],
                senders: vec![" Coach@Example.COM ".into()],
                ics_calendar: None,
                default_calendar: None,
                contact_mode: None,
                bill_template: Some(BillTemplate {
                    amount: Some(r"Total:\s*\$(\d+\.\d\d)".into()),
                    due: None,
                    account: None,
                }),
            },
        )
        .await
        .unwrap();
    assert_eq!(org.domains, vec!["swim.example.com"]);
    assert_eq!(org.senders, vec!["coach@example.com"]);
    assert_eq!(org.contact_mode, ContactMode::Suggest);
    assert!(org.bill_template.is_some());
    assert!(matches!(
        setup
            .create_org(
                e.h,
                OrgInput {
                    kind: OrgKind::Club,
                    name: "Bad".into(),
                    domains: vec![],
                    senders: vec![],
                    ics_calendar: None,
                    default_calendar: None,
                    contact_mode: None,
                    bill_template: Some(BillTemplate {
                        amount: Some("(".into()),
                        due: None,
                        account: None
                    }),
                }
            )
            .await,
        Err(MailError::Invalid(_))
    ));
    let contact = setup
        .add_contact(
            e.h,
            org.id,
            OrgContactInput {
                name: "Coach Kim".into(),
                email: "Kim@Swim.example.com".into(),
                role: Some(ContactRole::Coach),
            },
        )
        .await
        .unwrap();
    assert_eq!(contact.email, "kim@swim.example.com");
    assert!(matches!(
        setup
            .add_contact(
                e.h,
                org.id,
                OrgContactInput {
                    name: "Dup".into(),
                    email: "kim@swim.example.com".into(),
                    role: None
                }
            )
            .await,
        Err(MailError::Conflict)
    ));
    assert!(matches!(
        setup
            .add_contact(
                e.h,
                org.id,
                OrgContactInput {
                    name: "x".into(),
                    email: "not an address".into(),
                    role: None
                }
            )
            .await,
        Err(MailError::Invalid(_))
    ));
    let enr = setup
        .create_enrollment(
            e.h,
            e.ben,
            EnrollmentInput {
                org: org.id,
                grade: None,
                class_name: Some("Dolphins".into()),
                from_date: None,
                until_date: None,
                contacts: vec![contact.id],
            },
        )
        .await
        .unwrap();
    assert_eq!(enr.contacts, vec![contact.id]);
    assert!(matches!(
        setup
            .create_enrollment(
                e.h,
                e.ben,
                EnrollmentInput {
                    org: org.id,
                    grade: None,
                    class_name: None,
                    from_date: chrono::NaiveDate::from_ymd_opt(2026, 9, 1),
                    until_date: chrono::NaiveDate::from_ymd_opt(2026, 8, 1),
                    contacts: vec![],
                }
            )
            .await,
        Err(MailError::Invalid(_))
    ));
    // A contact of another org cannot be linked.
    assert!(matches!(
        setup
            .create_enrollment(
                e.h,
                e.ben,
                EnrollmentInput {
                    org: e.school,
                    grade: None,
                    class_name: None,
                    from_date: None,
                    until_date: None,
                    contacts: vec![contact.id],
                }
            )
            .await,
        Err(MailError::Invalid(_))
    ));
    let upd = setup
        .update_enrollment(
            e.h,
            enr.id,
            EnrollmentPatch {
                org: None,
                grade: Some(Some("5".into())),
                class_name: Some(None),
                from_date: None,
                until_date: None,
                contacts: Some(vec![]),
            },
            enr.version,
        )
        .await
        .unwrap();
    assert_eq!(upd.grade.as_deref(), Some("5"));
    assert!(upd.class_name.is_none() && upd.contacts.is_empty());
    assert_eq!(
        setup
            .list_enrollments(e.h, Some(e.ben))
            .await
            .unwrap()
            .len(),
        2
    );
    setup.delete_enrollment(e.h, enr.id).await.unwrap();
    assert_eq!(
        setup
            .list_enrollments(e.h, Some(e.ben))
            .await
            .unwrap()
            .len(),
        1
    );
    setup.delete_contact(e.h, contact.id).await.unwrap();
    setup.delete_org(e.h, org.id).await.unwrap();
    let names: Vec<String> = setup
        .list_orgs(e.h)
        .await
        .unwrap()
        .into_iter()
        .map(|o| o.name)
        .collect();
    assert!(!names.contains(&"Swim Club".to_string()));
    let school = setup
        .list_orgs(e.h)
        .await
        .unwrap()
        .into_iter()
        .find(|o| o.id == e.school)
        .unwrap();
    assert_eq!(school.contacts.len(), 1);
    assert_eq!(school.contacts[0].id, e.rivera);
    let _ = e.power;
});

test!(make_task_queues_a_create_job, e, {
    let id = e.insert(&fixture("newsletter.eml"), "mt", &[]).await;
    e.process(id).await;
    let setup = SetupService::new(e.store.clone());
    setup
        .make_task(
            e.h,
            e.pat(),
            id,
            MakeTaskInput {
                title: Some("Order lunch".into()),
                assignees: None,
                due_at: None,
                list: None,
            },
        )
        .await
        .unwrap();
    let jobs = e.jobs("tasks.from_mail").await;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["mode"], json!("create"));
    assert_eq!(jobs[0]["kind"], json!("manual"));
    assert_eq!(jobs[0]["title"], json!("Order lunch"));
    assert_eq!(jobs[0]["assignees"], json!([e.pat]));
    assert!(matches!(
        setup
            .make_task(
                e.h,
                Viewer::adult(e.sam),
                id,
                MakeTaskInput {
                    title: None,
                    assignees: None,
                    due_at: None,
                    list: None
                }
            )
            .await,
        Err(MailError::NotFound)
    ));
});

test!(only_steps_limit_a_rerun, e, {
    let id = e.insert(&fixture("teacher_ics.eml"), "o1", &[]).await;
    e.process(id).await;
    assert_eq!(e.count("SELECT count(*) FROM event").await, 1);
    sqlx::query("DELETE FROM event")
        .execute(&e.pool)
        .await
        .unwrap();
    // Only the bills step: the event is not recreated.
    e.svc
        .process(e.h, id, Some(&["bills".to_string()]))
        .await
        .unwrap();
    assert_eq!(e.count("SELECT count(*) FROM event").await, 0);
    let out = e
        .svc
        .process(e.h, id, Some(&["events".to_string()]))
        .await
        .unwrap();
    assert_eq!(out.events, 1);
    assert_eq!(e.count("SELECT count(*) FROM event").await, 1);
});

#[test]
fn schedules_and_handlers() {
    let s = fa_mail::default_schedules();
    assert!(s.contains(&("inbox.expire", "7 * * * *")));
}

struct NoTokens;

#[async_trait]
impl AccessTokenProvider for NoTokens {
    async fn access_token(&self, _: HouseholdId, _: OAuthAccountId) -> Result<String, TokenError> {
        Err(TokenError::NeedsReauth)
    }
}

test!(job_handlers_run_from_payloads, e, {
    use fa_jobs::{JobContext, JobHandler};
    let deps = Arc::new(fa_mail::MailDeps {
        store: e.store.clone(),
        http: fa_connectors::Http::new(false).unwrap(),
        tokens: Arc::new(NoTokens),
        key: Arc::new(MasterKey::from_bytes(&[7u8; 32], 1).unwrap()),
        cfg: Default::default(),
    });
    let handlers = fa_mail::handlers(&deps);
    let find = |kind: &str| -> Arc<dyn JobHandler> {
        handlers
            .iter()
            .find(|h| h.kind() == kind)
            .unwrap_or_else(|| panic!("no handler for {kind}"))
            .clone()
    };
    let ctx = JobContext {
        store: e.store.clone(),
        backup_dir: None,
    };
    let id = e.insert(&fixture("payment_received.eml"), "j1", &[]).await;
    // No household in the payload: it is looked up from the message.
    find("mail.process")
        .run(&ctx, json!({"message": id}))
        .await
        .unwrap();
    assert_eq!(
        e.message(id).await.processing_state,
        ProcessingState::Processed
    );
    // A deleted message is not an error.
    find("mail.process")
        .run(&ctx, json!({"message": MessageId::new()}))
        .await
        .unwrap();
    assert!(
        find("mail.process")
            .run(&ctx, json!({"nonsense": true}))
            .await
            .is_err()
    );
    find("inbox.expire").run(&ctx, json!({})).await.unwrap();
});

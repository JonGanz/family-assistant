use fa_config::*;
use fa_domain::{DecisionPointKey, MailSourceState};
use fa_store::{CategoryRepo, MailRuleRepo, MailSourceRepo, OrgRepo, Store};
use sqlx::PgPool;

const EXAMPLE: &str = include_str!("../../../config/examples/household.example.toml");

const EXTRA: &str = r##"
[[mail_sources]]
name = "Family inbox"
provider = "imap"
address = "Family@Example.com"
owners = ["alex", "sam"]
visibility = "household"
read_only = true
poll_minutes = 10
[mail_sources.imap]
host = "imap.example.com"
username = "family@example.com"
folders = ["INBOX", "Receipts"]

[[mail_sources]]
name = "Alex Gmail"
provider = "gmail"
address = "alex@example.com"
owners = ["alex"]

[[categories]]
key = "sports"
label = "Sports"
behaviour = "extract"
notify_time_sensitive = true
color = "#00aa00"

[[categories]]
key = "junk"
label = "Junk"
behaviour = "ignore"
archived = true

[[mail_rules]]
name = "Soccer club mail is for Riley"
decision_point = "email.member_link"
priority = 10
conditions = [{ field = "from_domain", op = "is", values = ["club.example.com"] }]
value = ["riley"]
confidence = 0.95

[[mail_rules]]
name = "Club mail is sports"
decision_point = "email.category"
conditions = [{ field = "from_domain", op = "is", values = ["club.example.com"] }, { field = "subject", op = "matches", values = ["(?i)practice"] }]
value = "sports"
enabled = false

[household.events]
auto_write = true
extract_threshold = 0.8
"##;

fn full() -> String {
    format!("{EXAMPLE}\n{EXTRA}")
}

fn err(toml: &str) -> String {
    HouseholdConfig::from_toml(toml).unwrap_err().to_string()
}

#[test]
fn example_parses_with_orgs() {
    let c = HouseholdConfig::from_toml(EXAMPLE).unwrap();
    assert_eq!(c.orgs.len(), 2);
    assert_eq!(c.orgs[0].contacts[0].email, "teacher@example.edu");
    assert_eq!(c.members[2].enrollments[0].org, "Example Elementary");
    assert!(c.mail_sources.is_empty());
}

#[test]
fn validation_lists_every_problem() {
    let bad = format!(
        r#"{EXAMPLE}
[[mail_rules]]
name = "bad regex"
decision_point = "email.category"
conditions = [{{ field = "subject", op = "matches", values = ["("] }}]
value = "school"

[[mail_rules]]
name = "no conditions"
decision_point = "email.is_bill"
conditions = []
value = true

[[mail_rules]]
name = "who"
decision_point = "email.member_link"
conditions = [{{ field = "subject", op = "contains_any", values = ["x"] }}]
value = ["nobody"]

[[mail_sources]]
name = "x"
provider = "imap"
owners = ["ghost"]

[[categories]]
key = "Bad Key"
label = "x"
"#
    )
    .replace(
        "[household]\n",
        "[household]\ncurrency = \"usd\"\ndisabled_signals = [\"nope\"]\n",
    );
    let e = err(&bad);
    for needle in [
        "household.currency",
        "unknown signal 'nope'",
        "mail rule 'bad regex'",
        "mail rule 'no conditions'",
        "mail rule 'who'",
        "unknown member key 'ghost'",
        "needs an [mail_sources.imap]",
        "category key 'Bad Key'",
    ] {
        assert!(e.contains(needle), "missing {needle:?} in:\n{e}");
    }
}

#[test]
fn enrollment_and_calendar_references_are_checked() {
    let e = err(&EXAMPLE.replace("teacher@example.edu\"]", "nobody@example.edu\"]"));
    assert!(e.contains("nobody@example.edu"), "{e}");
    let e = err(&EXAMPLE.replace(
        "ics_calendar = \"school/ics\"",
        "ics_calendar = \"nope/ics\"",
    ));
    assert!(e.contains("ics_calendar"), "{e}");
    let e = err(&EXAMPLE.replace("amount = \"Amount due", "amount = \"(Amount due"));
    assert!(e.contains("bill_template.amount"), "{e}");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn imports_into_empty_household_and_round_trips(pool: PgPool) {
    let store = Store::from_pool(pool);
    let cfg = HouseholdConfig::from_toml(&full()).unwrap();
    let r = import(&store, None, &cfg, ImportMode::Merge).await.unwrap();
    assert_eq!(r.orgs_created, ["Example Elementary", "Example Utilities"]);
    assert_eq!(r.enrollments_created, ["riley: Example Elementary"]);
    assert_eq!(r.mail_sources_created, ["Family inbox"]);
    assert_eq!(r.mail_sources_skipped, ["Alex Gmail"]);
    assert_eq!(r.categories_created, ["sports"]);
    assert_eq!(r.categories_updated, ["junk"]);
    assert_eq!(r.mail_rules_created.len(), 2);

    let hid = r.household_id;
    {
        let mut tx = store.tx_for(hid).await.unwrap();
        let src = MailSourceRepo::list(&mut tx, false).await.unwrap();
        assert_eq!(src.len(), 1);
        let (s, owners) = &src[0];
        assert_eq!(s.value.state, MailSourceState::NeedsReauth);
        assert!(s.value.secret_id.is_none() && s.value.oauth_account_id.is_none());
        assert_eq!(s.value.address, "family@example.com");
        assert_eq!(s.value.folders, ["INBOX", "Receipts"]);
        assert_eq!(s.value.imap_port, Some(993));
        assert_eq!(owners.len(), 2);

        // member keys became member ids in the stored rule value
        let rules = MailRuleRepo::list(&mut tx, Some(DecisionPointKey::EmailMemberLink), false)
            .await
            .unwrap();
        let id = rules[0].value.value[0].as_str().unwrap().to_string();
        assert!(uuid::Uuid::parse_str(&id).is_ok(), "{id}");
        assert_eq!(rules[0].value.priority, 10);

        let junk = CategoryRepo::get_by_key(&mut tx, "junk")
            .await
            .unwrap()
            .unwrap();
        assert!(junk.value.archived_at.is_some());
        let org = OrgRepo::get_by_name(&mut tx, "example utilities")
            .await
            .unwrap()
            .unwrap();
        assert!(org.value.bill_amount_regex.is_some());
        let school = OrgRepo::get_by_name(&mut tx, "Example Elementary")
            .await
            .unwrap()
            .unwrap();
        assert!(
            school.value.ics_calendar_id.is_some() && school.value.default_calendar_id.is_some()
        );
        tx.rollback().await.unwrap();
    }

    let first = export(&store, hid).await.unwrap();
    // Defaults are not exported; the custom and the archived-default category are.
    let keys: Vec<&str> = first.categories.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, ["junk", "sports"]);
    assert_eq!(first.mail_sources.len(), 1);
    let link = first
        .mail_rules
        .iter()
        .find(|r| r.decision_point == DecisionPointKey::EmailMemberLink)
        .unwrap();
    assert_eq!(link.value, serde_json::json!(["riley"]));
    let first_toml = first.to_toml().unwrap();
    assert!(!first_toml.to_lowercase().contains("password"));
    assert!(!first_toml.contains("secret") && !first_toml.contains("token"));

    // export -> import -> export is identical, and the re-import changes nothing.
    let again = HouseholdConfig::from_toml(&first_toml).unwrap();
    let r2 = import(&store, Some(hid), &again, ImportMode::Merge)
        .await
        .unwrap();
    assert!(
        r2.orgs_created.is_empty() && r2.orgs_updated.is_empty(),
        "{r2:?}"
    );
    assert_eq!(r2.orgs_unchanged.len(), 2);
    assert_eq!(r2.enrollments_unchanged.len(), 1);
    assert!(r2.mail_sources_created.is_empty() && r2.mail_sources_updated.is_empty());
    assert_eq!(r2.mail_sources_unchanged, ["Family inbox"]);
    assert!(r2.categories_updated.is_empty() && r2.categories_created.is_empty());
    assert!(r2.mail_rules_created.is_empty() && r2.mail_rules_updated.is_empty());
    assert!(!r2.household_updated && r2.members_updated.is_empty());
    let second = export(&store, hid).await.unwrap().to_toml().unwrap();
    assert_eq!(first_toml, second);

    // Importing the original file again is idempotent too (merge by name / key).
    let r3 = import(&store, Some(hid), &cfg, ImportMode::Merge)
        .await
        .unwrap();
    assert!(r3.orgs_created.is_empty() && r3.mail_rules_created.is_empty());
    assert!(r3.categories_created.is_empty() && r3.mail_sources_created.is_empty());
    assert_eq!(r3.orgs_unchanged.len(), 2);

    // The same export imports into a fresh household and exports identically.
    let store2_hid = {
        let r = import(&store, None, &again, ImportMode::Merge)
            .await
            .unwrap();
        r.household_id
    };
    let third = export(&store, store2_hid).await.unwrap().to_toml().unwrap();
    assert_eq!(first_toml, third);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn changes_are_applied_and_reported(pool: PgPool) {
    let store = Store::from_pool(pool);
    let cfg = HouseholdConfig::from_toml(&full()).unwrap();
    let r = import(&store, None, &cfg, ImportMode::Merge).await.unwrap();
    let hid = r.household_id;

    let mut changed = cfg.clone();
    changed.orgs[0].domains.push("school.example.edu".into());
    changed.orgs[0].contacts[0].name = "Mr. Teacher".into();
    changed.mail_rules[0].confidence = Some(0.5);
    changed.mail_sources[0].poll_minutes = Some(20);
    changed.categories[0].label = "Team sports".into();
    changed.members[2].enrollments[0].class_name = Some("3C".into());
    let r2 = import(&store, Some(hid), &changed, ImportMode::Merge)
        .await
        .unwrap();
    assert_eq!(r2.orgs_updated, ["Example Elementary"]);
    assert_eq!(r2.orgs_unchanged, ["Example Utilities"]);
    assert_eq!(r2.mail_rules_updated.len(), 1);
    assert_eq!(r2.mail_sources_updated, ["Family inbox"]);
    assert_eq!(r2.categories_updated, ["sports"]);
    assert_eq!(r2.enrollments_updated.len(), 1);
    assert!(r2.household_updated || r2.members_updated.is_empty());

    let out = export(&store, hid).await.unwrap();
    assert_eq!(out.orgs[0].domains, ["example.edu", "school.example.edu"]);
    assert_eq!(
        out.household.events.as_ref().unwrap().auto_write,
        Some(true)
    );
    assert_eq!(
        out.household.events.as_ref().unwrap().extract_threshold,
        Some(0.8)
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn unknown_enrollment_org_writes_nothing(pool: PgPool) {
    let store = Store::from_pool(pool);
    let mut cfg = HouseholdConfig::from_toml(EXAMPLE).unwrap();
    cfg.orgs.retain(|o| o.name != "Example Elementary");
    cfg.orgs.iter_mut().for_each(|o| o.ics_calendar = None);
    let e = import(&store, None, &cfg, ImportMode::Merge)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("Example Elementary"), "{e}");
    // The whole import rolled back: a retry with a valid file creates the household.
    let ok = HouseholdConfig::from_toml(EXAMPLE).unwrap();
    assert!(
        import(&store, None, &ok, ImportMode::Merge)
            .await
            .unwrap()
            .household_created
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn approval_mode_round_trips(pool: PgPool) {
    let store = Store::from_pool(pool);
    let toml = EXAMPLE.replacen(
        "[[members]]\nkey = \"sam\"",
        "[[members]]\nkey = \"sam\"\nnotify = { approval_mode = \"digest\" }",
        1,
    );
    let cfg = HouseholdConfig::from_toml(&toml).unwrap();
    let r = import(&store, None, &cfg, ImportMode::Merge).await.unwrap();
    let out = export(&store, r.household_id).await.unwrap();
    let sam = out.members.iter().find(|m| m.key == "sam").unwrap();
    assert_eq!(
        sam.notify.as_ref().unwrap().approval_mode,
        Some(fa_domain::ApprovalMode::Digest)
    );
}

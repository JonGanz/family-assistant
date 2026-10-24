use fa_config::*;
use fa_store::{AuditRepo, Store};
use sqlx::PgPool;

const EXAMPLE: &str = include_str!("../../../config/examples/household.example.toml");

#[test]
fn rejects_unknown_sections() {
    let e = HouseholdConfig::from_toml(&format!("{EXAMPLE}\n[calendar]\nx = 1\n"))
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("calendar") && e.contains("household, members, calendar_sources"),
        "{e}"
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn import_export_round_trip(pool: PgPool) {
    let store = Store::from_pool(pool);
    let cfg = HouseholdConfig::from_toml(EXAMPLE).unwrap();
    let r = import(&store, None, &cfg, ImportMode::Merge).await.unwrap();
    assert!(r.household_created);
    assert_eq!(r.members_created.len(), 4);

    assert_eq!(r.sources_created, ["school"]);
    assert_eq!(r.rules_created, ["Cold morning coats"]);

    let out = export(&store, r.household_id).await.unwrap();
    assert_eq!(out.members, cfg.members);
    assert_eq!(out.calendar_sources.len(), 1);
    assert_eq!(out.weather_rules, cfg.weather_rules);
    let first = out.to_toml().unwrap();
    insta::assert_snapshot!(first);

    // export -> import -> export is byte-stable, and a second import changes nothing.
    let again = HouseholdConfig::from_toml(&first).unwrap();
    let r_again = import(&store, Some(r.household_id), &again, ImportMode::Merge)
        .await
        .unwrap();
    assert!(r_again.sources_unchanged == ["school-calendar"] && r_again.rules_unchanged.len() == 1);
    assert!(!r_again.household_updated && r_again.members_updated.is_empty());
    let second = export(&store, r.household_id)
        .await
        .unwrap()
        .to_toml()
        .unwrap();
    assert_eq!(first, second);

    // Idempotent re-import, then replace archives dropped members.
    let r2 = import(&store, Some(r.household_id), &cfg, ImportMode::Merge)
        .await
        .unwrap();
    assert!(!r2.household_created && r2.members_unchanged.len() == 4);

    let mut smaller = cfg.clone();
    smaller.members.truncate(2);
    smaller.calendar_sources.clear();
    let r3 = import(&store, Some(r.household_id), &smaller, ImportMode::Replace)
        .await
        .unwrap();
    assert_eq!(r3.members_archived, ["riley", "jordan"]);
    assert_eq!(
        export(&store, r.household_id).await.unwrap().members,
        smaller.members
    );

    let mut tx = store.tx_for(r.household_id).await.unwrap();
    let audit = AuditRepo::list(&mut tx, r.household_id, 10).await.unwrap();
    assert_eq!(audit.len(), 4);
    assert!(
        audit
            .iter()
            .all(|a| a.action == "config.import" && a.actor_member.is_none())
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn secrets_never_exported_and_import_is_disabled(pool: PgPool) {
    use fa_domain::{CalendarSourceId, SourceKind, SourceState};
    use fa_store::{CalendarRepo, CalendarSourceRepo, ConfigExtRepo};

    let store = Store::from_pool(pool);
    let toml = format!(
        r#"{EXAMPLE}
[[calendar_sources]]
key = "dav"
kind = "caldav"
name = "Family DAV"
url = "https://dav.example.com/"
username = "alex"
enabled = true

[[calendar_sources.calendars]]
remote_id = "cal1"
enabled = true
members = ["alex", "sam"]
"#
    );
    let cfg = HouseholdConfig::from_toml(&toml).unwrap();
    let r = import(&store, None, &cfg, ImportMode::Merge).await.unwrap();
    let hid = r.household_id;

    let mut tx = store.tx_for(hid).await.unwrap();
    let list = CalendarSourceRepo::list(&mut tx, false).await.unwrap();
    assert_eq!(list.len(), 2);
    for v in &list {
        assert_eq!(v.value.state, SourceState::NeedsReauth);
        assert!(
            !ConfigExtRepo::source_enabled(&mut tx, v.value.id)
                .await
                .unwrap()
        );
        for c in CalendarRepo::list(&mut tx, Some(v.value.id)).await.unwrap() {
            assert!(!c.value.enabled);
        }
    }
    // Give the CalDAV source credentials, enable it and its calendar.
    let dav = list
        .iter()
        .find(|v| v.value.kind == SourceKind::Caldav)
        .unwrap();
    let sid: CalendarSourceId = dav.value.id;
    let secret = fa_domain::SecretId::new();
    sqlx::query("INSERT INTO secret (id, household_id, owner_type, owner_id, kind, ciphertext, nonce, wrapped_dek, key_version) VALUES ($1,$2,'calendar_source',$3,'other','x','x','x',1)")
        .bind(secret.as_uuid()).bind(hid.as_uuid()).bind(sid.as_uuid())
        .execute(&mut *tx).await.unwrap();
    CalendarSourceRepo::set_secret(&mut tx, sid, secret)
        .await
        .unwrap();
    CalendarSourceRepo::set_enabled(&mut tx, sid, true)
        .await
        .unwrap();
    CalendarSourceRepo::set_sync_result(&mut tx, sid, SourceState::Ok, None, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // Re-import keeps credentials, state and enabled flags.
    let r2 = import(&store, Some(hid), &cfg, ImportMode::Merge)
        .await
        .unwrap();
    assert_eq!(r2.sources_unchanged.len() + r2.sources_updated.len(), 2);
    let mut tx = store.tx_for(hid).await.unwrap();
    let v = CalendarSourceRepo::get(&mut tx, sid)
        .await
        .unwrap()
        .unwrap();
    assert!(v.value.secret_id.is_some() && v.value.state == SourceState::Ok);
    assert!(ConfigExtRepo::source_enabled(&mut tx, sid).await.unwrap());
    let cals = CalendarRepo::list(&mut tx, Some(sid)).await.unwrap();
    assert!(cals[0].value.enabled);
    assert_eq!(
        CalendarRepo::members(&mut tx, cals[0].value.id)
            .await
            .unwrap()
            .len(),
        2
    );
    tx.rollback().await.unwrap();

    let out = export(&store, hid).await.unwrap().to_toml().unwrap();
    assert!(
        !out.contains("school.ics") && !out.contains("password =") && !out.contains("secret_id")
    );
    assert!(out.contains("# ICS URL is not exported") && out.contains("# CalDAV password"));
    assert!(out.contains("https://dav.example.com/"));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn unarchives_members_found_in_file(pool: PgPool) {
    use fa_store::MemberRepo;
    let store = Store::from_pool(pool);
    let cfg = HouseholdConfig::from_toml(EXAMPLE).unwrap();
    let r = import(&store, None, &cfg, ImportMode::Merge).await.unwrap();
    let mut smaller = cfg.clone();
    smaller.members.truncate(2);
    smaller.calendar_sources.clear();
    import(&store, Some(r.household_id), &smaller, ImportMode::Replace)
        .await
        .unwrap();
    let r3 = import(&store, Some(r.household_id), &cfg, ImportMode::Merge)
        .await
        .unwrap();
    assert_eq!(r3.members_updated, ["riley", "jordan"]);
    let mut tx = store.tx_for(r.household_id).await.unwrap();
    assert_eq!(MemberRepo::list(&mut tx, false).await.unwrap().len(), 4);
}

#[test]
fn unknown_member_key_names_the_line() {
    let bad = EXAMPLE.replace("members = [\"riley\"]", "members = [\"nobody\"]");
    let line = bad.lines().position(|l| l.contains("nobody")).unwrap() + 1;
    let e = HouseholdConfig::from_toml(&bad).unwrap_err().to_string();
    assert!(e.contains("unknown member key 'nobody'"), "{e}");
    assert!(
        e.contains(&format!("line {line}")),
        "{e} (expected line {line})"
    );

    let bad_owner = EXAMPLE.replace("owner = \"alex\"", "owner = \"ghost\"");
    let line = bad_owner.lines().position(|l| l.contains("ghost")).unwrap() + 1;
    let e = HouseholdConfig::from_toml(&bad_owner)
        .unwrap_err()
        .to_string();
    assert!(e.contains(&format!("line {line}")), "{e}");
}

#[test]
fn household_defaults_and_ranges() {
    let cfg = HouseholdConfig::from_toml("[household]\nname='x'\ntz='UTC'\n").unwrap();
    assert!(cfg.household.units.is_none());
    let e = HouseholdConfig::from_toml(
        "[household]\nname='x'\ntz='UTC'\ncalendar_future_days=0\nunits='metric'\n",
    )
    .unwrap_err()
    .to_string();
    assert!(e.contains("calendar_future_days"), "{e}");
}

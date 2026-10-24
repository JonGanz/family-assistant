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
        e.contains("calendar") && e.contains("household, members"),
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

    let out = export(&store, r.household_id).await.unwrap();
    assert_eq!(out, cfg);
    insta::assert_snapshot!(out.to_toml().unwrap());

    // Idempotent re-import, then replace archives dropped members.
    let r2 = import(&store, Some(r.household_id), &cfg, ImportMode::Merge)
        .await
        .unwrap();
    assert!(!r2.household_created && r2.members_unchanged.len() == 4);

    let mut smaller = cfg.clone();
    smaller.members.truncate(2);
    let r3 = import(&store, Some(r.household_id), &smaller, ImportMode::Replace)
        .await
        .unwrap();
    assert_eq!(r3.members_archived, ["riley", "jordan"]);
    assert_eq!(export(&store, r.household_id).await.unwrap(), smaller);

    let mut tx = store.tx_for(r.household_id).await.unwrap();
    let audit = AuditRepo::list(&mut tx, r.household_id, 10).await.unwrap();
    assert_eq!(audit.len(), 3);
    assert!(
        audit
            .iter()
            .all(|a| a.action == "config.import" && a.actor_member.is_none())
    );
}

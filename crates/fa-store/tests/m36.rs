//! M3.6: member hue column and migration.

use chrono::Utc;
use fa_domain::*;
use fa_store::*;
use sqlx::PgPool;

fn member(h: HouseholdId, color: Option<Hue>) -> Member {
    Member {
        id: MemberId::new(),
        household_id: h,
        display_name: "Ada Example".into(),
        role: Role::Adult,
        avatar: None,
        color,
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

#[sqlx::test(migrations = "./migrations")]
async fn member_hue_round_trip_and_constraint(pool: PgPool) {
    let hid = HouseholdId::new();
    sqlx::query("INSERT INTO household (id, name, tz) VALUES ($1, 'Example', 'UTC')")
        .bind(hid.as_uuid())
        .execute(&pool)
        .await
        .unwrap();
    let store = Store::from_pool(pool.clone());
    let mut tx = store.tx_system().await.unwrap();
    let m = member(hid, Some(Hue::Purple));
    let got = MemberRepo::insert(&mut tx, &m, None, None).await.unwrap();
    assert_eq!(got.value.color, Some(Hue::Purple));
    let none = MemberRepo::insert(&mut tx, &member(hid, None), None, None)
        .await
        .unwrap();
    assert_eq!(none.value.color, None);
    tx.commit().await.unwrap();

    let bad = sqlx::query("UPDATE member SET color = '#ff0000' WHERE id = $1")
        .bind(m.id.as_uuid())
        .execute(&pool)
        .await;
    assert!(bad.is_err());
}

#[sqlx::test(migrations = "./migrations")]
async fn migration_turns_hex_into_blue(pool: PgPool) {
    let hid = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO household (id, name, tz) VALUES ($1, 'Example', 'UTC')")
        .bind(hid)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/20261029000001_m36_member_hue.down.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    for (name, color) in [("A", Some("#336699")), ("B", None)] {
        sqlx::query(
            "INSERT INTO member (id, household_id, display_name, role, color)
             VALUES ($1, $2, $3, 'adult', $4)",
        )
        .bind(uuid::Uuid::now_v7())
        .bind(hid)
        .bind(name)
        .bind(color)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::raw_sql(include_str!(
        "../migrations/20261029000001_m36_member_hue.up.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let a: Option<String> = sqlx::query_scalar("SELECT color FROM member WHERE display_name = 'A'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let b: Option<String> = sqlx::query_scalar("SELECT color FROM member WHERE display_name = 'B'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(a.as_deref(), Some("blue"));
    assert_eq!(b, None);
}

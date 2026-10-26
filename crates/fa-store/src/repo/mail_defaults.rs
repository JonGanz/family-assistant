//! Per-household mail seed: the D-054 categories and the five decision points with their
//! 'Rules only' pipeline (plan/M3.md, Seeding).

use fa_domain::{DecisionPointKey, HouseholdId, default_categories};
use sqlx::PgConnection;

use crate::Result;

/// Idempotent: existing categories, decision points and pipeline versions are left alone.
pub async fn ensure(conn: &mut PgConnection, household: HouseholdId) -> Result<()> {
    let cats = default_categories();
    let keys: Vec<&str> = cats.iter().map(|c| c.key).collect();
    let labels: Vec<&str> = cats.iter().map(|c| c.label).collect();
    let behaviours: Vec<&str> = cats.iter().map(|c| c.behaviour.as_str()).collect();
    let notify: Vec<bool> = cats.iter().map(|c| c.notify_time_sensitive).collect();
    let ords: Vec<i32> = (0..cats.len() as i32).collect();
    sqlx::query(
        "INSERT INTO category (id, household_id, key, label, behaviour, notify_time_sensitive, ord)
         SELECT gen_random_uuid(), $1, t.k, t.l, t.b, t.n, t.o
         FROM unnest($2::text[], $3::text[], $4::text[], $5::bool[], $6::int[]) AS t(k, l, b, n, o)
         ON CONFLICT (household_id, key) DO NOTHING",
    )
    .bind(household.as_uuid())
    .bind(&keys)
    .bind(&labels)
    .bind(&behaviours)
    .bind(&notify)
    .bind(&ords)
    .execute(&mut *conn)
    .await?;

    let dp_keys: Vec<&str> = DecisionPointKey::ALL.iter().map(|k| k.as_str()).collect();
    let dp_defaults: Vec<String> = DecisionPointKey::ALL
        .iter()
        .map(|k| k.default_value().to_string())
        .collect();
    sqlx::query(
        "INSERT INTO decision_point (id, household_id, key, default_value)
         SELECT gen_random_uuid(), $1, t.k, t.v::jsonb
         FROM unnest($2::text[], $3::text[]) AS t(k, v)
         ON CONFLICT (household_id, key) DO NOTHING",
    )
    .bind(household.as_uuid())
    .bind(&dp_keys)
    .bind(&dp_defaults)
    .execute(&mut *conn)
    .await?;

    // Stages only for the versions created by this call.
    sqlx::query(
        "WITH v AS (
             INSERT INTO pipeline_version (id, household_id, decision_point_id, version, active, name)
             SELECT gen_random_uuid(), dp.household_id, dp.id, 1, true, 'Rules only'
             FROM decision_point dp
             WHERE dp.household_id = $1
               AND NOT EXISTS (SELECT 1 FROM pipeline_version pv WHERE pv.decision_point_id = dp.id)
             RETURNING id, household_id
         )
         INSERT INTO pipeline_stage (id, household_id, pipeline_version_id, ord, kind, accept_threshold)
         SELECT gen_random_uuid(), v.household_id, v.id, s.ord, s.kind, s.thr
         FROM v CROSS JOIN (VALUES (0, 'rules', 0.8::real), (1, 'human', 0.0::real)) AS s(ord, kind, thr)",
    )
    .bind(household.as_uuid())
    .execute(&mut *conn)
    .await?;
    Ok(())
}

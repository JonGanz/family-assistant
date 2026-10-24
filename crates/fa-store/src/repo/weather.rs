use chrono::{DateTime, Utc};
use fa_domain::WeatherSnapshot;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::Result;

pub struct WeatherSnapshotRepo;

impl WeatherSnapshotRepo {
    /// Inserts for the transaction's household (`tx_for`).
    pub async fn insert(
        conn: &mut PgConnection,
        provider: &str,
        current: &serde_json::Value,
        hourly: &serde_json::Value,
        daily: &serde_json::Value,
    ) -> Result<Uuid> {
        Ok(sqlx::query_scalar!(
            r#"INSERT INTO weather_snapshot (id, household_id, provider, "current", hourly, daily)
               VALUES ($1, app_household(), $2, $3, $4, $5) RETURNING id"#,
            Uuid::now_v7(),
            provider,
            current,
            hourly,
            daily
        )
        .fetch_one(conn)
        .await?)
    }

    pub async fn latest(conn: &mut PgConnection) -> Result<Option<WeatherSnapshot>> {
        let r = sqlx::query!(
            r#"SELECT id, household_id, fetched_at, provider, "current", hourly, daily
               FROM weather_snapshot ORDER BY fetched_at DESC, id DESC LIMIT 1"#
        )
        .fetch_optional(conn)
        .await?;
        Ok(r.map(|r| WeatherSnapshot {
            id: r.id,
            household_id: r.household_id.into(),
            fetched_at: r.fetched_at,
            provider: r.provider,
            current: r.current,
            hourly: r.hourly,
            daily: r.daily,
        }))
    }

    pub async fn delete_older_than(conn: &mut PgConnection, before: DateTime<Utc>) -> Result<u64> {
        Ok(
            sqlx::query!("DELETE FROM weather_snapshot WHERE fetched_at < $1", before)
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }
}

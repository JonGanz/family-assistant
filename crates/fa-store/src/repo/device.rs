use chrono::{DateTime, Utc};
use fa_domain::{Device, DeviceElevation, DeviceId, MemberId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{device_kind_str, parse_device_kind};
use crate::{Result, StoreError, Versioned};

pub struct DeviceRepo;
pub struct DeviceElevationRepo;

struct Row {
    id: Uuid,
    household_id: Uuid,
    kind: String,
    name: String,
    token_hash: String,
    last_seen: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    version: i32,
}

impl TryFrom<Row> for Versioned<Device> {
    type Error = StoreError;
    fn try_from(r: Row) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: Device {
                id: r.id.into(),
                household_id: r.household_id.into(),
                kind: parse_device_kind(&r.kind)?,
                name: r.name,
                token_hash: r.token_hash,
                last_seen: r.last_seen,
                revoked_at: r.revoked_at,
                created_at: r.created_at,
            },
        })
    }
}

impl DeviceRepo {
    pub async fn insert(
        conn: &mut PgConnection,
        d: &Device,
        created_by: Option<MemberId>,
    ) -> Result<Versioned<Device>> {
        let row = sqlx::query_as!(
            Row,
            "INSERT INTO device (id, household_id, kind, name, token_hash, created_by)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, household_id, kind, name, token_hash, last_seen, revoked_at, created_at, version",
            d.id.as_uuid(),
            d.household_id.as_uuid(),
            device_kind_str(d.kind),
            d.name,
            d.token_hash,
            created_by.map(|c| c.as_uuid())
        )
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(conn: &mut PgConnection, id: DeviceId) -> Result<Option<Versioned<Device>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, kind, name, token_hash, last_seen, revoked_at, created_at, version
             FROM device WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Bearer-token lookup (system tx: the household is not known yet). Revoked devices are not returned.
    pub async fn get_by_token_hash(
        conn: &mut PgConnection,
        token_hash: &str,
    ) -> Result<Option<Versioned<Device>>> {
        let row = sqlx::query_as!(
            Row,
            "SELECT id, household_id, kind, name, token_hash, last_seen, revoked_at, created_at, version
             FROM device WHERE token_hash = $1 AND revoked_at IS NULL",
            token_hash
        )
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn list(
        conn: &mut PgConnection,
        include_revoked: bool,
    ) -> Result<Vec<Versioned<Device>>> {
        let rows = sqlx::query_as!(
            Row,
            "SELECT id, household_id, kind, name, token_hash, last_seen, revoked_at, created_at, version
             FROM device WHERE $1 OR revoked_at IS NULL ORDER BY created_at, id",
            include_revoked
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn rename(
        conn: &mut PgConnection,
        id: DeviceId,
        name: &str,
        expected_version: i32,
    ) -> Result<i32> {
        let v = sqlx::query_scalar!(
            "UPDATE device SET name = $2, version = version + 1 WHERE id = $1 AND version = $3 RETURNING version",
            id.as_uuid(),
            name,
            expected_version
        )
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(super::conflict_or_missing(conn, "device", id.as_uuid()).await),
        }
    }

    pub async fn touch(conn: &mut PgConnection, id: DeviceId) -> Result<()> {
        sqlx::query!(
            "UPDATE device SET last_seen = now() WHERE id = $1",
            id.as_uuid()
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Revocation is the device's archive. Idempotent.
    pub async fn archive(conn: &mut PgConnection, id: DeviceId) -> Result<()> {
        let n = sqlx::query!(
            "UPDATE device SET revoked_at = COALESCE(revoked_at, now()), version = version + 1 WHERE id = $1",
            id.as_uuid()
        )
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }
}

impl DeviceElevationRepo {
    /// Replaces any existing elevation of the device.
    pub async fn create(
        conn: &mut PgConnection,
        e: &DeviceElevation,
        household_id: fa_domain::HouseholdId,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM device_elevation WHERE device_id = $1",
            e.device_id.as_uuid()
        )
        .execute(&mut *conn)
        .await?;
        sqlx::query!(
            "INSERT INTO device_elevation (household_id, device_id, member_id, expires_at) VALUES ($1, $2, $3, $4)",
            household_id.as_uuid(),
            e.device_id.as_uuid(),
            e.member_id.as_uuid(),
            e.expires_at
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// The unexpired elevation of the device, if any.
    pub async fn active(
        conn: &mut PgConnection,
        device: DeviceId,
    ) -> Result<Option<DeviceElevation>> {
        let r = sqlx::query!(
            "SELECT device_id, member_id, expires_at FROM device_elevation
             WHERE device_id = $1 AND expires_at > now() ORDER BY expires_at DESC LIMIT 1",
            device.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        Ok(r.map(|r| DeviceElevation {
            device_id: r.device_id.into(),
            member_id: r.member_id.into(),
            expires_at: r.expires_at,
        }))
    }

    pub async fn clear(conn: &mut PgConnection, device: DeviceId) -> Result<()> {
        sqlx::query!(
            "DELETE FROM device_elevation WHERE device_id = $1",
            device.as_uuid()
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn delete_expired(conn: &mut PgConnection) -> Result<u64> {
        Ok(
            sqlx::query!("DELETE FROM device_elevation WHERE expires_at <= now()")
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }
}

//! First-run state (`GET /setup`, wizard completion). Use through `Store::tx_system`.

use chrono::{DateTime, Utc};
use fa_domain::HouseholdId;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{Result, StoreError};

pub struct SetupRepo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupState {
    pub household: Option<HouseholdId>,
    pub has_adult_account: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

impl SetupRepo {
    pub async fn state(conn: &mut PgConnection) -> Result<SetupState> {
        let hh: Option<(Uuid, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT id, setup_completed_at FROM household ORDER BY created_at, id LIMIT 1",
        )
        .fetch_optional(&mut *conn)
        .await?;
        let has_adult_account: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM user_account a JOIN member m ON m.id = a.member_id
                             WHERE m.role = 'adult' AND m.archived_at IS NULL)",
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(SetupState {
            household: hh.as_ref().map(|(id, _)| (*id).into()),
            has_adult_account,
            completed_at: hh.and_then(|(_, at)| at),
        })
    }

    /// Sets `setup_completed_at` once; later calls keep the first time.
    pub async fn complete(conn: &mut PgConnection, id: HouseholdId) -> Result<()> {
        let n = sqlx::query(
            "UPDATE household SET setup_completed_at = COALESCE(setup_completed_at, now()) WHERE id = $1",
        )
        .bind(id.as_uuid())
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

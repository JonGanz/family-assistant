use std::ops::{Deref, DerefMut};

use fa_domain::HouseholdId;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

use crate::Result;

/// Arbitrary application-wide advisory lock key guarding migrations.
const MIGRATE_LOCK: i64 = 0x6661_5f6d_6967; // "fa_mig"

/// A transaction with RLS context already applied. Derefs to `PgConnection`.
pub struct Tx(Transaction<'static, Postgres>);

impl Tx {
    pub async fn commit(self) -> Result<()> {
        self.0.commit().await?;
        Ok(())
    }

    pub async fn rollback(self) -> Result<()> {
        self.0.rollback().await?;
        Ok(())
    }
}

impl Deref for Tx {
    type Target = PgConnection;
    fn deref(&self) -> &PgConnection {
        &self.0
    }
}

impl DerefMut for Tx {
    fn deref_mut(&mut self) -> &mut PgConnection {
        &mut self.0
    }
}

#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    pub async fn connect(database_url: &str, max_connections: u32) -> Result<Self> {
        let opts: PgConnectOptions = database_url.parse()?;
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect_with(opts)
            .await?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run pending migrations while holding a Postgres advisory lock (safe with several replicas).
    pub async fn migrate(&self) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(MIGRATE_LOCK)
            .execute(&mut *conn)
            .await?;
        let res = sqlx::migrate!("./migrations").run(&mut *conn).await;
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(MIGRATE_LOCK)
            .execute(&mut *conn)
            .await?;
        res?;
        Ok(())
    }

    /// True when every embedded migration has been applied (for `/readyz`).
    pub async fn migrations_applied(&self) -> Result<bool> {
        let applied: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE success")
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default();
        let m = sqlx::migrate!("./migrations");
        Ok(m.iter()
            .filter(|x| !x.migration_type.is_down_migration())
            .all(|x| applied.contains(&x.version)))
    }

    /// Transaction scoped to one household: `SET LOCAL ROLE fa_app` + `app.household_id`.
    pub async fn tx_for(&self, household_id: HouseholdId) -> Result<Tx> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE fa_app")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT set_config('app.household_id', $1, true)")
            .bind(household_id.to_string())
            .execute(&mut *tx)
            .await?;
        Ok(Tx(tx))
    }

    /// Cross-household transaction (login lookup, device token, sessions, workers). RLS still
    /// applies as `fa_app`, with the `app.system` flag opening the tenant policies.
    pub async fn tx_system(&self) -> Result<Tx> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE fa_app")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT set_config('app.system', 'on', true)")
            .execute(&mut *tx)
            .await?;
        Ok(Tx(tx))
    }
}

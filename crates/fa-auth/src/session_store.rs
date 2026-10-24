use std::fmt;

use chrono::{DateTime, Utc};
use fa_domain::{HouseholdId, MemberId, UserAccountId};
use fa_store::{MemberRepo, SessionRecord, SessionRepo, Store};
use tower_sessions::cookie::time::OffsetDateTime;
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::{Error, Result, SessionStore};
use uuid::Uuid;

use crate::config::AuthConfig;

const K_HOUSEHOLD: &str = "household_id";
const K_MEMBER: &str = "member_id";
const K_ACCOUNT: &str = "account_id";

/// `tower-sessions` store over fa-store's `session` table.
///
/// A session is persisted only when its data carries `household_id`, `member_id` and
/// `account_id` (set by [`crate::AuthService::login`]); anonymous sessions are not stored.
/// The expiry is role-based and rolling (it replaces the layer's `expiry_date` on every save).
#[derive(Clone)]
pub struct PgSessionStore {
    store: Store,
    cfg: AuthConfig,
}

impl fmt::Debug for PgSessionStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PgSessionStore").finish_non_exhaustive()
    }
}

impl PgSessionStore {
    pub fn new(store: Store, cfg: AuthConfig) -> Self {
        Self { store, cfg }
    }
}

fn backend(e: impl fmt::Display) -> Error {
    Error::Backend(e.to_string())
}

fn uuid_of(r: &Record, key: &str) -> Option<Uuid> {
    r.data.get(key)?.as_str()?.parse().ok()
}

pub(crate) fn identity_data(
    h: HouseholdId,
    m: MemberId,
    a: UserAccountId,
) -> serde_json::Map<String, serde_json::Value> {
    let mut d = serde_json::Map::new();
    d.insert(K_HOUSEHOLD.into(), h.to_string().into());
    d.insert(K_MEMBER.into(), m.to_string().into());
    d.insert(K_ACCOUNT.into(), a.to_string().into());
    d
}

fn to_offset(t: DateTime<Utc>) -> Result<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp(t.timestamp()).map_err(backend)
}

#[async_trait::async_trait]
impl SessionStore for PgSessionStore {
    async fn create(&self, record: &mut Record) -> Result<()> {
        loop {
            let mut tx = self.store.tx_system().await.map_err(backend)?;
            let taken = SessionRepo::get(&mut tx, &record.id.to_string())
                .await
                .map_err(backend)?
                .is_some();
            tx.rollback().await.map_err(backend)?;
            if !taken {
                break;
            }
            record.id = Id::default();
        }
        self.save(record).await
    }

    async fn save(&self, record: &Record) -> Result<()> {
        let (Some(h), Some(m), Some(a)) = (
            uuid_of(record, K_HOUSEHOLD),
            uuid_of(record, K_MEMBER),
            uuid_of(record, K_ACCOUNT),
        ) else {
            return Ok(());
        };
        let household = HouseholdId::from(h);
        let mut tx = self.store.tx_for(household).await.map_err(backend)?;
        let Some(member) = MemberRepo::get(&mut tx, m.into()).await.map_err(backend)? else {
            return Err(backend("session member not found"));
        };
        let now = Utc::now();
        let existing = SessionRepo::get(&mut tx, &record.id.to_string())
            .await
            .map_err(backend)?;
        let ttl =
            chrono::Duration::from_std(self.cfg.session_ttl(member.value.role)).map_err(backend)?;
        let data = serde_json::to_vec(&record.data).map_err(|e| Error::Encode(e.to_string()))?;
        SessionRepo::put(
            &mut tx,
            &SessionRecord {
                id: record.id.to_string(),
                household_id: household,
                member_id: m.into(),
                account_id: a.into(),
                data,
                user_agent: existing.as_ref().and_then(|e| e.user_agent.clone()),
                created_at: existing.as_ref().map_or(now, |e| e.created_at),
                last_seen: now,
                expires_at: now + ttl,
            },
        )
        .await
        .map_err(backend)?;
        tx.commit().await.map_err(backend)
    }

    async fn load(&self, id: &Id) -> Result<Option<Record>> {
        let mut tx = self.store.tx_system().await.map_err(backend)?;
        let rec = SessionRepo::get(&mut tx, &id.to_string())
            .await
            .map_err(backend)?;
        tx.rollback().await.map_err(backend)?;
        let Some(rec) = rec else { return Ok(None) };
        let data = serde_json::from_slice(&rec.data).map_err(|e| Error::Decode(e.to_string()))?;
        Ok(Some(Record {
            id: *id,
            data,
            expiry_date: to_offset(rec.expires_at)?,
        }))
    }

    async fn delete(&self, id: &Id) -> Result<()> {
        let mut tx = self.store.tx_system().await.map_err(backend)?;
        SessionRepo::delete(&mut tx, &id.to_string())
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)
    }
}

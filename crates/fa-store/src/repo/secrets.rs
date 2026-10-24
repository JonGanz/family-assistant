use fa_domain::{HouseholdId, MemberId, SecretId, SecretKind};
use sqlx::PgConnection;
use uuid::Uuid;

use super::secret_kind_str;
use crate::secret::{MasterKey, SealedSecret};
use crate::{Result, StoreError};

pub struct SecretRepo;

impl SecretRepo {
    /// Seals `plaintext` and upserts it for (household, owner_type, owner_id, kind).
    #[allow(clippy::too_many_arguments)]
    pub async fn put(
        conn: &mut PgConnection,
        key: &MasterKey,
        household_id: HouseholdId,
        owner_type: &str,
        owner_id: Uuid,
        kind: SecretKind,
        plaintext: &[u8],
        created_by: Option<MemberId>,
    ) -> Result<SecretId> {
        let sealed = key.seal(plaintext)?;
        let id = sqlx::query_scalar!(
            "INSERT INTO secret (id, household_id, owner_type, owner_id, kind, ciphertext, nonce, wrapped_dek,
                                 key_version, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (household_id, owner_type, owner_id, kind) DO UPDATE SET
                 ciphertext = EXCLUDED.ciphertext, nonce = EXCLUDED.nonce,
                 wrapped_dek = EXCLUDED.wrapped_dek, key_version = EXCLUDED.key_version,
                 version = secret.version + 1
             RETURNING id",
            SecretId::new().as_uuid(),
            household_id.as_uuid(),
            owner_type,
            owner_id,
            secret_kind_str(kind),
            sealed.ciphertext,
            sealed.nonce,
            sealed.wrapped_dek,
            sealed.key_version,
            created_by.map(|m| m.as_uuid())
        )
        .fetch_one(conn)
        .await?;
        Ok(id.into())
    }

    pub async fn get(
        conn: &mut PgConnection,
        key: &MasterKey,
        household_id: HouseholdId,
        owner_type: &str,
        owner_id: Uuid,
        kind: SecretKind,
    ) -> Result<Option<Vec<u8>>> {
        let row = sqlx::query!(
            "SELECT ciphertext, nonce, wrapped_dek, key_version FROM secret
             WHERE household_id = $1 AND owner_type = $2 AND owner_id = $3 AND kind = $4",
            household_id.as_uuid(),
            owner_type,
            owner_id,
            secret_kind_str(kind)
        )
        .fetch_optional(conn)
        .await?;
        let Some(r) = row else { return Ok(None) };
        let sealed = SealedSecret {
            ciphertext: r.ciphertext,
            nonce: r.nonce,
            wrapped_dek: r.wrapped_dek,
            key_version: r.key_version,
        };
        key.open(&sealed).map(Some)
    }

    pub async fn delete(
        conn: &mut PgConnection,
        household_id: HouseholdId,
        owner_type: &str,
        owner_id: Uuid,
        kind: SecretKind,
    ) -> Result<bool> {
        Ok(sqlx::query!(
            "DELETE FROM secret WHERE household_id = $1 AND owner_type = $2 AND owner_id = $3 AND kind = $4",
            household_id.as_uuid(),
            owner_type,
            owner_id,
            secret_kind_str(kind)
        )
        .execute(conn)
        .await?
        .rows_affected()
            > 0)
    }

    /// Key rotation: re-wraps every DEK (all households; use a system tx) from `old` to `new`.
    /// Ciphertexts are untouched. Returns the number of secrets rewrapped.
    pub async fn rotate(conn: &mut PgConnection, old: &MasterKey, new: &MasterKey) -> Result<u64> {
        if old.version() == new.version() {
            return Err(StoreError::Key(
                "new master key must have a different version".into(),
            ));
        }
        let rows = sqlx::query!(
            "SELECT id, ciphertext, nonce, wrapped_dek, key_version FROM secret WHERE key_version = $1 FOR UPDATE",
            old.version()
        )
        .fetch_all(&mut *conn)
        .await?;
        let mut n = 0;
        for r in rows {
            let sealed = SealedSecret {
                ciphertext: r.ciphertext,
                nonce: r.nonce,
                wrapped_dek: r.wrapped_dek,
                key_version: r.key_version,
            };
            let re = old.rewrap(&sealed, new)?;
            sqlx::query!(
                "UPDATE secret SET wrapped_dek = $2, key_version = $3, version = version + 1 WHERE id = $1",
                r.id,
                re.wrapped_dek,
                re.key_version
            )
            .execute(&mut *conn)
            .await?;
            n += 1;
        }
        Ok(n)
    }
}

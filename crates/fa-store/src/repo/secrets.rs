use fa_domain::{HouseholdId, MemberId, SecretId, SecretKind};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use super::secret_kind_str;
use crate::secret::{AAD_BOUND, MasterKey, SealedSecret, secret_aad};
use crate::{Result, StoreError};

pub struct SecretRepo;

#[derive(FromRow)]
struct Row {
    ciphertext: Vec<u8>,
    nonce: Vec<u8>,
    wrapped_dek: Vec<u8>,
    key_version: i32,
    aad_version: i16,
}

#[derive(FromRow)]
struct RebindRow {
    id: Uuid,
    household_id: Uuid,
    owner_type: String,
    owner_id: Uuid,
    kind: String,
    ciphertext: Vec<u8>,
    nonce: Vec<u8>,
    wrapped_dek: Vec<u8>,
    key_version: i32,
}

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
        let aad = secret_aad(household_id, owner_type, owner_id, kind);
        let sealed = key.seal(plaintext, &aad)?;
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO secret (id, household_id, owner_type, owner_id, kind, ciphertext, nonce, wrapped_dek,
                                 key_version, aad_version, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (household_id, owner_type, owner_id, kind) DO UPDATE SET
                 ciphertext = EXCLUDED.ciphertext, nonce = EXCLUDED.nonce,
                 wrapped_dek = EXCLUDED.wrapped_dek, key_version = EXCLUDED.key_version,
                 aad_version = EXCLUDED.aad_version, version = secret.version + 1
             RETURNING id",
        )
        .bind(SecretId::new().as_uuid())
        .bind(household_id.as_uuid())
        .bind(owner_type)
        .bind(owner_id)
        .bind(secret_kind_str(kind))
        .bind(sealed.ciphertext)
        .bind(sealed.nonce)
        .bind(sealed.wrapped_dek)
        .bind(sealed.key_version)
        .bind(AAD_BOUND)
        .bind(created_by.map(|m| m.as_uuid()))
        .fetch_one(conn)
        .await?;
        Ok(id.into())
    }

    /// Opens with the owner-bound AAD (`aad_version` 1) or, for rows not yet re-bound, an empty one.
    pub async fn get(
        conn: &mut PgConnection,
        key: &MasterKey,
        household_id: HouseholdId,
        owner_type: &str,
        owner_id: Uuid,
        kind: SecretKind,
    ) -> Result<Option<Vec<u8>>> {
        let row: Option<Row> = sqlx::query_as(
            "SELECT ciphertext, nonce, wrapped_dek, key_version, aad_version FROM secret
             WHERE household_id = $1 AND owner_type = $2 AND owner_id = $3 AND kind = $4",
        )
        .bind(household_id.as_uuid())
        .bind(owner_type)
        .bind(owner_id)
        .bind(secret_kind_str(kind))
        .fetch_optional(conn)
        .await?;
        let Some(r) = row else { return Ok(None) };
        let aad = if r.aad_version >= AAD_BOUND {
            secret_aad(household_id, owner_type, owner_id, kind)
        } else {
            Vec::new()
        };
        let sealed = SealedSecret {
            ciphertext: r.ciphertext,
            nonce: r.nonce,
            wrapped_dek: r.wrapped_dek,
            key_version: r.key_version,
        };
        key.open(&sealed, &aad).map(Some)
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

    /// One-time pass (all households; use a system tx): re-seals every `aad_version = 0` row
    /// under the owner-bound AAD. Rows sealed under another key version are skipped.
    /// Returns the number of rows re-bound.
    pub async fn rebind_all(conn: &mut PgConnection, key: &MasterKey) -> Result<u64> {
        let rows: Vec<RebindRow> = sqlx::query_as(
            "SELECT id, household_id, owner_type, owner_id, kind, ciphertext, nonce, wrapped_dek, key_version
             FROM secret WHERE aad_version = 0 AND key_version = $1 FOR UPDATE",
        )
        .bind(key.version())
        .fetch_all(&mut *conn)
        .await?;
        let mut n = 0;
        for r in rows {
            let kind = match r.kind.as_str() {
                "oauth_token" => SecretKind::OauthToken,
                "api_key" => SecretKind::ApiKey,
                _ => SecretKind::Other,
            };
            let plain = key.open(
                &SealedSecret {
                    ciphertext: r.ciphertext,
                    nonce: r.nonce,
                    wrapped_dek: r.wrapped_dek,
                    key_version: r.key_version,
                },
                &[],
            )?;
            let aad = secret_aad(r.household_id.into(), &r.owner_type, r.owner_id, kind);
            let sealed = key.seal(&plain, &aad)?;
            sqlx::query(
                "UPDATE secret SET ciphertext = $2, nonce = $3, wrapped_dek = $4, key_version = $5,
                     aad_version = $6, version = version + 1 WHERE id = $1",
            )
            .bind(r.id)
            .bind(sealed.ciphertext)
            .bind(sealed.nonce)
            .bind(sealed.wrapped_dek)
            .bind(sealed.key_version)
            .bind(AAD_BOUND)
            .execute(&mut *conn)
            .await?;
            n += 1;
        }
        Ok(n)
    }
}

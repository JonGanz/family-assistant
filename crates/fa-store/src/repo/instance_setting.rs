//! System-wide settings (`instance_setting`, no household): plain JSON values, or secrets
//! sealed with the master key (D-075). Use through `Store::tx_system`.

use serde_json::Value;
use sqlx::{FromRow, PgConnection};

use crate::secret::{AAD_BOUND, MasterKey, SealedSecret, instance_setting_aad};
use crate::{Result, StoreError};

pub struct InstanceSettingRepo;

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceSettingRow {
    pub key: String,
    /// `None` for secrets and for unset values.
    pub value: Option<Value>,
    pub has_secret: bool,
}

#[derive(FromRow)]
struct Row {
    key: String,
    value: Option<Value>,
    has_secret: bool,
}

#[derive(FromRow)]
struct SecretRow {
    ciphertext: Option<Vec<u8>>,
    nonce: Option<Vec<u8>>,
    wrapped_dek: Option<Vec<u8>>,
    key_version: Option<i32>,
    aad_version: i16,
}

#[derive(FromRow)]
struct RebindRow {
    key: String,
    ciphertext: Vec<u8>,
    nonce: Vec<u8>,
    wrapped_dek: Vec<u8>,
    key_version: i32,
}

impl InstanceSettingRepo {
    pub async fn all(conn: &mut PgConnection) -> Result<Vec<InstanceSettingRow>> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT key, value, ciphertext IS NOT NULL AS has_secret FROM instance_setting ORDER BY key",
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| InstanceSettingRow {
                key: r.key,
                value: r.value,
                has_secret: r.has_secret,
            })
            .collect())
    }

    /// Sets a plain value, or deletes the key (back to the default) when `None`.
    /// The crypto columns are cleared. Calls `notify_changed`.
    pub async fn set(conn: &mut PgConnection, key: &str, value: Option<&Value>) -> Result<()> {
        match value {
            Some(v) => {
                sqlx::query(
                    "INSERT INTO instance_setting (key, value) VALUES ($1, $2)
                     ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, ciphertext = NULL,
                         nonce = NULL, wrapped_dek = NULL, key_version = NULL, aad_version = 0",
                )
                .bind(key)
                .bind(v)
                .execute(&mut *conn)
                .await?;
            }
            None => {
                sqlx::query("DELETE FROM instance_setting WHERE key = $1")
                    .bind(key)
                    .execute(&mut *conn)
                    .await?;
            }
        }
        Self::notify_changed(conn).await
    }

    /// Seals `plaintext` (envelope encryption), or deletes the key when `None`.
    /// Calls `notify_changed`. The secret is never logged.
    pub async fn set_secret(
        conn: &mut PgConnection,
        key: &str,
        master: &MasterKey,
        plaintext: Option<&str>,
    ) -> Result<()> {
        match plaintext {
            Some(p) => {
                let sealed = master.seal(p.as_bytes(), &instance_setting_aad(key))?;
                sqlx::query(
                    "INSERT INTO instance_setting (key, value, ciphertext, nonce, wrapped_dek, key_version, aad_version)
                     VALUES ($1, NULL, $2, $3, $4, $5, $6)
                     ON CONFLICT (key) DO UPDATE SET value = NULL, ciphertext = EXCLUDED.ciphertext,
                         nonce = EXCLUDED.nonce, wrapped_dek = EXCLUDED.wrapped_dek,
                         key_version = EXCLUDED.key_version, aad_version = EXCLUDED.aad_version",
                )
                .bind(key)
                .bind(sealed.ciphertext)
                .bind(sealed.nonce)
                .bind(sealed.wrapped_dek)
                .bind(sealed.key_version)
                .bind(AAD_BOUND)
                .execute(&mut *conn)
                .await?;
            }
            None => {
                sqlx::query("DELETE FROM instance_setting WHERE key = $1")
                    .bind(key)
                    .execute(&mut *conn)
                    .await?;
            }
        }
        Self::notify_changed(conn).await
    }

    /// The decrypted secret, or `None` when the key is absent or holds no secret.
    pub async fn secret(
        conn: &mut PgConnection,
        key: &str,
        master: &MasterKey,
    ) -> Result<Option<String>> {
        let row: Option<SecretRow> = sqlx::query_as(
            "SELECT ciphertext, nonce, wrapped_dek, key_version, aad_version FROM instance_setting WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&mut *conn)
        .await?;
        let Some(SecretRow {
            ciphertext: Some(ciphertext),
            nonce: Some(nonce),
            wrapped_dek: Some(wrapped_dek),
            key_version: Some(key_version),
            aad_version,
        }) = row
        else {
            return Ok(None);
        };
        let aad = if aad_version >= AAD_BOUND {
            instance_setting_aad(key)
        } else {
            Vec::new()
        };
        let plain = master.open(
            &SealedSecret {
                ciphertext,
                nonce,
                wrapped_dek,
                key_version,
            },
            &aad,
        )?;
        String::from_utf8(plain)
            .map(Some)
            .map_err(|_| StoreError::Corrupt("instance setting is not UTF-8".into()))
    }

    /// One-time pass: re-seals every `aad_version = 0` secret under its key-bound AAD.
    /// Rows sealed under another key version are skipped. Returns the number re-bound.
    /// Does not notify: the values do not change.
    pub async fn rebind_all(conn: &mut PgConnection, master: &MasterKey) -> Result<u64> {
        let rows: Vec<RebindRow> = sqlx::query_as(
            "SELECT key, ciphertext, nonce, wrapped_dek, key_version FROM instance_setting
             WHERE aad_version = 0 AND ciphertext IS NOT NULL AND nonce IS NOT NULL
               AND wrapped_dek IS NOT NULL AND key_version = $1 FOR UPDATE",
        )
        .bind(master.version())
        .fetch_all(&mut *conn)
        .await?;
        let mut n = 0;
        for r in rows {
            let plain = master.open(
                &SealedSecret {
                    ciphertext: r.ciphertext,
                    nonce: r.nonce,
                    wrapped_dek: r.wrapped_dek,
                    key_version: r.key_version,
                },
                &[],
            )?;
            let sealed = master.seal(&plain, &instance_setting_aad(&r.key))?;
            sqlx::query(
                "UPDATE instance_setting SET ciphertext = $2, nonce = $3, wrapped_dek = $4,
                     key_version = $5, aad_version = $6 WHERE key = $1",
            )
            .bind(&r.key)
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

    /// `pg_notify('fa_settings', '')`: delivered when the transaction commits.
    pub async fn notify_changed(conn: &mut PgConnection) -> Result<()> {
        sqlx::query("SELECT pg_notify('fa_settings', '')")
            .execute(conn)
            .await?;
        Ok(())
    }
}

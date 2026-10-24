use fa_domain::MemberId;
use sqlx::PgConnection;

use crate::Result;

pub struct MemberAvatarRepo;

impl MemberAvatarRepo {
    /// Inserts or replaces the member's avatar.
    pub async fn put(
        conn: &mut PgConnection,
        member: MemberId,
        mime: &str,
        bytes: &[u8],
        sha256: &str,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO member_avatar (member_id, household_id, mime, bytes, sha256)
             SELECT id, household_id, $2, $3, $4 FROM member WHERE id = $1
             ON CONFLICT (member_id) DO UPDATE SET
                 mime = EXCLUDED.mime, bytes = EXCLUDED.bytes, sha256 = EXCLUDED.sha256",
            member.as_uuid(),
            mime,
            bytes,
            sha256
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// `(mime, bytes, sha256)`.
    pub async fn get(
        conn: &mut PgConnection,
        member: MemberId,
    ) -> Result<Option<(String, Vec<u8>, String)>> {
        let r = sqlx::query!(
            "SELECT mime, bytes, sha256 FROM member_avatar WHERE member_id = $1",
            member.as_uuid()
        )
        .fetch_optional(conn)
        .await?;
        Ok(r.map(|r| (r.mime, r.bytes, r.sha256)))
    }

    /// The sha256 of every avatar (for `avatar_url` cache-busting) without loading the bytes.
    pub async fn hashes(conn: &mut PgConnection) -> Result<Vec<(MemberId, String)>> {
        let rows = sqlx::query!("SELECT member_id, sha256 FROM member_avatar")
            .fetch_all(conn)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.member_id.into(), r.sha256))
            .collect())
    }

    pub async fn delete(conn: &mut PgConnection, member: MemberId) -> Result<()> {
        sqlx::query!(
            "DELETE FROM member_avatar WHERE member_id = $1",
            member.as_uuid()
        )
        .execute(conn)
        .await?;
        Ok(())
    }
}

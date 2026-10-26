//! Repository extensions for M3 task T-10 (mail processing, inbox, labelling, setup).

use fa_domain::{EnrollmentId, OrgContact, OrgContactId};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{EnrollmentRepo, OrgContactRepo, enum_str};
use crate::Result;

impl OrgContactRepo {
    /// Inserts the contact unless `(org, email)` exists in any state (archived included).
    /// Returns the new id, or `None` when the address is already known.
    pub async fn insert_if_new(
        conn: &mut PgConnection,
        c: &OrgContact,
    ) -> Result<Option<OrgContactId>> {
        let id: Option<Uuid> = sqlx::query_scalar(
            "INSERT INTO org_contact (id, household_id, org_id, name, email, role)
             VALUES ($1, $2, $3, $4, lower($5), $6)
             ON CONFLICT (org_id, email) DO NOTHING RETURNING id",
        )
        .bind(c.id.as_uuid())
        .bind(c.household_id.as_uuid())
        .bind(c.org_id.as_uuid())
        .bind(&c.name)
        .bind(&c.email)
        .bind(enum_str(&c.role))
        .fetch_optional(conn)
        .await?;
        Ok(id.map(Into::into))
    }

    /// The non-archived contact of the org with this address.
    pub async fn find_in_org(
        conn: &mut PgConnection,
        org: fa_domain::OrgId,
        email: &str,
    ) -> Result<Option<OrgContactId>> {
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM org_contact
             WHERE org_id = $1 AND email = lower($2) AND archived_at IS NULL",
        )
        .bind(org.as_uuid())
        .bind(email)
        .fetch_optional(conn)
        .await?;
        Ok(id.map(Into::into))
    }
}

impl EnrollmentRepo {
    /// Links one more contact to the enrollment (keeps the existing links).
    pub async fn add_contact(
        conn: &mut PgConnection,
        enrollment: EnrollmentId,
        contact: OrgContactId,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO enrollment_contact (enrollment_id, contact_id, household_id)
             SELECT $1, $2, household_id FROM enrollment WHERE id = $1
             ON CONFLICT DO NOTHING",
        )
        .bind(enrollment.as_uuid())
        .bind(contact.as_uuid())
        .execute(conn)
        .await?;
        Ok(())
    }

    /// `(enrollment, contact email)` for every enrollment contact of the household, so the rules
    /// context can be built in one query.
    pub async fn contact_emails(conn: &mut PgConnection) -> Result<Vec<(EnrollmentId, String)>> {
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT ec.enrollment_id, c.email
             FROM enrollment_contact ec JOIN org_contact c ON c.id = ec.contact_id
             WHERE c.archived_at IS NULL ORDER BY ec.enrollment_id, c.email",
        )
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(|(e, m)| (e.into(), m)).collect())
    }
}

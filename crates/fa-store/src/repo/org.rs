//! Organisations, their contacts, enrollments and contact suggestions.

use chrono::{DateTime, NaiveDate, Utc};
use fa_domain::{
    ContactSuggestion, ContactSuggestionId, Enrollment, EnrollmentId, MemberId, Org, OrgContact,
    OrgContactId, OrgId, SuggestionStatus,
};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError, Versioned};

fn one(n: u64) -> Result<()> {
    if n == 0 {
        Err(StoreError::NotFound)
    } else {
        Ok(())
    }
}

// ---- org ----

pub struct OrgRepo;

#[derive(sqlx::FromRow)]
struct OrgRow {
    id: Uuid,
    household_id: Uuid,
    kind: String,
    name: String,
    domains: Vec<String>,
    senders: Vec<String>,
    ics_calendar_id: Option<Uuid>,
    default_calendar_id: Option<Uuid>,
    contact_mode: String,
    bill_amount_regex: Option<String>,
    bill_due_regex: Option<String>,
    bill_account_regex: Option<String>,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const ORG_COLS: &str = "id, household_id, kind, name, domains, senders, ics_calendar_id,
    default_calendar_id, contact_mode, bill_amount_regex, bill_due_regex, bill_account_regex,
    archived_at, version, created_at, updated_at";

impl TryFrom<OrgRow> for Versioned<Org> {
    type Error = StoreError;
    fn try_from(r: OrgRow) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: Org {
                id: r.id.into(),
                household_id: r.household_id.into(),
                kind: parse_enum(&r.kind)?,
                name: r.name,
                domains: r.domains,
                senders: r.senders,
                ics_calendar_id: r.ics_calendar_id.map(Into::into),
                default_calendar_id: r.default_calendar_id.map(Into::into),
                contact_mode: parse_enum(&r.contact_mode)?,
                bill_amount_regex: r.bill_amount_regex,
                bill_due_regex: r.bill_due_regex,
                bill_account_regex: r.bill_account_regex,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

impl OrgRepo {
    pub async fn insert(conn: &mut PgConnection, o: &Org) -> Result<Versioned<Org>> {
        let row: OrgRow = sqlx::query_as(&format!(
            "INSERT INTO org (id, household_id, kind, name, domains, senders, ics_calendar_id,
                              default_calendar_id, contact_mode, bill_amount_regex, bill_due_regex,
                              bill_account_regex)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING {ORG_COLS}"
        ))
        .bind(o.id.as_uuid())
        .bind(o.household_id.as_uuid())
        .bind(enum_str(&o.kind))
        .bind(&o.name)
        .bind(&o.domains)
        .bind(&o.senders)
        .bind(o.ics_calendar_id.map(|c| c.as_uuid()))
        .bind(o.default_calendar_id.map(|c| c.as_uuid()))
        .bind(enum_str(&o.contact_mode))
        .bind(&o.bill_amount_regex)
        .bind(&o.bill_due_regex)
        .bind(&o.bill_account_regex)
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(conn: &mut PgConnection, id: OrgId) -> Result<Option<Versioned<Org>>> {
        let row: Option<OrgRow> =
            sqlx::query_as(&format!("SELECT {ORG_COLS} FROM org WHERE id = $1"))
                .bind(id.as_uuid())
                .fetch_optional(conn)
                .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// A non-archived org by name, case-insensitive.
    pub async fn get_by_name(
        conn: &mut PgConnection,
        name: &str,
    ) -> Result<Option<Versioned<Org>>> {
        let row: Option<OrgRow> = sqlx::query_as(&format!(
            "SELECT {ORG_COLS} FROM org WHERE lower(name) = lower($1) AND archived_at IS NULL"
        ))
        .bind(name)
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn list(
        conn: &mut PgConnection,
        include_archived: bool,
    ) -> Result<Vec<Versioned<Org>>> {
        let rows: Vec<OrgRow> = sqlx::query_as(&format!(
            "SELECT {ORG_COLS} FROM org WHERE $1 OR archived_at IS NULL ORDER BY lower(name), id"
        ))
        .bind(include_archived)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Returns the new version, or `Conflict` if `expected_version` is stale.
    pub async fn update(conn: &mut PgConnection, o: &Org, expected_version: i32) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE org SET kind = $2, name = $3, domains = $4, senders = $5, ics_calendar_id = $6,
                    default_calendar_id = $7, contact_mode = $8, bill_amount_regex = $9,
                    bill_due_regex = $10, bill_account_regex = $11, version = version + 1
             WHERE id = $1 AND version = $12 RETURNING version",
        )
        .bind(o.id.as_uuid())
        .bind(enum_str(&o.kind))
        .bind(&o.name)
        .bind(&o.domains)
        .bind(&o.senders)
        .bind(o.ics_calendar_id.map(|c| c.as_uuid()))
        .bind(o.default_calendar_id.map(|c| c.as_uuid()))
        .bind(enum_str(&o.contact_mode))
        .bind(&o.bill_amount_regex)
        .bind(&o.bill_due_regex)
        .bind(&o.bill_account_regex)
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "org", o.id.as_uuid()).await),
        }
    }

    /// Soft delete. Idempotent.
    pub async fn archive(conn: &mut PgConnection, id: OrgId) -> Result<()> {
        one(sqlx::query(
            "UPDATE org SET archived_at = COALESCE(archived_at, now()), version = version + 1
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .execute(conn)
        .await?
        .rows_affected())
    }
}

// ---- org contact ----

pub struct OrgContactRepo;

#[derive(sqlx::FromRow)]
struct ContactRow {
    id: Uuid,
    household_id: Uuid,
    org_id: Uuid,
    name: String,
    email: String,
    role: String,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const CONTACT_COLS: &str =
    "id, household_id, org_id, name, email, role, archived_at, version, created_at, updated_at";

impl TryFrom<ContactRow> for Versioned<OrgContact> {
    type Error = StoreError;
    fn try_from(r: ContactRow) -> Result<Self> {
        Ok(Versioned {
            version: r.version,
            value: OrgContact {
                id: r.id.into(),
                household_id: r.household_id.into(),
                org_id: r.org_id.into(),
                name: r.name,
                email: r.email,
                role: parse_enum(&r.role)?,
                archived_at: r.archived_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
    }
}

impl OrgContactRepo {
    pub async fn insert(conn: &mut PgConnection, c: &OrgContact) -> Result<Versioned<OrgContact>> {
        let row: ContactRow = sqlx::query_as(&format!(
            "INSERT INTO org_contact (id, household_id, org_id, name, email, role)
             VALUES ($1, $2, $3, $4, lower($5), $6) RETURNING {CONTACT_COLS}"
        ))
        .bind(c.id.as_uuid())
        .bind(c.household_id.as_uuid())
        .bind(c.org_id.as_uuid())
        .bind(&c.name)
        .bind(&c.email)
        .bind(enum_str(&c.role))
        .fetch_one(conn)
        .await?;
        row.try_into()
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: OrgContactId,
    ) -> Result<Option<Versioned<OrgContact>>> {
        let row: Option<ContactRow> = sqlx::query_as(&format!(
            "SELECT {CONTACT_COLS} FROM org_contact WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Non-archived contacts of an org.
    pub async fn list(conn: &mut PgConnection, org: OrgId) -> Result<Vec<Versioned<OrgContact>>> {
        let rows: Vec<ContactRow> = sqlx::query_as(&format!(
            "SELECT {CONTACT_COLS} FROM org_contact WHERE org_id = $1 AND archived_at IS NULL
             ORDER BY lower(name), id"
        ))
        .bind(org.as_uuid())
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn update(
        conn: &mut PgConnection,
        c: &OrgContact,
        expected_version: i32,
    ) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE org_contact SET name = $2, email = lower($3), role = $4, version = version + 1
             WHERE id = $1 AND version = $5 RETURNING version",
        )
        .bind(c.id.as_uuid())
        .bind(&c.name)
        .bind(&c.email)
        .bind(enum_str(&c.role))
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "org_contact", c.id.as_uuid()).await),
        }
    }

    pub async fn archive(conn: &mut PgConnection, id: OrgContactId) -> Result<()> {
        one(sqlx::query(
            "UPDATE org_contact SET archived_at = COALESCE(archived_at, now()), version = version + 1
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// Non-archived contacts with this address (lower-cased by the caller or here) in
    /// non-archived orgs, with the org id.
    pub async fn by_email(
        conn: &mut PgConnection,
        email: &str,
    ) -> Result<Vec<(OrgContact, OrgId)>> {
        let rows: Vec<ContactRow> = sqlx::query_as(
            "SELECT c.id, c.household_id, c.org_id, c.name, c.email, c.role, c.archived_at, c.version,
                    c.created_at, c.updated_at
             FROM org_contact c JOIN org o ON o.id = c.org_id
             WHERE c.email = lower($1) AND c.archived_at IS NULL AND o.archived_at IS NULL
             ORDER BY c.created_at, c.id",
        )
        .bind(email)
        .fetch_all(conn)
        .await?;
        rows.into_iter()
            .map(|r| {
                let org = OrgId::from(r.org_id);
                let v: Versioned<OrgContact> = r.try_into()?;
                Ok((v.value, org))
            })
            .collect()
    }
}

// ---- enrollment ----

pub struct EnrollmentRepo;

#[derive(sqlx::FromRow)]
struct EnrollmentRow {
    id: Uuid,
    household_id: Uuid,
    member_id: Uuid,
    org_id: Uuid,
    grade: Option<String>,
    class_name: Option<String>,
    from_date: Option<NaiveDate>,
    until_date: Option<NaiveDate>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const ENROLLMENT_COLS: &str = "id, household_id, member_id, org_id, grade, class_name, from_date,
    until_date, version, created_at, updated_at";

impl From<EnrollmentRow> for Versioned<Enrollment> {
    fn from(r: EnrollmentRow) -> Self {
        Versioned {
            version: r.version,
            value: Enrollment {
                id: r.id.into(),
                household_id: r.household_id.into(),
                member_id: r.member_id.into(),
                org_id: r.org_id.into(),
                grade: r.grade,
                class_name: r.class_name,
                from_date: r.from_date,
                until_date: r.until_date,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        }
    }
}

impl EnrollmentRepo {
    pub async fn insert(conn: &mut PgConnection, e: &Enrollment) -> Result<Versioned<Enrollment>> {
        let row: EnrollmentRow = sqlx::query_as(&format!(
            "INSERT INTO enrollment (id, household_id, member_id, org_id, grade, class_name,
                                     from_date, until_date)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {ENROLLMENT_COLS}"
        ))
        .bind(e.id.as_uuid())
        .bind(e.household_id.as_uuid())
        .bind(e.member_id.as_uuid())
        .bind(e.org_id.as_uuid())
        .bind(&e.grade)
        .bind(&e.class_name)
        .bind(e.from_date)
        .bind(e.until_date)
        .fetch_one(conn)
        .await?;
        Ok(row.into())
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: EnrollmentId,
    ) -> Result<Option<Versioned<Enrollment>>> {
        let row: Option<EnrollmentRow> = sqlx::query_as(&format!(
            "SELECT {ENROLLMENT_COLS} FROM enrollment WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(conn)
        .await?;
        Ok(row.map(Into::into))
    }

    /// Enrollments of one member, or of everybody.
    pub async fn list(
        conn: &mut PgConnection,
        member: Option<MemberId>,
    ) -> Result<Vec<Versioned<Enrollment>>> {
        let rows: Vec<EnrollmentRow> = sqlx::query_as(&format!(
            "SELECT {ENROLLMENT_COLS} FROM enrollment
             WHERE $1::uuid IS NULL OR member_id = $1 ORDER BY created_at, id"
        ))
        .bind(member.map(|m| m.as_uuid()))
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn update(
        conn: &mut PgConnection,
        e: &Enrollment,
        expected_version: i32,
    ) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE enrollment SET org_id = $2, grade = $3, class_name = $4, from_date = $5,
                    until_date = $6, version = version + 1
             WHERE id = $1 AND version = $7 RETURNING version",
        )
        .bind(e.id.as_uuid())
        .bind(e.org_id.as_uuid())
        .bind(&e.grade)
        .bind(&e.class_name)
        .bind(e.from_date)
        .bind(e.until_date)
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "enrollment", e.id.as_uuid()).await),
        }
    }

    pub async fn delete(conn: &mut PgConnection, id: EnrollmentId) -> Result<()> {
        one(sqlx::query("DELETE FROM enrollment WHERE id = $1")
            .bind(id.as_uuid())
            .execute(conn)
            .await?
            .rows_affected())
    }

    /// Replaces the contacts linked to the enrollment.
    pub async fn set_contacts(
        conn: &mut PgConnection,
        enrollment: EnrollmentId,
        contacts: &[OrgContactId],
    ) -> Result<()> {
        sqlx::query("DELETE FROM enrollment_contact WHERE enrollment_id = $1")
            .bind(enrollment.as_uuid())
            .execute(&mut *conn)
            .await?;
        let ids: Vec<Uuid> = contacts.iter().map(|c| c.as_uuid()).collect();
        sqlx::query(
            "INSERT INTO enrollment_contact (enrollment_id, contact_id, household_id)
             SELECT $1, c, (SELECT household_id FROM enrollment WHERE id = $1)
             FROM unnest($2::uuid[]) AS c ON CONFLICT DO NOTHING",
        )
        .bind(enrollment.as_uuid())
        .bind(&ids)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn contacts(
        conn: &mut PgConnection,
        enrollment: EnrollmentId,
    ) -> Result<Vec<OrgContactId>> {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT contact_id FROM enrollment_contact WHERE enrollment_id = $1 ORDER BY contact_id",
        )
        .bind(enrollment.as_uuid())
        .fetch_all(conn)
        .await?;
        Ok(ids.into_iter().map(Into::into).collect())
    }
}

// ---- contact suggestion ----

pub struct ContactSuggestionRepo;

#[derive(sqlx::FromRow)]
struct SuggestionRow {
    id: Uuid,
    household_id: Uuid,
    org_id: Uuid,
    email: String,
    name: Option<String>,
    member_id: Option<Uuid>,
    message_id: Option<Uuid>,
    status: String,
    decided_by: Option<Uuid>,
    decided_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const SUGGESTION_COLS: &str =
    "id, household_id, org_id, email, name, member_id, message_id, status,
    decided_by, decided_at, created_at, updated_at";

impl TryFrom<SuggestionRow> for ContactSuggestion {
    type Error = StoreError;
    fn try_from(r: SuggestionRow) -> Result<Self> {
        Ok(ContactSuggestion {
            id: r.id.into(),
            household_id: r.household_id.into(),
            org_id: r.org_id.into(),
            email: r.email,
            name: r.name,
            member_id: r.member_id.map(Into::into),
            message_id: r.message_id.map(Into::into),
            status: parse_enum(&r.status)?,
            decided_by: r.decided_by.map(Into::into),
            decided_at: r.decided_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

impl ContactSuggestionRepo {
    /// Inserts a `proposed` suggestion. Returns `false` when `(org, email)` already exists in any status.
    pub async fn insert_if_new(conn: &mut PgConnection, s: &ContactSuggestion) -> Result<bool> {
        let n = sqlx::query(
            "INSERT INTO contact_suggestion (id, household_id, org_id, email, name, member_id, message_id)
             VALUES ($1, $2, $3, lower($4), $5, $6, $7)
             ON CONFLICT (org_id, email) DO NOTHING",
        )
        .bind(s.id.as_uuid())
        .bind(s.household_id.as_uuid())
        .bind(s.org_id.as_uuid())
        .bind(&s.email)
        .bind(&s.name)
        .bind(s.member_id.map(|m| m.as_uuid()))
        .bind(s.message_id.map(|m| m.as_uuid()))
        .execute(conn)
        .await?
        .rows_affected();
        Ok(n > 0)
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: ContactSuggestionId,
    ) -> Result<Option<ContactSuggestion>> {
        let row: Option<SuggestionRow> = sqlx::query_as(&format!(
            "SELECT {SUGGESTION_COLS} FROM contact_suggestion WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(conn)
        .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn list(
        conn: &mut PgConnection,
        status: Option<SuggestionStatus>,
    ) -> Result<Vec<ContactSuggestion>> {
        let rows: Vec<SuggestionRow> = sqlx::query_as(&format!(
            "SELECT {SUGGESTION_COLS} FROM contact_suggestion
             WHERE $1::text IS NULL OR status = $1 ORDER BY created_at, id"
        ))
        .bind(status.as_ref().map(enum_str))
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    /// Records the decision (`decided_at = now()`; `decided_by` may be `None` for system changes).
    pub async fn set_status(
        conn: &mut PgConnection,
        id: ContactSuggestionId,
        status: SuggestionStatus,
        decided_by: Option<MemberId>,
    ) -> Result<()> {
        one(sqlx::query(
            "UPDATE contact_suggestion SET status = $2, decided_by = $3, decided_at = now()
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(enum_str(&status))
        .bind(decided_by.map(|m| m.as_uuid()))
        .execute(conn)
        .await?
        .rows_affected())
    }

    /// `proposed` suggestions created before `cutoff` become `expired`.
    pub async fn expire_older_than(conn: &mut PgConnection, cutoff: DateTime<Utc>) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE contact_suggestion SET status = 'expired', decided_at = now()
             WHERE status = 'proposed' AND created_at < $1",
        )
        .bind(cutoff)
        .execute(conn)
        .await?
        .rows_affected())
    }
}

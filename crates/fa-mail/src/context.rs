//! The rules context of a household and source, built from the store, and the `Viewer`.

use std::collections::HashMap;
use std::str::FromStr;

use chrono::Utc;
use chrono_tz::Tz;
use fa_domain::{
    Category, CategoryBehaviour, Household, HouseholdId, MailSource, MemberId, Org, Role,
};
use fa_rules::{ContactCtx, Context, EnrollCtx, MemberCtx, OrgCtx};
use fa_store::{
    CategoryRepo, EnrollmentRepo, HouseholdRepo, MailRuleRepo, MailSourceRepo, MemberRepo,
    OrgContactRepo, OrgRepo,
};
use sqlx::PgConnection;

use crate::{MailError, Result};

/// Who is looking: a signed-in member or a dashboard device (like `fa_tasks::Viewer`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Viewer {
    pub member: Option<MemberId>,
    pub role: Option<Role>,
    pub device: bool,
    /// A dashboard device with a valid PIN elevation (D-019).
    pub elevated: bool,
}

impl Viewer {
    pub fn adult(member: MemberId) -> Self {
        Self {
            member: Some(member),
            role: Some(Role::Adult),
            device: false,
            elevated: false,
        }
    }

    /// The adult member, if the viewer is one.
    pub fn adult_member(&self) -> Option<MemberId> {
        match (self.member, self.role) {
            (Some(m), Some(Role::Adult)) if !self.device => Some(m),
            _ => None,
        }
    }
}

/// Everything a processing run needs, loaded once per job.
pub struct ProcessCtx {
    pub rules: Context,
    pub household: Household,
    /// Non-archived categories by key.
    pub categories: HashMap<String, Category>,
    pub orgs: Vec<Org>,
    /// Owners of the source (empty without a source).
    pub owners: Vec<MemberId>,
    /// Non-archived adults.
    pub adults: Vec<MemberId>,
}

impl ProcessCtx {
    pub fn tz(&self) -> Tz {
        self.rules.tz
    }

    pub fn behaviour(&self, key: &str) -> CategoryBehaviour {
        self.categories
            .get(key)
            .map_or(CategoryBehaviour::Keep, |c| c.behaviour)
    }

    pub fn member(&self, id: MemberId) -> Option<&MemberCtx> {
        self.rules.members.iter().find(|m| m.id == id)
    }

    pub fn org(&self, id: fa_domain::OrgId) -> Option<&Org> {
        self.orgs.iter().find(|o| o.id == id)
    }

    /// Adults who may read the source: its adult owners, or every adult for a household source.
    pub fn readers(&self, source: &MailSource) -> Vec<MemberId> {
        match source.visibility {
            fa_domain::MailVisibility::Household => self.adults.clone(),
            fa_domain::MailVisibility::Owners => self
                .adults
                .iter()
                .copied()
                .filter(|a| self.owners.contains(a))
                .collect(),
        }
    }
}

pub fn parse_tz(name: &str) -> Tz {
    Tz::from_str(name).unwrap_or(Tz::UTC)
}

/// Loads the context for `household`; `source` supplies the owners.
pub async fn load(
    conn: &mut PgConnection,
    household: HouseholdId,
    source: Option<&MailSource>,
) -> Result<ProcessCtx> {
    let hh = HouseholdRepo::get(conn, household)
        .await?
        .ok_or(MailError::NotFound)?
        .value;
    let tz = parse_tz(&hh.tz);
    let today = Utc::now().with_timezone(&tz).date_naive();

    let mut org_ctx = Vec::new();
    let orgs: Vec<Org> = OrgRepo::list(conn, false)
        .await?
        .into_iter()
        .map(|o| o.value)
        .collect();
    for o in &orgs {
        let contacts = OrgContactRepo::list(conn, o.id)
            .await?
            .into_iter()
            .map(|c| ContactCtx {
                email: c.value.email,
                name: Some(c.value.name).filter(|n| !n.is_empty()),
            })
            .collect();
        org_ctx.push(OrgCtx {
            id: o.id,
            kind: o.kind,
            name: o.name.clone(),
            domains: o.domains.clone(),
            senders: o.senders.clone(),
            contacts,
            bill_amount_regex: o.bill_amount_regex.clone(),
            bill_due_regex: o.bill_due_regex.clone(),
            bill_account_regex: o.bill_account_regex.clone(),
        });
    }

    let mut contact_emails: HashMap<fa_domain::EnrollmentId, Vec<String>> = HashMap::new();
    for (e, email) in EnrollmentRepo::contact_emails(conn).await? {
        contact_emails.entry(e).or_default().push(email);
    }
    let mut enrollments: HashMap<MemberId, Vec<EnrollCtx>> = HashMap::new();
    for e in EnrollmentRepo::list(conn, None).await? {
        let e = e.value;
        let valid =
            e.from_date.is_none_or(|f| f <= today) && e.until_date.is_none_or(|u| u >= today);
        if !valid {
            continue;
        }
        enrollments.entry(e.member_id).or_default().push(EnrollCtx {
            org: e.org_id,
            grade: e.grade,
            class_name: e.class_name,
            contacts: contact_emails.remove(&e.id).unwrap_or_default(),
        });
    }

    let members = MemberRepo::list(conn, false).await?;
    let adults: Vec<MemberId> = members
        .iter()
        .filter(|m| m.value.role == Role::Adult)
        .map(|m| m.value.id)
        .collect();
    let members: Vec<MemberCtx> = members
        .into_iter()
        .map(|m| {
            let m = m.value;
            MemberCtx {
                id: m.id,
                name: m.display_name,
                aliases: m.aliases,
                grade: m.grade,
                gender: m.gender,
                tags: m.relevance_tags,
                role: m.role,
                enrollments: enrollments.remove(&m.id).unwrap_or_default(),
            }
        })
        .collect();

    let cats: Vec<Category> = CategoryRepo::list(conn, false)
        .await?
        .into_iter()
        .map(|c| c.value)
        .collect();
    let rules: Vec<_> = MailRuleRepo::list(conn, None, false)
        .await?
        .into_iter()
        .map(|r| r.value)
        .filter(|r| r.enabled)
        .collect();
    let owners = match source {
        Some(s) => MailSourceRepo::owners(conn, s.id).await?,
        None => Vec::new(),
    };

    let ctx = Context {
        orgs: org_ctx,
        members,
        gender_terms: hh.gender_terms.clone(),
        rules,
        disabled_signals: hh.disabled_signals.iter().cloned().collect(),
        categories: cats.iter().map(|c| (c.key.clone(), c.behaviour)).collect(),
        owners: owners.clone(),
        tz,
    };
    Ok(ProcessCtx {
        rules: ctx,
        household: hh,
        categories: cats.into_iter().map(|c| (c.key.clone(), c)).collect(),
        orgs,
        owners,
        adults,
    })
}

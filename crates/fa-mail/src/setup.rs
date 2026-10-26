//! Mail setup: organisations, contacts, enrollments, rules, signals, categories, and the message
//! queries of the mail views.

use chrono::Utc;
use fa_domain::{
    BillTemplate, Category, CategoryBehaviour, CategoryId, CategoryInput, CategoryPatch,
    CategoryView, ContactMode, DecisionPointKey, Enrollment, EnrollmentId, EnrollmentInput,
    EnrollmentPatch, EnrollmentView, HouseholdId, MailRule, MailRuleId, MailRuleInput,
    MailRulePatch, MailRuleView, MemberId, MessageDetail, MessageId, MessagePage, MessageQuery,
    MessageSummary, Org, OrgContact, OrgContactId, OrgContactInput, OrgContactPatch,
    OrgContactView, OrgId, OrgInput, OrgPatch, OrgView, RuleCondition, RuleField, RuleOp,
    RuleTestMatch, RuleTestRequest, RuleTestResult, SignalView, normalise_address,
    valid_category_key,
};
use fa_store::{
    CategoryRepo, EnrollmentRepo, HouseholdRepo, MailRuleRepo, MailSourceRepo, MemberRepo,
    MessageFilter, MessageRepo, OrgContactRepo, OrgRepo, Store, Versioned, mail_defaults,
};
use sqlx::PgConnection;

use crate::context::{self, Viewer};
use crate::process::views::{self, readable_sources, sources_by_id};
use crate::process::{OwnedFacts, enqueue, short_hash};
use crate::{MailError, Result};

const DEFAULT_RULE_PRIORITY: i32 = 100;
const DEFAULT_RULE_CONFIDENCE: f32 = 0.9;
const TEST_DEFAULT_LIMIT: u32 = 100;
const TEST_MAX_LIMIT: u32 = 500;

#[derive(Clone)]
pub struct SetupService {
    store: Store,
}

fn invalid(msg: impl Into<String>) -> MailError {
    MailError::Invalid(msg.into())
}

fn clean_domain(d: &str) -> Option<String> {
    let d = d
        .trim()
        .trim_start_matches(['@', '.'])
        .trim_end_matches('.');
    (!d.is_empty()).then(|| d.to_lowercase())
}

fn clean_list<T>(v: &[String], f: impl Fn(&str) -> Option<T>) -> Vec<T>
where
    T: PartialEq,
{
    let mut out: Vec<T> = Vec::new();
    for s in v {
        if let Some(x) = f(s)
            && !out.contains(&x)
        {
            out.push(x);
        }
    }
    out
}

fn clean_senders(v: &[String]) -> Vec<String> {
    clean_list(v, |s| {
        let a = normalise_address(s);
        (!a.is_empty()).then_some(a)
    })
}

fn clean_email(e: &str) -> Result<String> {
    let e = normalise_address(e);
    match e.split_once('@') {
        Some((l, d)) if !l.is_empty() && !d.is_empty() && !e.contains(char::is_whitespace) => Ok(e),
        _ => Err(invalid("not a valid e-mail address")),
    }
}

/// A bill-template regex must compile under the same limits as rule regexes.
fn check_regex(label: &str, pattern: &Option<String>) -> Result<()> {
    let Some(p) = pattern.as_deref().filter(|p| !p.trim().is_empty()) else {
        return Ok(());
    };
    fa_rules::validate_rule(&[RuleCondition {
        field: RuleField::Body,
        op: RuleOp::Matches,
        header: None,
        values: vec![p.to_owned()],
    }])
    .map_err(|e| invalid(format!("{label}: {e}")))
}

fn template_parts(
    t: Option<&BillTemplate>,
) -> Result<(Option<String>, Option<String>, Option<String>)> {
    let Some(t) = t else {
        return Ok((None, None, None));
    };
    let norm = |s: &Option<String>| s.clone().filter(|s| !s.trim().is_empty());
    let (a, d, c) = (norm(&t.amount), norm(&t.due), norm(&t.account));
    check_regex("bill amount pattern", &a)?;
    check_regex("bill due pattern", &d)?;
    check_regex("bill account pattern", &c)?;
    Ok((a, d, c))
}

fn org_view(o: &Versioned<Org>, contacts: Vec<Versioned<OrgContact>>) -> OrgView {
    let v = &o.value;
    let has_template = v.bill_amount_regex.is_some()
        || v.bill_due_regex.is_some()
        || v.bill_account_regex.is_some();
    OrgView {
        id: v.id,
        kind: v.kind,
        name: v.name.clone(),
        domains: v.domains.clone(),
        senders: v.senders.clone(),
        ics_calendar: v.ics_calendar_id,
        default_calendar: v.default_calendar_id,
        contact_mode: v.contact_mode,
        bill_template: has_template.then(|| BillTemplate {
            amount: v.bill_amount_regex.clone(),
            due: v.bill_due_regex.clone(),
            account: v.bill_account_regex.clone(),
        }),
        contacts: contacts.iter().map(contact_view).collect(),
        version: o.version,
    }
}

fn contact_view(c: &Versioned<OrgContact>) -> OrgContactView {
    OrgContactView {
        id: c.value.id,
        org: c.value.org_id,
        name: c.value.name.clone(),
        email: c.value.email.clone(),
        role: c.value.role,
        version: c.version,
    }
}

async fn enrollment_view(
    conn: &mut PgConnection,
    e: &Versioned<Enrollment>,
) -> Result<EnrollmentView> {
    let v = &e.value;
    Ok(EnrollmentView {
        id: v.id,
        member: v.member_id,
        org: v.org_id,
        grade: v.grade.clone(),
        class_name: v.class_name.clone(),
        from_date: v.from_date,
        until_date: v.until_date,
        contacts: EnrollmentRepo::contacts(conn, v.id).await?,
        version: e.version,
    })
}

fn rule_view(r: &Versioned<MailRule>) -> MailRuleView {
    let v = &r.value;
    MailRuleView {
        id: v.id,
        decision_point: v.decision_point,
        name: v.name.clone(),
        priority: v.priority,
        conditions: v.conditions.clone(),
        value: v.value.clone(),
        confidence: v.confidence,
        enabled: v.enabled,
        version: r.version,
    }
}

fn category_view(c: &Versioned<Category>) -> CategoryView {
    let v = &c.value;
    CategoryView {
        id: v.id,
        key: v.key.clone(),
        label: v.label.clone(),
        behaviour: v.behaviour,
        notify_time_sensitive: v.notify_time_sensitive,
        auto_actions: v.auto_actions.clone(),
        email_tasks: v.email_tasks,
        color: v.color.clone(),
        ord: v.ord,
        archived: v.archived_at.is_some(),
        version: c.version,
    }
}

fn dedupe<T: PartialEq + Clone>(v: &[T]) -> Vec<T> {
    let mut out: Vec<T> = Vec::new();
    for x in v {
        if !out.contains(x) {
            out.push(x.clone());
        }
    }
    out
}

/// A rule's value must fit its decision point.
async fn check_rule(
    conn: &mut PgConnection,
    dp: DecisionPointKey,
    name: &str,
    conditions: &[RuleCondition],
    value: &serde_json::Value,
    confidence: f32,
) -> Result<()> {
    if name.trim().is_empty() {
        return Err(invalid("a rule needs a name"));
    }
    if !(0.0..=1.0).contains(&confidence) {
        return Err(invalid("confidence must be between 0 and 1"));
    }
    fa_rules::validate_rule(conditions).map_err(|e| invalid(e.to_string()))?;
    match dp {
        DecisionPointKey::EmailCategory => {
            let key = value
                .as_str()
                .ok_or_else(|| invalid("the value must be a category key"))?;
            let ok = CategoryRepo::get_by_key(conn, key)
                .await?
                .is_some_and(|c| c.value.archived_at.is_none());
            if !ok {
                return Err(invalid(format!("unknown category `{key}`")));
            }
        }
        DecisionPointKey::EmailTimeSensitive | DecisionPointKey::EmailIsBill => {
            if !value.is_boolean() {
                return Err(invalid("the value must be true or false"));
            }
        }
        DecisionPointKey::EmailMemberLink => {
            let ok = match value {
                serde_json::Value::String(_) => true,
                serde_json::Value::Array(a) => a.iter().all(|v| v.is_string()),
                _ => false,
            };
            if !ok {
                return Err(invalid("the value must be a member id or a list of them"));
            }
        }
        DecisionPointKey::EventCalendarRoute => {
            return Err(invalid("mail rules cannot decide calendar routing"));
        }
    }
    Ok(())
}

impl SetupService {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    // ---- organisations ----

    pub async fn list_orgs(&self, household: HouseholdId) -> Result<Vec<OrgView>> {
        let mut tx = self.store.tx_for(household).await?;
        let mut out = Vec::new();
        for o in OrgRepo::list(&mut tx, false).await? {
            let contacts = OrgContactRepo::list(&mut tx, o.value.id).await?;
            out.push(org_view(&o, contacts));
        }
        Ok(out)
    }

    pub async fn create_org(&self, household: HouseholdId, i: OrgInput) -> Result<OrgView> {
        let name = i.name.trim().to_owned();
        if name.is_empty() {
            return Err(invalid("an organisation needs a name"));
        }
        let (amount, due, account) = template_parts(i.bill_template.as_ref())?;
        let now = Utc::now();
        let org = Org {
            id: OrgId::new(),
            household_id: household,
            kind: i.kind,
            name,
            domains: clean_list(&i.domains, clean_domain),
            senders: clean_senders(&i.senders),
            ics_calendar_id: i.ics_calendar,
            default_calendar_id: i.default_calendar,
            contact_mode: i.contact_mode.unwrap_or(ContactMode::Suggest),
            bill_amount_regex: amount,
            bill_due_regex: due,
            bill_account_regex: account,
            archived_at: None,
            created_at: now,
            updated_at: now,
        };
        let mut tx = self.store.tx_for(household).await?;
        let created = OrgRepo::insert(&mut tx, &org).await?;
        tx.commit().await?;
        Ok(org_view(&created, Vec::new()))
    }

    pub async fn update_org(
        &self,
        household: HouseholdId,
        id: OrgId,
        p: OrgPatch,
        version: i32,
    ) -> Result<OrgView> {
        let mut tx = self.store.tx_for(household).await?;
        let mut o = OrgRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        if o.value.archived_at.is_some() {
            return Err(MailError::NotFound);
        }
        if let Some(k) = p.kind {
            o.value.kind = k;
        }
        if let Some(n) = p.name {
            let n = n.trim().to_owned();
            if n.is_empty() {
                return Err(invalid("an organisation needs a name"));
            }
            o.value.name = n;
        }
        if let Some(d) = p.domains {
            o.value.domains = clean_list(&d, clean_domain);
        }
        if let Some(s) = p.senders {
            o.value.senders = clean_senders(&s);
        }
        if let Some(c) = p.ics_calendar {
            o.value.ics_calendar_id = c;
        }
        if let Some(c) = p.default_calendar {
            o.value.default_calendar_id = c;
        }
        if let Some(m) = p.contact_mode {
            o.value.contact_mode = m;
        }
        if let Some(t) = p.bill_template {
            let (a, d, c) = template_parts(t.as_ref())?;
            o.value.bill_amount_regex = a;
            o.value.bill_due_regex = d;
            o.value.bill_account_regex = c;
        }
        OrgRepo::update(&mut tx, &o.value, version).await?;
        let o = OrgRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        let contacts = OrgContactRepo::list(&mut tx, id).await?;
        tx.commit().await?;
        Ok(org_view(&o, contacts))
    }

    /// Archives the organisation (its mail keeps its links).
    pub async fn delete_org(&self, household: HouseholdId, id: OrgId) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        OrgRepo::archive(&mut tx, id).await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- contacts ----

    pub async fn add_contact(
        &self,
        household: HouseholdId,
        org: OrgId,
        i: OrgContactInput,
    ) -> Result<OrgContactView> {
        let email = clean_email(&i.email)?;
        let mut tx = self.store.tx_for(household).await?;
        OrgRepo::get(&mut tx, org)
            .await?
            .ok_or(MailError::NotFound)?;
        let now = Utc::now();
        let c = OrgContact {
            id: OrgContactId::new(),
            household_id: household,
            org_id: org,
            name: i.name.trim().to_owned(),
            email,
            role: i.role.unwrap_or_default(),
            archived_at: None,
            created_at: now,
            updated_at: now,
        };
        let id = OrgContactRepo::insert_if_new(&mut tx, &c)
            .await?
            .ok_or(MailError::Conflict)?;
        let created = OrgContactRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        tx.commit().await?;
        Ok(contact_view(&created))
    }

    pub async fn update_contact(
        &self,
        household: HouseholdId,
        id: OrgContactId,
        p: OrgContactPatch,
        version: i32,
    ) -> Result<OrgContactView> {
        let mut tx = self.store.tx_for(household).await?;
        let mut c = OrgContactRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        if let Some(n) = p.name {
            c.value.name = n.trim().to_owned();
        }
        if let Some(e) = p.email {
            c.value.email = clean_email(&e)?;
        }
        if let Some(r) = p.role {
            c.value.role = r;
        }
        OrgContactRepo::update(&mut tx, &c.value, version).await?;
        let c = OrgContactRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        tx.commit().await?;
        Ok(contact_view(&c))
    }

    pub async fn delete_contact(&self, household: HouseholdId, id: OrgContactId) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        OrgContactRepo::archive(&mut tx, id).await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- enrollments ----

    pub async fn list_enrollments(
        &self,
        household: HouseholdId,
        member: Option<MemberId>,
    ) -> Result<Vec<EnrollmentView>> {
        let mut tx = self.store.tx_for(household).await?;
        let mut out = Vec::new();
        for e in EnrollmentRepo::list(&mut tx, member).await? {
            out.push(enrollment_view(&mut tx, &e).await?);
        }
        Ok(out)
    }

    async fn check_enrollment(
        conn: &mut PgConnection,
        org: OrgId,
        contacts: &[OrgContactId],
        from: Option<chrono::NaiveDate>,
        until: Option<chrono::NaiveDate>,
    ) -> Result<()> {
        if let (Some(f), Some(u)) = (from, until)
            && f > u
        {
            return Err(invalid("`from` must not be after `until`"));
        }
        OrgRepo::get(conn, org).await?.ok_or(MailError::NotFound)?;
        for c in contacts {
            let ok = OrgContactRepo::get(conn, *c)
                .await?
                .is_some_and(|c| c.value.org_id == org);
            if !ok {
                return Err(invalid("a contact does not belong to the organisation"));
            }
        }
        Ok(())
    }

    pub async fn create_enrollment(
        &self,
        household: HouseholdId,
        member: MemberId,
        i: EnrollmentInput,
    ) -> Result<EnrollmentView> {
        let mut tx = self.store.tx_for(household).await?;
        MemberRepo::get(&mut tx, member)
            .await?
            .ok_or(MailError::NotFound)?;
        Self::check_enrollment(&mut tx, i.org, &i.contacts, i.from_date, i.until_date).await?;
        let now = Utc::now();
        let e = Enrollment {
            id: EnrollmentId::new(),
            household_id: household,
            member_id: member,
            org_id: i.org,
            grade: i.grade.filter(|g| !g.trim().is_empty()),
            class_name: i.class_name.filter(|g| !g.trim().is_empty()),
            from_date: i.from_date,
            until_date: i.until_date,
            created_at: now,
            updated_at: now,
        };
        let created = EnrollmentRepo::insert(&mut tx, &e).await?;
        EnrollmentRepo::set_contacts(&mut tx, e.id, &dedupe(&i.contacts)).await?;
        let v = enrollment_view(&mut tx, &created).await?;
        tx.commit().await?;
        Ok(v)
    }

    pub async fn update_enrollment(
        &self,
        household: HouseholdId,
        id: EnrollmentId,
        p: EnrollmentPatch,
        version: i32,
    ) -> Result<EnrollmentView> {
        let mut tx = self.store.tx_for(household).await?;
        let mut e = EnrollmentRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        if let Some(o) = p.org {
            e.value.org_id = o;
        }
        if let Some(g) = p.grade {
            e.value.grade = g.filter(|g| !g.trim().is_empty());
        }
        if let Some(c) = p.class_name {
            e.value.class_name = c.filter(|g| !g.trim().is_empty());
        }
        if let Some(f) = p.from_date {
            e.value.from_date = f;
        }
        if let Some(u) = p.until_date {
            e.value.until_date = u;
        }
        let contacts = match &p.contacts {
            Some(c) => dedupe(c),
            None if p.org.is_some() => Vec::new(),
            None => EnrollmentRepo::contacts(&mut tx, id).await?,
        };
        Self::check_enrollment(
            &mut tx,
            e.value.org_id,
            &contacts,
            e.value.from_date,
            e.value.until_date,
        )
        .await?;
        EnrollmentRepo::update(&mut tx, &e.value, version).await?;
        if p.contacts.is_some() || p.org.is_some() {
            EnrollmentRepo::set_contacts(&mut tx, id, &contacts).await?;
        }
        let e = EnrollmentRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        let v = enrollment_view(&mut tx, &e).await?;
        tx.commit().await?;
        Ok(v)
    }

    pub async fn delete_enrollment(&self, household: HouseholdId, id: EnrollmentId) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        EnrollmentRepo::delete(&mut tx, id).await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- rules ----

    pub async fn list_rules(&self, household: HouseholdId) -> Result<Vec<MailRuleView>> {
        let mut tx = self.store.tx_for(household).await?;
        Ok(MailRuleRepo::list(&mut tx, None, false)
            .await?
            .iter()
            .map(rule_view)
            .collect())
    }

    pub async fn create_rule(
        &self,
        household: HouseholdId,
        i: MailRuleInput,
    ) -> Result<MailRuleView> {
        let mut tx = self.store.tx_for(household).await?;
        mail_defaults::ensure(&mut tx, household).await?;
        let confidence = i.confidence.unwrap_or(DEFAULT_RULE_CONFIDENCE);
        check_rule(
            &mut tx,
            i.decision_point,
            &i.name,
            &i.conditions,
            &i.value,
            confidence,
        )
        .await?;
        let now = Utc::now();
        let rule = MailRule {
            id: MailRuleId::new(),
            household_id: household,
            decision_point: i.decision_point,
            name: i.name.trim().to_owned(),
            priority: i.priority.unwrap_or(DEFAULT_RULE_PRIORITY),
            conditions: i.conditions,
            value: i.value,
            confidence,
            enabled: i.enabled.unwrap_or(true),
            archived_at: None,
            created_at: now,
            updated_at: now,
        };
        let created = MailRuleRepo::insert(&mut tx, &rule).await?;
        tx.commit().await?;
        Ok(rule_view(&created))
    }

    pub async fn update_rule(
        &self,
        household: HouseholdId,
        id: MailRuleId,
        p: MailRulePatch,
        version: i32,
    ) -> Result<MailRuleView> {
        let mut tx = self.store.tx_for(household).await?;
        let mut r = MailRuleRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        if let Some(n) = p.name {
            r.value.name = n;
        }
        if let Some(x) = p.priority {
            r.value.priority = x;
        }
        if let Some(c) = p.conditions {
            r.value.conditions = c;
        }
        if let Some(v) = p.value {
            r.value.value = v;
        }
        if let Some(c) = p.confidence {
            r.value.confidence = c;
        }
        if let Some(e) = p.enabled {
            r.value.enabled = e;
        }
        check_rule(
            &mut tx,
            r.value.decision_point,
            &r.value.name,
            &r.value.conditions,
            &r.value.value,
            r.value.confidence,
        )
        .await?;
        MailRuleRepo::update(&mut tx, &r.value, version).await?;
        let r = MailRuleRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        tx.commit().await?;
        Ok(rule_view(&r))
    }

    pub async fn delete_rule(&self, household: HouseholdId, id: MailRuleId) -> Result<()> {
        let mut tx = self.store.tx_for(household).await?;
        MailRuleRepo::archive(&mut tx, id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Runs one rule (alone, without the built-in signals) over the last N readable messages that
    /// still have content.
    pub async fn test_rule(
        &self,
        household: HouseholdId,
        viewer: Viewer,
        req: RuleTestRequest,
    ) -> Result<RuleTestResult> {
        let i = req.rule;
        let confidence = i.confidence.unwrap_or(DEFAULT_RULE_CONFIDENCE);
        let mut tx = self.store.tx_for(household).await?;
        mail_defaults::ensure(&mut tx, household).await?;
        check_rule(
            &mut tx,
            i.decision_point,
            &i.name,
            &i.conditions,
            &i.value,
            confidence,
        )
        .await?;
        let sources = readable_sources(&mut tx, viewer).await?;
        let limit = req
            .limit
            .unwrap_or(TEST_DEFAULT_LIMIT)
            .clamp(1, TEST_MAX_LIMIT);
        let candidates = MessageRepo::list(
            &mut tx,
            &MessageFilter {
                sources: &sources,
                limit: i64::from(limit),
                ..Default::default()
            },
        )
        .await?;
        let mut pc = context::load(&mut tx, household, None).await?;
        let now = Utc::now();
        pc.rules.rules = vec![MailRule {
            id: MailRuleId::new(),
            household_id: household,
            decision_point: i.decision_point,
            name: i.name,
            priority: 0,
            conditions: i.conditions,
            value: i.value,
            confidence,
            enabled: true,
            archived_at: None,
            created_at: now,
            updated_at: now,
        }];
        pc.rules.disabled_signals = fa_rules::signals()
            .iter()
            .map(|s| s.key.to_owned())
            .collect();
        let mut matched = Vec::new();
        let mut scanned = 0;
        for c in candidates {
            let Some(m) = MessageRepo::get_with_content(&mut tx, c.id).await? else {
                continue;
            };
            if m.content_purged_at.is_some() {
                continue;
            }
            scanned += 1;
            let of = OwnedFacts::new(&m);
            if !fa_rules::evaluate(i.decision_point, &of.facts(), &pc.rules).is_empty() {
                matched.push(RuleTestMatch {
                    message: m.id,
                    subject: m.subject,
                    from_addr: m.from_addr,
                    received_at: m.received_at,
                });
            }
        }
        Ok(RuleTestResult { matched, scanned })
    }

    // ---- signals ----

    fn signal_views(disabled: &[String]) -> Vec<SignalView> {
        fa_rules::signals()
            .iter()
            .map(|s| SignalView {
                key: s.key.to_owned(),
                decision_point: s.decision_point,
                description: s.description.to_owned(),
                confidence: s.confidence,
                enabled: !disabled.iter().any(|d| d == s.key),
            })
            .collect()
    }

    pub async fn list_signals(&self, household: HouseholdId) -> Result<Vec<SignalView>> {
        let mut tx = self.store.tx_for(household).await?;
        let disabled = HouseholdRepo::disabled_signals(&mut tx, household).await?;
        Ok(Self::signal_views(&disabled))
    }

    pub async fn set_signal(
        &self,
        household: HouseholdId,
        key: &str,
        enabled: bool,
    ) -> Result<SignalView> {
        if !fa_rules::signals().iter().any(|s| s.key == key) {
            return Err(MailError::NotFound);
        }
        let mut tx = self.store.tx_for(household).await?;
        let mut disabled = HouseholdRepo::disabled_signals(&mut tx, household).await?;
        disabled.retain(|d| d != key);
        if !enabled {
            disabled.push(key.to_owned());
        }
        HouseholdRepo::set_disabled_signals(&mut tx, household, &disabled).await?;
        tx.commit().await?;
        Self::signal_views(&disabled)
            .into_iter()
            .find(|s| s.key == key)
            .ok_or(MailError::NotFound)
    }

    // ---- categories ----

    pub async fn list_categories(
        &self,
        household: HouseholdId,
        include_archived: bool,
    ) -> Result<Vec<CategoryView>> {
        let mut tx = self.store.tx_for(household).await?;
        mail_defaults::ensure(&mut tx, household).await?;
        let out = CategoryRepo::list(&mut tx, include_archived)
            .await?
            .iter()
            .map(category_view)
            .collect();
        tx.commit().await?;
        Ok(out)
    }

    pub async fn create_category(
        &self,
        household: HouseholdId,
        i: CategoryInput,
    ) -> Result<CategoryView> {
        if !valid_category_key(&i.key) {
            return Err(invalid(
                "the key must start with a letter and use lower-case letters, digits and _ (32 at most)",
            ));
        }
        if i.label.trim().is_empty() {
            return Err(invalid("a category needs a label"));
        }
        let mut tx = self.store.tx_for(household).await?;
        mail_defaults::ensure(&mut tx, household).await?;
        if CategoryRepo::get_by_key(&mut tx, &i.key).await?.is_some() {
            return Err(MailError::Conflict);
        }
        let ord = match i.ord {
            Some(o) => o,
            None => CategoryRepo::list(&mut tx, true)
                .await?
                .iter()
                .map(|c| c.value.ord)
                .max()
                .map_or(0, |m| m + 1),
        };
        let now = Utc::now();
        let cat = Category {
            id: CategoryId::new(),
            household_id: household,
            key: i.key,
            label: i.label.trim().to_owned(),
            behaviour: i.behaviour,
            notify_time_sensitive: i.notify_time_sensitive,
            auto_actions: dedupe(
                &i.auto_actions
                    .unwrap_or_else(|| vec![fa_domain::AutoAction::Label]),
            ),
            email_tasks: i.email_tasks.unwrap_or_default(),
            color: i.color.filter(|c| !c.trim().is_empty()),
            ord,
            archived_at: None,
            created_at: now,
            updated_at: now,
        };
        let created = CategoryRepo::insert(&mut tx, &cat).await?;
        tx.commit().await?;
        Ok(category_view(&created))
    }

    /// The key is immutable. Archiving is allowed except for `other`, the default category.
    pub async fn update_category(
        &self,
        household: HouseholdId,
        id: CategoryId,
        p: CategoryPatch,
        version: i32,
    ) -> Result<CategoryView> {
        let mut tx = self.store.tx_for(household).await?;
        let mut c = CategoryRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        if let Some(l) = p.label {
            if l.trim().is_empty() {
                return Err(invalid("a category needs a label"));
            }
            c.value.label = l.trim().to_owned();
        }
        if let Some(b) = p.behaviour {
            c.value.behaviour = b;
        }
        if let Some(n) = p.notify_time_sensitive {
            c.value.notify_time_sensitive = n;
        }
        if let Some(a) = p.auto_actions {
            c.value.auto_actions = dedupe(&a);
        }
        if let Some(t) = p.email_tasks {
            c.value.email_tasks = t;
        }
        if let Some(col) = p.color {
            c.value.color = col.filter(|c| !c.trim().is_empty());
        }
        if let Some(o) = p.ord {
            c.value.ord = o;
        }
        match p.archived {
            Some(true) => {
                if c.value.key == "other" {
                    return Err(invalid("the `other` category is the default and stays"));
                }
                c.value.archived_at = c.value.archived_at.or(Some(Utc::now()));
            }
            Some(false) => c.value.archived_at = None,
            None => {}
        }
        CategoryRepo::update(&mut tx, &c.value, version).await?;
        let c = CategoryRepo::get(&mut tx, id)
            .await?
            .ok_or(MailError::NotFound)?;
        tx.commit().await?;
        Ok(category_view(&c))
    }

    // ---- messages ----

    /// Queues a task for the message (`tasks.from_mail`, mode `create`). The assignees default to
    /// the viewer. `input.list` is not part of the job payload and is ignored.
    pub async fn make_task(
        &self,
        household: HouseholdId,
        viewer: Viewer,
        message: MessageId,
        input: fa_domain::MakeTaskInput,
    ) -> Result<()> {
        let member = viewer.adult_member().ok_or(MailError::Forbidden)?;
        let mut tx = self.store.tx_for(household).await?;
        let sources = readable_sources(&mut tx, viewer).await?;
        let msg = MessageRepo::get(&mut tx, message)
            .await?
            .filter(|m| sources.contains(&m.source_id))
            .ok_or(MailError::NotFound)?;
        let source = MailSourceRepo::get(&mut tx, msg.source_id)
            .await?
            .map(|s| s.value);
        let title = input
            .title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| {
                if msg.subject.trim().is_empty() {
                    "Follow up".to_owned()
                } else {
                    msg.subject.trim().to_owned()
                }
            });
        let assignees = input
            .assignees
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| vec![member]);
        let payload = serde_json::json!({
            "household": household,
            "message": message,
            "kind": "manual",
            "title": title,
            "due_at": input.due_at,
            "assignees": assignees,
            "mode": "create",
            "external_url": source.as_ref().and_then(|s| views::external_url(s, &msg.remote_id)),
        });
        enqueue(
            &mut tx,
            household,
            "tasks.from_mail",
            payload,
            format!("tasks.from_mail:{message}:{}", short_hash(&title)),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Messages of the sources the viewer may read, newest first. Categories with behaviour
    /// `ignore` are left out unless `include_ignored`.
    pub async fn list_messages(
        &self,
        household: HouseholdId,
        viewer: Viewer,
        q: MessageQuery,
    ) -> Result<MessagePage> {
        let mut tx = self.store.tx_for(household).await?;
        let mut sources = readable_sources(&mut tx, viewer).await?;
        if let Some(s) = q.source {
            sources.retain(|x| *x == s);
        }
        let limit = i64::from(q.limit.unwrap_or(50).clamp(1, 100));
        let ignored: Vec<String> = if q.include_ignored.unwrap_or(false) {
            Vec::new()
        } else {
            mail_defaults::ensure(&mut tx, household).await?;
            CategoryRepo::list(&mut tx, true)
                .await?
                .into_iter()
                .filter(|c| c.value.behaviour == CategoryBehaviour::Ignore)
                .map(|c| c.value.key)
                .collect()
        };
        let hide = (!q.include_ignored.unwrap_or(false)).then_some(ignored.as_slice());
        let mut rows = MessageRepo::list(
            &mut tx,
            &MessageFilter {
                sources: &sources,
                category: q.category.as_deref(),
                member: q.member,
                state: q.state,
                q: q.q.as_deref().filter(|s| !s.trim().is_empty()),
                include_ignored_keys: hide,
                before: q.before,
                limit: limit + 1,
            },
        )
        .await?;
        let more = rows.len() as i64 > limit;
        rows.truncate(limit as usize);
        let next_before = more.then(|| rows.last().map(|m| m.received_at)).flatten();
        let ids: Vec<MessageId> = rows.iter().map(|m| m.id).collect();
        let mut members = MessageRepo::members_of(&mut tx, &ids).await?;
        let by_source = sources_by_id(&mut tx).await?;
        let items: Vec<MessageSummary> = rows
            .iter()
            .map(|m| {
                views::summary(
                    m,
                    members.remove(&m.id).unwrap_or_default(),
                    by_source.get(&m.source_id),
                )
            })
            .collect();
        Ok(MessagePage { items, next_before })
    }

    /// One message with sections (relevance for `member`), decisions and derived items.
    pub async fn message_detail(
        &self,
        household: HouseholdId,
        viewer: Viewer,
        id: MessageId,
        member: Option<MemberId>,
    ) -> Result<MessageDetail> {
        let mut tx = self.store.tx_for(household).await?;
        let sources = readable_sources(&mut tx, viewer).await?;
        let msg = MessageRepo::get(&mut tx, id)
            .await?
            .filter(|m| sources.contains(&m.source_id))
            .ok_or(MailError::NotFound)?;
        let can_act = MailSourceRepo::get(&mut tx, msg.source_id)
            .await?
            .is_some_and(|s| !s.value.read_only);
        views::detail(&mut tx, household, id, member, can_act).await
    }
}

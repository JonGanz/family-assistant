use std::sync::Arc;

use chrono::NaiveTime;
use fa_domain::{HouseholdId, MemberId, NotifyPrefPatch, NotifyPrefView, NotifyTrigger, Role};
use fa_store::{MemberRepo, NotifyPref, NotifyPrefRepo, Store, Versioned};
use rand::Rng;
use uuid::Uuid;

use crate::{Notifier, NotifyError, NtfyMessage, NtfyNotifier};

/// Process settings handed in by `fa-server`.
#[derive(Debug, Clone)]
pub struct NotifyConfig {
    /// `FA_NTFY_URL`; no notifications without it.
    pub ntfy_url: Option<String>,
    /// `FA_NTFY_TOKEN`.
    pub ntfy_token: Option<String>,
    /// `FA_PUBLIC_BASE_URL`, the prefix of every click link.
    pub public_base_url: String,
}

#[derive(Clone)]
pub struct NotifyService {
    pub(crate) store: Store,
    pub(crate) config: Arc<NotifyConfig>,
    pub(crate) notifier: Arc<dyn Notifier>,
}

struct Unconfigured;

#[async_trait::async_trait]
impl Notifier for Unconfigured {
    async fn publish(&self, _: &str, _: &NtfyMessage) -> Result<(), NotifyError> {
        Err(NotifyError::NotConfigured)
    }
}

const DEFAULT_TRIGGERS: [NotifyTrigger; 3] = [
    NotifyTrigger::Assigned,
    NotifyTrigger::Overdue,
    NotifyTrigger::Digest,
];

fn new_topic() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..24)
        .map(|_| char::from(CHARS[rng.random_range(0..CHARS.len())]))
        .collect()
}

impl NotifyService {
    pub fn new(store: Store, config: NotifyConfig) -> Self {
        let notifier: Arc<dyn Notifier> = match &config.ntfy_url {
            Some(url) => match NtfyNotifier::new(url.clone(), config.ntfy_token.clone()) {
                Ok(n) => Arc::new(n),
                Err(_) => Arc::new(Unconfigured),
            },
            None => Arc::new(Unconfigured),
        };
        Self::with_notifier(store, config, notifier)
    }

    pub fn with_notifier(store: Store, config: NotifyConfig, notifier: Arc<dyn Notifier>) -> Self {
        Self {
            store,
            config: Arc::new(config),
            notifier,
        }
    }

    pub(crate) fn configured(&self) -> bool {
        self.config.ntfy_url.is_some()
    }

    pub(crate) fn click_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.config.public_base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    fn view(&self, p: &Versioned<NotifyPref>) -> NotifyPrefView {
        let v = &p.value;
        NotifyPrefView {
            member: v.member_id,
            enabled_triggers: v.triggers.iter().filter_map(|t| t.parse().ok()).collect(),
            due_soon_minutes: v.due_soon_minutes,
            nudge_minutes: v.nudge_minutes,
            quiet_from: v.quiet_from,
            quiet_until: v.quiet_until,
            copy_to_member: v.copy_to_member_id,
            topic: Some(v.topic.clone()),
            subscribe_url: self
                .config
                .ntfy_url
                .as_ref()
                .map(|u| format!("{}/{}", u.trim_end_matches('/'), v.topic)),
            server_configured: self.configured(),
            version: p.version,
        }
    }

    async fn ensure(
        &self,
        conn: &mut sqlx::PgConnection,
        household: HouseholdId,
        member: MemberId,
    ) -> Result<Versioned<NotifyPref>, NotifyError> {
        if MemberRepo::get(conn, member).await?.is_none() {
            return Err(NotifyError::NotFound);
        }
        if let Some(p) = NotifyPrefRepo::get_for_member(conn, member).await? {
            return Ok(p);
        }
        let pref = NotifyPref {
            id: Uuid::new_v4(),
            household_id: household,
            member_id: member,
            channel: "ntfy".into(),
            topic: new_topic(),
            triggers: DEFAULT_TRIGGERS.iter().map(|t| t.as_str().into()).collect(),
            due_soon_minutes: 120,
            nudge_minutes: 10,
            quiet_from: None,
            quiet_until: None,
            copy_to_member_id: None,
            archived_at: None,
        };
        Ok(NotifyPrefRepo::insert(conn, &pref).await?)
    }

    /// Creates the defaults and a topic when the member has no preferences yet.
    pub async fn get_prefs(
        &self,
        household: HouseholdId,
        member: MemberId,
    ) -> Result<NotifyPrefView, NotifyError> {
        let mut tx = self.store.tx_for(household).await?;
        let p = self.ensure(&mut tx, household, member).await?;
        tx.commit().await?;
        Ok(self.view(&p))
    }

    pub async fn put_prefs(
        &self,
        household: HouseholdId,
        member: MemberId,
        patch: NotifyPrefPatch,
        expected_version: i32,
    ) -> Result<NotifyPrefView, NotifyError> {
        let mut tx = self.store.tx_for(household).await?;
        let cur = self.ensure(&mut tx, household, member).await?;
        let mut p = cur.value;
        if let Some(t) = patch.enabled_triggers {
            let mut names: Vec<String> = Vec::new();
            for t in t {
                if !names.iter().any(|n| n == t.as_str()) {
                    names.push(t.as_str().into());
                }
            }
            p.triggers = names;
        }
        if let Some(m) = patch.due_soon_minutes {
            if !(5..=1440).contains(&m) {
                return Err(NotifyError::Invalid(
                    "due_soon_minutes must be 5..=1440".into(),
                ));
            }
            p.due_soon_minutes = m;
        }
        if let Some(m) = patch.nudge_minutes {
            if !(1..=120).contains(&m) {
                return Err(NotifyError::Invalid("nudge_minutes must be 1..=120".into()));
            }
            p.nudge_minutes = m;
        }
        if let Some(t) = patch.quiet_from {
            p.quiet_from = t;
        }
        if let Some(t) = patch.quiet_until {
            p.quiet_until = t;
        }
        let set = |t: Option<NaiveTime>| t.is_some();
        if set(p.quiet_from) != set(p.quiet_until) {
            return Err(NotifyError::Invalid(
                "quiet_from and quiet_until must be set together".into(),
            ));
        }
        if let Some(c) = patch.copy_to_member {
            if let Some(c) = c {
                let ok = MemberRepo::get(&mut tx, c)
                    .await?
                    .is_some_and(|m| m.value.role == Role::Adult && m.value.archived_at.is_none());
                if !ok || c == member {
                    return Err(NotifyError::Invalid(
                        "copy_to_member must be another adult of the household".into(),
                    ));
                }
            }
            p.copy_to_member_id = c;
        }
        let saved = NotifyPrefRepo::upsert_for_member(&mut tx, &p, Some(expected_version)).await?;
        tx.commit().await?;
        Ok(self.view(&saved))
    }

    /// Publishes immediately, bypassing the outbox and quiet hours.
    pub async fn send_test(
        &self,
        household: HouseholdId,
        member: MemberId,
    ) -> Result<(), NotifyError> {
        if !self.configured() {
            return Err(NotifyError::NotConfigured);
        }
        let mut tx = self.store.tx_for(household).await?;
        let p = self.ensure(&mut tx, household, member).await?;
        tx.commit().await?;
        let msg = NtfyMessage {
            title: "Test notification".into(),
            body: "Notifications from Family Assistant are working.".into(),
            priority: 3,
            tags: vec!["white_check_mark".into()],
            click: Some(self.click_url("")),
        };
        self.notifier.publish(&p.value.topic, &msg).await
    }
}

//! Pure authorization model implementing the RBAC matrix in arch/Domain.md (D-006).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::entities::Role;
use crate::ids::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    User {
        member_id: MemberId,
        role: Role,
        household_id: HouseholdId,
    },
    Device {
        device_id: DeviceId,
        household_id: HouseholdId,
    },
    ElevatedDevice {
        device_id: DeviceId,
        household_id: HouseholdId,
        adult: MemberId,
    },
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    ReadMemberPublic,
    ReadMemberFull,
    ManageMember,
    ManageChildAccount,
    SetOwnPin,
    ManageDevices,
    Elevate,
    ReadOwnData,
    ReadAllData,
    /// The "dashboard subset" of household data (devices, with or without PIN).
    ReadDashboard,
    RevokeChildSession,
    ManageHouseholdSettings,
    EditOtherAdultSettings,
    // M2 / M3
    CompleteTask,
    AddGroceryItem,
    ApproveEvent,
    GmailAction,
}

impl Action {
    pub const ALL: [Action; 17] = [
        Action::ReadMemberPublic,
        Action::ReadMemberFull,
        Action::ManageMember,
        Action::ManageChildAccount,
        Action::SetOwnPin,
        Action::ManageDevices,
        Action::Elevate,
        Action::ReadOwnData,
        Action::ReadAllData,
        Action::ReadDashboard,
        Action::RevokeChildSession,
        Action::ManageHouseholdSettings,
        Action::EditOtherAdultSettings,
        Action::CompleteTask,
        Action::AddGroceryItem,
        Action::ApproveEvent,
        Action::GmailAction,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    Member(MemberId, Role),
    Household,
    Device(DeviceId),
    Session { owner: MemberId, owner_role: Role },
    Task { assignee: Option<MemberId> },
    GroceryItem,
    Event,
    MailSource { owner: MemberId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum ListCompletionPolicy {
    #[default]
    Assignee,
    AdultOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PolicyCtx {
    pub dashboard_completion_override: bool,
    pub list_completion_policy: ListCompletionPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("forbidden")]
pub struct Denied;

fn allow(ok: bool) -> Result<(), Denied> {
    if ok { Ok(()) } else { Err(Denied) }
}

/// Decide whether `actor` may perform `action` on `resource`. Deny by default.
pub fn authorize(
    actor: &Actor,
    action: Action,
    resource: &Resource,
    ctx: &PolicyCtx,
) -> Result<(), Denied> {
    if matches!(actor, Actor::System) {
        // Background jobs and imports; EditOtherAdultSettings is the one thing nobody does.
        return allow(true);
    }

    // The adult user, if the actor is one.
    let adult_user = match actor {
        Actor::User {
            member_id,
            role: Role::Adult,
            ..
        } => Some(*member_id),
        _ => None,
    };
    // The acting member for adults acting directly or via PIN elevation.
    let adult_member = adult_user.or(match actor {
        Actor::ElevatedDevice { adult, .. } => Some(*adult),
        _ => None,
    });
    let user_member = match actor {
        Actor::User { member_id, .. } => Some(*member_id),
        _ => None,
    };
    let is_device = matches!(actor, Actor::Device { .. } | Actor::ElevatedDevice { .. });

    match action {
        Action::ReadMemberPublic | Action::AddGroceryItem => allow(true),
        Action::ReadDashboard => allow(true),
        Action::ReadMemberFull => match resource {
            Resource::Member(id, _) => allow(adult_user.is_some() || user_member == Some(*id)),
            _ => allow(adult_user.is_some()),
        },
        Action::ReadOwnData => match resource {
            Resource::Member(id, _) => {
                allow(is_device || adult_user.is_some() || user_member == Some(*id))
            }
            _ => allow(is_device || user_member.is_some()),
        },
        Action::ReadAllData | Action::ManageHouseholdSettings => allow(adult_user.is_some()),
        Action::ManageMember => match (adult_user, resource) {
            // Adults manage children and themself, never another adult.
            (Some(me), Resource::Member(id, role)) => allow(*role == Role::Child || *id == me),
            (Some(_), Resource::Household) => allow(true),
            _ => Err(Denied),
        },
        Action::ManageChildAccount => match (adult_user, resource) {
            (Some(_), Resource::Member(_, Role::Child)) => Ok(()),
            _ => Err(Denied),
        },
        Action::SetOwnPin => match (adult_user, resource) {
            (Some(me), Resource::Member(id, Role::Adult)) => allow(me == *id),
            _ => Err(Denied),
        },
        Action::ManageDevices => allow(adult_user.is_some()),
        Action::Elevate => match (actor, resource) {
            (
                Actor::Device { .. } | Actor::ElevatedDevice { .. },
                Resource::Member(_, Role::Adult),
            ) => Ok(()),
            _ => Err(Denied),
        },
        Action::RevokeChildSession => match (adult_user, resource) {
            (
                Some(_),
                Resource::Session {
                    owner_role: Role::Child,
                    ..
                },
            ) => Ok(()),
            _ => Err(Denied),
        },
        Action::EditOtherAdultSettings => Err(Denied),
        Action::CompleteTask => {
            if adult_member.is_some() {
                return Ok(());
            }
            match actor {
                Actor::User {
                    member_id,
                    role: Role::Child,
                    ..
                } => match resource {
                    Resource::Task { assignee: Some(a) } => allow(
                        ctx.list_completion_policy == ListCompletionPolicy::Assignee
                            && a == member_id,
                    ),
                    _ => Err(Denied),
                },
                Actor::Device { .. } => allow(
                    ctx.dashboard_completion_override
                        && ctx.list_completion_policy == ListCompletionPolicy::Assignee,
                ),
                _ => Err(Denied),
            }
        }
        Action::ApproveEvent => allow(adult_member.is_some()),
        Action::GmailAction => match resource {
            // Another adult's personal sources are off limits.
            Resource::MailSource { owner } => allow(adult_member == Some(*owner)),
            _ => allow(adult_member.is_some()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    struct Fx {
        hh: HouseholdId,
        adult: MemberId,
        adult2: MemberId,
        child: MemberId,
        child2: MemberId,
        dev: DeviceId,
    }

    fn fx() -> Fx {
        Fx {
            hh: HouseholdId::new(),
            adult: MemberId::new(),
            adult2: MemberId::new(),
            child: MemberId::new(),
            child2: MemberId::new(),
            dev: DeviceId::new(),
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum A {
        Adult,
        Child,
        Device,
        Elevated,
        System,
    }

    fn actor(f: &Fx, a: A) -> Actor {
        match a {
            A::Adult => Actor::User {
                member_id: f.adult,
                role: Role::Adult,
                household_id: f.hh,
            },
            A::Child => Actor::User {
                member_id: f.child,
                role: Role::Child,
                household_id: f.hh,
            },
            A::Device => Actor::Device {
                device_id: f.dev,
                household_id: f.hh,
            },
            A::Elevated => Actor::ElevatedDevice {
                device_id: f.dev,
                household_id: f.hh,
                adult: f.adult,
            },
            A::System => Actor::System,
        }
    }

    use A::*;
    const ALLOW: bool = true;
    const DENY: bool = false;

    fn run(action: Action, resource: &Resource, ctx: &PolicyCtx, f: &Fx, expect: [(A, bool); 5]) {
        for (a, want) in expect {
            let got = authorize(&actor(f, a), action, resource, ctx).is_ok();
            assert_eq!(got, want, "{action:?} / {a:?} / {resource:?} / {ctx:?}");
        }
    }

    #[test]
    fn matrix() {
        let f = fx();
        let c = PolicyCtx::default();
        let mut covered: HashSet<Action> = HashSet::new();
        let mut t = |action, res: Resource, ctx: PolicyCtx, exp: [(A, bool); 5]| {
            covered.insert(action);
            run(action, &res, &ctx, &f, exp);
        };
        // Order: Adult, Child, Device, Elevated, System.
        let m_adult_self = Resource::Member(f.adult, Role::Adult);
        let m_adult_other = Resource::Member(f.adult2, Role::Adult);
        let m_child = Resource::Member(f.child, Role::Child);
        let m_child_other = Resource::Member(f.child2, Role::Child);

        for r in [&m_adult_self, &m_child] {
            t(
                Action::ReadMemberPublic,
                r.clone(),
                c,
                [
                    (Adult, ALLOW),
                    (Child, ALLOW),
                    (Device, ALLOW),
                    (Elevated, ALLOW),
                    (System, ALLOW),
                ],
            );
            t(
                Action::ReadDashboard,
                r.clone(),
                c,
                [
                    (Adult, ALLOW),
                    (Child, ALLOW),
                    (Device, ALLOW),
                    (Elevated, ALLOW),
                    (System, ALLOW),
                ],
            );
        }
        t(
            Action::ReadMemberFull,
            m_child_other.clone(),
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );
        t(
            Action::ReadMemberFull,
            m_child.clone(),
            c,
            [
                (Adult, ALLOW),
                (Child, ALLOW),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        t(
            Action::ManageMember,
            m_child.clone(),
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );
        t(
            Action::ManageMember,
            m_adult_self.clone(),
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );
        t(
            Action::ManageMember,
            Resource::Household,
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );
        // An adult cannot manage another adult.
        t(
            Action::ManageMember,
            m_adult_other.clone(),
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        t(
            Action::ManageChildAccount,
            m_child.clone(),
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );
        t(
            Action::ManageChildAccount,
            m_adult_other.clone(),
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        t(
            Action::SetOwnPin,
            m_adult_self.clone(),
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );
        t(
            Action::SetOwnPin,
            m_adult_other.clone(),
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );
        t(
            Action::SetOwnPin,
            m_child.clone(),
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        t(
            Action::ManageDevices,
            Resource::Device(f.dev),
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        t(
            Action::Elevate,
            m_adult_other.clone(),
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, ALLOW),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );
        t(
            Action::Elevate,
            m_child.clone(),
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        t(
            Action::ReadOwnData,
            m_child.clone(),
            c,
            [
                (Adult, ALLOW),
                (Child, ALLOW),
                (Device, ALLOW),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );
        t(
            Action::ReadOwnData,
            m_child_other.clone(),
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, ALLOW),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );

        t(
            Action::ReadAllData,
            Resource::Household,
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        let sess_child = Resource::Session {
            owner: f.child,
            owner_role: Role::Child,
        };
        let sess_adult = Resource::Session {
            owner: f.adult2,
            owner_role: Role::Adult,
        };
        t(
            Action::RevokeChildSession,
            sess_child,
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );
        t(
            Action::RevokeChildSession,
            sess_adult,
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        t(
            Action::ManageHouseholdSettings,
            Resource::Household,
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        // Nobody (except system imports) edits another adult's personal settings.
        t(
            Action::EditOtherAdultSettings,
            m_adult_other.clone(),
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        // CompleteTask: policy and override matrix.
        let own = Resource::Task {
            assignee: Some(f.child),
        };
        let theirs = Resource::Task {
            assignee: Some(f.child2),
        };
        let unassigned = Resource::Task { assignee: None };
        let ctx = |o, p| PolicyCtx {
            dashboard_completion_override: o,
            list_completion_policy: p,
        };
        use ListCompletionPolicy::*;
        t(
            Action::CompleteTask,
            own.clone(),
            ctx(false, Assignee),
            [
                (A::Adult, ALLOW),
                (A::Child, ALLOW),
                (A::Device, DENY),
                (A::Elevated, ALLOW),
                (A::System, ALLOW),
            ],
        );
        t(
            Action::CompleteTask,
            theirs,
            ctx(false, Assignee),
            [
                (A::Adult, ALLOW),
                (A::Child, DENY),
                (A::Device, DENY),
                (A::Elevated, ALLOW),
                (A::System, ALLOW),
            ],
        );
        t(
            Action::CompleteTask,
            unassigned,
            ctx(false, Assignee),
            [
                (A::Adult, ALLOW),
                (A::Child, DENY),
                (A::Device, DENY),
                (A::Elevated, ALLOW),
                (A::System, ALLOW),
            ],
        );
        t(
            Action::CompleteTask,
            own.clone(),
            ctx(true, Assignee),
            [
                (A::Adult, ALLOW),
                (A::Child, ALLOW),
                (A::Device, ALLOW),
                (A::Elevated, ALLOW),
                (A::System, ALLOW),
            ],
        );
        t(
            Action::CompleteTask,
            own.clone(),
            ctx(true, AdultOnly),
            [
                (A::Adult, ALLOW),
                (A::Child, DENY),
                (A::Device, DENY),
                (A::Elevated, ALLOW),
                (A::System, ALLOW),
            ],
        );
        t(
            Action::CompleteTask,
            own,
            ctx(false, AdultOnly),
            [
                (A::Adult, ALLOW),
                (A::Child, DENY),
                (A::Device, DENY),
                (A::Elevated, ALLOW),
                (A::System, ALLOW),
            ],
        );

        t(
            Action::AddGroceryItem,
            Resource::GroceryItem,
            c,
            [
                (Adult, ALLOW),
                (Child, ALLOW),
                (Device, ALLOW),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );
        t(
            Action::ApproveEvent,
            Resource::Event,
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );
        t(
            Action::GmailAction,
            Resource::Event,
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );
        // Elevated actor is `f.adult`; a source owned by the other adult is off limits.
        t(
            Action::GmailAction,
            Resource::MailSource { owner: f.adult },
            c,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, DENY),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );
        t(
            Action::GmailAction,
            Resource::MailSource { owner: f.adult2 },
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );

        let all: HashSet<Action> = Action::ALL.into_iter().collect();
        assert_eq!(covered, all, "every Action variant needs a policy test");
    }

    #[test]
    fn all_lists_every_variant() {
        // Compile-time exhaustiveness: adding a variant breaks this match.
        for a in Action::ALL {
            match a {
                Action::ReadMemberPublic
                | Action::ReadMemberFull
                | Action::ManageMember
                | Action::ManageChildAccount
                | Action::SetOwnPin
                | Action::ManageDevices
                | Action::Elevate
                | Action::ReadOwnData
                | Action::ReadAllData
                | Action::ReadDashboard
                | Action::RevokeChildSession
                | Action::ManageHouseholdSettings
                | Action::EditOtherAdultSettings
                | Action::CompleteTask
                | Action::AddGroceryItem
                | Action::ApproveEvent
                | Action::GmailAction => {}
            }
        }
        assert_eq!(Action::ALL.iter().collect::<HashSet<_>>().len(), 17);
    }
}

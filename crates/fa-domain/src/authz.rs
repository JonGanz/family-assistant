//! Pure authorization model implementing the RBAC matrix in arch/Domain.md (D-006).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::entities::{ChildCanAdd, Role};
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
    // M1
    ManageCalendarSource,
    ReadCalendar,
    ManageWeatherRules,
    DismissAdvisory,
    ManageDeviceSettings,
    // M2
    ManageTasks,
    AddTask,
    EditTask,
    ClaimTask,
    TickRoutineStep,
    ManageNotifyPrefs,
}

impl Action {
    pub const ALL: [Action; 28] = [
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
        Action::ManageCalendarSource,
        Action::ReadCalendar,
        Action::ManageWeatherRules,
        Action::DismissAdvisory,
        Action::ManageDeviceSettings,
        Action::ManageTasks,
        Action::AddTask,
        Action::EditTask,
        Action::ClaimTask,
        Action::TickRoutineStep,
        Action::ManageNotifyPrefs,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    Member(MemberId, Role),
    Household,
    Device(DeviceId),
    Session {
        owner: MemberId,
        owner_role: Role,
    },
    Task {
        assignees: Vec<MemberId>,
        created_by: Option<MemberId>,
        /// True when the task was materialised from a definition.
        definition: bool,
    },
    TaskList {
        child_can_add: ChildCanAdd,
        dashboard_add: bool,
    },
    GroceryItem,
    Event,
    MailSource {
        owner: MemberId,
    },
    /// A calendar source. `owner: None` is a household source; `owner_role` is the owner's role.
    Source {
        owner: Option<MemberId>,
        owner_role: Option<Role>,
    },
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    sqlx::Type,
    TS,
    utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[ts(export, export_to = "../../../apps/web/src/gen/types/")]
pub enum ListCompletionPolicy {
    #[default]
    Assignee,
    AdultOnly,
}

impl ListCompletionPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Assignee => "assignee",
            Self::AdultOnly => "adult_only",
        }
    }
}

impl std::str::FromStr for ListCompletionPolicy {
    type Err = crate::entities::ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "assignee" => Ok(Self::Assignee),
            "adult_only" => Ok(Self::AdultOnly),
            _ => Err(crate::entities::ParseEnumError {
                ty: "ListCompletionPolicy",
                value: s.to_string(),
            }),
        }
    }
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
                    Resource::Task { assignees, .. } => allow(
                        ctx.list_completion_policy == ListCompletionPolicy::Assignee
                            && assignees.contains(member_id),
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
        Action::ManageCalendarSource => match (adult_user, resource) {
            // Household sources, own and children's; never another adult's (D-006).
            (Some(me), Resource::Source { owner, owner_role }) => match owner {
                None => Ok(()),
                Some(o) if *o == me => Ok(()),
                Some(_) => allow(*owner_role == Some(Role::Child)),
            },
            _ => Err(Denied),
        },
        Action::ReadCalendar | Action::DismissAdvisory => allow(true),
        // D-019: no settings on the dashboard, not even elevated.
        Action::ManageWeatherRules | Action::ManageDeviceSettings => allow(adult_user.is_some()),
        // Settings-like (D-019): not on the dashboard, not even elevated.
        Action::ManageTasks => allow(adult_user.is_some()),
        Action::AddTask => match resource {
            Resource::TaskList {
                child_can_add,
                dashboard_add,
            } => {
                if adult_member.is_some() {
                    return Ok(());
                }
                match actor {
                    Actor::User {
                        role: Role::Child, ..
                    } => allow(*child_can_add == ChildCanAdd::Own),
                    Actor::Device { .. } => allow(*dashboard_add),
                    _ => Err(Denied),
                }
            }
            _ => Err(Denied),
        },
        Action::EditTask => match resource {
            Resource::Task {
                created_by,
                definition,
                ..
            } => match actor {
                _ if adult_member.is_some() => Ok(()),
                Actor::User {
                    member_id,
                    role: Role::Child,
                    ..
                } => allow(!*definition && *created_by == Some(*member_id)),
                _ => Err(Denied),
            },
            _ => Err(Denied),
        },
        Action::ClaimTask => match resource {
            Resource::Task { assignees, .. } => {
                allow(adult_member.is_some() || user_member.is_some_and(|m| assignees.contains(&m)))
            }
            _ => Err(Denied),
        },
        // The resource is the run's member.
        Action::TickRoutineStep => match resource {
            Resource::Member(id, _) => {
                if adult_member.is_some() {
                    return Ok(());
                }
                match actor {
                    Actor::User { member_id, .. } => allow(member_id == id),
                    Actor::Device { .. } => allow(ctx.dashboard_completion_override),
                    _ => Err(Denied),
                }
            }
            _ => Err(Denied),
        },
        Action::ManageNotifyPrefs => match resource {
            Resource::Member(id, role) => match actor {
                Actor::User {
                    member_id,
                    role: me_role,
                    ..
                } => allow(member_id == id || (*me_role == Role::Adult && *role == Role::Child)),
                _ => Err(Denied),
            },
            _ => Err(Denied),
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
        let task = |assignees: Vec<MemberId>, created_by, definition| Resource::Task {
            assignees,
            created_by,
            definition,
        };
        let own = task(vec![f.child], None, false);
        let theirs = task(vec![f.child2], None, false);
        let unassigned = task(vec![], None, false);
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

        let src = |owner, owner_role| Resource::Source { owner, owner_role };
        let adult_only = [
            (Adult, ALLOW),
            (Child, DENY),
            (Device, DENY),
            (Elevated, DENY),
            (System, ALLOW),
        ];
        t(Action::ManageCalendarSource, src(None, None), c, adult_only);
        t(
            Action::ManageCalendarSource,
            src(Some(f.adult), Some(Role::Adult)),
            c,
            adult_only,
        );
        t(
            Action::ManageCalendarSource,
            src(Some(f.child), Some(Role::Child)),
            c,
            adult_only,
        );
        t(
            Action::ManageCalendarSource,
            src(Some(f.adult2), Some(Role::Adult)),
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
            Action::ManageCalendarSource,
            src(Some(f.adult2), None),
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );
        for action in [Action::ReadCalendar, Action::DismissAdvisory] {
            t(
                action,
                Resource::Household,
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
        for action in [Action::ManageWeatherRules, Action::ManageDeviceSettings] {
            t(action, Resource::Household, c, adult_only);
        }

        // M2 actions.
        let adult_only = [
            (Adult, ALLOW),
            (Child, DENY),
            (Device, DENY),
            (Elevated, DENY),
            (System, ALLOW),
        ];
        t(Action::ManageTasks, Resource::Household, c, adult_only);
        let list = |child_can_add, dashboard_add| Resource::TaskList {
            child_can_add,
            dashboard_add,
        };
        t(
            Action::AddTask,
            list(ChildCanAdd::None, false),
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
            Action::AddTask,
            list(ChildCanAdd::Own, false),
            c,
            [
                (Adult, ALLOW),
                (Child, ALLOW),
                (Device, DENY),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );
        t(
            Action::AddTask,
            list(ChildCanAdd::None, true),
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
            Action::AddTask,
            Resource::Household,
            c,
            [
                (Adult, DENY),
                (Child, DENY),
                (Device, DENY),
                (Elevated, DENY),
                (System, ALLOW),
            ],
        );
        // EditTask: children only their own one-offs.
        for (res, child) in [
            (task(vec![], Some(f.child), false), ALLOW),
            (task(vec![f.child], Some(f.child), true), DENY),
            (task(vec![f.child], Some(f.adult), false), DENY),
            (task(vec![f.child], None, false), DENY),
        ] {
            t(
                Action::EditTask,
                res,
                c,
                [
                    (Adult, ALLOW),
                    (Child, child),
                    (Device, DENY),
                    (Elevated, ALLOW),
                    (System, ALLOW),
                ],
            );
        }
        // ClaimTask: an assignee or an adult.
        for (res, child) in [
            (task(vec![f.child, f.child2], None, false), ALLOW),
            (task(vec![f.child2], None, false), DENY),
            (task(vec![], None, false), DENY),
        ] {
            t(
                Action::ClaimTask,
                res,
                c,
                [
                    (Adult, ALLOW),
                    (Child, child),
                    (Device, DENY),
                    (Elevated, ALLOW),
                    (System, ALLOW),
                ],
            );
        }
        // TickRoutineStep: the run's member, adults, devices while the override is on.
        let on = PolicyCtx {
            dashboard_completion_override: true,
            ..PolicyCtx::default()
        };
        t(
            Action::TickRoutineStep,
            m_child.clone(),
            on,
            [
                (Adult, ALLOW),
                (Child, ALLOW),
                (Device, ALLOW),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );
        t(
            Action::TickRoutineStep,
            m_child.clone(),
            c,
            [
                (Adult, ALLOW),
                (Child, ALLOW),
                (Device, DENY),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );
        t(
            Action::TickRoutineStep,
            m_child_other.clone(),
            on,
            [
                (Adult, ALLOW),
                (Child, DENY),
                (Device, ALLOW),
                (Elevated, ALLOW),
                (System, ALLOW),
            ],
        );
        // ManageNotifyPrefs: self, or an adult for a child.
        t(
            Action::ManageNotifyPrefs,
            m_adult_self.clone(),
            c,
            adult_only,
        );
        t(
            Action::ManageNotifyPrefs,
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
            Action::ManageNotifyPrefs,
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
            Action::ManageNotifyPrefs,
            m_child_other.clone(),
            c,
            adult_only,
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
                | Action::GmailAction
                | Action::ManageCalendarSource
                | Action::ReadCalendar
                | Action::ManageWeatherRules
                | Action::DismissAdvisory
                | Action::ManageDeviceSettings
                | Action::ManageTasks
                | Action::AddTask
                | Action::EditTask
                | Action::ClaimTask
                | Action::TickRoutineStep
                | Action::ManageNotifyPrefs => {}
            }
        }
        assert_eq!(Action::ALL.iter().collect::<HashSet<_>>().len(), 28);
    }
}

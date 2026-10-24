//! Event visibility (arch/Feeds.md §3.5, D-036). Pure; shared by fa-calendar and fa-api.

use crate::api::EventView;
use crate::entities::{Role, Visibility};
use crate::ids::MemberId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventMeta {
    pub is_private: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CalendarMeta<'a> {
    /// Members linked to the calendar with their roles; empty for a household calendar.
    pub members: &'a [(MemberId, Role)],
    /// The source owner's role (`None` for a household source), used when `members` is empty.
    pub owner_role: Option<Role>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Viewer {
    AdultUser,
    ChildUser(MemberId),
    Device,
    ElevatedDevice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HouseholdFlags {
    pub private_events_as_busy: bool,
}

fn adult_owned(cal: &CalendarMeta<'_>) -> bool {
    if cal.members.is_empty() {
        cal.owner_role.is_none_or(|r| r == Role::Adult)
    } else {
        cal.members.iter().any(|(_, r)| *r == Role::Adult)
    }
}

/// Scope (which events a viewer gets at all) is applied in the query, not here.
pub fn view(
    event: &EventMeta,
    cal: &CalendarMeta<'_>,
    viewer: &Viewer,
    flags: &HouseholdFlags,
) -> Visibility {
    match viewer {
        Viewer::AdultUser => Visibility::Full,
        Viewer::ChildUser(_) | Viewer::Device | Viewer::ElevatedDevice => {
            if flags.private_events_as_busy && event.is_private && adult_owned(cal) {
                Visibility::Busy
            } else {
                Visibility::Full
            }
        }
    }
}

/// Blank title, location and description for `Busy` events; the time block and members stay.
pub fn mask(mut ev: EventView) -> EventView {
    if ev.visibility == Visibility::Busy {
        ev.title = "Busy".into();
        ev.location = None;
        ev.description = None;
        ev.external_url = None;
    }
    ev
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn truth_table() {
        let a = (MemberId::new(), Role::Adult);
        let c = (MemberId::new(), Role::Child);
        let adult_cal: &[(MemberId, Role)] = &[a];
        let child_cal: &[(MemberId, Role)] = &[c];
        let mixed: &[(MemberId, Role)] = &[c, a];
        let none: &[(MemberId, Role)] = &[];
        // (members, owner_role, adult_owned)
        type Case<'a> = (&'a [(MemberId, Role)], Option<Role>, bool);
        let cals: [Case; 7] = [
            (adult_cal, None, true),
            (child_cal, None, false),
            (mixed, None, true),
            (none, Some(Role::Adult), true),
            (none, Some(Role::Child), false),
            (none, None, true),
            // members win over the owner
            (child_cal, Some(Role::Adult), false),
        ];
        let viewers = [
            (Viewer::AdultUser, true),
            (Viewer::ChildUser(c.0), false),
            (Viewer::Device, false),
            (Viewer::ElevatedDevice, false),
        ];
        for (members, owner_role, owned) in cals {
            let cal = CalendarMeta {
                members,
                owner_role,
            };
            for (viewer, is_adult) in viewers {
                for private in [false, true] {
                    for flag in [false, true] {
                        let got = view(
                            &EventMeta {
                                is_private: private,
                            },
                            &cal,
                            &viewer,
                            &HouseholdFlags {
                                private_events_as_busy: flag,
                            },
                        );
                        let busy = !is_adult && flag && private && owned;
                        let want = if busy {
                            Visibility::Busy
                        } else {
                            Visibility::Full
                        };
                        assert_eq!(
                            got, want,
                            "{viewer:?} private={private} flag={flag} owned={owned} owner={owner_role:?}"
                        );
                    }
                }
            }
        }
    }

    fn ev(vis: Visibility) -> EventView {
        EventView {
            id: crate::ids::EventId::new(),
            calendar: None,
            title: "Dentist".into(),
            location: Some("Main St".into()),
            description: Some("notes".into()),
            start_at: Utc::now(),
            end_at: Utc::now(),
            all_day: false,
            start_date: None,
            end_date: None,
            members: vec![MemberId::new()],
            color: Some("#fff".into()),
            visibility: vis,
            external_url: Some("https://example.com/e".into()),
        }
    }

    #[test]
    fn mask_blanks_only_busy() {
        let full = ev(Visibility::Full);
        assert_eq!(mask(full.clone()), full);
        let m = mask(ev(Visibility::Busy));
        assert_eq!(m.title, "Busy");
        assert!(m.location.is_none() && m.description.is_none() && m.external_url.is_none());
        assert_eq!(m.members.len(), 1);
        assert_eq!(m.color.as_deref(), Some("#fff"));
    }
}

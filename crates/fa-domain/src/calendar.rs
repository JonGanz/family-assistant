//! Event visibility (arch/Feeds.md §3.5, D-036). Pure; shared by fa-calendar and fa-api.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};

use crate::api::EventView;
use crate::entities::{Role, Visibility};
use crate::ids::{CalendarId, MemberId};

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

fn norm(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Folds copies of one event from different calendars (same title, time and location, compared
/// after trimming, collapsing whitespace and ignoring case) into the first copy: the members are
/// the union. Call it on unmasked views, so two
/// different private events that both show as "Busy" never merge. Two events of one calendar, and
/// events without a calendar, are never merged.
pub fn merge_duplicates(events: Vec<EventView>) -> Vec<EventView> {
    type Key = (
        String,
        DateTime<Utc>,
        DateTime<Utc>,
        bool,
        Option<NaiveDate>,
        Option<NaiveDate>,
        String,
        Visibility,
    );
    struct Group {
        ev: EventView,
        cals: Vec<CalendarId>,
        copies: usize,
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut index: HashMap<Key, Vec<usize>> = HashMap::new();
    for ev in events {
        let key: Key = (
            norm(&ev.title),
            ev.start_at,
            ev.end_at,
            ev.all_day,
            ev.start_date,
            ev.end_date,
            norm(ev.location.as_deref().unwrap_or("")),
            ev.visibility,
        );
        let target = ev.calendar.and_then(|c| {
            index
                .get(&key)?
                .iter()
                .copied()
                .find(|&i| !groups[i].cals.contains(&c))
        });
        match target {
            Some(i) => {
                let g = &mut groups[i];
                g.cals.extend(ev.calendar);
                g.copies += 1;
                for m in ev.members {
                    if !g.ev.members.contains(&m) {
                        g.ev.members.push(m);
                    }
                }
            }
            None => {
                index.entry(key).or_default().push(groups.len());
                groups.push(Group {
                    cals: ev.calendar.into_iter().collect(),
                    copies: 1,
                    ev,
                });
            }
        }
    }
    groups
        .into_iter()
        .map(|g| {
            let mut ev = g.ev;
            ev.merged = g.copies > 1;
            ev
        })
        .collect()
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
            merged: false,
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
    }

    fn copy(cal: CalendarId, title: &str) -> EventView {
        let mut e = ev(Visibility::Full);
        e.title = title.into();
        e.calendar = Some(cal);
        e
    }

    #[test]
    fn merges_copies_from_different_calendars() {
        let mut a = copy(CalendarId::new(), "Soccer ");
        a.location = Some("Main St".into());
        let mut b = copy(CalendarId::new(), "soccer");
        b.start_at = a.start_at;
        b.end_at = a.end_at;
        b.location = Some("  main  st".into());
        let (ma, mb) = (a.members[0], b.members[0]);
        let out = merge_duplicates(vec![a, b]);
        assert_eq!(out.len(), 1);
        assert!(out[0].merged);
        assert_eq!(out[0].members, [ma, mb]);
    }

    #[test]
    fn keeps_events_that_differ_or_share_a_calendar() {
        let cal = CalendarId::new();
        let a = copy(cal, "Soccer");
        let mut same_cal = a.clone();
        same_cal.id = crate::ids::EventId::new();
        let mut other_time = copy(CalendarId::new(), "Soccer");
        other_time.start_at = a.start_at;
        other_time.end_at = a.end_at + chrono::Duration::hours(1);
        let mut no_cal = a.clone();
        no_cal.calendar = None;
        assert_eq!(
            merge_duplicates(vec![a, same_cal, other_time, no_cal]).len(),
            4
        );
    }
}

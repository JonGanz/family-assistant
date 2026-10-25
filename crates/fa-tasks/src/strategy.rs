//! Assignment strategies (plan/M2.md, Strategies).

use std::collections::HashSet;

use chrono::NaiveDate;
use fa_domain::{AdvanceOn, MemberId, Strategy, TaskDefinition};
use fa_store::MemberAwayRepo;
use sqlx::PgConnection;

use crate::error::Result;

/// Candidates sorted by `ord`.
pub struct AssignCtx<'a> {
    pub candidates: &'a [(MemberId, i32)],
    pub cursor: i32,
    pub away: &'a HashSet<MemberId>,
}

/// What happened to the task that may move the rotation cursor.
pub struct AdvanceCtx<'a> {
    pub candidates: &'a [(MemberId, i32)],
    pub advance_on: AdvanceOn,
    pub skip_advances: bool,
    pub assignees: &'a [MemberId],
    /// `Some` on completion, `None` on skip.
    pub completed_by: Option<MemberId>,
    pub skipped: bool,
}

pub trait AssignmentStrategy {
    fn assign(&self, ctx: &AssignCtx) -> Vec<MemberId>;
    /// The new cursor, or `None` to keep it.
    fn advance(&self, _ctx: &AdvanceCtx) -> Option<i32> {
        None
    }
}

pub struct Fixed;
pub struct Rotation;
pub struct Pool;

impl AssignmentStrategy for Fixed {
    fn assign(&self, ctx: &AssignCtx) -> Vec<MemberId> {
        ctx.candidates.first().map(|c| c.0).into_iter().collect()
    }
}

impl AssignmentStrategy for Rotation {
    fn assign(&self, ctx: &AssignCtx) -> Vec<MemberId> {
        let n = ctx.candidates.len();
        if n == 0 {
            return Vec::new();
        }
        let start = ctx
            .candidates
            .iter()
            .position(|c| c.1 >= ctx.cursor)
            .unwrap_or(0);
        (0..n)
            .map(|i| ctx.candidates[(start + i) % n].0)
            .find(|m| !ctx.away.contains(m))
            .or(Some(ctx.candidates[start].0))
            .into_iter()
            .collect()
    }

    fn advance(&self, ctx: &AdvanceCtx) -> Option<i32> {
        let n = ctx.candidates.len() as i32;
        if n == 0 {
            return None;
        }
        let (assignee_ord, assignee) = ctx
            .candidates
            .iter()
            .find(|c| ctx.assignees.contains(&c.0))
            .map(|c| (c.1, c.0))?;
        let moves = if ctx.skipped {
            ctx.skip_advances
        } else {
            match ctx.advance_on {
                AdvanceOn::AnyCompletion => true,
                AdvanceOn::AssigneeCompletion => ctx.completed_by == Some(assignee),
            }
        };
        moves.then_some((assignee_ord + 1) % n)
    }
}

impl AssignmentStrategy for Pool {
    fn assign(&self, ctx: &AssignCtx) -> Vec<MemberId> {
        let free: Vec<MemberId> = ctx
            .candidates
            .iter()
            .map(|c| c.0)
            .filter(|m| !ctx.away.contains(m))
            .collect();
        if free.is_empty() {
            ctx.candidates.iter().map(|c| c.0).collect()
        } else {
            free
        }
    }
}

pub fn strategy_for(s: Strategy) -> &'static dyn AssignmentStrategy {
    match s {
        Strategy::Fixed => &Fixed,
        Strategy::Rotation => &Rotation,
        Strategy::Pool => &Pool,
    }
}

/// Assignees of an occurrence due on `date`.
pub async fn assign_for(
    conn: &mut PgConnection,
    def: &TaskDefinition,
    candidates: &[(MemberId, i32)],
    date: NaiveDate,
) -> Result<Vec<MemberId>> {
    let away: HashSet<MemberId> = MemberAwayRepo::away_on(conn, date)
        .await?
        .into_iter()
        .collect();
    Ok(strategy_for(def.strategy).assign(&AssignCtx {
        candidates,
        cursor: def.rotation_cursor,
        away: &away,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cands(n: usize) -> Vec<(MemberId, i32)> {
        (0..n).map(|i| (MemberId::new(), i as i32)).collect()
    }

    fn assign(
        s: &dyn AssignmentStrategy,
        c: &[(MemberId, i32)],
        cursor: i32,
        away: &[usize],
    ) -> Vec<MemberId> {
        let away: HashSet<MemberId> = away.iter().map(|i| c[*i].0).collect();
        s.assign(&AssignCtx {
            candidates: c,
            cursor,
            away: &away,
        })
    }

    #[test]
    fn rotation_cursor_and_absence() {
        let c = cands(3);
        assert_eq!(assign(&Rotation, &c, 1, &[]), vec![c[1].0]);
        assert_eq!(assign(&Rotation, &c, 1, &[1]), vec![c[2].0]);
        assert_eq!(assign(&Rotation, &c, 2, &[2]), vec![c[0].0]);
        assert_eq!(assign(&Rotation, &c, 1, &[0, 1, 2]), vec![c[1].0]);
        assert_eq!(assign(&Rotation, &c, 7, &[]), vec![c[0].0]);
    }

    #[test]
    fn pool_excludes_away_unless_all() {
        let c = cands(3);
        assert_eq!(assign(&Pool, &c, 0, &[0]), vec![c[1].0, c[2].0]);
        assert_eq!(assign(&Pool, &c, 0, &[0, 1, 2]).len(), 3);
    }

    #[test]
    fn fixed_ignores_absence() {
        let c = cands(2);
        assert_eq!(assign(&Fixed, &c, 0, &[0]), vec![c[0].0]);
    }

    fn adv(
        c: &[(MemberId, i32)],
        on: AdvanceOn,
        skip_adv: bool,
        by: Option<MemberId>,
        skipped: bool,
    ) -> Option<i32> {
        Rotation.advance(&AdvanceCtx {
            candidates: c,
            advance_on: on,
            skip_advances: skip_adv,
            assignees: &[c[1].0],
            completed_by: by,
            skipped,
        })
    }

    #[test]
    fn advance_rules() {
        let c = cands(3);
        let other = MemberId::new();
        assert_eq!(
            adv(&c, AdvanceOn::AnyCompletion, true, Some(other), false),
            Some(2)
        );
        assert_eq!(
            adv(&c, AdvanceOn::AssigneeCompletion, true, Some(other), false),
            None
        );
        assert_eq!(
            adv(&c, AdvanceOn::AssigneeCompletion, true, Some(c[1].0), false),
            Some(2)
        );
        assert_eq!(
            adv(&c, AdvanceOn::AssigneeCompletion, true, None, true),
            Some(2)
        );
        assert_eq!(adv(&c, AdvanceOn::AnyCompletion, false, None, true), None);
    }

    #[test]
    fn advance_wraps() {
        let c = cands(2);
        let r = Rotation.advance(&AdvanceCtx {
            candidates: &c,
            advance_on: AdvanceOn::AnyCompletion,
            skip_advances: true,
            assignees: &[c[1].0],
            completed_by: None,
            skipped: false,
        });
        assert_eq!(r, Some(0));
    }
}

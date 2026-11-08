//! Matching a cleaned item against the catalogue: exact name, then alias, then trigram
//! similarity at or above the household's threshold (arch/Meals.md §4.2 step 3).

use fa_domain::MatchState;
use fa_store::{IngredientRepo, IngredientRow, NewLine};
use sqlx::PgConnection;

use crate::error::Result;
use crate::normalise::Normaliser;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MatchKind {
    Exact,
    Alias,
    /// The similarity score.
    Trigram(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Matched {
    pub ingredient: IngredientRow,
    pub kind: MatchKind,
}

#[derive(Debug, Clone, Copy)]
pub struct IngredientMatcher {
    pub threshold: f32,
}

impl IngredientMatcher {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    /// Tries each candidate name in order by exact name and alias; only then the closest name
    /// by similarity to the first candidate.
    pub async fn find(
        &self,
        conn: &mut PgConnection,
        candidates: &[&str],
    ) -> Result<Option<Matched>> {
        let names: Vec<&str> = candidates
            .iter()
            .copied()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        for name in &names {
            if let Some(i) = IngredientRepo::match_exact(conn, name).await? {
                return Ok(Some(Matched {
                    ingredient: i,
                    kind: MatchKind::Exact,
                }));
            }
            if let Some(i) = IngredientRepo::match_alias(conn, name).await? {
                return Ok(Some(Matched {
                    ingredient: i,
                    kind: MatchKind::Alias,
                }));
            }
        }
        let Some(first) = names.first() else {
            return Ok(None);
        };
        if let Some((id, _, score)) = IngredientRepo::nearest(conn, first, 1)
            .await?
            .into_iter()
            .next()
            && score >= self.threshold
            && let Some(i) = IngredientRepo::get(conn, id).await?
        {
            return Ok(Some(Matched {
                ingredient: i,
                kind: MatchKind::Trigram(score),
            }));
        }
        Ok(None)
    }

    /// Parses, cleans and matches one raw line into the repo's line type.
    pub async fn line(
        &self,
        conn: &mut PgConnection,
        n: &Normaliser,
        raw: &str,
    ) -> Result<NewLine> {
        let l = n.normalise(raw);
        let mut m = None;
        if let Some(item) = &l.item {
            let parsed = Normaliser::parse_line(raw);
            let folded = n.singular(&crate::normalise::fold(&parsed.item));
            m = self.find(conn, &[item.as_str(), folded.as_str()]).await?;
        }
        Ok(NewLine {
            raw_text: raw.to_string(),
            qty: l.qty,
            unit: l.unit,
            item: l.item,
            note: l.note,
            ingredient_id: m.as_ref().map(|m| m.ingredient.id),
            match_state: if m.is_some() {
                MatchState::Matched
            } else {
                MatchState::Unmatched
            },
            proposal: None,
        })
    }
}

use chrono::{DateTime, Utc};
use fa_domain::{Advisory, AdvisoryId, AdvisorySource, Severity, WeatherRuleId};
use sqlx::PgConnection;

use super::{enum_str, parse_enum};
use crate::Result;

pub struct AdvisoryRepo;

impl AdvisoryRepo {
    /// Inserts or refreshes the advisory with this `dedupe_key` for the transaction's household.
    /// Returns its id and whether anything changed.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_by_dedupe(
        conn: &mut PgConnection,
        rule: Option<WeatherRuleId>,
        text: &str,
        severity: Severity,
        valid_from: DateTime<Utc>,
        valid_until: DateTime<Utc>,
        source: AdvisorySource,
        dedupe_key: &str,
    ) -> Result<(AdvisoryId, bool)> {
        let changed = sqlx::query_scalar!(
            "INSERT INTO advisory (id, household_id, rule_id, text, severity, valid_from, valid_until,
                                   source, dedupe_key)
             VALUES ($1, app_household(), $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (household_id, dedupe_key) DO UPDATE SET
                 rule_id = EXCLUDED.rule_id, text = EXCLUDED.text, severity = EXCLUDED.severity,
                 valid_from = EXCLUDED.valid_from, valid_until = EXCLUDED.valid_until,
                 source = EXCLUDED.source
             WHERE (advisory.rule_id, advisory.text, advisory.severity, advisory.valid_from,
                    advisory.valid_until, advisory.source)
                   IS DISTINCT FROM
                   (EXCLUDED.rule_id, EXCLUDED.text, EXCLUDED.severity, EXCLUDED.valid_from,
                    EXCLUDED.valid_until, EXCLUDED.source)
             RETURNING id",
            AdvisoryId::new().as_uuid(),
            rule.map(|r| r.as_uuid()),
            text,
            enum_str(&severity),
            valid_from,
            valid_until,
            enum_str(&source),
            dedupe_key
        )
        .fetch_optional(&mut *conn)
        .await?;
        if let Some(id) = changed {
            return Ok((id.into(), true));
        }
        let id = sqlx::query_scalar!(
            "SELECT id FROM advisory WHERE dedupe_key = $1 AND household_id = app_household()",
            dedupe_key
        )
        .fetch_one(conn)
        .await?;
        Ok((id.into(), false))
    }

    /// Not expired at `now`, newest severity first.
    pub async fn active(conn: &mut PgConnection, now: DateTime<Utc>) -> Result<Vec<Advisory>> {
        let rows = sqlx::query!(
            "SELECT id, household_id, rule_id, text, severity, valid_from, valid_until, source,
                    dedupe_key
             FROM advisory WHERE valid_until > $1
             ORDER BY CASE severity WHEN 'severe' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END, valid_from, id",
            now
        )
        .fetch_all(conn)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(Advisory {
                    id: r.id.into(),
                    household_id: r.household_id.into(),
                    rule: r.rule_id.map(Into::into),
                    text: r.text,
                    severity: parse_enum(&r.severity)?,
                    valid_from: r.valid_from,
                    valid_until: r.valid_until,
                    source: parse_enum(&r.source)?,
                    dedupe_key: r.dedupe_key,
                })
            })
            .collect()
    }

    /// Deletes `source = 'rule'` advisories whose `dedupe_key` is not in `keep`; `s2` advisories stay.
    pub async fn retain_rule_advisories(conn: &mut PgConnection, keep: &[String]) -> Result<u64> {
        Ok(sqlx::query!(
            "DELETE FROM advisory WHERE source = 'rule' AND NOT (dedupe_key = ANY($1))",
            keep
        )
        .execute(conn)
        .await?
        .rows_affected())
    }

    pub async fn delete_expired(conn: &mut PgConnection, before: DateTime<Utc>) -> Result<u64> {
        Ok(
            sqlx::query!("DELETE FROM advisory WHERE valid_until < $1", before)
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }
}

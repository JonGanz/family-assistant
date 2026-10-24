use sqlx::PgConnection;

use super::WeatherRuleRepo;
use crate::Result;

impl WeatherRuleRepo {
    /// Whether a rule with this name exists for the transaction's household, in any status
    /// (archived included) and whoever suggested it.
    pub async fn name_exists(conn: &mut PgConnection, name: &str) -> Result<bool> {
        Ok(sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM weather_rule WHERE name = $1) AS "exists!""#,
            name
        )
        .fetch_one(conn)
        .await?)
    }
}

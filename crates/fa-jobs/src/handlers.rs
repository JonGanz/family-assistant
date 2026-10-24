use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use fa_store::{DeviceElevationRepo, ScheduleRepo, SessionRepo, SystemStatusRepo};

use crate::{BoxFuture, JobContext, JobError, JobHandler};

const BACKUP_MAX_AGE_HOURS: f64 = 26.0;

/// `session.cleanup`: deletes expired sessions and device elevations.
pub struct SessionCleanup;

impl JobHandler for SessionCleanup {
    fn kind(&self) -> &'static str {
        "session.cleanup"
    }

    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let mut tx = ctx.store.tx_system().await?;
            let sessions = SessionRepo::delete_expired(&mut tx).await?;
            let elevations = DeviceElevationRepo::delete_expired(&mut tx).await?;
            tx.commit().await?;
            tracing::info!(
                sessions,
                elevations,
                "expired sessions and elevations removed"
            );
            Ok(())
        })
    }
}

/// `backup.verify`: checks the newest dump under `backup_dir` and records it in `system_status`.
pub struct BackupVerify;

impl JobHandler for BackupVerify {
    fn kind(&self) -> &'static str {
        "backup.verify"
    }

    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            let Some(dir) = ctx.backup_dir.clone() else {
                tracing::info!("backup_dir not configured; skipping backup.verify");
                return Ok(());
            };
            let newest = tokio::task::spawn_blocking(move || newest_file(&dir))
                .await
                .map_err(|e| JobError::Handler(e.to_string()))?;
            let value = match newest {
                Some((path, mtime)) => {
                    let mtime: DateTime<Utc> = mtime.into();
                    let age_hours = (Utc::now() - mtime).num_seconds() as f64 / 3600.0;
                    let ok = age_hours <= BACKUP_MAX_AGE_HOURS;
                    if !ok {
                        tracing::warn!(file = %path.display(), age_hours, "newest backup is older than 26 hours");
                    }
                    serde_json::json!({
                        "newest_file": path.display().to_string(),
                        "mtime": mtime.to_rfc3339(),
                        "age_hours": age_hours,
                        "ok": ok,
                    })
                }
                None => {
                    tracing::warn!("no backup files found");
                    serde_json::json!({
                        "newest_file": null, "mtime": null, "age_hours": null, "ok": false,
                    })
                }
            };
            let mut tx = ctx.store.tx_system().await?;
            SystemStatusRepo::set(&mut tx, "backup", &value).await?;
            tx.commit().await?;
            Ok(())
        })
    }
}

/// Newest regular file (symlinks followed) under `dir`, recursively.
fn newest_file(dir: &Path) -> Option<(PathBuf, SystemTime)> {
    let mut best: Option<(PathBuf, SystemTime)> = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file()
                && let Ok(m) = meta.modified()
                && best.as_ref().is_none_or(|(_, b)| m > *b)
            {
                best = Some((path, m));
            }
        }
    }
    best
}

/// Idempotently seeds the default schedules (keeps `last_enqueued_at`).
pub async fn ensure_default_schedules(store: &fa_store::Store) -> Result<(), JobError> {
    let mut tx = store.tx_system().await?;
    let empty = serde_json::json!({});
    ScheduleRepo::upsert(
        &mut tx,
        "session.cleanup",
        "17 * * * *",
        "session.cleanup",
        &empty,
        true,
    )
    .await?;
    ScheduleRepo::upsert(
        &mut tx,
        "backup.verify",
        "30 6 * * *",
        "backup.verify",
        &empty,
        true,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

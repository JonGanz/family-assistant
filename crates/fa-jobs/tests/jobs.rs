use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{TimeZone, Utc};
use fa_jobs::*;
use fa_store::{Job, JobRepo, ScheduleRepo, Store, SystemStatusRepo};
use sqlx::PgPool;

struct Recorder {
    seen: Arc<Mutex<Vec<String>>>,
    fail: bool,
}

impl JobHandler for Recorder {
    fn kind(&self) -> &'static str {
        "test.job"
    }
    fn run<'a>(
        &'a self,
        _ctx: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<(), JobError>> {
        Box::pin(async move {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            self.seen.lock().unwrap().push(payload["n"].to_string());
            if self.fail {
                Err(JobError::Handler("boom".into()))
            } else {
                Ok(())
            }
        })
    }
}

fn worker(store: &Store, seen: &Arc<Mutex<Vec<String>>>, fail: bool) -> Worker {
    let ctx = JobContext {
        store: store.clone(),
        backup_dir: None,
    };
    let h: Arc<dyn JobHandler> = Arc::new(Recorder {
        seen: seen.clone(),
        fail,
    });
    Worker::new(ctx, vec![h], WorkerConfig::default())
}

async fn all_jobs(pool: &PgPool) -> Vec<(String, i32, Option<String>)> {
    sqlx::query_as("SELECT status, attempts, last_error FROM job ORDER BY created_at")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn idempotent_enqueue(pool: PgPool) {
    let store = Store::from_pool(pool.clone());
    let k = Some("k1".to_string());
    assert!(
        enqueue(
            &store,
            "test.job",
            serde_json::json!({}),
            None,
            0,
            k.clone()
        )
        .await
        .unwrap()
    );
    assert!(
        !enqueue(&store, "test.job", serde_json::json!({}), None, 0, k)
            .await
            .unwrap()
    );
    assert_eq!(all_jobs(&pool).await.len(), 1);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn concurrent_workers_never_double_process(pool: PgPool) {
    let store = Store::from_pool(pool.clone());
    for n in 0..40 {
        enqueue(
            &store,
            "test.job",
            serde_json::json!({ "n": n }),
            None,
            0,
            None,
        )
        .await
        .unwrap();
    }
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut tasks = Vec::new();
    for _ in 0..6 {
        let w = worker(&store, &seen, false);
        tasks.push(tokio::spawn(
            async move { while w.run_once().await.unwrap() {} },
        ));
    }
    for t in tasks {
        t.await.unwrap();
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    for s in seen.lock().unwrap().iter() {
        *counts.entry(s.clone()).or_default() += 1;
    }
    assert_eq!(counts.len(), 40);
    assert!(counts.values().all(|c| *c == 1));
    assert!(all_jobs(&pool).await.iter().all(|j| j.0 == "done"));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn failures_back_off_then_die(pool: PgPool) {
    let store = Store::from_pool(pool.clone());
    enqueue(
        &store,
        "test.job",
        serde_json::json!({ "n": 1 }),
        None,
        0,
        None,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE job SET max_attempts = 3")
        .execute(&pool)
        .await
        .unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let w = worker(&store, &seen, true);

    assert!(w.run_once().await.unwrap());
    let j = &all_jobs(&pool).await[0];
    assert_eq!((j.0.as_str(), j.1), ("queued", 1));
    // Backed off: not claimable until run_at.
    assert!(!w.run_once().await.unwrap());
    let delay: f64 =
        sqlx::query_scalar("SELECT extract(epoch FROM run_at - now())::float8 FROM job")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(delay > 20.0);

    for _ in 0..2 {
        sqlx::query("UPDATE job SET run_at = now() WHERE status = 'queued'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(w.run_once().await.unwrap());
    }
    let j = &all_jobs(&pool).await[0];
    assert_eq!(
        (j.0.as_str(), j.1, j.2.as_deref()),
        ("dead", 3, Some("boom"))
    );
    assert_eq!(seen.lock().unwrap().len(), 3);
    let mut c = pool.acquire().await.unwrap();
    assert_eq!(JobRepo::list_dead(&mut c).await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn expired_lease_is_reclaimed(pool: PgPool) {
    let store = Store::from_pool(pool.clone());
    enqueue(
        &store,
        "test.job",
        serde_json::json!({ "n": 1 }),
        None,
        0,
        None,
    )
    .await
    .unwrap();
    let mut tx = store.tx_system().await.unwrap();
    let job: Job = JobRepo::claim(&mut tx, "crashed", 60)
        .await
        .unwrap()
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(job.status, "running");

    let seen = Arc::new(Mutex::new(Vec::new()));
    let w = worker(&store, &seen, false);
    // Lease still valid: nothing to do.
    assert!(!w.run_once().await.unwrap());
    sqlx::query("UPDATE job SET locked_until = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(w.run_once().await.unwrap());
    let j = &all_jobs(&pool).await[0];
    assert_eq!((j.0.as_str(), j.1), ("done", 2));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn scheduler_enqueues_once_per_fire_time(pool: PgPool) {
    let store = Store::from_pool(pool.clone());
    let mut tx = store.tx_system().await.unwrap();
    ScheduleRepo::upsert(
        &mut tx,
        "t",
        "* * * * *",
        "test.job",
        &serde_json::json!({ "n": 1 }),
        true,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 10).unwrap();
    assert_eq!(scheduler_tick(&store, t0).await.unwrap(), 1);
    assert_eq!(scheduler_tick(&store, t0).await.unwrap(), 0);
    // Another replica that lost `last_enqueued_at` still cannot duplicate the fire time.
    sqlx::query("UPDATE schedule SET last_enqueued_at = NULL")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(scheduler_tick(&store, t0).await.unwrap(), 0);
    assert_eq!(all_jobs(&pool).await.len(), 1);
    // Next minute fires again.
    let t1 = t0 + chrono::Duration::seconds(60);
    assert_eq!(scheduler_tick(&store, t1).await.unwrap(), 1);
    assert_eq!(all_jobs(&pool).await.len(), 2);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn default_schedules_are_idempotent(pool: PgPool) {
    let store = Store::from_pool(pool.clone());
    ensure_default_schedules(&store).await.unwrap();
    ensure_default_schedules(&store).await.unwrap();
    let mut tx = store.tx_system().await.unwrap();
    assert_eq!(ScheduleRepo::list(&mut tx, true).await.unwrap().len(), 2);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn backup_verify_reports_newest_file(pool: PgPool) {
    let store = Store::from_pool(pool.clone());
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("daily")).unwrap();
    std::fs::write(dir.path().join("daily/fa-20260101.sql.gz"), b"x").unwrap();
    let ctx = JobContext {
        store: store.clone(),
        backup_dir: Some(dir.path().into()),
    };

    BackupVerify.run(&ctx, serde_json::json!({})).await.unwrap();
    let mut c = pool.acquire().await.unwrap();
    let (v, _) = SystemStatusRepo::get(&mut c, "backup")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(v["ok"], true);
    assert!(
        v["newest_file"]
            .as_str()
            .unwrap()
            .ends_with("fa-20260101.sql.gz")
    );

    // Stale file -> not ok.
    let f = std::fs::File::options()
        .write(true)
        .open(dir.path().join("daily/fa-20260101.sql.gz"))
        .unwrap();
    f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(48 * 3600))
        .unwrap();
    BackupVerify.run(&ctx, serde_json::json!({})).await.unwrap();
    let (v, _) = SystemStatusRepo::get(&mut c, "backup")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(v["ok"], false);

    // Unconfigured: ok, status untouched.
    let ctx = JobContext {
        store,
        backup_dir: None,
    };
    BackupVerify.run(&ctx, serde_json::json!({})).await.unwrap();
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn session_cleanup_runs(pool: PgPool) {
    let ctx = JobContext {
        store: Store::from_pool(pool),
        backup_dir: None,
    };
    SessionCleanup
        .run(&ctx, serde_json::json!({}))
        .await
        .unwrap();
}

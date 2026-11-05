use chrono::Utc;
use fa_domain::*;
use fa_pipeline::{Candidate, Evaluator, Pipeline, validate_version};
use fa_store::*;
use serde_json::json;
use sqlx::PgPool;

struct Fixed(Vec<Candidate>);
impl Evaluator for Fixed {
    fn candidates(&self, _: DecisionPointKey) -> Vec<Candidate> {
        self.0.clone()
    }
}

fn cand(v: serde_json::Value, c: f32, s: &str) -> Candidate {
    Candidate {
        value: v,
        confidence: c,
        source: s.into(),
    }
}

async fn setup(pool: PgPool) -> (Store, HouseholdId) {
    let store = Store::from_pool(pool);
    let id = HouseholdId::new();
    let h = Household {
        id,
        name: "H".into(),
        tz: "UTC".into(),
        location: None,
        meal_plan_horizon_weeks: 2,
        dashboard_completion_override: false,
        units: Units::Metric,
        week_start: Default::default(),
        calendar_past_days: 30,
        calendar_future_days: 400,
        weather_retention_days: 7,
        private_events_as_busy: true,
        now_list_size: 7,
        now_weights: fa_domain::tasks::NowWeights::default(),
        quiet_from: chrono::NaiveTime::from_hms_opt(21, 0, 0).expect("time"),
        quiet_until: chrono::NaiveTime::from_hms_opt(7, 0, 0).expect("time"),
        currency: DEFAULT_CURRENCY.to_string(),
        gender_terms: Default::default(),
        disabled_signals: Vec::new(),
        events: Default::default(),
        inbox: Default::default(),
        retention_mail: Default::default(),
        dashboard_pending_approvals: true,
        setup_completed_at: None,
        bill_lead_days: DEFAULT_BILL_LEAD_DAYS,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let mut tx = store.tx_system().await.unwrap();
    HouseholdRepo::insert(&mut tx, &h).await.unwrap();
    Pipeline::ensure_defaults(&mut tx, id).await.unwrap();
    tx.commit().await.unwrap();
    (store, id)
}

fn subject() -> Subject {
    Subject::message(MessageId::new())
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn rules_accepted(pool: PgPool) {
    let (store, h) = setup(pool).await;
    let s = subject();
    let mut tx = store.tx_for(h).await.unwrap();
    let ev = Fixed(vec![
        cand(json!("school"), 0.6, "a"),
        cand(json!("bills"), 0.9, "b"),
        cand(json!("x"), 0.9, "c"),
    ]);
    let out = Pipeline::run(&mut tx, h, DecisionPointKey::EmailCategory, s, &ev)
        .await
        .unwrap();
    assert!(out.is_final);
    assert_eq!(out.stage, StageKind::Rules);
    assert_eq!(out.values.len(), 1);
    assert_eq!(out.values[0].value, json!("bills"));
    let cur = Pipeline::current(&mut tx, DecisionPointKey::EmailCategory, s)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cur.value, json!("bills"));
    assert_eq!(cur.source.as_deref(), Some("b"));
    assert_eq!(Pipeline::all_current(&mut tx, s).await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn falls_through_to_human_default(pool: PgPool) {
    let (store, h) = setup(pool).await;
    let s = subject();
    let mut tx = store.tx_for(h).await.unwrap();
    let ev = Fixed(vec![cand(json!(true), 0.1, "weak")]);
    let out = Pipeline::run(&mut tx, h, DecisionPointKey::EmailIsBill, s, &ev)
        .await
        .unwrap();
    assert!(!out.is_final);
    assert_eq!(out.stage, StageKind::Human);
    assert_eq!(
        out.values[0].value,
        DecisionPointKey::EmailIsBill.default_value()
    );
    assert_eq!(out.values[0].source, "default");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn member_link_is_multi_valued(pool: PgPool) {
    let (store, h) = setup(pool).await;
    let s = subject();
    let mut tx = store.tx_for(h).await.unwrap();
    let ev = Fixed(vec![
        cand(json!("m1"), 0.6, "a"),
        cand(json!("m2"), 0.9, "b"),
        cand(json!("m1"), 0.8, "c"),
        cand(json!("m3"), 0.1, "d"),
    ]);
    let out = Pipeline::run(&mut tx, h, DecisionPointKey::EmailMemberLink, s, &ev)
        .await
        .unwrap();
    assert!(out.is_final);
    assert_eq!(out.values.len(), 2);
    let m1 = out.values.iter().find(|c| c.value == json!("m1")).unwrap();
    assert_eq!(m1.source, "c");
    let dpid = DecisionPointRepo::get_by_key(&mut tx, DecisionPointKey::EmailMemberLink)
        .await
        .unwrap()
        .unwrap()
        .id;
    let batch = DecisionRepo::latest_batch(&mut tx, dpid, s).await.unwrap();
    assert_eq!(batch.len(), 2);
}

async fn set_stages(
    tx: &mut sqlx::PgConnection,
    dp: DecisionPointKey,
    f: impl Fn(&mut Vec<PipelineStage>),
) {
    let p = DecisionPointRepo::get_by_key(tx, dp)
        .await
        .unwrap()
        .unwrap();
    let (v, mut stages) = PipelineRepo::active_version(tx, p.id)
        .await
        .unwrap()
        .unwrap();
    f(&mut stages);
    let nv = PipelineVersion {
        id: PipelineVersionId::new(),
        version: v.version + 1,
        name: "t".into(),
        ..v
    };
    for s in &mut stages {
        s.id = PipelineStageId::new();
        s.pipeline_version_id = nv.id;
    }
    PipelineRepo::insert_version(tx, &nv, &stages)
        .await
        .unwrap();
}

fn stage(h: HouseholdId, ord: i32, kind: StageKind, thr: f32) -> PipelineStage {
    PipelineStage {
        id: PipelineStageId::new(),
        household_id: h,
        pipeline_version_id: PipelineVersionId::new(),
        ord,
        kind,
        enabled: true,
        accept_threshold: thr,
        on_unavailable: OnUnavailable::Skip,
        config: json!({}),
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn disabled_and_unavailable_stages_are_skipped(pool: PgPool) {
    let (store, h) = setup(pool).await;
    let mut tx = store.tx_for(h).await.unwrap();
    let dp = DecisionPointKey::EmailCategory;
    // [llm (unavailable), rules (disabled), rules, human]
    set_stages(&mut tx, dp, |st| {
        st.clear();
        st.push(stage(h, 0, StageKind::Llm, 0.5));
        let mut off = stage(h, 1, StageKind::Rules, 0.0);
        off.enabled = false;
        st.push(off);
        st.push(stage(h, 2, StageKind::Rules, 0.7));
        st.push(stage(h, 3, StageKind::Human, 0.0));
    })
    .await;
    let s = subject();
    let ev = Fixed(vec![
        cand(json!("a"), 0.3, "low"),
        cand(json!("b"), 0.8, "hi"),
    ]);
    let out = Pipeline::run(&mut tx, h, dp, s, &ev).await.unwrap();
    assert_eq!(out.stage, StageKind::Rules);
    assert_eq!(out.values[0].value, json!("b"));
    // the disabled rules stage (threshold 0) would have accepted a weak candidate
    let weak = Fixed(vec![cand(json!("a"), 0.3, "low")]);
    let out = Pipeline::run(&mut tx, h, dp, subject(), &weak)
        .await
        .unwrap();
    assert_eq!(out.stage, StageKind::Human);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn record_human_then_current(pool: PgPool) {
    let (store, h) = setup(pool).await;
    let s = subject();
    let dp = DecisionPointKey::EmailTimeSensitive;
    let ev = Fixed(vec![]);
    {
        let mut tx = store.tx_for(h).await.unwrap();
        let out = Pipeline::run(&mut tx, h, dp, s, &ev).await.unwrap();
        assert!(!out.is_final);
        tx.commit().await.unwrap();
    }
    {
        let mut tx = store.tx_for(h).await.unwrap();
        let cur = Pipeline::current(&mut tx, dp, s).await.unwrap().unwrap();
        assert!(!cur.is_final);
        tx.commit().await.unwrap();
    }
    {
        let mut tx = store.tx_system().await.unwrap();
        Pipeline::record_human(&mut tx, h, dp, s, json!(true), None)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let mut tx = store.tx_for(h).await.unwrap();
    let cur = Pipeline::current(&mut tx, dp, s).await.unwrap().unwrap();
    assert!(cur.is_final);
    assert_eq!(cur.stage_kind, StageKind::Human);
    assert_eq!(cur.value, json!(true));
}

#[test]
fn validate_version_cases() {
    let h = HouseholdId::new();
    assert!(validate_version(&[]).is_err());
    assert!(validate_version(&[stage(h, 0, StageKind::Rules, 0.5)]).is_ok());
    assert!(
        validate_version(&[
            stage(h, 0, StageKind::Rules, 0.5),
            stage(h, 1, StageKind::Human, 0.0)
        ])
        .is_ok()
    );
    assert!(
        validate_version(&[
            stage(h, 0, StageKind::Rules, 0.5),
            stage(h, 1, StageKind::Llm, 0.5)
        ])
        .is_err()
    );
    assert!(validate_version(&[stage(h, 0, StageKind::Rules, 1.5)]).is_err());
    let mut off = stage(h, 1, StageKind::Llm, 0.5);
    off.enabled = false;
    assert!(validate_version(&[stage(h, 0, StageKind::Rules, 0.5), off.clone()]).is_ok());
    let mut only_off = stage(h, 0, StageKind::Rules, 0.5);
    only_off.enabled = false;
    assert!(validate_version(&[only_off]).is_err());
}

async fn stage_kinds(store: &Store, h: HouseholdId, dp: DecisionPointKey) -> (i32, Vec<StageKind>) {
    let mut tx = store.tx_for(h).await.unwrap();
    let p = DecisionPointRepo::get_by_key(&mut tx, dp)
        .await
        .unwrap()
        .unwrap();
    let (v, st) = PipelineRepo::active_version(&mut tx, p.id)
        .await
        .unwrap()
        .unwrap();
    (v.version, st.iter().map(|s| s.kind).collect())
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn llm_stage_versions_and_pending(pool: PgPool) {
    let (store, h) = setup(pool).await;
    let dp = DecisionPointKey::EmailCategory;
    let mut tx = store.tx_for(h).await.unwrap();
    // Disabling a stage that is not there changes nothing.
    let v1 = Pipeline::set_llm_stage(&mut tx, h, dp, false, 0.7)
        .await
        .unwrap();
    let v2 = Pipeline::set_llm_stage(&mut tx, h, dp, true, 0.7)
        .await
        .unwrap();
    assert_ne!(v1, v2);
    tx.commit().await.unwrap();
    let mut tx = store.tx_for(h).await.unwrap();
    assert_eq!(
        stage_kinds(&store, h, dp).await.1,
        vec![StageKind::Rules, StageKind::Llm, StageKind::Human]
    );
    // Same settings: same version; a new threshold: a new version.
    assert_eq!(
        Pipeline::set_llm_stage(&mut tx, h, dp, true, 0.7)
            .await
            .unwrap(),
        v2
    );
    let v3 = Pipeline::set_llm_stage(&mut tx, h, dp, true, 0.9)
        .await
        .unwrap();
    assert_ne!(v2, v3);
    assert!(
        Pipeline::set_llm_stage(&mut tx, h, dp, true, 1.5)
            .await
            .is_err()
    );
    // Rules below the threshold with an enabled llm stage: pending, default recorded.
    let s = subject();
    let low = Fixed(vec![cand(json!("school"), 0.5, "a")]);
    let out = Pipeline::run(&mut tx, h, dp, s, &low).await.unwrap();
    assert!(out.pending_llm && !out.is_final);
    assert_eq!(out.stage, StageKind::Human);
    // Rules at the threshold: no pending.
    let high = Fixed(vec![cand(json!("school"), 0.9, "a")]);
    let out = Pipeline::run(&mut tx, h, dp, subject(), &high)
        .await
        .unwrap();
    assert!(!out.pending_llm && out.stage == StageKind::Rules);
    // The LLM stage: at or above its threshold (0.9) is recorded, below falls to the default.
    let o = Pipeline::record_stage(
        &mut tx,
        s,
        dp,
        StageKind::Llm,
        vec![cand(json!("bills"), 0.85, "llm")],
        out.version,
    )
    .await
    .unwrap();
    assert_eq!(o.stage, StageKind::Human);
    assert!(
        Pipeline::current(&mut tx, dp, s)
            .await
            .unwrap()
            .is_some_and(|d| d.stage_kind == StageKind::Human)
    );
    let o = Pipeline::record_stage(
        &mut tx,
        s,
        dp,
        StageKind::Llm,
        vec![cand(json!("bills"), 0.95, "llm")],
        out.version,
    )
    .await
    .unwrap();
    assert!(o.is_final && o.stage == StageKind::Llm);
    let cur = Pipeline::current(&mut tx, dp, s).await.unwrap().unwrap();
    assert_eq!(cur.stage_kind, StageKind::Llm);
    assert_eq!(cur.value, json!("bills"));
    // Disabling keeps the stage row but the last enabled stage stays human.
    Pipeline::set_llm_stage(&mut tx, h, dp, false, 0.9)
        .await
        .unwrap();
    let out = Pipeline::run(&mut tx, h, dp, subject(), &low)
        .await
        .unwrap();
    assert!(!out.pending_llm);
    let mut stages = {
        let p = DecisionPointRepo::get_by_key(&mut tx, dp)
            .await
            .unwrap()
            .unwrap();
        PipelineRepo::active_version(&mut tx, p.id)
            .await
            .unwrap()
            .unwrap()
            .1
    };
    assert!(validate_version(&stages).is_ok());
    // An llm stage cannot be the last enabled one.
    stages.retain(|s| s.kind != StageKind::Human);
    for s in &mut stages {
        s.enabled = s.kind == StageKind::Llm;
    }
    stages.push(stage(h, 9, StageKind::Llm, 0.5));
    stages
        .iter_mut()
        .for_each(|s| s.enabled = s.kind == StageKind::Llm);
    assert!(stages.is_empty() || validate_version(&stages).is_err());
}

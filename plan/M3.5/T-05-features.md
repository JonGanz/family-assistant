---
id: T-05
title: "Feature crates: advisory clearing, private inbox, approval recipients, private-host guard"
milestone: M3.5
wave: 2
depends_on: [T-02]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-weather/**", "crates/fa-mail/**", "crates/fa-calendar/**", "crates/fa-connectors/**", "crates/fa-tasks/src/ingest.rs"]
decisions: [D-068, D-075, D-078]
---

# T-05: Feature crates: advisory clearing, private inbox, approval recipients, private-host guard

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `plan/M3.5.md`: *Contracts fixed up front* → fa-store, Feature crates
- `arch/Feeds.md` (advisory lifecycle), `arch/Ingest.md` §10 and §12, D-078
- `crates/fa-weather/src/{eval,jobs,service}.rs`, `crates/fa-mail/src/inbox.rs`, `crates/fa-mail/src/process/extract.rs`, `crates/fa-calendar/src/ingest.rs`, `crates/fa-tasks/src/ingest.rs`, `crates/fa-connectors/src/http.rs`
- IMPROVEMENTS.md C-05
- T-02's outcomes (added below by the main session)

## You own
- `crates/fa-weather/**`
- `crates/fa-mail/**`
- `crates/fa-calendar/**`
- `crates/fa-connectors/**`
- `crates/fa-tasks/src/ingest.rs`

## Work
1. **Advisories (D-068):** remove the dismiss service path. After each **successful** `weather.evaluate`, call `AdvisoryRepo::retain_rule_advisories` with the dedupe keys that still match, so rule advisories disappear when the forecast stops triggering them. Keep `valid_until` expiry and the daily retention. A failed fetch or evaluation clears nothing.
2. **Private inbox (D-078):** in `InboxService`, an adult viewer gets only the items whose source message is on a source they may read (`MailSourceRepo::readable_by`). Items with no message, or whose source is gone or archived, are visible to every adult. Device viewers, elevated or not, get `MailError::Forbidden` (remove `Access::CountsOnly` and the elevated-device branch). Apply the same check to every approve, edit and reject: refused with `Forbidden` when out of scope. `counts` takes the viewer and is scoped the same way.
3. **Recipients:** every `ApprovalNeeded` insert (`fa-mail` extract, `fa-calendar` ingest, `fa-tasks` ingest) uses `NotificationRepo::insert_for_source_adults` with the item's message source (or `None`). Leave other triggers alone.
4. **C-05:** `Http::with_policy(allow_http, allow_private_hosts)` per the contract. Use a custom DNS resolver that filters resolved addresses, check literal-IP URLs too, and enforce it in the redirect policy. `Http::new` is unchanged. `ConnectorConfig`, `WeatherConfig` and `MailConfig` gain `allow_private_hosts` and build their clients with it. Tests use `allow_private_hosts: true` wherever they hit a local stub.
5. Update the tests in your crates for all of the above; delete tests of removed behaviour.

## Acceptance
- [ ] `cargo test -p fa-weather -p fa-mail -p fa-calendar -p fa-connectors -p fa-tasks` pass (fresh container port 55405)
- [ ] new tests: an advisory disappears on the next evaluation when its rule no longer matches; adult A doesn't see or approve an item from adult B's owners-only source but does from a household source; an orphaned item is visible to both; devices are refused; `approval_needed` rows go only to the source's adults; `with_policy(_, false)` refuses `127.0.0.1`, `10.0.0.1`, `169.254.169.254`, a hostname resolving to loopback, and a redirect to a private address, while `with_policy(_, true)` allows them
- [ ] `cargo clippy` on those crates with `--all-targets -- -D warnings` and `cargo fmt --check` clean

## Wave 1 outcomes: T-02 (domain + store), read before starting
- Done; fa-domain 262 tests, fa-store all binaries green on a fresh DB. The main session re-ran fa-domain, fa-store, fa-pipeline, fa-notify and fa-tasks together: 383 passed, 0 failed.
- New: `fa_domain::system_api` (every contract type; `serde_json::Value` fields use `#[ts(type = "unknown")]`), `fa_store::{InstanceSettingRepo, SetupRepo}` (files `repo/instance_setting.rs`, `repo/setup.rs`), `AdvisoryRepo::retain_rule_advisories`, `NotificationRepo::insert_for_source_adults`, `InboxRepo::pending_by_adult` (one row per active adult, zeros included).
- Removed: `Action::DismissAdvisory`, `AdvisoryRepo::dismiss`, the advisory dismissed fields, `DashboardView.inbox`. `ApproveEvent` and `ReadInbox` now need an adult user (devices refused, elevated or not).
- `Household` gained `dashboard_pending_approvals: bool` and `setup_completed_at: Option<DateTime<Utc>>`; every struct literal must set them (tests: `true`, `None`).
- `pin_lockout` / `UserAccountRepo::lockout` read `FOR UPDATE`; `record_pin_failure` / `record_failure` don't update a currently locked row.
- **Your breakage to fix:** Household literals in `crates/fa-weather/tests/{tasks.rs,service.rs}`, `crates/fa-calendar/tests/{sync.rs,ingest.rs}`, `crates/fa-mail/tests/{sync_common.rs,process_flow.rs}`; `crates/fa-weather/tests/service.rs:139` uses the removed `AdvisoryRepo::dismiss`. Move `InboxRepo::counts` / inbox callers in your crates to the scoped functions. The fa-pipeline, fa-notify and fa-tasks test literals are already fixed by the main session.

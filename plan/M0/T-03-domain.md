---
id: T-03
title: fa-domain: IDs, enums, entities, Actor/policy types, ts-rs
milestone: M0
wave: 2
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-domain/**"]
---

# T-03: fa-domain: IDs, enums, entities, Actor/policy types, ts-rs

You are a sub-agent implementing one task of the M0 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Domain.md (Persistence conventions, Identity & Access package, RBAC matrix)
- arch/Platform.md §4, §5

## You own
- `crates/fa-domain/**`

## Work
1. **ID newtypes** over `Uuid` (v7): `HouseholdId`, `MemberId`, `UserAccountId`, `SessionId`, `DeviceId`, `AuditEntryId`, `SecretId`, `JobId`. Derive serde, `sqlx::Type` (transparent), ts-rs, Display, FromStr, and add a `new()` that uses `Uuid::now_v7()`.
2. **Enums** with serde `snake_case`, sqlx text mapping, and ts-rs: `Role {Adult, Child}`, `DeviceKind {Dashboard}`, `SecretKind {OauthToken, Other}`.
3. **Entities** (plain structs, no DB code): `Household`, `Member`, `UserAccount` (without the hash in its public DTO), `Device`, `DeviceElevation`, `AuditEntry`, plus a `SourceRef` for later use. Each entity has an API **DTO** variant that ts-rs exports to `apps/web/src/gen/types/` (set this up with `#[ts(export, export_to = "../../apps/web/src/gen/types/")]`, or a test-driven export. Choose one and document it).
4. **Authorization model:**
   - `Actor` enum: `User{member_id, role, household_id}`, `Device{device_id, household_id}`, `ElevatedDevice{device_id, household_id, adult: MemberId}`, `System`.
   - `Action` enum covering the RBAC matrix rows in Domain.md for M0: ReadMemberPublic, ReadMemberFull, ManageMember, ManageChildAccount, SetOwnPin, ManageDevices, Elevate, ReadOwnData, ReadAllData, RevokeChildSession, ManageHouseholdSettings, EditOtherAdultSettings, plus the M2/M3 ones (CompleteTask, AddGroceryItem, ApproveEvent, GmailAction), listed now for completeness.
   - `Resource` enum: `Member(MemberId, Role)`, `Household`, `Device(DeviceId)`, `Session{owner: MemberId, owner_role: Role}`, and others.
   - A **pure** `fn authorize(actor: &Actor, action: Action, resource: &Resource, ctx: &PolicyCtx) -> Result<(), Denied>` that implements the matrix exactly. `PolicyCtx` carries `dashboard_completion_override` and the list completion policy for later.
   - **Exhaustive table-driven tests** that enumerate actor × action combinations against the expected matrix. Include the rule that an adult can't edit another adult's personal settings.
5. **Validation helpers:** password policy (adults ≥12, children ≥4, D-031) and PIN policy (4 to 6 digits), returning typed errors.

## Acceptance
- [ ] `cargo test -p fa-domain` passes, with policy tests covering every Action variant
- [ ] `cargo clippy -p fa-domain -- -D warnings` passes
- [ ] The TS type export produces files under `apps/web/src/gen/types/` (list them in the report)

## Ground rules (all M0 tasks)
- Repo: `/home/jon/projects/family-assistant`. **First read** `CLAUDE.md`, `plan/M0.md` (especially the contracts), and the arch sections listed under *Read first*.
- **Edit only the paths you own** (listed below). If you need a change somewhere else, don't make it: say so in your report.
- Code style: idiomatic Rust 2024 edition, with `thiserror` in libraries, `anyhow` only in bins, no `unwrap()` outside tests, and `tracing` for logs. Keep comments sparse and useful.
- Public AGPL repo (D-035): no real personal data. Fixtures use `example.com`.
- Tools: pnpm is `corepack pnpm@9`. `just` and `psql` aren't installed (use `docker compose -f deploy/compose.dev.yaml exec postgres psql -U fa`). Dev DB: `postgres://fa:fa@127.0.0.1:55432/fa`.
- Don't commit, and don't run `git` commands that change state.
- **Report** (your final message), kept concise:
  1. what you built (files);
  2. the acceptance command results (pass or fail, with output on failure);
  3. any deviations from the arch docs or contracts, and why;
  4. follow-ups or requests for other tasks' paths.

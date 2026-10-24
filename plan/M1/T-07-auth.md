---
id: T-07
title: "fa-auth: Google OAuthService, System-capable services"
milestone: M1
wave: 3
depends_on: [T-02, T-04]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-auth/**"]
---

# T-07: fa-auth: Google OAuthService, System-capable services

You are a sub-agent implementing one task of the M1 milestone (Dashboard core) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- arch/Feeds.md §1, §3.1, §7 (OAuth, secrets, `oauth_pending`)
- arch/Platform.md §5 (Security)
- plan/M1.md: API rows `/oauth/google/*` and `/oauth/accounts`, the Jobs table (`oauth.cleanup`), and *Carried in from M0*
- crates/fa-connectors (the Google token-exchange client from T-02), and the fa-store `oauth_account`/`oauth_pending`/`secret` repos (from T-04)
- log/2026-10-24.md: the T-08 follow-up about the CLI re-implementing device pairing

## You own
- `crates/fa-auth/**`

## Work
1. **`OAuthService`** (Google, authorisation code + PKCE S256). It implements `fa_domain::AccessTokenProvider`: refresh the access token from the stored refresh token (cache it in memory until 60 s before expiry), and on `invalid_grant` set the account to `needs_reauth` and return `TokenError::NeedsReauth`:
   - `start(actor, return_to) -> Url`:
     - adults only;
     - create an `oauth_pending{state (32 random bytes, base64url), session, member, pkce_verifier, return_to, expires_at = now + 10 min}`;
     - build the Google auth URL with `access_type=offline`, `prompt=consent`, and scopes `openid email https://www.googleapis.com/auth/calendar.readonly`;
     - take the redirect URI from `FA_PUBLIC_URL` + `/api/v1/oauth/google/callback`.
   - `callback(actor, code, state) -> Result<OAuthAccountId>`:
     - look up and **delete** the pending row (single use);
     - reject it if expired, or if its session or member differ from the caller;
     - exchange the code through fa-connectors;
     - read the email from the ID token or userinfo;
     - upsert the `oauth_account` by `(owner_member, email)`, storing the refresh token as a `Secret` (envelope-encrypted) and setting `state=ok`.

     A response without a refresh token is an error that tells the user to revoke access and retry.
   - `access_token(account) -> Result<String>`: refreshes the token and caches it in memory until 60 s before expiry. On `invalid_grant`, it sets `state=needs_reauth` and returns a typed error (`AuthError::NeedsReauth`). fa-calendar calls this method.
   - `list_accounts(actor)`: covers the caller's own accounts and children's accounts, under the D-006 owner rules. It never returns secrets.
   - `cleanup_expired()` deletes expired `oauth_pending` rows. Expose it as the job handler `oauth.cleanup`, together with `handlers()` and `default_schedules()` per the plan's Jobs contract.
   - If `FA_GOOGLE_CLIENT_ID` or `FA_GOOGLE_CLIENT_SECRET` is unset, `OAuthService::enabled()` is false and every call returns `AuthError::NotConfigured`.
2. **System-capable services (M0 follow-up):** add `DeviceService::pair_system(household, name)` and `AccountService` (create the account, reset the password) that accept `Actor::System`, so `fa-server` stops duplicating the token and hash logic. The existing user-facing paths keep calling `Policy::authorize`. System calls are audited with actor `system`.
3. All authorisation goes through `Policy::authorize` with the new M1 actions from fa-domain (T-01). Don't add ad-hoc role checks.
4. **Tests** (sqlx::test plus wiremock standing in for Google's token endpoint):
   - start → callback happy path;
   - a reused state, an expired state, and the wrong session are each rejected;
   - the refresh token is stored encrypted (it doesn't appear in plain text in the `secret` row);
   - refresh caching;
   - `invalid_grant` → `needs_reauth`;
   - not configured → `NotConfigured`;
   - `pair_system` produces a token that the existing device resolver accepts.

## Acceptance
- [ ] `cargo test -p fa-auth` passes
- [ ] `cargo clippy -p fa-auth -- -D warnings` passes
- [ ] `cargo fmt --check` passes

## Ground rules (all M1 tasks)
- Repo: `/home/jon/projects/family-assistant`. **First read** `CLAUDE.md`, `plan/M1.md` (especially *Contracts fixed up front*), and the arch sections listed under *Read first*. The contracts in `plan/M1.md` are the source of truth. If one is wrong or missing, don't invent a new one: report it.
- **Edit only the paths you own.** If you need a change somewhere else, don't make it: say so in your report.
- Run `df -h /` before building. **If less than 6 GB is free, stop and report.**
- Rust: idiomatic 2024 edition, with `thiserror` in libraries, `anyhow` only in bins, no `unwrap()` outside tests, and `tracing` for logs. Run `cargo fmt` before you finish. Keep comments sparse and useful.
- Web: TypeScript strict. **API types come from `apps/web/src/gen/` only. Never hand-write API types.** Run prettier and eslint (`corepack pnpm@9 lint`, `corepack pnpm@9 format` or the equivalent scripts in package.json) before you finish.
- **Verify every external name** before using it: crate features with `cargo add --dry-run`, images with `docker manifest inspect`, and API paths against the vendor docs. Don't guess.
- CI-equivalent checks run against a **fresh** Postgres (`scripts/ci-local.sh`, `SQLX_OFFLINE=true`), not only the dev DB.
- Public AGPL repo (D-035): no real personal data. Fixtures use `example.com`.
- Tools: pnpm is `corepack pnpm@9`. `just` and `psql` aren't installed (use `docker compose -f deploy/compose.dev.yaml exec postgres psql -U fa`). Dev DB: `postgres://fa:fa@127.0.0.1:55432/fa`.
- Don't commit, and don't run `git` commands that change state.
- **README:** only T-12 owns `README.md`. Every other task lists the README changes it needs in its report.
- **Report** (your final message), kept concise:
  1. what you built (files);
  2. the acceptance command results (pass or fail, with output on failure);
  3. any deviations from the arch docs or contracts, and why;
  4. follow-ups, requests for other tasks' paths, and README changes needed.

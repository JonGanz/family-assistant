---
id: T-02
title: "fa-connectors: ntfy publish client"
milestone: M2
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-connectors/**"]
---

# T-02: fa-connectors: ntfy publish client

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: *Notifications* (the verified ntfy API)
- `crates/fa-connectors/src` (`http.rs` for the shared client, timeouts and http/https policy; `openmeteo.rs` as a small example; `error.rs`)

## You own
- `crates/fa-connectors/**`

## Work
1. `ntfy.rs`: `pub struct NtfyMessage { title, body, priority: u8 (1..=5), tags: Vec<String>, click: Option<String> }` and `pub async fn publish(http: &Http, base_url: &str, topic: &str, token: Option<&str>, msg: &NtfyMessage) -> Result<String, ConnectorError>` returning the message `id` from the JSON response.
   - `POST {base}/{topic}`, body = `body` as text, headers `Title`, `Priority`, `Tags` (comma-joined, only if any), `Click` (only if set), `Authorization: Bearer {token}` (only if set).
   - Header values must be ASCII-safe: ntfy accepts UTF-8 titles via the `Title` header only when encoded. Use RFC 2047 encoding (`=?UTF-8?B?…?=`) for non-ASCII titles, which ntfy supports. If you cannot confirm that, fall back to putting the title in the JSON publish API instead: `POST {base}/` with JSON `{topic, title, message, priority, tags, click}`. **Check whichever you choose against a real container** (`docker run -d --name fa-t02-ntfy -p 127.0.0.1:55402:80 binwiederhier/ntfy:v2.17.0 serve`; read back with `GET /{topic}/json?poll=1&since=all`; remove it with `docker rm -f -v fa-t02-ntfy`), with an ASCII title and a title containing `é` and an emoji.
   - Map 401/403 to an auth error variant, other non-2xx to an HTTP error with the status (no body echo beyond 200 chars), and network errors as the existing client does. Never log the topic or token.
   - The `Http` policy (https only unless `allow_http`) applies to the ntfy base URL too. `http://` on the tailnet or localhost is common for ntfy, so document in the function docs that `FA_ALLOW_HTTP_FEEDS` is not the switch here: add `pub fn publish_allow_http` or a parameter so fa-notify can allow plain http for ntfy specifically. Choose the smallest change and report it.
2. Re-export from `lib.rs`.
3. Tests with `wiremock`: headers present or absent as specified, token header, error mapping, the returned id. Plus one `#[ignore]` integration test against a local ntfy URL from `FA_TEST_NTFY_URL`.

## Acceptance
- [ ] `cargo test -p fa-connectors` passes
- [ ] The real-container check (ASCII and non-ASCII titles) succeeded; describe what you ran and what came back
- [ ] `cargo clippy -p fa-connectors --all-targets -- -D warnings` and `cargo fmt --check` are clean

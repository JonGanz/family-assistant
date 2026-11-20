---
id: T-05
title: "Tailnet path from the app to Ollama (outbound proxy)"
milestone: M5.3
wave: 2
depends_on: []
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-llm/**", "crates/fa-config/**", "bins/fa-server/src/wiring.rs", "crates/fa-api/src/ai.rs", "deploy/compose.yaml", "deploy/.env.example", "deploy/README.md", "README.md"]
decisions: [D-127]
---

# T-05: Tailnet path from the app to Ollama

You are a sub-agent on milestone M5.3 of the Family Assistant project (Rust workspace, axum; deployment in `deploy/`).

Bug: with the Tailscale sidecar, an Ollama host on the tailnet works only when entered by its LAN IP. Cause (R-009, D-127): the sidecar runs in userspace mode and only serves inbound; the `app` container is not on the tailnet and the LLM client has no proxy. Fix chosen: the sidecar's outbound HTTP proxy plus a routed proxy in the LLM client.

## Read first
- `plan/_rules-common.md`, `plan/M5.3/_rules.md` (Rust rules apply: fmt, disk, test your crates with `-p`, fresh Postgres only where a test needs it), `plan/M5.3.md`
- `decisions/D-127-tailnet-proxy-for-ai.md`, `research/R-009-tailnet-outbound-from-app.md`, `arch/Platform.md` (the tailscale row)
- `crates/fa-llm/src/client.rs` (`default_http`), `provider.rs`, `crates/fa-api/src/ai.rs` (`fn http()` and the LlmClient construction sites ~443, ~530), `bins/fa-server/src/wiring.rs` (`llm_provider`), `crates/fa-config/src/process.rs` and `runtime.rs` (how `FA_*` variables become typed fields: copy the `tailnet_base_url` / `ntfy_public_url` Option<String> pattern with `empty_none`), `deploy/compose.yaml`, `deploy/.env.example`, `README.md` env table

## You own
The paths in `owns`. Don't touch the other crates' HTTP clients (calendars, feeds, recipes keep going direct).

## Work
1. **Config:** `ProcessConfig.tailnet_proxy: Option<String>` from `FA_TAILNET_PROXY` (empty/unset = off; validate it parses as an `http://` URL at startup with a clear error). Add it wherever env keys are listed (runtime key list / locks / tests) as for `FA_TAILNET_BASE_URL`.
2. **LLM client:** `fa_llm::http_client(proxy: Option<&Url>) -> reqwest::Result<reqwest::Client>` (keep `default_http()` as the no-proxy form). With a proxy, use `reqwest::Proxy::custom(...)` so a request goes through it only when the URL host is a MagicDNS name (`*.ts.net`, case-insensitive), an IPv4 in `100.64.0.0/10`, or an IPv6 in `fd7a:115c:a1e0::/48`; everything else is direct (notably single-label hosts like `ollama` and LAN IPs). The pure predicate `is_tailnet_host(&Url) -> bool` is public and table-tested (names with trailing dot, uppercase, `evil-ts.net`, `100.63.255.255`, `100.64.0.1`, `100.127.255.255`, `100.128.0.1`, an IPv6 inside and outside the /48, a LAN IP, `ollama`, `localhost`). Redirects stay off, as today. Build with `default-features = false` as the workspace does: no new cargo feature or dependency (report one if you think you need it).
3. **Wiring:** use it in `wiring.rs::llm_provider` and `ai.rs::http()` (so the Settings > AI "test connection" and model listing use the same path); pass the config the cheapest way that fits the existing `AppState` (it already carries process config or services; check, don't add a global).
4. **Test with a fake proxy** in `crates/fa-llm/tests/`: a tokio TCP listener that records the request line, answers a minimal HTTP/1.1 200 with a JSON body; assert that (a) a call to `http://100.64.0.7:11434/...` and to `http://gpu.example.ts.net:11434/...` reaches the fake proxy in absolute-form (`GET http://100.64.0.7:11434/... HTTP/1.1`), and (b) a call to a local direct server (another listener on 127.0.0.1) does NOT touch the proxy. This is the check R-009 left open (reqwest + rustls, plain http through an http:// proxy); put the finding in your report (update the "NOT verified" line of R-009 only if you own it: you don't, report it instead).
5. **Compose:** in `deploy/compose.yaml` add `TS_OUTBOUND_HTTP_PROXY_LISTEN: 172.30.89.10:1055` to the `tailscale` service (address must match the service's fixed `ipv4_address`; no `ports:`), and `FA_TAILNET_PROXY: ${FA_TAILNET_PROXY-http://tailscale:1055}` to `app` (use `-` not `:-` so an explicitly empty value turns it off; check the compose syntax). When the tailscale profile is off the host is simply unreachable, which only matters for tailnet addresses that could not work anyway; say so in a comment. `deploy/.env.example` documents the variable.
6. **Docs (README rules apply):** the `README.md` env table and Local AI section and `deploy/README.md` (the line that says to bind Ollama to a LAN or tailnet address): how to use a tailnet Ollama host (enter its MagicDNS name or 100.x address and port in Settings > AI; needs the `tailscale` profile; Ollama must listen on the tailnet interface; `OLLAMA_HOST`), the security note (the proxy has no authentication and anything on the compose network can use it; restrict the sidecar node with Tailscale ACLs; do not publish port 1055), that only the AI client uses it, and that this was verified against Tailscale's source and a local fake proxy, not a live tailnet.
7. Check nothing else sets `FA_*` through a fixed list that the new variable must join (grep `FA_TAILNET_BASE_URL` across `crates`, `bins`, `deploy`, `scripts`, `e2e`) and `docker compose -f deploy/compose.yaml config` still renders (with and without `--profile tailscale`).

## Acceptance
- [ ] `cargo fmt --all`; `cargo clippy -p fa-llm -p fa-config -p fa-api -p fa-server --all-targets -- -D warnings` (or the crate names that exist); `cargo test -p fa-llm -p fa-config`; `cargo test -p fa-api --test ai` against a fresh Postgres per the common rules
- [ ] `docker compose config` renders both ways
- [ ] Report: files changed, the fake-proxy finding, anything outside your paths that needs changing

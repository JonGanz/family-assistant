---
id: R-009
title: Reaching a tailnet Ollama host from the app container (Tailscale outbound proxy)
date: 2026-11-20
method: Main-session sub-agent. Fetched tailscale/tailscale source at tag v1.104.2 (the version pinned in deploy/compose.yaml) from raw.githubusercontent.com. Files read are cmd/containerboot/settings.go, cmd/containerboot/tailscaled.go, cmd/containerboot/main.go, cmd/tailscaled/proxy.go, net/proxymux/mux.go and net/tsdial/tsdial.go. Docs read are the Tailscale docker-params page and the userspace-networking kb page. reqwest docs from docs.rs (0.13.5 features page and struct Proxy).
verified: Verified from source at v1.104.2 = env var names and their mapping to tailscaled flags; no default (proxies are off when unset); HTTP proxy handles absolute-form http requests and CONNECT; SOCKS5 accepts domain-name addresses and resolves them in tailscaled; MagicDNS is tried first, then system DNS and a direct dial for non-tailnet hosts. Verified from docs.rs = reqwest has an opt-in `socks` feature, and Proxy::all and Proxy::custom exist without it. Verified by a local fake proxy (M5.3 T-05, crates/fa-llm/tests/tailnet_proxy.rs) = reqwest 0.13.5 with rustls and no default features sends `GET http://100.64.0.7:11434/... HTTP/1.1` (absolute-form) through an http:// proxy for plain http targets, for an IP and for a *.ts.net name, and a direct local target bypasses the proxy. NOT verified = anything by running it against a real tailnet (no tailnet here); exact ACL behaviour for proxied traffic (reasoned from the proxy dialing as the node); the nearest-release question is moot because the tag v1.104.2 was checked directly; the kb text on binding is silent, the claims on binding come from the flags' help text and the net.Listen calls.
informs: []
---

# R-009: Reaching a tailnet Ollama host from the app container

## Problem
`deploy/compose.yaml` runs a `tailscale` sidecar (v1.104.2, `TS_USERSPACE=true`) on the compose network `fa`. The `app` container is not on the tailnet, so an AI endpoint set to a `100.x` address or a MagicDNS name cannot be reached. Only the LAN IP works. The app uses reqwest 0.13.5 with `default-features = false, features = ["rustls", "json", "gzip", "http2", "query", "form"]` (workspace `Cargo.toml`), so no `socks` feature.

## Findings

### 1. Env vars (containerboot, v1.104.2)
- `cmd/containerboot/settings.go`: `SOCKSProxyAddr: os.Getenv("TS_SOCKS5_SERVER")` and `HTTPProxyAddr: os.Getenv("TS_OUTBOUND_HTTP_PROXY_LISTEN")`.
- `cmd/containerboot/tailscaled.go`: if non-empty they become `--socks5-server=<v>` and `--outbound-http-proxy-listen=<v>`. Values are `[ip]:port` strings (docs: "`TS_SOCKS5_SERVER=:1055` creates a SOCKS5 proxy on port 1055"; flag help: `[ip]:port`, e.g. `localhost:1080`). Unset means disabled: there is no default listener.
- Docs: https://tailscale.com/docs/features/containers/docker/docker-params (TS_SOCKS5_SERVER, TS_OUTBOUND_HTTP_PROXY_LISTEN, TS_USERSPACE).
- Flags are registered in `cmd/tailscaled/proxy.go` (build tag `!ts_omit_outboundproxy`; the official image does not omit it, and containerboot passes the flags to it).
- Both can use the same `[ip]:port`: `net/proxymux/mux.go` reads the first byte and routes SOCKS5 to one listener and everything else to the HTTP proxy. The kb page (https://tailscale.com/kb/1112/userspace-networking) says this too.

### 2. HTTP proxy semantics and name resolution
- `cmd/tailscaled/proxy.go`, `httpProxyHandler`: a non-CONNECT request must have an absolute-URL `RequestURI` (else 400 "must be absolute URL or CONNECT") and is forwarded by an `httputil.ReverseProxy` whose transport dials with `dialer.UserDial`. So plain `http://` works (absolute-form), and so does CONNECT (`dialer(ctx, "tcp", r.RequestURI)`).
- SOCKS5: `net/socks5/socks5.go` parses address type `domainName` and passes the name to `Dialer: dialer.UserDial`. So names are resolved by tailscaled (socks5h behaviour).
- `net/tsdial/tsdial.go`, `UserDial` -> `userDialResolveAll`: it first tries `resolveAddr` (MagicDNS and names in the netmap), then falls back to a normal resolver lookup. Both proxies reach it, so MagicDNS names resolve for plain HTTP, CONNECT and SOCKS5. No client-side DNS is needed.

### 3. Non-tailnet destinations
`dialOneUser` in `tsdial.go`: netstack for tailnet IPs (`UseNetstackForIP`); if a route table is set, tailnet routes use the peer dialer and everything else uses `SystemDial` (the host/container's own interface); with no table, a standard `net.Dialer`. So other destinations are dialed directly from the sidecar, not refused. A proxy used for the LLM client only would not change other traffic anyway. (The route-table branch was read, not exercised.)

### 4. Binding and security
- No default bind: the listener exists only if the env var is set. `localhost:1055` (as in the kb examples) is reachable only inside the sidecar's own network namespace, so `app` cannot use it. `:1055` or `0.0.0.0:1055` binds every interface; `net.Listen("tcp", addr)` in `proxy.go` takes any `[ip]:port`. The sidecar already has the static address 172.30.89.10 on `fa`, so `TS_OUTBOUND_HTTP_PROXY_LISTEN=172.30.89.10:1055` binds only the compose interface (the app then uses `http://tailscale:1055` or the IP).
- There is no authentication on either proxy. Any container on `fa` (or anything that can reach that port) can dial any tailnet node and the open internet through the sidecar.
- Mitigations: keep `fa` a private bridge and do not publish the port; bind to the one interface; give the sidecar's node a tag (auth key with `tag:`) and use tailnet ACLs so that tag may reach only the Ollama host and port (the proxy dials as the node, so node ACLs apply; reasoned, not tested); list only trusted services on `fa`. A compose `internal: true` network would also stop the direct internet path, but the app and sidecar need outbound access for other features, so it is not a drop-in.

### 5. reqwest 0.13.5
- docs.rs features page: `socks` exists and is not default; defaults are `charset`, `default-tls`, `http2`, `system-proxy`. Our workspace disables defaults, so env proxy detection (`system-proxy`) is off, which is good here: only an explicit `Proxy` applies.
- https://docs.rs/reqwest/0.13.5/reqwest/struct.Proxy.html: `Proxy::all(url) -> Result<Proxy>`, `Proxy::http`, `Proxy::https`, and `Proxy::custom(Fn(&Url) -> Option<U>)` all exist without extra features for `http://` proxy URLs. "By enabling the socks feature it is possible to use a socks proxy" (so `socks5://` and `socks5h://` need the feature; `socks5h` is not named on the page I read, treat as unverified).
- `ClientBuilder::proxy(...)` per client. Because the LLM client is separate (`fa-llm`), a proxy set only on it leaves connectors (IMAP, calendar, weather) alone.
- Recommended without the `socks` feature: use the HTTP proxy. For a `http://` Ollama URL, reqwest sends the request to the proxy; for `https://` it uses CONNECT. Both are supported by the Tailscale handler.

### 6. Recommendation and trade-offs
Smallest change:
1. Sidecar: add `TS_OUTBOUND_HTTP_PROXY_LISTEN: 172.30.89.10:1055` (or `${TS_PROXY_LISTEN:-...}`) to the `tailscale` service. No image or network change; `app` already shares `fa`.
2. App: add an optional proxy URL (default empty = direct), configurable in the AI settings UI, with a suggested value `http://tailscale:1055` when the Tailscale profile is on. Build the LLM `reqwest::Client` with `.proxy(Proxy::all(url)?)` when set. Per-endpoint is more flexible (LAN endpoints stay direct); a global value plus a per-endpoint "use proxy" toggle covers both. `Proxy::custom` can instead route only hosts ending `.ts.net` or inside `100.64.0.0/10` and send others direct, so one global setting is safe for mixed endpoints.
3. Add a compose comment and README note (the profile is optional; ACL tag advice).

Alternatives:
- Kernel networking (`TS_USERSPACE=false`, `NET_ADMIN`, `/dev/net/tun`) with `app` on `network_mode: service:tailscale`: the whole app goes on the tailnet transparently, no app code, but it needs privileges, couples the app's network to the sidecar (port publishing and `fa` service names then go through the sidecar's namespace), and breaks `TS_USERSPACE=true` simplicity (D-065, D-077). Not verified here.
- Tailscale on the host: the app container reaches tailnet IPs only if routed via the host (`network_mode: host` or an `extra_hosts`/route trick), MagicDNS is not visible inside containers by default, and it is a manual step per install. Simplest for people who already run Tailscale on the host, but not self-contained.
- Do nothing: use the LAN IP (works today); fails only when the Ollama host is off-LAN.

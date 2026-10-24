# Deploying Family Assistant

`compose.example.yaml` contains every service, grouped by profile. How you deploy and update is your choice (D-033). We provide versioned images, migrations that run on start and stay backward compatible within a minor version, and release notes.

| Profile | Services |
|---|---|
| default | `postgres`, `api`, `worker`, `ntfy`, `backup` |
| `ai-cpu` | `s1-small`, `embed` (placeholders until M4) |
| `gpu` | `ollama`, `s1-large` (placeholder until M4); needs the NVIDIA container runtime |

## Prerequisites
- Docker with the compose plugin.
- Tailscale installed **on the host** (not in a container), so `tailscale serve` can reach localhost.
- A backup target: a NAS mount (NFS/SMB) or any directory.
- For the `gpu` profile: an NVIDIA GPU, driver, and nvidia-container-toolkit.

## Configure
```sh
cp deploy/.env.example deploy/.env      # edit every value
head -c 32 /dev/urandom > fa_master_key && chmod 400 fa_master_key
```
Set `FA_MASTER_KEY_FILE` to that file. It is mounted as the docker secret `/run/secrets/fa_master_key`.

**Back up the master key offline** (password manager or paper), separately from the database backups. It is never in the DB or the dumps. Without it the encrypted secrets (for example OAuth tokens) cannot be recovered. You would have to re-authorise each account.

**Outbound network:** both `api` and `worker` need outbound HTTPS. The worker syncs calendars and fetches the forecast (Open-Meteo, Google, your ICS and CalDAV servers). The api tests and discovers sources when you add them and completes the Google OAuth exchange.

## Feeds and Google settings
All of these are optional and go in `deploy/.env` (see `.env.example` for the defaults):

| Variable | Default | Meaning |
|---|---|---|
| `FA_PUBLIC_URL` | `FA_PUBLIC_BASE_URL` | Base of the Google OAuth redirect: `<FA_PUBLIC_URL>/api/v1/oauth/google/callback`. Must be https |
| `FA_GOOGLE_CLIENT_ID`, `FA_GOOGLE_CLIENT_SECRET` | unset | The OAuth client below. When unset, Google is hidden in the UI (ICS and CalDAV still work) |
| `FA_OPEN_METEO_URL` | `https://api.open-meteo.com` | Weather provider base URL |
| `FA_ALLOW_HTTP_FEEDS` | `false` | Accept plain `http` for feeds (for example a CalDAV server on the LAN) and for the OAuth redirect |
| `FA_HOST_API_PORT`, `FA_HOST_NTFY_PORT` | `8080`, `8090` | Host ports (on 127.0.0.1) that the compose file publishes |

## Google OAuth client
This is the one-time setup (human step H-05) that lets the app read Google calendars. Skip it if you only use ICS or CalDAV.
1. In the [Google Cloud console](https://console.cloud.google.com/), create a project (or reuse one).
2. Enable the **Google Calendar API** (APIs & Services, Library).
3. Configure the OAuth consent screen: user type External, publishing status **Testing**, and add every Google account you will connect as a **test user**.
4. Create credentials, **OAuth client ID**, application type **Web application**. Under *Authorised redirect URIs* add exactly `https://<host>.<tailnet>.ts.net/api/v1/oauth/google/callback` (your `FA_PUBLIC_URL`, plus that path).
5. Put the client ID and secret in `deploy/.env` as `FA_GOOGLE_CLIENT_ID` and `FA_GOOGLE_CLIENT_SECRET`, set `FA_PUBLIC_URL` if it differs from `FA_PUBLIC_BASE_URL`, and restart: `docker compose --env-file deploy/.env -f deploy/compose.example.yaml up -d`.
6. In the app, open Settings, Calendars, **Connect a Google account**.

The app asks only for the read-only calendar scope. While the consent screen is in **Testing** mode, Google expires the refresh token after **7 days**. The source then shows *needs reauth*: use Reconnect. Moving the consent screen to *In production* avoids this (Google may require verification for sensitive scopes), and so does adding the calendar through its secret iCal address as an ICS source (slower to refresh, no OAuth).

## Layouts
**Single machine:** run everything on one host: `docker compose --env-file deploy/.env -f deploy/compose.example.yaml --profile gpu up -d` (omit `--profile gpu` if there is no GPU).

**Two machines:**
- Core node (always on): the default profile, plus `ai-cpu` from M4.
- GPU node: `docker compose --env-file deploy/.env -f deploy/compose.example.yaml --profile gpu up -d ollama`, with `OLLAMA_BIND` set so the core can reach it. `fa-gpu-agent` runs natively with `deploy/systemd/fa-gpu-agent.service` (a stub until M4; a container variant is unsupported). Wake-on-LAN needs both machines on the same LAN.

## Start
```sh
docker compose --env-file deploy/.env -f deploy/compose.example.yaml up -d
```
Images come from `ghcr.io/<owner>/fa-server:${FA_VERSION}`. To build locally use `up -d --build`.

## Tailscale
On the core host (HTTPS on the tailnet; nothing is exposed publicly):
```sh
tailscale serve --bg --https=443 http://127.0.0.1:8080     # the app
tailscale serve --bg --https=8443 http://127.0.0.1:8090    # ntfy
```
Set `FA_PUBLIC_BASE_URL` to the app URL and `NTFY_BASE_URL` to the ntfy URL.

## First run
```sh
alias fa='docker compose --env-file deploy/.env -f deploy/compose.example.yaml exec api fa-server'
fa config import /path/in/container/household.toml   # mount or `docker cp` your TOML first
fa user add --member <member-key> --username <name>   # prompts for the password
fa device pair --name kitchen-display                 # prints the token and pairing URL
```
Check `https://<host>.<tailnet>.ts.net/readyz`.

Then open the app, set the household location (Settings, Household) and add calendars (Settings, Calendars). To show the dashboard on a screen, pair it: Settings, Devices, then open the pairing link on the screen (an Echo Show uses its browser), or run `fa device pair --name echo-show` and open the printed URL.

## Updates
Your choice (D-033): manual `docker compose pull && up -d`, Watchtower, or anything else. Pin `FA_VERSION` to a release tag and read the release notes first. Migrations run when the api starts.

## Backups and restore
The `backup` service writes nightly custom-format dumps to `${BACKUP_DIR}` (keeping 7 daily, 4 weekly, 6 monthly). See `runbooks/restore.md` and `runbooks/rotate-key.md`.

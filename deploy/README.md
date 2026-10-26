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
| `FA_NTFY_URL`, `FA_NTFY_TOKEN` | `http://ntfy:80`, unset | See [ntfy notifications](#ntfy-notifications) |
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

## Mail
Mail ingest (M3) reads one or more mailboxes, sorts the mail, and proposes events, bills and tasks that wait for approval in the **Inbox**. Add a mailbox in **Settings, Mail**, either through the Gmail API or over IMAP. Nothing is written to a mailbox or a calendar until a person approves it, and a source is **read only** unless you switch that off.

### Gmail API (human step H-10)
Reuses the Google OAuth client of [Google OAuth client](#google-oauth-client). Do this once, then connect each account.
1. In the same Google Cloud project, enable the **Gmail API** (APIs & Services, Library).
2. OAuth consent screen, **Data access / Scopes**: add `https://www.googleapis.com/auth/gmail.readonly` (read only mail) or `https://www.googleapis.com/auth/gmail.modify` (needed to label, archive or mark mail read), and `https://www.googleapis.com/auth/calendar.events` (only if approved events should be written to a Google calendar). The app asks for them one at a time (*incremental consent*), only when you press the matching button, so granting `gmail.readonly` first and `gmail.modify` later works.
3. Keep every account you will connect listed as a **test user**.
4. In the app, **Settings, Mail, Add a mail source, Gmail**: if the account has no Gmail access yet, press **Connect Gmail (read only)** or **Connect Gmail (read and archive)**, approve in Google, then pick the account and add the source.
5. **Testing mode re-consent:** the Gmail scopes are *restricted* scopes. While the consent screen is in *Testing* mode, Google expires the refresh token after **7 days**: the source then shows *needs reauth*. Reconnect the Google account (**Settings, Calendars, Reconnect** next to the account; it is the same account as for calendars). The *needs reconnecting* notification (Settings, Notifications) tells the owners. Publishing the app avoids this but needs Google's verification and a security assessment for restricted scopes, which a private household project normally does not want; use IMAP with an app password instead if the weekly reconnect is a burden.

### IMAP with an app password
Works with any provider, never expires, and needs no Google client.
1. For Gmail: turn on **2-step verification** on the Google account, then create an **app password** (Google Account, Security, 2-Step Verification, App passwords). Other providers: create an app-specific password if the provider offers one.
2. **Settings, Mail, Add a mail source, IMAP.** Server `imap.gmail.com`, port `993` (typing a `@gmail.com` address pre-fills both), the full address as user name, the app password.
3. **Folder to read:** `INBOX` by default. For Gmail, set the archive folder to `[Gmail]/All Mail` so archived mail is still read (and the archive action has somewhere to go). Press **Test connection**: it logs in, lists the folders and says whether Gmail extensions (real labels) are available.
4. IMAP on Gmail uses real Gmail labels (`FA/School` and so on). On other servers labels are stored as IMAP keywords (`FA_...`), which some mail apps show as tags.
5. The password is encrypted with the master key and never shown again, and a TOML export never contains it. Plain (unencrypted) IMAP is refused: the app connects with TLS (port 993) or STARTTLS. Only tests may turn that off (`FA_ALLOW_PLAIN_IMAP`, `FA_IMAP_TLS_INSECURE`, see `.env.example`; compose does not pass them through).

### Read only or writable
A source starts **read only**: the app only reads. Turn *Read only* off on the source to let it act, and, for Gmail, reconnect with *read and archive* (`gmail.modify`). Writable sources get the automatic **labels** (`FA/<category>`, `FA/<member>`) and whatever else a category's *automatic actions* list says (archive, mark read), and manual actions on a message. Every action can be **undone for 30 days**. Approving an event writes to the calendar you pick, which must be on a writable calendar source: a CalDAV source with *Allow writing (approved events)* on, or a Google source connected with calendar write access. Mark one calendar per member (or one for the household) as **Default target**.

### Privacy and retention
- Message metadata is always kept. Message **content** follows **Settings, Household, Mail and approvals, Keeping email content**: keep everything, keep for learning (the default: bodies are purged after processing except for messages kept as labelling examples), or purge after processing. Unprocessed mail is marked expired after the maximum pending days.
- Sources are visible to their **owners**, or to every adult (*household*). **Children and dashboards never list or read mail.** A dashboard shows only the counts of what waits in the inbox, and approving from a dashboard needs an adult's PIN elevation.
- Mail never leaves your machines: classification is rule based in M3, nothing is sent to a third party. Mail text is not written to the logs.
- Everything extracted waits for approval in M3: events, bills that are not clearly bills, suggested tasks and new contacts. *Household, Mail and approvals* has the switches (automatic writes of high-confidence events are off by default).

### Steps to do after deploying M3 (H-10, H-11)
1. **H-10:** the Gmail setup above, or an IMAP source with an app password. Update the stack first (`docker compose ... pull && ... up -d`; the migration runs when the api starts).
2. **H-11:** add the household's schools, clubs and billers under **Settings, Organizations** (domains, teachers or coaches, bill patterns), each child's **enrollments** (grade, class, teachers, also under Organizations), connect one mailbox **read only**, then look at the **Inbox** and the **Mail, Label mail** view for a week and label about 200 messages. Those labels become the evaluation sample for the later AI milestone.

## ntfy notifications
Notifications (assigned tasks, overdue, the daily digest, routine nudges, calendar re-connect) go through [ntfy](https://ntfy.sh/), a self-hosted push service. The default profile runs it as the `ntfy` service (pinned to `binwiederhier/ntfy:v2.17.0`), and `api` and `worker` publish to it over the compose network.

| Variable | Default | Meaning |
|---|---|---|
| `FA_NTFY_URL` | `http://ntfy:80` | Where fa-server publishes. Plain `http` is fine on the compose network (this is independent of `FA_ALLOW_HTTP_FEEDS`). Set it empty to turn notifications off: the app then says so under Settings, Notifications |
| `FA_NTFY_TOKEN` | unset | Bearer token for publishing, needed only with the access control below |
| `NTFY_BASE_URL` | `http://localhost:8090` | The address phones use for ntfy (the `tailscale serve` URL). ntfy puts it in links |
| `FA_PUBLIC_BASE_URL` | none | Also the base of the click links in notifications, so tapping one opens the app |

`notify.send` runs on the worker every minute. A message waits at least a minute so several updates become one ("3 updates"), and nothing is sent during a member's quiet hours.

**1. Expose ntfy on the tailnet** (the compose file publishes it only on `127.0.0.1:8090`):
```sh
tailscale serve --bg --https=8443 http://127.0.0.1:8090
```
Set `NTFY_BASE_URL=https://<host>.<tailnet>.ts.net:8443` in `deploy/.env` and restart (`up -d`).

**2. Phone setup** (each person, on their own phone):
1. Install the ntfy app (Android: Google Play or F-Droid; iOS: App Store) and join the tailnet.
2. In the app, open Settings and set the default server to the ntfy URL above. Skip this if you subscribe with the link in the next step.
3. In Family Assistant open **Settings, Notifications** (choose your own name; adults can also choose a child) and either open the subscribe address on the phone or scan the QR code. The address is `<ntfy URL>/<your topic>`.
4. Press **Send test**; a notification should arrive within seconds.

The topic is a random 24-character name and acts as the password: anyone who knows it can read that member's notifications, so don't share the subscribe address. The API shows a topic only to that member and to adults, and `config export` leaves topics out.

**3. Optional: access control.** Topic secrecy is enough on a private tailnet. If you want ntfy itself to check who reads and writes, enable its authentication and give each phone a login:
```sh
# in deploy/.env (or the ntfy service's environment): NTFY_AUTH_FILE and NTFY_AUTH_DEFAULT_ACCESS
# are the commented lines in compose.example.yaml. Uncomment both, then `up -d`.
alias ntfy='docker compose --env-file deploy/.env -f deploy/compose.example.yaml exec ntfy ntfy'
ntfy user add fa-server                      # prompts for a password
ntfy access fa-server '*' write-only         # may publish everywhere, read nothing
ntfy token add --label fa-server fa-server   # prints tk_...; put it in deploy/.env as FA_NTFY_TOKEN
ntfy user add alex                           # one login per person
ntfy access alex <alex-topic> read-only      # the topic from Settings, Notifications
```
With `auth-default-access: deny-all`, nobody else can read or publish. In the ntfy app, add the login under Settings, Users, and subscribe as before. Restart `api` and `worker` after setting `FA_NTFY_TOKEN`. The app does not manage ntfy users.

**4. Human step H-08 (after deploying M2):**
1. Set `FA_NTFY_URL` (the default is right for the compose file), plus `FA_NTFY_TOKEN` if you enabled access control, and `NTFY_BASE_URL`.
2. `docker compose ... up -d`, then check Settings, Notifications shows a subscribe address and no "not configured" note.
3. Install the ntfy app on each phone (on the tailnet) and subscribe with the link or QR code.
4. Press **Send test** on every phone. From the shell: `fa notify test --member <member-key>` prints the subscribe URL too.

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

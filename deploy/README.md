# Install Family Assistant on a home server

This guide takes you from an empty Linux machine to Family Assistant running in Docker, reachable from
your family's phones, with calendars, mail and notifications connected. You don't need to know Rust or
Docker. You type the commands shown, and fill in a few values along the way.

The app is private: it's reachable only through [Tailscale](https://tailscale.com/), a free private
network between your own devices. The one exception is optional: a wall screen that can't run
Tailscale can show the dashboard over your home network ([7.5](#75-the-wall-screen-dashboard)).
**Never expose it to the public internet** (no port forwarding on your router).

**Contents**
1. [What you need](#1-what-you-need)
2. [Install Docker and Tailscale](#2-install-docker-and-tailscale)
3. [Give the server an HTTPS address](#3-give-the-server-an-https-address)
4. [Get the code and configure it](#4-get-the-code-and-configure-it)
5. [Start it](#5-start-it)
6. [Describe your household and create logins](#6-describe-your-household-and-create-logins)
7. [Connect phones, calendars, mail and the wall screen](#7-connect-phones-calendars-mail-and-the-wall-screen)
8. [Backups, updates and troubleshooting](#8-backups-updates-and-troubleshooting)
9. [Reference](#9-reference)

---

## 1. What you need
- **A Linux machine that is always on**, such as a mini PC, an old laptop, a NAS that runs Docker, or a VM.
  - **Architecture:** 64-bit (x86-64 or ARM64).
  - **Memory:** at least **2 GB of RAM** to run it.
  - **Building:** the app is built on the machine the first time. That needs about **4 GB of RAM** and
    **15 GB of free disk**, and can take half an hour on a small machine. If the server is too small,
    build somewhere else; see [Build on another computer](#build-on-another-computer).
  - The commands below are for Ubuntu or Debian. Other distributions work the same once Docker is installed.
- **A Tailscale account** (the free plan is enough). Every phone, tablet and screen that opens the app needs the
  Tailscale app.
- **Optional:**
  - a NAS folder for backups;
  - a Google Cloud account, only if you want to sign in with Google for calendars or Gmail
    ([7.4](#74-google-sign-in-optional)). It's free, but takes 15 minutes to set up.

Everything you need to fill in is listed here, so you can collect it first:

| Value | Where it comes from | Needed for |
|---|---|---|
| Database password | you make it up (step 4) | always |
| The server's tailnet address | Tailscale (step 3) | always |
| Google client ID and secret | Google Cloud console ([7.4](#74-google-sign-in-optional)) | only for Google sign-in |
| Calendar links or app passwords | each calendar provider ([7.2](#72-calendars)) | calendars |
| Mail app passwords | each mail provider ([7.3](#73-mail)) | mail |

## 2. Install Docker and Tailscale
Log in to the server (directly or over SSH) and run:
```sh
# Docker Engine with the compose plugin (official convenience script)
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker "$USER"      # lets you run docker without sudo; log out and back in afterwards

# Tailscale
curl -fsSL https://tailscale.com/install.sh | sh
sudo tailscale up                    # prints a link: open it and sign in to add this machine to your tailnet
```
Check that both work. Log out and back in first, so the docker group applies:
```sh
docker compose version               # prints "Docker Compose version v2..." or later
tailscale status                     # lists this machine
```
Other systems: [Docker install docs](https://docs.docker.com/engine/install/),
[Tailscale install docs](https://tailscale.com/download).

## 3. Give the server an HTTPS address
Tailscale gives each machine a name such as `homeserver.tail1234.ts.net`, and can serve real HTTPS
certificates for it.
1. Open the [Tailscale admin console, DNS page](https://login.tailscale.com/admin/dns). Make sure
   **MagicDNS** is on, then scroll down and press **Enable HTTPS** (HTTPS Certificates).
2. On the server, forward HTTPS to the two ports the app will use: 443 for the app and 8443 for
   notifications. Nothing listens on them yet, which is fine.
   ```sh
   sudo tailscale serve --bg --https=443  http://127.0.0.1:8080
   sudo tailscale serve --bg --https=8443 http://127.0.0.1:8090
   sudo tailscale serve status
   ```
3. `serve status` prints the address, for example `https://homeserver.tail1234.ts.net`. **Write it
   down**: it is your `FA_PUBLIC_BASE_URL` in the next step.

These settings survive reboots. Visiting the address before the app runs gives an error page, which
is expected.

## 4. Get the code and configure it
```sh
git clone https://github.com/JonGanz/family-assistant.git
cd family-assistant/deploy
cp .env.example .env
```
**All the `docker compose` commands in this guide run from this `deploy/` folder.**

Open `.env` in an editor (`nano .env`; save with Ctrl+O, Enter, then exit with Ctrl+X) and fill in the
three values under *Required*:

| Setting | What to put |
|---|---|
| `POSTGRES_PASSWORD` | a long random password. Run `openssl rand -hex 24` and paste the output. You never need to type it again |
| `FA_PUBLIC_BASE_URL` | the address from step 3, such as `https://homeserver.tail1234.ts.net` (no slash at the end) |
| `NTFY_BASE_URL` | the same address with `:8443` added, such as `https://homeserver.tail1234.ts.net:8443` |

The rest has working defaults. Backups go to `deploy/backups/`; point `BACKUP_DIR` at a NAS
folder if you have one (see [Backups](#backups)). Every setting is explained in the file.

**Create the master key.** It encrypts the passwords and sign-ins you add later:
```sh
mkdir -p secrets && chmod 700 secrets
head -c 32 /dev/urandom > secrets/fa_master_key
chmod 444 secrets/fa_master_key      # the app runs as its own user and must read it; the folder keeps others out
```
**Back up the master key now, away from the server.** It is never in the database or in the backups.
Without it, a restored backup has every calendar and mail sign-in locked, and you would have to add them
all again. Copy it into a password manager:
```sh
base64 secrets/fa_master_key         # prints one line of text; store it as a secure note
```
(To recreate the file from that line: `echo '<the line>' | base64 -d > secrets/fa_master_key`.)

## 5. Start it
```sh
docker compose up -d --build
```
The first run builds the app, which takes a while. Later starts take seconds. Then check:
```sh
docker compose ps                    # postgres, api, worker, ntfy and backup "Up"; api "(healthy)" after ~30 s
curl -fsS http://127.0.0.1:8080/readyz && echo   # prints "ready"
```
Now open the address from step 3 on a device that has Tailscale. It shows the login page. You can't
log in yet, because there are no accounts.

If something is not up, `docker compose logs api` shows why (see [Troubleshooting](#troubleshooting)).

### Build on another computer
If the server is too small to build on, build the image on a bigger computer with the same
architecture (x86-64 or ARM64) and copy it over:
```sh
# on the bigger computer, in a checkout of the repository
docker build -t ghcr.io/jonganz/fa-server:latest .
docker save ghcr.io/jonganz/fa-server:latest | ssh you@homeserver docker load
```
Then start on the server with `docker compose up -d`, without `--build`. Published release images
(set `FA_VERSION` to a release tag in `.env`, then `docker compose pull`) skip building entirely.

## 6. Describe your household and create logins
The household (its name, time zone, location and members) is loaded once from a small text file.
Create `deploy/household.toml` (`nano household.toml`) and adapt it:
```toml
[household]
name = "The Example Family"
tz = "America/New_York"          # your time zone, from https://en.wikipedia.org/wiki/List_of_tz_database_time_zones
location = { label = "Home", lat = 40.0, lon = -75.0 }   # for the weather; any point in your town will do
units = "metric"                 # or "imperial"
currency = "USD"                 # for bills

[[members]]
key = "alex"                     # a short id, lowercase, used in commands below
display_name = "Alex"
role = "adult"

[[members]]
key = "riley"
display_name = "Riley"
role = "child"
grade = "3"                      # optional; helps link school mail to the right child
aliases = ["Ri"]                 # optional nicknames used in mail
```
Add one `[[members]]` block per person. [`config/examples/household.example.toml`](../config/examples/household.example.toml)
shows every option: interests, enrollments, schools and billers, chores and more. You can also
add all of that later in the app. The file holds no passwords, and it is git-ignored.

Load it, then create a login for each person who will use the app:
```sh
alias fa='docker compose exec api fa-server'      # a shortcut for this shell session

docker compose cp household.toml api:/tmp/household.toml
fa config import /tmp/household.toml              # prints what it created; safe to run again after edits

fa user add --member alex  --username alex        # asks for a password (adults: 12+ characters)
fa user add --member riley --username riley       # children: 4+ characters
fa pin set  --member alex                         # a 4-6 digit PIN, used to approve things on the wall screen
```
Log in at your tailnet address with `alex` and the password. If you forget a password, run
`fa user reset-password --username alex`.

In the app, open **Settings, Household** to check the time zone, location and units, and **Members** to
add colours, interests and responsibilities. These are what the rest of the app works from.

## 7. Connect phones, calendars, mail and the wall screen
Everything in this step happens in the app, except the provider-side setup, which this section
explains for each provider. Do the parts you need, in any order.

### 7.1 Phones and notifications
For each person's phone:
1. **Tailscale.** Install the Tailscale app ([Android](https://play.google.com/store/apps/details?id=com.tailscale.ipn),
   [iPhone](https://apps.apple.com/app/tailscale/id1470499037)) and sign in to your tailnet. Children's
   phones can sign in with a parent's account. Open the tailnet address in the browser and log in.
   On Android, *Add to Home screen* installs it like an app.
2. **ntfy** (notifications). Install the ntfy app ([Android](https://play.google.com/store/apps/details?id=io.heckel.ntfy),
   [iPhone](https://apps.apple.com/app/ntfy/id1625396347)).
3. In Family Assistant open **Settings, Notifications**. It shows that person's *subscribe address* and
   a QR code. Scan the QR code with the phone, or in the ntfy app press **+**, choose *Use another
   server*, enter the `NTFY_BASE_URL` address, and type the topic name shown.
4. Press **Send test**. A notification should arrive within seconds.

**iPhones:** a self-hosted ntfy can wake the iOS app only through ntfy.sh. Set
`NTFY_UPSTREAM_BASE_URL=https://ntfy.sh` in `.env` and run `docker compose up -d`. Only a message ID
goes through ntfy.sh, never the text.

Each person's topic is a random name that works like a password, so don't share it. Quiet hours,
batching and which events notify are set per person on the same page.

### 7.2 Calendars
Add calendars in **Settings, Calendars, Add a calendar source**. There are three kinds:
- **ICS link:** a read-only subscription address. It works with almost every provider, and is the easiest.
- **CalDAV:** a server, a user name and a password. It is read and write, so approved events can be added to it.
- **Google:** sign in with Google. It needs [7.4](#74-google-sign-in-optional) first.

After adding a source, tick which family members each calendar belongs to (none means the whole
household), and press **Sync now** to see events right away.

| Your calendar | How to add it | Where to get what it asks for |
|---|---|---|
| **Google Calendar** (easy) | ICS link | Google Calendar on the web: the gear, **Settings**, pick the calendar on the left, **Integrate calendar**, copy **Secret address in iCal format**. Google refreshes these slowly (hours) |
| **Google Calendar** (full) | Google | [7.4](#74-google-sign-in-optional), then **Connect a Google account**. Refreshes every 15 minutes and can receive approved events |
| **Apple iCloud** | CalDAV | Server `https://caldav.icloud.com`; user name: your Apple Account email; password: an **app-specific password**. Create it at [account.apple.com](https://account.apple.com), **Sign-In and Security, App-Specific Passwords**. Needs two-factor authentication on the account |
| **Outlook.com / Microsoft 365** | ICS link | Outlook on the web: **Settings, Calendar, Shared calendars, Publish a calendar**, pick the calendar and *Can view all details*, **Publish**, copy the **ICS** link. (Microsoft offers no CalDAV.) |
| **Nextcloud** | CalDAV | Server `https://<your-nextcloud>/remote.php/dav`; your user name; an app password from **Personal settings, Security, Devices & sessions, Create new app password** |
| **Fastmail** | CalDAV | Server `https://caldav.fastmail.com`; your Fastmail address; an app password from **Settings, Privacy & Security, App passwords** with CalDAV access |
| **School, club, team, holidays** | ICS link | Look for *Subscribe*, *iCal*, *ICS* or *Add to calendar* on their website and copy that link. `webcal://` links work as they are |
| A CalDAV server on your LAN (`http://`) | CalDAV | Set `FA_ALLOW_HTTP_FEEDS=true` in `.env` and run `docker compose up -d` first |

**Approved events need a writable calendar.** Mail can turn into proposed events, and approving one
writes it to a calendar. ICS links are read only. For a CalDAV source, switch on *Allow writing
(approved events)*. A Google source does this through Google sign-in. Then mark one calendar per member,
or one for the household, as **Default target**.

Links and passwords are stored encrypted with the master key, and are never shown again or exported.

### 7.3 Mail
Mail is read in the background. Each message is sorted (school, bills, activities, newsletters…),
linked to the family member it's about, and turned into **proposals**: events, bills, tasks and
new teacher contacts. Proposals wait in the **Inbox** for an adult to approve. Nothing is written
to your mailbox or your calendars until a person approves it.

**Before adding a mailbox**, open **Settings, Organizations** and add your children's schools and
clubs (their email domains, such as `school.example.edu`), the teachers and coaches, and the billers you
pay. Then add each child's **enrollment** (school, grade and class). That's what links "Grade 3 field
trip" mail to the right child.

Add a mailbox in **Settings, Mail, Add a mail source**. **IMAP with an app password** works with
most providers and never expires:

| Provider | Server, port | Password |
|---|---|---|
| **Gmail / Google Workspace** | `imap.gmail.com`, `993` (typing a `@gmail.com` address fills these in) | An **app password**: turn on [2-Step Verification](https://myaccount.google.com/signinoptions/twosv), then create one at [myaccount.google.com/apppasswords](https://myaccount.google.com/apppasswords). Workspace admins can turn app passwords off |
| **iCloud Mail** | `imap.mail.me.com`, `993` | An app-specific password, as for iCloud calendars above |
| **Yahoo Mail** | `imap.mail.yahoo.com`, `993` | **Account security, Generate app password** |
| **Fastmail** | `imap.fastmail.com`, `993` | An app password with IMAP access |
| **Outlook.com / Hotmail / Microsoft 365** | not supported yet | Microsoft no longer accepts passwords over IMAP, only its own sign-in |
| **Others** | your provider's IMAP server, port `993` | an app password if the provider offers them, else your mail password |

The user name is your full email address. Press **Test connection**: it logs in, lists the folders and
says whether Gmail labels are available. For Gmail, set the archive folder to `[Gmail]/All Mail` so
archived mail is read too. If Gmail says IMAP is off, switch it on under **Gmail settings, Forwarding
and POP/IMAP**.

Gmail can also be added through **Google sign-in** instead ([7.4](#74-google-sign-in-optional)). That
avoids app passwords, but while your Google project is in *Testing* mode Google signs you out every
**7 days**, and you then press *Reconnect*. Most households are happier with IMAP.

**Read only or writable.** A new source is **read only**. Switch *Read only* off on the source to let the
app tidy the mailbox: it labels mail (`FA/School`, `FA/Riley`), and archives or marks it read if a
category says so. Every action can be **undone for 30 days**. For Gmail through Google sign-in,
reconnect with *read and archive* too.

**Privacy.** Mail never leaves your server: sorting is done by rules on the server, and nothing is sent
to an outside service. The mail text is kept according to **Settings, Household, Mail and approvals**:
keep everything, keep only labelling examples (the default), or delete it after processing. Each
source is visible to its owners or to all adults. **Children and wall screens never see mail**: a
screen shows only how many items wait in the inbox.

**The first week.** Look at the **Inbox** and at **Mail, Label mail** each day. Labelling about 200
messages (one tap each when the guess is right) teaches you how the rules behave. It also gives the
AI features planned for the next release a set to be measured against.

### 7.4 Google sign-in (optional)
This sets up a Google Cloud "OAuth client", which lets the app sign in to Google Calendar and Gmail
for you. You need it only for the **Google** calendar kind or Gmail through Google sign-in: ICS links
and IMAP work without it. It's free, and takes about 15 minutes, once.
1. Open the [Google Cloud console](https://console.cloud.google.com/) with your Google account. Create a
   project with the project picker at the top, **New project**, named for example `family-assistant`.
2. **APIs & Services, Library:** search for and **Enable** the **Google Calendar API**. Enable the
   **Gmail API** too if you want Gmail through Google sign-in.
3. **Google Auth Platform**. The console may call this the *OAuth consent screen*.
   - Press **Get started**. Fill in the app name and your email, choose **External** as the audience,
     and finish.
   - **Audience:** leave it in **Testing**, and under **Test users** add every Google account you will
     connect. Accounts that are not listed get "Access blocked".
   - **Data access, Add or remove scopes:** add `.../auth/calendar.readonly`, plus
     `.../auth/calendar.events` if approved events should be written to Google.
   - For Gmail, also add `.../auth/gmail.readonly`, or `.../auth/gmail.modify` to let the app label
     and archive. The app asks for each permission only when you press the matching button.
4. **Clients, Create client:** application type **Web application**. Under **Authorised redirect URIs**
   add your address followed by `/api/v1/oauth/google/callback`, exactly, for example
   `https://homeserver.tail1234.ts.net/api/v1/oauth/google/callback`. Create it, then **copy the client
   ID and the client secret straight away**: Google may not show the secret again.
5. Put them in `deploy/.env`:
   ```sh
   FA_GOOGLE_CLIENT_ID=1234567890-abc.apps.googleusercontent.com
   FA_GOOGLE_CLIENT_SECRET=GOCSPX-...
   ```
   and apply them with `docker compose up -d`.
6. In the app: **Settings, Calendars, Connect a Google account**. For Gmail, use **Settings, Mail, Add a
   mail source, Gmail**, then **Connect Gmail (read only)** or **(read and archive)**.

**The 7-day sign-out.** While the project is in *Testing*, Google ends the sign-in after 7 days. The
source then shows *needs reauth*, and the owners get a notification. Press **Reconnect** under
**Settings, Calendars**. Publishing the project removes the limit, but Gmail permissions then need a
Google security review, which a family project doesn't want. For a sign-in that never expires, use a
calendar's secret iCal address and IMAP instead.

### 7.5 The wall screen (dashboard)
The dashboard is a full-screen page for a kitchen tablet or a screen on the wall. It shows the clock,
the weather, the agenda, today's tasks and the inbox count. In **Settings, Devices** press **Pair a
device** and open the pairing link (or scan the QR code) on the screen. The screen remembers itself,
and **Edit layout** picks its panels, orientation and theme. Approving or ticking off something on the
screen asks for an adult's PIN (step 6).

A screen that runs **Tailscale** (an Android tablet, or a small PC or Raspberry Pi showing a browser in
full-screen "kiosk" mode) just opens the pairing link. That's the most secure setup.

**A screen without Tailscale (an Amazon Echo Show)** can use a separate **home-network address**
instead. It's off until you turn it on, and it serves the dashboard and nothing else:
- no login page, settings or mail;
- only screens you paired;
- only devices on your home network.

To turn it on:
1. **Give the server a fixed address on your home network.** In your router's settings, find the
   server in the list of connected devices and reserve its IP address (often called *DHCP
   reservation* or *static lease*). On the server, `hostname -I` prints the address. The first one is
   usually the LAN address, such as `192.168.1.10`.
2. **Add two lines to `deploy/.env`** with that address:
   ```sh
   FA_LAN_DASHBOARD_HOST_IP=192.168.1.10
   FA_LAN_DASHBOARD_URL=http://192.168.1.10:8081
   ```
   Then run `docker compose up -d`.
3. **Pair the screen.** Under **Settings, Devices**, pairing now also shows a *home-network address*
   and its QR code. On the Echo Show, open the **Silk** browser, type that address (it's long, so
   type carefully, or pair with `fa device pair --name echo-show`, which prints it), and
   bookmark the page that opens.

What stays protected, and what to know:
- **Who can connect.** Only devices on private home-network addresses can connect. To allow only the
  screen, set `FA_LAN_ALLOWED_CIDRS=<screen's IP>/32`, after giving the screen a fixed address in the
  router too.
- **No encryption.** The connection isn't encrypted (plain `http`). Someone already on your Wi-Fi who
  watches the traffic could copy the screen's token, and see what the screen shows. If a screen is
  lost or you suspect that, **Revoke** it under **Settings, Devices**.
- **PIN entry is off** on this address, because a PIN typed there would cross the network
  unencrypted. Approvals and other adult actions are then done from a phone. Ticking off tasks on the
  screen still works, if the household allows it (**Settings, Household**). To allow PIN entry
  anyway, set `FA_LAN_ALLOW_PIN=true`.
- **Firewall.** Docker opens the port itself, and a host firewall such as `ufw` does not block it.
  Never forward port 8081 on your router.

## 8. Backups, updates and troubleshooting
### Backups
The `backup` service dumps the database **every night** into `BACKUP_DIR`, keeping 7 daily, 4 weekly
and 6 monthly copies. Take one now to check it works:
```sh
docker compose exec backup /backup.sh
ls -R backups/                      # or your BACKUP_DIR
```
**Keep backups somewhere other than the server's own disk**, for example on a NAS. Mount the NAS folder
on the server (NFS or SMB), set `BACKUP_DIR=/mnt/nas/fa-backups` in `.env`, and run `docker compose up -d`.
The backup container writes as root: on an NFS share with root squashing, make the folder writable for
everyone (`chmod 777`), or back up locally and copy the files to the NAS.

A backup and the master key together restore everything:
[`runbooks/restore.md`](runbooks/restore.md). Rehearse it once with
[`scripts/restore-rehearsal.sh`](../scripts/restore-rehearsal.sh).

### Updates
```sh
docker compose exec backup /backup.sh   # a backup first
git pull
docker compose up -d --build
```
The database is upgraded automatically when the api starts. Read the release notes before updating. If
you use published images instead of building, set `FA_VERSION` to the new release tag and run
`docker compose pull && docker compose up -d`.

### Troubleshooting
| What you see | What to do |
|---|---|
| `docker compose` says *no configuration file provided* | You are not in the `deploy/` folder, or `.env` is missing (step 4) |
| *required variable POSTGRES_PASSWORD is missing* (or another variable) | Fill it in `.env` |
| `api` keeps restarting; `docker compose logs api` mentions the master key | The key file is missing or unreadable: redo *Create the master key* in step 4, then `docker compose up -d` |
| The address from step 3 doesn't load | The device must have Tailscale on. Check `sudo tailscale serve status` on the server and `curl http://127.0.0.1:8080/readyz` |
| The login page reloads without logging in | Use the `https://…ts.net` address, not `http://<server-ip>:8080`: the session cookie only works over HTTPS |
| The weather shows nothing | Set the location in **Settings, Household**; the forecast updates every 30 minutes |
| Google says **Error 400: redirect_uri_mismatch** | The redirect URI in Google Cloud must equal `FA_PUBLIC_BASE_URL` + `/api/v1/oauth/google/callback`, exactly |
| Google says **Access blocked** or *has not completed verification* | Add the account under **Audience, Test users** ([7.4](#74-google-sign-in-optional)) |
| A Google source shows *needs reauth* every week | The 7-day sign-out of Testing mode; press **Reconnect**, or switch to the iCal address or IMAP |
| IMAP **Test connection** fails | Check the server and port `993`, use the full email as user name, and use an **app password**, not your normal password |
| No notifications on a phone | The phone needs Tailscale on; `NTFY_BASE_URL` must be the `:8443` address; press **Send test**. On iPhone see [7.1](#71-phones-and-notifications) |
| The Echo Show can't open the home-network address | Check `FA_LAN_DASHBOARD_HOST_IP` is the server's current LAN address (`hostname -I`) and you ran `docker compose up -d`. *This address is not allowed* means the screen is outside `FA_LAN_ALLOWED_CIDRS` |
| `api` won't start: *cannot assign requested address* | The server's LAN address changed: reserve it in the router, or update `FA_LAN_DASHBOARD_HOST_IP` |
| The wall screen says *PIN entry is turned off* | Expected on the home-network address; approve from a phone, or see [7.5](#75-the-wall-screen-dashboard) |
| The backup command fails with *permission denied* | Make `BACKUP_DIR` writable (see [Backups](#backups)) |

Logs: `docker compose logs -f api worker` (Ctrl+C stops following). Restart everything:
`docker compose restart`. Stop everything: `docker compose down`. This keeps the data; **never add `-v`**,
which deletes the database.

## 9. Reference
### Services and profiles
[`compose.example.yaml`](compose.example.yaml) has every service, grouped by profile. Images are
versioned, and migrations run on start and stay backward compatible within a minor version (D-033).

| Profile | Services |
|---|---|
| default | `postgres` (not reachable from outside), `api` (web app, on `127.0.0.1:8080`; the optional home-network dashboard on `FA_LAN_DASHBOARD_HOST_IP:8081`), `worker` (background jobs: syncing, mail, notifications), `ntfy` (on `127.0.0.1:8090`), `backup` |
| `ai-cpu` | `s1-small`, `embed` (placeholders until M4) |
| `gpu` | `ollama`, `s1-large` (placeholder until M4); needs an NVIDIA GPU, its driver and nvidia-container-toolkit |

**One machine** runs everything: `docker compose --profile gpu up -d` (omit `--profile gpu` without a
GPU). **Two machines:** the always-on server runs the default profile, and a GPU machine runs
`docker compose --profile gpu up -d ollama` with `OLLAMA_BIND` set so the server can reach it.
`fa-gpu-agent` runs natively there with [`systemd/fa-gpu-agent.service`](systemd/fa-gpu-agent.service)
(a stub until M4). Wake-on-LAN needs both machines on the same LAN.

Both `api` and `worker` need outbound HTTPS: calendars, the weather (Open-Meteo), Google and your mail
server.

### Settings
Every setting is described in [`.env.example`](.env.example). The full list, including the ones the
compose file sets itself, is in the main README's
[Configuration reference](../README.md#configuration-reference). To apply a change to `.env`, run
`docker compose up -d`.

### ntfy access control (optional)
On a private tailnet the secret topic names are enough. If you want ntfy itself to require a login:
uncomment `NTFY_AUTH_FILE` and `NTFY_AUTH_DEFAULT_ACCESS` in `compose.example.yaml`, run
`docker compose up -d`, then:
```sh
alias ntfy='docker compose exec ntfy ntfy'
ntfy user add fa-server                      # asks for a password
ntfy access fa-server '*' write-only         # may publish everywhere, read nothing
ntfy token add --label fa-server fa-server   # prints tk_...; put it in .env as FA_NTFY_TOKEN
ntfy user add alex                           # one login per person
ntfy access alex <alex-topic> read-only      # the topic from Settings, Notifications
docker compose up -d                         # applies FA_NTFY_TOKEN
```
In the ntfy app, add the login under **Settings, Users**. The app does not manage ntfy users.

### Gmail permissions in detail
The app asks for Google permissions one at a time (*incremental consent*), only when you press the
matching button:

| Permission | Asked when |
|---|---|
| `calendar.readonly` | **Connect a Google account** |
| `calendar.events` | approved events are written to a Google calendar |
| `gmail.readonly` | **Connect Gmail (read only)** |
| `gmail.modify` | **Connect Gmail (read and archive)**, which allows labelling, archiving and marking read |

Plain (unencrypted) IMAP is always refused. The app connects with TLS (port 993) or STARTTLS. The test
switches `FA_ALLOW_PLAIN_IMAP` and `FA_IMAP_TLS_INSECURE` are not passed through by the compose file.

### Other runbooks
- [`runbooks/install.md`](runbooks/install.md): this guide as a checklist.
- [`runbooks/restore.md`](runbooks/restore.md): restoring a backup.
- [`runbooks/rotate-key.md`](runbooks/rotate-key.md): replacing the master key (planned).

### Human steps of the milestones
| Step | Where in this guide |
|---|---|
| H-02 to H-04 (deploy, Tailscale, backups) | steps 2 to 5, [Backups](#backups) |
| H-05 to H-07 (Google client, calendars, the wall screen) | [7.4](#74-google-sign-in-optional), [7.2](#72-calendars), [7.5](#75-the-wall-screen-dashboard) |
| H-08, H-09 (notifications, chores) | [7.1](#71-phones-and-notifications), then **Settings, Tasks** in the app |
| H-10, H-11 (mail, organizations, a week of labelling) | [7.3](#73-mail) |

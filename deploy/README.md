# Install Family Assistant on a home server

This guide takes you from an empty Linux machine to Family Assistant running in Docker, reachable from
your family's phones, with calendars, mail and notifications connected. You don't need to know Rust or
Docker. You type the commands shown, and click through a setup wizard in the browser.

**Contents**
1. [What you need](#1-what-you-need)
2. [Install in three steps](#2-install-in-three-steps)
3. [Remote access with Tailscale](#3-remote-access-with-tailscale)
4. [Connect phones, calendars, mail and the wall screen](#4-connect-phones-calendars-mail-and-the-wall-screen)
5. [Backups and the secrets volume](#5-backups-and-the-secrets-volume)
6. [Updates](#6-updates)
7. [Troubleshooting](#7-troubleshooting)
8. [Advanced](#8-advanced)
9. [Recovery and the command line](#9-recovery-and-the-command-line)
10. [Reference](#10-reference)

**Never expose the app to the public internet** (no port forwarding on your router). It is meant for
your home network and, for phones away from home, for [Tailscale](https://tailscale.com/), a free
private network between your own devices.

---

## 1. What you need
- **A Linux machine that is always on**, such as a mini PC, an old laptop, a NAS that runs Docker, or a VM.
  - **Architecture:** 64-bit x86-64. The release is built for x86-64 only; on ARM64 (a Raspberry Pi, for
    example) build the image yourself ([8.3](#83-build-the-image-locally)).
  - **Memory:** at least **2 GB of RAM** to run it. Building locally needs about **4 GB** and **15 GB of free
    disk**.
- **Docker Engine with Compose 2.23.1 or newer** (`docker compose version`).
- **Optional:**
  - a Tailscale account (the free plan is enough), for phones away from home ([3](#3-remote-access-with-tailscale));
  - a NAS folder for backups ([5](#5-backups-and-the-secrets-volume));
  - a Google Cloud account, only for Google sign-in ([4.4](#44-google-sign-in-optional)).

## 2. Install in three steps
**1. Get Docker.** On Ubuntu or Debian:
```sh
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker "$USER"      # lets you run docker without sudo; log out and back in afterwards
docker compose version               # must print v2.23.1 or later
```
Other systems: [Docker install docs](https://docs.docker.com/engine/install/).

**2. Download the compose file.** It is one file, and nothing in it needs editing:
```sh
mkdir family-assistant && cd family-assistant
curl -fsSLO https://raw.githubusercontent.com/JonGanz/family-assistant/<release-tag>/deploy/compose.yaml
```
Use the tag of the release you want in place of `<release-tag>`. Or clone the repository and work in
its `deploy/` folder. **All the `docker compose` commands in this guide run in the folder that holds
`compose.yaml`.**

**3. Start it, then finish setup right away.**
```sh
docker compose up -d
docker compose ps                    # init exited (0); postgres, app, ntfy and backup "Up"; app "(healthy)" after ~30 s
curl -fsS http://127.0.0.1:8080/readyz && echo   # prints "ready"
```
Open `http://<server>:8080` in a browser (the server's address on your home network, or
`http://localhost:8080` on the machine itself). The **setup wizard** opens.
**Finish it right away.** Until the first adult account exists, the first visitor who reaches the page
can run the setup, and there is no code to stop them.

On the first start, the `init` service creates a random **master key** and a **database password** in a Docker
volume (`fa-secrets`). You don't see or type either. Nothing is overwritten on later starts.

The wizard asks for:
1. **Welcome:** the address it detected, and whether you are on plain HTTP or HTTPS.
2. **Household:** name, time zone, location (for the weather), units, currency.
3. **You:** the first adult's name, username, a password (12+ characters) and an optional PIN. The PIN
   approves things on the wall screen.
4. **Family:** the other adults and children, each with an optional login and PIN (children: 4+ character
   passwords).
5. **Access and notifications:** the address people use to reach the app, and the notifications (ntfy)
   address, with a **Send test** button.
6. **Master key backup:** the wizard shows the key as one line of text. **Copy or download it and store it
   somewhere safe, away from the server** (a password manager). It is never in the database or in the
   backups, so without it a restored backup has every calendar and mail sign-in locked, and you would add
   them all again. You must tick "I stored it somewhere safe" to continue. **Settings, System** warns
   until you do.
7. **Start:** links to calendars, mail and devices.

Plain HTTP on a home network is a supported mode: login, the wall screen, pairing and notifications all
work over it. Traffic on your Wi-Fi is not encrypted, though. For HTTPS, and for reaching the app from
outside, see [3](#3-remote-access-with-tailscale).

**Optional settings.** Every compose variable has a default. To change ports, the version or the backup
folder, copy [`.env.example`](.env.example) to `.env` next to `compose.yaml`, uncomment what you want and run
`docker compose up -d`. For example, the app and ntfy ports are `FA_HTTP_PORT` (8080) and `FA_NTFY_PORT`
(8090), and `FA_HTTP_HOST_IP=127.0.0.1` keeps both local to the server.

## 3. Remote access with Tailscale
The Tailscale **profile** adds a small Tailscale container (a *sidecar*) that joins your tailnet and serves
the app over real HTTPS at `https://<name>.<tailnet>.ts.net`, and ntfy on the same name with port `:8443`.
Without the profile, nothing Tailscale-related runs.

1. In the [Tailscale admin console, DNS page](https://login.tailscale.com/admin/dns) make sure **MagicDNS**
   is on, and press **Enable HTTPS** (HTTPS Certificates).
2. In the [keys page](https://login.tailscale.com/admin/settings/keys) create an **auth key**. A reusable one
   is handy; it is needed on the first start only.
3. Start with the profile. `TS_HOSTNAME` is the machine name on your tailnet (default `family`):
   ```sh
   TS_AUTHKEY=tskey-auth-... TS_HOSTNAME=family docker compose --profile tailscale up -d
   ```
   (or put both in `.env`, as in [`.env.example`](.env.example)). Later, add `--profile tailscale` to every
   `docker compose` command, or set `COMPOSE_PROFILES=tailscale` in `.env`.
4. Open `https://family.<tailnet>.ts.net` on a device that is on the tailnet. The address is shown in the
   Tailscale admin console under *Machines*. Run the wizard there, or, if you already ran it over
   `http://<server>:8080`, fill in **Settings, System, Tailscale address** with it.
5. Install the Tailscale app on each phone and sign in to your tailnet ([4.1](#41-phones-and-notifications)).

The app trusts the sidecar's `X-Forwarded-*` headers (the compose file gives it a fixed address and lists
it in **Trusted proxies**), so the session cookie is marked `Secure` exactly when the request came in over HTTPS.
Plain `http://<server>:8080` keeps working alongside.

*This step needs a Tailscale account, an auth key and real devices, so the commands above could not be
exercised in the documentation check; the compose file is validated with the profile.*

## 4. Connect phones, calendars, mail and the wall screen
Everything in this step happens in the app, except the provider-side setup, which this section
explains for each provider. Do the parts you need, in any order. Settings that belong to the whole
installation (addresses, Google client, ntfy, trusted proxies) are under **Settings, System**; see
[10](#10-reference).

### 4.1 Phones and notifications
For each person's phone:
1. **Reach the app.** At home, open `http://<server>:8080`. Away from home, install the Tailscale app
   ([Android](https://play.google.com/store/apps/details?id=com.tailscale.ipn),
   [iPhone](https://apps.apple.com/app/tailscale/id1470499037)), sign in to your tailnet (children's
   phones can use a parent's account), and open the tailnet address ([3](#3-remote-access-with-tailscale)).
   On Android, *Add to Home screen* installs it like an app.
2. **ntfy** (notifications). Install the ntfy app ([Android](https://play.google.com/store/apps/details?id=io.heckel.ntfy),
   [iPhone](https://apps.apple.com/app/ntfy/id1625396347)).
3. **Check the address phones use.** The app sends to ntfy inside Docker (`http://ntfy`, the
   **Notifications server** setting; leave it). Phones subscribe at a different address, **Notifications
   address for phones** in **Settings, System**. Left empty, it is the Tailscale address on `:8443`
   when Tailscale is set up, otherwise this server's address on `:8090` (the `FA_NTFY_PORT` port).
   Set it only if phones reach ntfy some other way, for example after changing `FA_NTFY_PORT`.
4. In Family Assistant open **Settings, Notifications**. It shows that person's *subscribe address* and
   a QR code built from that address. Scan the QR code with the phone, or in the ntfy app press **+**,
   choose *Use another server*, enter the server part of the address, and type the topic name shown.
5. Press **Send test**. A notification should arrive within seconds.

**iPhones:** a self-hosted ntfy can wake the iOS app only through ntfy.sh. Set
`NTFY_BASE_URL` to the address phones reach and `NTFY_UPSTREAM_BASE_URL=https://ntfy.sh` in `.env`, then run
`docker compose up -d`. Only a message ID goes through ntfy.sh, never the text.

Each person's topic is a random name that works like a password, so don't share it. Quiet hours,
batching and which events notify are set per person on the same page.

### 4.2 Calendars
Add calendars in **Settings, Calendars, Add a calendar source**. There are three kinds:
- **ICS link:** a read-only subscription address. It works with almost every provider, and is the easiest.
- **CalDAV:** a server, a user name and a password. It is read and write, so approved events can be added to it.
- **Google:** sign in with Google. It needs [4.4](#44-google-sign-in-optional) first, and HTTPS: over
  plain HTTP the form shows *needs HTTPS (turn on Tailscale)* instead, because Google refuses `http://`
  sign-in addresses.

After adding a source, tick which family members each calendar belongs to (none means the whole
household), and press **Sync now** to see events right away.

| Your calendar | How to add it | Where to get what it asks for |
|---|---|---|
| **Google Calendar** (easy) | ICS link | Google Calendar on the web: the gear, **Settings**, pick the calendar on the left, **Integrate calendar**, copy **Secret address in iCal format**. Google refreshes these slowly (hours) |
| **Google Calendar** (full) | Google | [4.4](#44-google-sign-in-optional), then **Connect a Google account**. Refreshes every 15 minutes and can receive approved events |
| **Apple iCloud** | CalDAV | Server `https://caldav.icloud.com`; user name: your Apple Account email; password: an **app-specific password**. Create it at [account.apple.com](https://account.apple.com), **Sign-In and Security, App-Specific Passwords**. Needs two-factor authentication on the account |
| **Outlook.com / Microsoft 365** | ICS link | Outlook on the web: **Settings, Calendar, Shared calendars, Publish a calendar**, pick the calendar and *Can view all details*, **Publish**, copy the **ICS** link. (Microsoft offers no CalDAV.) |
| **Nextcloud** | CalDAV | Server `https://<your-nextcloud>/remote.php/dav`; your user name; an app password from **Personal settings, Security, Devices & sessions, Create new app password** |
| **Fastmail** | CalDAV | Server `https://caldav.fastmail.com`; your Fastmail address; an app password from **Settings, Privacy & Security, App passwords** with CalDAV access |
| **School, club, team, holidays** | ICS link | Look for *Subscribe*, *iCal*, *ICS* or *Add to calendar* on their website and copy that link. `webcal://` links work as they are |
| A CalDAV server on your LAN (`http://`, a private address) | CalDAV | Turn on **Allow plain-HTTP feeds** and **Allow feeds on private addresses** under **Settings, System** first |

**Approved events need a writable calendar.** Mail can turn into proposed events, and approving one
writes it to a calendar. ICS links are read only. For a CalDAV source, switch on *Allow writing
(approved events)*. A Google source does this through Google sign-in. Then mark one calendar per member,
or one for the household, as **Default target**.

Links and passwords are stored encrypted with the master key, and are never shown again or exported.

### 4.3 Mail
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

Gmail can also be added through **Google sign-in** instead ([4.4](#44-google-sign-in-optional)). That
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

### 4.4 Google sign-in (optional)
This sets up a Google Cloud "OAuth client", which lets the app sign in to Google Calendar and Gmail
for you. You need it only for the **Google** calendar kind or Gmail through Google sign-in: ICS links
and IMAP work without it. It's free, and takes about 15 minutes, once. **Google only accepts `https://`
redirect addresses** (`http://localhost` aside), so this needs [HTTPS](#3-remote-access-with-tailscale)
(the Tailscale profile or your own proxy).
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
   add your public address followed by `/api/v1/oauth/google/callback`, exactly, for example
   `https://family.tail1234.ts.net/api/v1/oauth/google/callback`. Create it, then **copy the client
   ID and the client secret straight away**: Google may not show the secret again.
5. In the app open **Settings, System** and fill in **Google client ID** and **Google client secret**
   (stored encrypted). The app restarts by itself in about a second and the Google options appear.
   The redirect uses the Tailscale address, else the public address.
6. **Settings, Calendars, Connect a Google account**. For Gmail, use **Settings, Mail, Add a
   mail source, Gmail**, then **Connect Gmail (read only)** or **(read and archive)**.

**The 7-day sign-out.** While the project is in *Testing*, Google ends the sign-in after 7 days. The
source then shows *needs reauth*, and the owners get a notification. Press **Reconnect** under
**Settings, Calendars**. Publishing the project removes the limit, but Gmail permissions then need a
Google security review, which a family project doesn't want. For a sign-in that never expires, use a
calendar's secret iCal address and IMAP instead.

### 4.5 The wall screen (dashboard)
The dashboard is a full-screen page for a kitchen tablet or a screen on the wall. It shows the clock,
the weather, the agenda, today's tasks, the family members and, if the household allows it, how many
items wait for each adult's approval (**Settings, Household**). It never shows mail. In **Settings, Devices**
press **Pair a device** and open the pairing link (or scan the QR code) on the screen. The screen
remembers itself, and **Edit layout** picks its panels and theme. Approving or ticking off
something on the screen asks for an adult's PIN (step 3 of the wizard, or **Settings, Members**).

A screen that can open the app's normal address (a tablet on your Wi-Fi, or a device on the tailnet, or
a small PC or Raspberry Pi showing a browser in full-screen "kiosk" mode) just opens the pairing link.

**A screen that can't use Tailscale or reach the normal address (an Amazon Echo Show)** can use a
separate **home-network dashboard address** (D-064). It is off until you turn it on, and it serves the dashboard
and nothing else:
- no login page, settings or mail;
- only screens you paired;
- only devices on your home network.

To turn it on:
1. **Give the server a fixed address on your home network.** In your router's settings, find the
   server in the list of connected devices and reserve its IP address (often called *DHCP
   reservation* or *static lease*). On the server, `hostname -I` prints the address, such as `192.168.1.10`.
2. **Publish the extra port.** The compose file doesn't publish it. Create `compose.override.yaml` next to
   `compose.yaml` (Compose reads it automatically):
   ```yaml
   services:
     app:
       ports:
         - "192.168.1.10:8081:8081"
   ```
   and run `docker compose up -d`.
3. **Tell the app.** In **Settings, System, Home-network dashboard address** enter
   `http://192.168.1.10:8081` (or set `FA_LAN_DASHBOARD_URL` on the `app` service). The app restarts by
   itself in about a second and starts the second listener (`FA_LAN_DASHBOARD_BIND`, inside the container,
   defaults to `0.0.0.0:8081`).
4. **Pair the screen.** Under **Settings, Devices**, pairing now also shows a *home-network address*
   and its QR code. On the Echo Show, open the **Silk** browser, type that address (it's long, so
   type carefully, or run `docker compose exec app fa-server device pair --name echo-show`, which prints it),
   and bookmark the page that opens.

What stays protected, and what to know:
- **Who can connect.** Only devices on private home-network addresses can connect. To allow only the
  screen, set **Home-network dashboard: allowed networks** to `<screen's IP>/32`, after giving the
  screen a fixed address in the router too.
- **No encryption.** The connection isn't encrypted (plain `http`). Someone already on your Wi-Fi who
  watches the traffic could copy the screen's token, and see what the screen shows. If a screen is
  lost or you suspect that, **Revoke** it under **Settings, Devices**.
- **PIN entry is off** on this address, because a PIN typed there would cross the network
  unencrypted. Approvals and other adult actions are then done from a phone. Ticking off tasks on the
  screen still works, if the household allows it (**Settings, Household**). To allow PIN entry
  anyway, switch on **Home-network dashboard: allow PIN elevation**.
- **Firewall.** Docker opens the port itself, and a host firewall such as `ufw` does not block it.
  Binding the port to the server's LAN address (step 2) keeps it off other interfaces. Never forward
  port 8081 on your router.

## 5. Backups and the secrets volume
Two things hold your data, and they are saved differently:

| What | Where it lives | How it is saved |
|---|---|---|
| The database (members, events, tasks, mail, settings, encrypted secrets) | volume `pgdata` | The `backup` service dumps it **every night**, keeping 7 daily, 4 weekly and 6 monthly copies, into `BACKUP_DIR` (default: the volume `fa-backups`) |
| The master key and the database password | volume `fa-secrets` (`master_key`, `db_password`) | **Not** in the backups. Keep the master key's base64 line from the wizard (step 6). The database password is recreated by a fresh install, so it needs no backup |

Take a backup now to check that it works, and list the files:
```sh
docker compose exec backup /backup.sh
docker compose exec backup ls -R /backups
```
**Keep backups somewhere other than the server's own disk**, for example on a NAS. Mount the NAS folder
on the server (NFS or SMB), set `BACKUP_DIR=/mnt/nas/fa-backups` in `.env`, and run `docker compose up -d`.
The backup container writes as root: on an NFS share with root squashing, make the folder writable for
everyone (`chmod 777`), or back up locally and copy the files to the NAS.

**Lost the master key note?** If the server is still running, **Settings, System, Master key backup** shows
it again (it asks for your password). If both the server and the note are gone, the backup still restores,
but each calendar, mail and Google sign-in has to be added again.

To restore, follow [`runbooks/restore.md`](runbooks/restore.md); rehearse it once with
[`scripts/restore-rehearsal.sh`](../scripts/restore-rehearsal.sh).

## 6. Updates
```sh
docker compose exec backup /backup.sh   # a backup first
docker compose pull                     # fetch the new release (set FA_VERSION in .env to pin a tag)
docker compose up -d
```
The database is upgraded automatically when the app starts. Read the release notes before updating.
If you build locally, `git pull` and then run `docker compose -f compose.yaml -f compose.build.yaml up -d --build`
([8.3](#83-build-the-image-locally)).

## 7. Troubleshooting
| What you see | What to do |
|---|---|
| `docker compose` says *no configuration file provided* | You are not in the folder that holds `compose.yaml` |
| *unknown field "content"* or a `configs` error | Docker Compose is older than 2.23.1: update Docker |
| `pull access denied` or *manifest unknown* for `ghcr.io/jonganz/fa-server` | No release image is published for that tag yet: build it locally ([8.3](#83-build-the-image-locally)) |
| `app` keeps restarting; `docker compose logs app` mentions `invalid configuration` | The log names the setting. Fix the variable in `.env` or the `compose.override.yaml` you added |
| `app` can't read the master key | The `init` service didn't run, or `fa-secrets` was replaced: restore the key ([runbooks/restore.md](runbooks/restore.md)) |
| Every page shows *finish the first-run setup first* or opens `/setup` | The setup was never finished: open the app and complete the wizard |
| The address doesn't load from a phone | At home use `http://<server's LAN address>:8080`. Away from home the phone needs Tailscale on ([3](#3-remote-access-with-tailscale)) |
| The login page reloads without logging in | Over the Tailscale profile use the `https://…ts.net` address; if you use your own HTTPS proxy, add its address to **Trusted proxies** ([8.2](#82-your-own-reverse-proxy)) |
| The weather shows nothing | Set the location in **Settings, Household**; the forecast updates every 30 minutes |
| Google says **Error 400: redirect_uri_mismatch** | The redirect URI in Google Cloud must equal your public (or Tailscale) address + `/api/v1/oauth/google/callback`, exactly |
| Google says **Access blocked** or *has not completed verification* | Add the account under **Audience, Test users** ([4.4](#44-google-sign-in-optional)) |
| A Google source shows *needs reauth* every week | The 7-day sign-out of Testing mode; press **Reconnect**, or switch to the iCal address or IMAP |
| A CalDAV or feed address on your LAN is refused | Turn on **Allow feeds on private addresses** (and, for `http://`, **Allow plain-HTTP feeds**) under **Settings, System** |
| IMAP **Test connection** fails | Check the server and port `993`, use the full email as user name, and use an **app password**, not your normal password. IMAP on a private address is not blocked |
| No notifications on a phone | Check that the phone can open the subscribe address in **Settings, Notifications** (set **Notifications address for phones** in **Settings, System** if not), and press **Send test** ([4.1](#41-phones-and-notifications)). On iPhone see the same section |
| A setting in **Settings, System** is greyed out | It is set by an environment variable, which wins over the UI. Remove the variable to edit it there |
| The Echo Show can't open the home-network address | Check the port is published ([4.5](#45-the-wall-screen-dashboard)) and the server's LAN address hasn't changed. *This address is not allowed* means the screen is outside **allowed networks** |
| `app` won't start: *cannot assign requested address* | The `ports:` address in your `compose.override.yaml` is not the server's current LAN address |
| The wall screen says *PIN entry is turned off* | Expected on the home-network address; approve from a phone, or see [4.5](#45-the-wall-screen-dashboard) |
| The backup command fails with *permission denied* | Make `BACKUP_DIR` writable (see [5](#5-backups-and-the-secrets-volume)) |

Logs: `docker compose logs -f app` (Ctrl+C stops following). Restart everything: `docker compose restart`.
Stop everything: `docker compose down`. This keeps the data; **never add `-v`**, which deletes the
database, the backups volume and the master key.

## 8. Advanced
### 8.1 Tailscale on the host
If the server already runs Tailscale, or you don't want the sidecar, use the host's:
```sh
sudo tailscale serve --bg --https=443  http://127.0.0.1:8080
sudo tailscale serve --bg --https=8443 http://127.0.0.1:8090
sudo tailscale serve status          # prints the https://<name>.<tailnet>.ts.net address
```
Enter that address in **Settings, System, Tailscale address**. The proxy reaches the app through Docker's
published port, so the app may see it as the Docker network gateway (`172.30.89.1` in the compose
network) rather than as loopback. If login or the secure cookie misbehaves, add that gateway as a `/32` to
**Trusted proxies**, as in the next section. *(This host-Tailscale path needs a real tailnet and was not exercised in
the documentation check.)*

### 8.2 Your own reverse proxy
Put Caddy, nginx or Traefik in front of port 8080 and let it terminate HTTPS. The proxy must send
`X-Forwarded-For` and `X-Forwarded-Proto`. The app only believes those headers from addresses in
**Trusted proxies** (**Settings, System**, a comma-separated CIDR list; the default is loopback plus the Tailscale sidecar).
Add your proxy's address, for example `192.168.1.5/32`. Then the cookie becomes `Secure` for HTTPS requests,
and login rate limiting sees the real client address. Also set the **Public address** to your HTTPS
address. Live updates are server-sent events on `/api/v1/stream`, so turn off proxy buffering
for that path.

### 8.3 Build the image locally
For developers, for ARM64, or before a release image exists, build from a checkout:
```sh
git clone https://github.com/JonGanz/family-assistant.git
cd family-assistant/deploy
docker compose -f compose.yaml -f compose.build.yaml up -d --build
```
The first build takes a while (about 4 GB of RAM and 15 GB of disk). To build on a bigger computer
with the same architecture and copy the image over:
```sh
docker build -t ghcr.io/jonganz/fa-server:latest .          # in the checkout, on the bigger computer
docker save ghcr.io/jonganz/fa-server:latest | ssh you@homeserver docker load
```
and run `docker compose up -d` on the server (with `FA_VERSION=latest`).

### 8.4 The dashboard-only listener (D-064)
See [4.5](#45-the-wall-screen-dashboard): a second port that serves the wall-screen dashboard to paired
devices on the home network only. Its settings are under **Settings, System**.

### 8.5 Other profiles
`--profile gpu` runs Ollama (a GPU node, or one machine running everything; needs an NVIDIA GPU, its driver and
nvidia-container-toolkit). `--profile ai-cpu` is a placeholder until M4. Profiles combine:
`docker compose --profile tailscale --profile gpu up -d`. For two machines, the GPU machine runs
`docker compose --profile gpu up -d ollama` with `OLLAMA_BIND` set so the server can reach it.
`fa-gpu-agent` runs natively there with [`systemd/fa-gpu-agent.service`](systemd/fa-gpu-agent.service) (a stub
until M4).

## 9. Recovery and the command line
Everyday setup needs no command line. For recovery and scripting, `fa-server` runs inside the app container:
```sh
alias fa='docker compose exec app fa-server'
fa user reset-password --username alex          # forgot a password (asks for a new one)
fa user add --member <name> --username <name>   # an account for an existing member (by display name)
fa pin set --member <name>                      # an adult's PIN
fa device pair --name kitchen                   # prints a pairing link
```
Everything else the wizard does can be done in Settings. If you lose the master key, see
[`runbooks/restore.md`](runbooks/restore.md).

## 10. Reference
### Services and profiles
[`compose.yaml`](compose.yaml) has every service, grouped by profile. Images are versioned, and migrations
run on start and stay backward compatible within a minor version (D-033).

| Profile | Services |
|---|---|
| default | `init` (one-shot: creates the secrets), `postgres` (not reachable from outside), `app` (web app and API on port 8080, and the background jobs: syncing, mail, notifications), `ntfy` (port 8090), `backup` |
| `tailscale` | `tailscale` (the sidecar) |
| `ai-cpu` | placeholders until M4 |
| `gpu` | `ollama` |

The `app` needs outbound HTTPS: calendars, the weather (Open-Meteo), Google and your mail server.

### Settings
Two layers. **Compose variables** ([`.env.example`](.env.example)) change ports, the version and backups; all are
optional. **App settings** are in **Settings, System** (public address, Tailscale address, ntfy, Google client,
plain-HTTP and private-address feeds, trusted proxies, the home-network dashboard). A setting
changed there restarts the app by itself in about a second. A setting given as an environment variable wins and
shows as locked in the UI; `FA_DEFAULT_<KEY>` sets only a default. The full list is in the main README's
[Configuration reference](../README.md#configuration-reference).

### ntfy access control
On a private tailnet the secret topic names are enough. If you want ntfy itself to require a login:
add `NTFY_AUTH_FILE` and `NTFY_AUTH_DEFAULT_ACCESS` to the `ntfy` service in a `compose.override.yaml` (an
advanced step; the defaults need none of it), run `docker compose up -d`, then:
```sh
alias ntfy='docker compose exec ntfy ntfy'
ntfy user add fa-server                      # asks for a password
ntfy access fa-server '*' write-only         # may publish everywhere, read nothing
ntfy token add --label fa-server fa-server   # prints tk_...; paste it into Settings, System, Notifications token
ntfy user add alex                           # one login per person
ntfy access alex <alex-topic> read-only      # the topic from Settings, Notifications
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
- [`runbooks/restore.md`](runbooks/restore.md): restoring a backup and the master key.
- [`runbooks/rotate-key.md`](runbooks/rotate-key.md): replacing the master key (planned).

### Human steps of the milestones
| Step | Where in this guide |
|---|---|
| H-02 to H-04 (deploy, Tailscale, backups) | [2](#2-install-in-three-steps), [3](#3-remote-access-with-tailscale), [5](#5-backups-and-the-secrets-volume) |
| H-05 to H-07 (Google client, calendars, the wall screen) | [4.4](#44-google-sign-in-optional), [4.2](#42-calendars), [4.5](#45-the-wall-screen-dashboard) |
| H-08, H-09 (notifications, chores) | [4.1](#41-phones-and-notifications), then **Settings, Tasks** in the app |
| H-10, H-11 (mail, organizations, a week of labelling) | [4.3](#43-mail) |

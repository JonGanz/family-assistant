import sys; sys.path.insert(0, "tools")
from drawio import Page, write, COMP, EXT, TITLE, PKG, NOTE, NODE, FRAME

# Page 1: Components (UML 2 component diagram of apps/web)
pc = Page("components", "Components", 1400, 850)
b, e = pc.box, pc.edge
b("Frontend: apps/web structure (UML 2 component), DRAFT v4 (M3.5)", 20, 10, 900, 30, TITLE)
b("«subsystem» apps/web (Vue 3 PWA)", 20, 50, 1000, 720, PKG)
router = b("router\nguards: setup | user | adult | device\nsetup needed → /setup", 60, 100, 240, 60, COMP)
setup = b("SetupLayout + setup/\nwizard: welcome · household · you · family · access · key backup · start", 340, 540, 400, 70, COMP)
app = b("AppLayout + views\nToday · Calendar · Tasks · Routines · Mail · Message · Inbox · Labelling · Bills · SettingsLayout: Personal · Household (incl. members) · Sources and rules · System", 340, 100, 640, 60, COMP)
kiosk = b("KioskLayout + dashboard/ (at a glance, no mail)\nlayout engine · panels: clock, weather, advisories (self-clearing), agenda, week, members, now, member_tasks, routine", 340, 200, 640, 60, COMP)
kmod = b("kiosk/\nkeepAlive · watchdog · deviceToken · sse", 60, 200, 240, 60, COMP)
stores = b("stores (Pinia)\nsetup · session · members · household · calendar · weather · sources · devices · dashboard · tasks · mail · inbox · system", 340, 320, 640, 60, COMP)
live = b("composables\nuseLiveStream · useUnits · useRange · useClock", 60, 320, 240, 60, COMP)
client = b("api/client.ts + http.ts\nX-FA-Request · bearer opt-in · If-Match", 340, 440, 400, 60, COMP)
gen = b("gen/api.d.ts\n(generated from openapi.json)", 780, 440, 200, 60, COMP)
sw = b("service worker (vite-plugin-pwa)\nsecure contexts only · shell + avatar cache", 60, 440, 240, 60, COMP)
ls = b("localStorage\ndevice token · dashboard snapshot · fa.theme", 60, 560, 240, 50, COMP)
themes = b("themes/ (D-079..D-081)\ncatalogue as data · applyTheme → CSS tokens\nper-device choice · kiosk: device.settings.theme", 800, 650, 200, 90, COMP)
srv = b("«external» fa-api /api/v1 (REST, SSE)", 1080, 400, 280, 60, EXT)
for s, t, l in [(router, setup, ""), (setup, stores, ""), (router, app, ""), (router, kiosk, ""), (app, stores, ""), (kiosk, stores, ""), (kiosk, kmod, ""),
                (stores, client, ""), (live, stores, "invalidate"), (client, gen, "types"), (kmod, ls, ""), (stores, ls, "snapshot cache"), (themes, ls, "fa.theme"), (app, themes, ""), (kiosk, themes, "")]:
    e(s, t, l)
e(client, srv, "HTTP(S) JSON", False); e(live, srv, "SSE /stream", False)
b("No UI framework: plain CSS tokens, Intl dates in household tz", 780, 560, 220, 70, NOTE)
b("Insecure context (plain HTTP): no service worker, lib/ids instead of randomUUID, lib/copy clipboard fallback, Google shown as needs HTTPS", 340, 640, 440, 70, NOTE)

# Page 2: Deployment of clients
pd = Page("clients", "Clients", 1200, 600)
b, e = pd.box, pd.edge
b("Frontend: clients (UML 2 deployment)", 20, 10, 700, 30, TITLE)
echo = b("«device» Echo Show 15\nSilk browser · /dashboard (kiosk)", 40, 80, 300, 90, NODE)
phone = b("«device» Phones\nPWA · app shell", 40, 220, 300, 90, NODE)
desk = b("«device» Desktop browser\napp shell", 40, 360, 300, 90, NODE)
core = b("«device» Core node (docker compose)\napp :8080 (HTTP on the LAN)\noptional tailscale sidecar: HTTPS :443 → app\noptional D-064 dashboard-only :8081", 600, 200, 360, 120, NODE)
for s, l in [(echo, "HTTP(S) + bearer device token"), (phone, "HTTP(S) + session cookie"), (desk, "HTTP(S) + session cookie")]:
    e(s, core, l, False)

# Page 3: Live update (UML 2 sequence)
ps = Page("live", "Live update", 1400, 700)
ps.box("Frontend: change → dashboard (UML 2 sequence)", 20, 10, 800, 30, TITLE)
ps.box("sd live update", 20, 50, 1340, 600, FRAME)
lanes = ["worker job", "Postgres", "api PgListener + Hub", "/stream (SSE)", "useLiveStream", "dashboard store"]
xs = {}
for i, n in enumerate(lanes):
    x = 60 + i * 215
    ps.box(n, x, 90, 170, 36, "html=1;whiteSpace=wrap;")
    ps.box("", x + 84, 126, 2, 500, "html=1;dashed=1;strokeColor=#999999;")
    xs[n] = x + 80
def msg(a, c, y, label, dashed=False):
    s = ps.box("", xs[a], y, 10, 10, "html=1;fillColor=#dae8fc;")
    t = ps.box("", xs[c], y, 10, 10, "html=1;fillColor=#dae8fc;")
    ps.edge(s, t, label, dashed)
msg("worker job", "Postgres", 160, "write + pg_notify(fa_change) · COMMIT")
msg("Postgres", "api PgListener + Hub", 210, "NOTIFY {household, kind, data}", True)
msg("api PgListener + Hub", "/stream (SSE)", 260, "assign id · ring buffer")
msg("/stream (SSE)", "useLiveStream", 310, "event: calendar.updated {from,to}", True)
msg("useLiveStream", "dashboard store", 360, "invalidate(kind, range)")
msg("dashboard store", "api PgListener + Hub", 410, "GET /api/v1/dashboard (if overlapping)")
msg("dashboard store", "dashboard store", 460, "render · save snapshot to localStorage")
msg("useLiveStream", "/stream (SSE)", 530, "reconnect with Last-Event-ID (or receive resync)", True)

write("arch/Frontend.drawio", [pc, pd, ps])

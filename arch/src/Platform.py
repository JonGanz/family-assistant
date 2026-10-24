import sys; sys.path.insert(0, "tools")
from drawio import Page, write, NODE, COMP, EXT, ART, TITLE, PKG, NOTE, assoc

# Page 1: Packages (crate dependency view)
pk = Page("packages", "Packages", 1400, 800)
b = pk.box
b("Platform: Workspace packages (UML 2 package diagram), DRAFT v3", 20, 10, 900, 30, TITLE)
P = lambda name, x, y, w=170: b(name, x, y, w, 60, PKG)
dom = P("fa-domain", 600, 620)
store = P("fa-store", 380, 500); cfg = P("fa-config", 600, 500); auth = P("fa-auth", 820, 500); jobs = P("fa-jobs", 1040, 500)
feats = b("feature crates\nfa-connectors · fa-rules · fa-pipeline · fa-decide · fa-llm · fa-backends · fa-sysprobe · fa-eval · fa-calendar · fa-tasks · fa-weather · fa-meals · fa-notify · fa-gpu-node",
          380, 340, 830, 90, PKG)
api = P("fa-api", 600, 190)
srv = P("bins/fa-server", 500, 60); agt = P("bins/fa-gpu-agent", 820, 60)
web = P("apps/web (Vue)", 120, 190, 190)
for s, t in [(store, dom), (cfg, dom), (auth, dom), (auth, store), (jobs, store), (feats, dom), (feats, store), (feats, jobs),
             (api, feats), (api, auth), (api, cfg), (srv, api), (srv, jobs), (agt, dom)]:
    assoc(pk, s, t, label="«import»", kind="dependency")
assoc(pk, web, api, label="generated TS client / ts-rs types", kind="dependency")
b("M0 builds: fa-domain, fa-store, fa-config, fa-auth, fa-jobs, fa-api, fa-server, apps/web shell\nM1 adds: fa-connectors, fa-calendar, fa-weather (see Feeds.md)", 20, 620, 400, 70, NOTE)

# Page 2: Deployment (detailed)
dp = Page("deploy", "Deployment", 1400, 900)
b, e = dp.box, dp.edge
b("Platform: Deployment (UML 2): compose.example.yaml profiles, DRAFT v3", 20, 10, 700, 30, TITLE)
b("«device» Core node: Ubuntu Server", 300, 60, 600, 520, NODE)
ts = b("«executionEnvironment» host: tailscaled\n(tailscale serve HTTPS → 127.0.0.1)", 320, 100, 260, 60, ART)
mnt = b("«artifact» /mnt/nas/family-assistant (NFS/SMB)", 620, 100, 260, 50, ART)
b("«executionEnvironment» Docker Compose (deploy/compose.example.yaml: default profile)", 320, 180, 560, 380, NODE)
api = b("«container» api\nfa-server serve api :8080", 340, 230, 200, 60, ART)
wrk = b("«container» worker\nfa-server serve worker", 560, 230, 200, 60, ART)
pg = b("«container» postgres\npgvector/pgvector:pg16", 340, 320, 200, 60, ART)
ntfy = b("«container» ntfy :8090", 560, 320, 200, 60, ART)
bk = b("«container» backup\npostgres-backup-local", 340, 410, 200, 60, ART)
ai = b("«container» s1-small, embed\n(profile: ai-cpu, M4)", 560, 410, 200, 60, ART)
key = b("«secret» fa_master_key", 340, 490, 200, 40, ART)
gpu = b("«device» GPU node(s)", 960, 300, 300, 200, NODE)
gc = b("«compose profile: gpu» ollama, s1-large", 980, 350, 260, 50, ART)
ga = b("«systemd» fa-gpu-agent (native release binary)", 980, 420, 260, 50, ART)
nas = b("«device» NAS", 960, 100, 200, 70, NODE)
cl = b("«device» Clients (Echo Show, phones, desktop)", 20, 200, 240, 80, NODE)
g = b("«external» Google OAuth / APIs", 20, 400, 240, 60, EXT)
e(cl, ts, "HTTPS (tailnet)", False); e(ts, api, "proxy", False); e(ts, ntfy, "proxy", False)
e(api, pg, "SQL", False); e(wrk, pg, "SQL", False); e(bk, pg, "pg_dump", False); e(bk, mnt, "write", False); e(mnt, nas, "NFS/SMB", False)
e(api, key, "read", True); e(wrk, key, "read", True); e(wrk, gc, "HTTP (M4)", False); e(ga, api, "heartbeat (token)", False)
e(wrk, g, "HTTPS", False)

# Page 3: Security — auth contexts
sc = Page("security", "Security", 1400, 700)
b, e = sc.box, sc.edge
b("Platform: Security contexts (UML 2 component), DRAFT v3", 20, 10, 800, 30, TITLE)
u = b("«actor context» User session\n(cookie; adult 30d, child 180d)", 20, 80, 220, 60, COMP)
d = b("«actor context» Dashboard device\n(bearer token, hashed)", 20, 180, 220, 60, COMP)
de = b("«actor context» Device + adult elevation\n(PIN → 5 min)", 20, 280, 220, 60, COMP)
ag = b("«actor context» GPU agent\n(node token)", 20, 380, 220, 60, COMP)
mw = b("«component» fa-auth middleware\n(resolve Actor, CSRF header check, rate limits)", 320, 180, 260, 70, COMP)
pol = b("«component» Policy::authorize(actor, action, resource)", 650, 180, 260, 70, COMP)
rls = b("«component» fa-store: SET app.household_id → Postgres RLS", 980, 180, 260, 70, COMP)
sec = b("«component» Secret store: envelope AES-256-GCM\n(master key file → per-secret DEK)", 650, 340, 260, 70, COMP)
aud = b("«component» AuditEntry (append-only)", 980, 340, 260, 60, COMP)
for s in (u, d, de, ag): e(s, mw, "", True)
e(mw, pol, "Actor"); e(pol, rls, "allowed → query"); e(pol, aud, "privileged actions"); e(rls, sec, "secret columns via")

write("arch/Platform.drawio", [pk, dp, sc])

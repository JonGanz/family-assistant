import sys; sys.path.insert(0, "tools")
from drawio import Page, write, NODE, COMP, EXT, ART, FRAME, TITLE

dep = Page("deploy", "Deployment")
b, e = dep.box, dep.edge
b("Family Assistant: Deployment (UML 2), reference hardware, DRAFT v7", 20, 10, 800, 30, TITLE)
b("«device» Core node: Ubuntu Server (always on)\n«executionEnvironment» Docker Compose", 300, 60, 480, 430, NODE)
srv = b("«artifact» fa-server (api + worker + sysprobe)", 330, 120, 200, 50, ART)
pg = b("«artifact» postgres:16 + pgvector", 560, 120, 200, 50, ART)
nt = b("«artifact» ntfy", 330, 200, 200, 50, ART)
bk = b("«artifact» backup (pg_dump nightly)", 560, 200, 200, 50, ART)
ts = b("«artifact» tailscale (serve HTTPS)", 330, 280, 200, 50, ART)
s1s = b("«artifact» s1-small: System One sidecar (Python, CPU)", 560, 280, 200, 60, ART)
wb = b("«artifact» web PWA bundle", 330, 360, 200, 50, ART)
emb = b("«artifact» embed: embedding model", 560, 360, 200, 50, ART)
gpu = b("«device» GPU node(s) 0..*: ref. Linux PC, RTX 3070 8GB (WoL, on demand)", 300, 530, 480, 150, NODE)
s1l = b("«artifact» s1-large: System One sidecar", 320, 590, 140, 60, ART)
ol = b("«artifact» llm: Ollama 7–8B Q4", 475, 590, 140, 60, ART)
ag = b("«artifact» fa-gpu-agent (hw report)", 630, 590, 135, 60, ART)
nas = b("«device» NAS", 840, 180, 160, 80, NODE)
echo = b("«device» Echo Show 15: Silk\n/dashboard (device token, keep-alive)", 20, 80, 230, 70, NODE)
ph = b("«device» Android / Desktop\nPWA + ntfy app", 20, 200, 230, 70, NODE)
tui = b("«device» Terminal: ratatui (stretch)", 20, 320, 230, 70, NODE)
ext = b("«external» Gmail · Google Cal · CalDAV · ICS · Open-Meteo · Recipe sites", 840, 380, 240, 80, EXT)
for c in (echo, ph, tui): e(c, ts, "HTTPS over tailnet", False)
e(ph, nt, "ntfy subscribe", False)
e(srv, s1s, "/v1/systemone", False); e(srv, emb, "embed", False)
e(srv, s1l, "/v1/systemone (when available)", False); e(srv, ol, "S2 jobs (when available)", False)
e(srv, gpu, "WoL magic packet", True); e(ag, srv, "heartbeat + hardware", False)
e(bk, nas, "SMB/NFS", False); e(srv, ext, "HTTPS", False); e(srv, pg, "SQL", False)

cmp = Page("comp", "Components")
b, e = cmp.box, cmp.edge
b("Family Assistant: Components (UML 2), DRAFT v7", 20, 10, 700, 30, TITLE)
b("«subsystem» fa-server (Rust workspace)", 260, 50, 760, 740, FRAME)
W, H = 220, 46
api = b("«component» fa-api (+RBAC, PIN step-up)", 290, 100, W, H, COMP)
cfg = b("«component» fa-config (import/export)", 530, 100, W, H, COMP)
wrk = b("«component» fa-jobs (+ fa-auth in fa-api)", 770, 100, W, H, COMP)
pip = b("«component» fa-pipeline (decision engine)", 530, 190, W, H, COMP)
con = b("«component» fa-connectors", 770, 190, W, H, COMP)
rul = b("«component» fa-rules", 290, 190, W, H, COMP)
cal = b("«component» fa-calendar", 290, 280, W, H, COMP)
dec = b("«component» fa-decide (System One + kNN)", 530, 280, W, H, COMP)
llm = b("«component» fa-llm (S2)", 770, 280, W, H, COMP)
tsk = b("«component» fa-tasks", 290, 370, W, H, COMP)
bkd = b("«component» fa-backends (registry, suggester)", 530, 370, W, H, COMP)
gpuc = b("«component» fa-gpu-node (WoL, agent state)", 770, 370, W, H, COMP)
mea = b("«component» fa-meals", 290, 460, W, H, COMP)
ev = b("«component» fa-eval (dry-run)", 530, 460, W, H, COMP)
probe = b("«component» fa-sysprobe", 770, 460, W, H, COMP)
wx = b("«component» fa-weather", 290, 550, W, H, COMP)
nt = b("«component» fa-notify", 530, 550, W, H, COMP)
dom = b("«component» fa-domain", 290, 640, W, H, COMP)
st = b("«component» fa-store", 530, 640, W, H, COMP)
web = b("«component» web (Vue 3 + TS)\nincl. Pipeline & Backends settings", 20, 100, 210, 60, COMP)
tui = b("«component» fa-tui", 20, 190, 210, H, COMP)
agent = b("«component» fa-gpu-agent (separate binary)", 20, 640, 210, 60, COMP)
s1x = b("«external» System One sidecars", 1060, 280, 180, H, EXT)
oll = b("«external» Ollama (S2)", 1060, 370, 180, H, EXT)
src = b("«external» Mail/Cal/ICS/Weather/Recipe", 1060, 190, 190, H, EXT)
ntfy = b("«external» ntfy", 1060, 550, 180, H, EXT)
pgx = b("«external» Postgres + pgvector", 1060, 640, 180, H, EXT)
for s, t, l in [(web, api, "«use» REST/SSE"), (tui, api, "«use» REST"), (api, cfg, "«use»"), (api, pip, "settings / dry-run"),
                (wrk, con, "«use»"), (wrk, pip, "run decision points"), (pip, rul, "rules stage"), (pip, dec, "knn / system_one stage"),
                (pip, llm, "llm stage"), (pip, bkd, "backend routing"), (dec, s1x, "/v1/systemone"), (llm, oll, "HTTP"),
                (bkd, gpuc, "availability"), (bkd, probe, "hardware"), (ev, pip, "replay"), (cal, pip, "calendar_route"),
                (tsk, pip, "task.urgency"), (wx, llm, "advisories"), (mea, llm, "recipe fallback"), (cal, nt, "approvals"),
                (tsk, nt, "priority"), (nt, ntfy, "HTTP"), (con, src, "HTTPS"), (dom, st, "«use»"), (st, pgx, "SQLx"),
                (agent, gpuc, "heartbeat + hardware")]:
    e(s, t, l)

write("arch/Overall.drawio", [dep, cmp])

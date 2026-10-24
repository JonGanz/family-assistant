import sys; sys.path.insert(0, "tools")
from drawio import Page, write, NODE, COMP, EXT, ART, FRAME, TITLE

dep = Page("deploy", "Deployment")
b, e = dep.box, dep.edge
b("Family Assistant: Deployment (UML 2), DRAFT v3", 20, 10, 700, 30, TITLE)
b("«device» Core node: Ubuntu Server (always on)\n«executionEnvironment» Docker Compose", 300, 60, 480, 430, NODE)
srv = b("«artifact» fa-server (api + worker)", 330, 120, 200, 50, ART)
pg = b("«artifact» postgres:16 + pgvector", 560, 120, 200, 50, ART)
nt = b("«artifact» ntfy", 330, 200, 200, 50, ART)
bk = b("«artifact» backup (pg_dump nightly)", 560, 200, 200, 50, ART)
ts = b("«artifact» tailscale (serve HTTPS)", 330, 280, 200, 50, ART)
cpu = b("«artifact» ollama-s1: System 1 embed + 1.5–3B (toggle, probed, D-022)", 560, 280, 200, 60, ART)
wb = b("«artifact» web PWA bundle", 330, 360, 200, 50, ART)
gpu = b("«device» GPU node: Linux gaming PC, RTX 3070 8GB (WoL, D-020)", 300, 530, 480, 150, NODE)
ol = b("«artifact» Ollama System 2 (7–8B Q4)", 330, 590, 200, 60, ART)
ag = b("«artifact» fa-gpu-agent (systemd)", 560, 590, 200, 60, ART)
nas = b("«device» NAS", 840, 180, 160, 80, NODE)
echo = b("«device» Echo Show 15: Silk\n/dashboard (device token, keep-alive)", 20, 80, 230, 70, NODE)
ph = b("«device» Android / Desktop\nPWA + ntfy app", 20, 200, 230, 70, NODE)
tui = b("«device» Terminal: ratatui (stretch)", 20, 320, 230, 70, NODE)
ext = b("«external» Gmail · Google Cal · CalDAV · ICS · Open-Meteo · Recipe sites", 840, 380, 240, 80, EXT)
for c in (echo, ph, tui): e(c, ts, "HTTPS over tailnet", False)
e(ph, nt, "ntfy subscribe", False); e(srv, ol, "S2 jobs (HTTP, when available)", False)
e(srv, gpu, "WoL magic packet", True); e(ag, srv, "heartbeat: available | busy_gaming", False)
e(srv, cpu, "S1 embed / classify", False); e(bk, nas, "SMB/NFS", False); e(srv, ext, "HTTPS", False); e(srv, pg, "SQL", False)

cmp = Page("comp", "Components")
b, e = cmp.box, cmp.edge
b("Family Assistant: Components (UML 2), DRAFT v3", 20, 10, 700, 30, TITLE)
b("«subsystem» fa-server (Rust workspace)", 260, 50, 760, 650, FRAME)
W, H = 220, 46
api = b("«component» fa-api (+RBAC policy, PIN step-up)", 290, 100, W, H, COMP)
cfg = b("«component» fa-config (import/export)", 530, 100, W, H, COMP)
wrk = b("«component» fa-worker", 770, 100, W, H, COMP)
pip = b("«component» fa-pipeline", 530, 190, W, H, COMP)
con = b("«component» fa-connectors", 770, 190, W, H, COMP)
cal = b("«component» fa-calendar", 290, 280, W, H, COMP)
tsk = b("«component» fa-tasks", 530, 280, W, H, COMP)
llm = b("«component» fa-llm (tier router S1/S2, kNN)", 770, 280, W, H, COMP)
mea = b("«component» fa-meals", 290, 370, W, H, COMP)
wx = b("«component» fa-weather", 530, 370, W, H, COMP)
nt = b("«component» fa-notify", 770, 370, W, H, COMP)
dom = b("«component» fa-domain", 290, 460, W, H, COMP)
st = b("«component» fa-store", 530, 460, W, H, COMP)
gpuc = b("«component» fa-gpu-node (WoL + agent state)", 770, 460, W, H, COMP)
probe = b("«component» fa-sysprobe", 770, 550, W, H, COMP)
web = b("«component» web (Vue 3 + TS)", 20, 100, 210, H, COMP)
tui = b("«component» fa-tui", 20, 190, 210, H, COMP)
agent = b("«component» fa-gpu-agent (separate binary)", 20, 560, 210, 60, COMP)
oll = b("«external» Ollama S1 (core) / S2 (GPU)", 1060, 280, 170, H, EXT)
ntfy = b("«external» ntfy", 1060, 370, 160, H, EXT)
pgx = b("«external» Postgres", 1060, 460, 160, H, EXT)
src = b("«external» Mail/Cal/ICS/Weather/Recipe", 1060, 190, 190, H, EXT)
for s, t, l in [(web, api, "«use» REST/SSE"), (tui, api, "«use» REST"), (api, cfg, "«use»"), (wrk, con, "«use»"), (wrk, pip, "«use»"),
                (wrk, tsk, "tick"), (wrk, wx, "poll"), (pip, cal, "emit events"), (pip, tsk, "emit tasks/bills"), (pip, llm, "enqueue"),
                (mea, llm, "enqueue"), (wx, llm, "advisories"), (wx, tsk, "weather rules"), (cal, llm, "route hint"), (llm, gpuc, "availability"),
                (llm, oll, "HTTP"), (nt, ntfy, "HTTP"), (cal, nt, "approvals"), (tsk, nt, "priority"), (con, src, "HTTPS"), (st, pgx, "SQLx"),
                (api, cal, "«use»"), (api, mea, "«use»"), (dom, st, "«use»"), (agent, gpuc, "heartbeat API"), (llm, probe, "S1 profile"), (llm, st, "pgvector kNN")]:
    e(s, t, l)

write("arch/Overall.drawio", [dep, cmp])

import sys; sys.path.insert(0, "tools")
from drawio import Page, write, COMP, EXT, TITLE, PKG, NOTE, ENUM, FRAME, cls, assoc

# Page 1: Components (UML 2 component diagram)
pc = Page("components", "Components", 1400, 800)
b, e = pc.box, pc.edge
b("Feeds: calendar read & weather (UML 2 component), DRAFT v1 (M1)", 20, 10, 900, 30, TITLE)
api = b("fa-api\nREST + SSE Hub + PgListener", 520, 60, 260, 60, COMP)
wrk = b("fa-server worker\n(fa-jobs scheduler)", 900, 60, 240, 60, COMP)
cal = b("fa-calendar\nsync · expand · link · visibility · queries", 380, 220, 300, 70, COMP)
wx = b("fa-weather\nfetch · DSL · evaluate · advisories", 760, 220, 280, 70, COMP)
oa = b("fa-auth OAuthService\nGoogle code + PKCE", 60, 220, 260, 70, COMP)
con = b("fa-connectors\ngoogle · caldav · ics · openmeteo (no DB)", 480, 380, 380, 70, COMP)
st = b("fa-store\nM1 repos · ChangeBus (pg_notify)", 480, 520, 380, 60, COMP)
pg = b("PostgreSQL\nchannel fa_change", 520, 650, 300, 50, COMP)
g = b("«external» Google Calendar API / OAuth", 40, 400, 260, 50, EXT)
dav = b("«external» CalDAV server", 40, 470, 260, 40, EXT)
ics = b("«external» ICS feeds (school, sports)", 40, 530, 260, 40, EXT)
om = b("«external» Open-Meteo /v1/forecast", 1000, 400, 260, 50, EXT)
for s, t, l in [(api, cal, "queries"), (api, wx, "rules CRUD, latest"), (api, oa, "start/callback"),
                (wrk, cal, "calendar.sync*"), (wrk, wx, "weather.*"), (cal, con, ""), (wx, con, ""), (oa, con, "token exchange"),
                (cal, st, ""), (wx, st, ""), (oa, st, "Secret"), (st, pg, "SQL + NOTIFY"), (pg, api, "LISTEN fa_change")]:
    e(s, t, l)
for s, t in [(con, g), (con, dav), (con, ics), (con, om)]:
    e(s, t, "HTTPS", False)
b("feature crates never depend on each other; fa-connectors is a leaf client crate", 960, 520, 300, 60, NOTE)

# Page 2: Model (UML 2 class)
pm = Page("model", "Model", 1500, 900)
pm.box("Feeds: M1 tables (UML 2 class), DRAFT v1", 20, 10, 700, 30, TITLE)
pm.box("Calendar (M1 read)", 20, 50, 900, 480, PKG)
hh = cls(pm, "Household (+M1)", ["units: metric|imperial", "calendar_past_days = 30", "calendar_future_days = 400", "weather_retention_days = 7", "private_events_as_busy = true (D-036)"], 40, 90, 240)
oac = cls(pm, "OAuthAccount", ["provider: google", "email, scopes[]", "secret (refresh token)", "state"], 320, 90)
src = cls(pm, "CalendarSource", ["kind: google|caldav|ics", "owner_member?", "url?/username?", "secret? (password | ICS URL)", "poll_minutes", "state, last_synced_at"], 560, 90, 220)
calc = cls(pm, "Calendar", ["remote_id, name, color", "enabled", "sync_state"], 560, 300, 220)
ev = cls(pm, "Event", ["remote_uid, instance_key", "title, location?, description?", "start_at, end_at, all_day", "is_private, cancelled", "origin, status?", "dedupe_key, version"], 300, 300, 220)
mem = cls(pm, "Member", ["display_name, role, color: Hue"], 40, 330)
vis = cls(pm, "Visibility", ["full", "busy"], 40, 430, 120, "enumeration", ENUM)
assoc(pm, oac, src, "0..1", "*"); assoc(pm, src, calc, "1", "*", kind="composition")
assoc(pm, calc, ev, "1", "*", kind="composition"); assoc(pm, calc, mem, "*", "*", "calendar_member")
assoc(pm, ev, mem, "*", "*", "event_member (M3)"); assoc(pm, src, mem, "*", "0..1 owner")
pm.box("Weather (M1)", 940, 50, 540, 480, PKG)
snap = cls(pm, "WeatherSnapshot", ["fetched_at", "provider: open-meteo", "current/hourly/daily jsonb"], 960, 90, 220)
rule = cls(pm, "WeatherRule", ["name, condition (DSL)", "action: alert|task", "task_mode (M2)", "severity", "status, suggested_by"], 1220, 90, 230)
adv = cls(pm, "Advisory", ["text, severity", "valid_from/until", "source: rule|s2", "dedupe_key"], 1220, 300, 230)
assoc(pm, rule, adv, "0..1", "*", "fires"); assoc(pm, adv, snap, "*", "1", "evaluated on", kind="dependency")
pm.box("Platform additions", 20, 550, 900, 160, PKG)
cls(pm, "MemberAvatar", ["mime, bytes ≤ 512 KB", "sha256 (ETag)"], 40, 590)
cls(pm, "Device (+M1)", ["settings jsonb (layout)"], 280, 590)
cls(pm, "OAuthPending", ["state (PK), pkce_verifier", "return_to, expires_at"], 520, 590, 220)

# Page 3: Sync (UML 2 sequence)
ps = Page("sync", "Sync sequence", 1400, 800)
ps.box("Feeds: calendar.sync for one source (UML 2 sequence), DRAFT v1", 20, 10, 800, 30, TITLE)
ps.box("sd calendar.sync", 20, 50, 1340, 700, FRAME)
lanes = ["worker", "fa-calendar", "fa-connectors", "remote server", "fa-store / Postgres", "fa-api Hub"]
xs = {}
for i, n in enumerate(lanes):
    x = 60 + i * 215
    ps.box(n, x, 90, 170, 36, "html=1;whiteSpace=wrap;")
    ps.box("", x + 84, 126, 2, 590, "html=1;dashed=1;strokeColor=#999999;")
    xs[n] = x + 80
def msg(a, c, y, label, dashed=False):
    s = ps.box("", xs[a], y, 10, 10, "html=1;fillColor=#dae8fc;")
    t = ps.box("", xs[c], y, 10, 10, "html=1;fillColor=#dae8fc;")
    ps.edge(s, t, label, dashed)
msg("worker", "fa-calendar", 160, "sync(source)")
msg("fa-calendar", "fa-connectors", 210, "list_calendars(creds)")
msg("fa-connectors", "remote server", 250, "CalendarList / PROPFIND")
msg("fa-calendar", "fa-connectors", 310, "fetch(cal, window, sync_state)")
msg("fa-connectors", "remote server", 350, "events.list syncToken | REPORT | GET If-None-Match")
msg("remote server", "fa-connectors", 390, "items | 304 | 410 (full resync)", True)
msg("fa-calendar", "fa-calendar", 440, "expand RRULE/EXDATE/RECURRENCE-ID (CalDAV, ICS)")
msg("fa-calendar", "fa-store / Postgres", 490, "BEGIN · upsert occurrences · delete unseen in window")
msg("fa-calendar", "fa-store / Postgres", 540, "ChangeBus.publish(calendar.updated) · COMMIT")
msg("fa-store / Postgres", "fa-api Hub", 590, "NOTIFY fa_change → PgListener", True)
msg("fa-calendar", "fa-store / Postgres", 650, "on 401/invalid_grant: source.state = needs_reauth")

write("arch/Feeds.drawio", [pc, pm, ps])

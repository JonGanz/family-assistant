import sys; sys.path.insert(0, "tools")
from drawio import Page, write, COMP, EXT, TITLE, PKG, NOTE, ENUM, cls, assoc

# Page 1: Components (UML 2 component diagram)
pc = Page("components", "Components", 1400, 760)
b, e = pc.box, pc.edge
b("Tasks: lists, recurrence, assignment, routines, notifications (UML 2 component), DRAFT v1 (M2)", 20, 10, 1000, 30, TITLE)
api = b("fa-api\n/tasks /lists /definitions /notify-prefs + SSE", 460, 60, 320, 60, COMP)
wrk = b("fa-server worker\n(fa-jobs scheduler)", 900, 60, 240, 60, COMP)
tk = b("fa-tasks\nscheduling · strategies · policy · Now score · routines", 300, 220, 340, 70, COMP)
nt = b("fa-notify\nNotifier · prefs · quiet hours · outbox", 760, 220, 300, 70, COMP)
wx = b("fa-weather\ntask rules → TaskSink", 40, 220, 220, 70, COMP)
dom = b("fa-domain\nTaskSink · CalendarQuery traits", 40, 380, 220, 50, COMP)
cal = b("fa-calendar\nall-day events (school days)", 40, 560, 220, 60, COMP)
con = b("fa-connectors\nntfy client (no DB)", 760, 380, 300, 60, COMP)
st = b("fa-store\nM2 repos · ChangeBus", 380, 520, 340, 60, COMP)
ntfy = b("«container» ntfy\nbinwiederhier/ntfy", 760, 520, 300, 60, EXT)
ph = b("«external» ntfy Android app\n(member's phone, tailnet)", 1100, 520, 260, 60, EXT)
for s, t, l in [(api, tk, "CRUD, complete"), (api, nt, "prefs, test"), (wrk, tk, "tasks.materialize / tick, routines.materialize"),
                (wrk, nt, "notify.send"), (tk, nt, "enqueue notification"), (wx, dom, "uses TaskSink"), (tk, dom, "implements TaskSink, uses CalendarQuery"), (cal, dom, "implements CalendarQuery"),
                (tk, st, ""), (nt, st, "outbox"), (nt, con, ""), (wx, st, "")]:
    e(s, t, l)
e(con, ntfy, "HTTPS publish + token", False)
e(ntfy, ph, "subscribe", False)
b("feature crates never depend on each other: fa-server wires the TaskSink and CalendarQuery implementations", 300, 640, 360, 60, NOTE)
b("D-037 … D-048; goals (D-048) are not in M2; Q-046 only changes a default", 1100, 220, 260, 60, NOTE)

# Page 2: Model (UML 2 class)
pm = Page("model", "Model", 1500, 1000)
pm.box("Tasks: M2 tables (UML 2 class), DRAFT v1", 20, 10, 700, 30, TITLE)
pm.box("Tasks (M2)", 20, 50, 1000, 680, PKG)
lst = cls(pm, "TaskList", ["kind: chores|homework|general", "completion_policy: assignee|adult_only", "child_can_add: own|none", "dashboard_add", "calendar_default", "claim_ttl"], 40, 90, 260)
dfn = cls(pm, "TaskDefinition", ["title, steps_template[]", "schedule_kind: rrule|after_done", "rrule? / interval?, lead_time", "on_missed: roll_over|auto_skip|pile_up", "strategy, rotation_cursor", "advance_on, skip_advances", "priority_flag", "season_from/until?, season_condition?", "last_done_at?, next_due_at"], 360, 90, 260)
tsk = cls(pm, "Task", ["title, due_at?, visible_from?", "status: open|done|skipped|suggested", "origin: manual|system|weather", "priority_flag", "calendar_display: hidden|until_done|always", "completed_by?, completed_via_device?", "claimed_by?/claimed_until?"], 360, 330, 260)
stp = cls(pm, "TaskStep", ["ord, text", "done_at?, done_by?"], 700, 360)
mem = cls(pm, "Member", ["display_name, role"], 40, 360)
away = cls(pm, "MemberAway", ["from, until, note?"], 40, 480)
strat = cls(pm, "Strategy", ["fixed", "rotation", "pool", "smart (M6)"], 700, 90, 140, "enumeration", ENUM)
assoc(pm, lst, dfn, "1", "*", kind="composition"); assoc(pm, lst, tsk, "1", "*", kind="composition")
assoc(pm, dfn, tsk, "0..1", "*", "occurrences"); assoc(pm, tsk, stp, "1", "*", kind="composition")
assoc(pm, dfn, mem, "*", "1..* {ordered}", "candidates"); assoc(pm, tsk, mem, "*", "*", "assignees")
assoc(pm, mem, away, "1", "*", kind="composition"); assoc(pm, dfn, strat, "", "", kind="dependency")
rt = cls(pm, "Routine", ["name, starts_at, done_by", "day_filter: daily|weekdays|weekends|school_days", "school_calendar?, no_school_match[]", "show_on_dashboard"], 40, 580, 280)
rs = cls(pm, "RoutineStep", ["ord, text, target_time?"], 360, 600)
rr = cls(pm, "RoutineRun", ["date, status", "progress = done/all"], 600, 600)
rrs = cls(pm, "RoutineRunStep", ["done_at?, done_by?", "done_via_device?"], 820, 680, 180)
assoc(pm, rt, rs, "1", "*", kind="composition"); assoc(pm, rt, rr, "1", "*", kind="composition")
assoc(pm, rr, rrs, "1", "*", kind="composition"); assoc(pm, rr, mem, "*", "1")
pm.box("Notify (M2)", 1040, 50, 440, 680, PKG)
pref = cls(pm, "NotifyPref", ["topic (random), token (Secret)", "triggers: assigned, overdue, digest on;", "due_soon, routine_nudge off", "quiet_from?/until? (override)", "copy_to_member?"], 1060, 90, 240)
ntf = cls(pm, "Notification", ["trigger, dedupe_key", "title, body, click_url?", "not_before (quiet hours)", "sent_at?, attempts, last_error?"], 1060, 330, 240)
assoc(pm, pref, ntf, "1", "*", "outbox")
pm.box("Weather (M1, Feeds.md) and Household", 20, 800, 1000, 160, PKG)
rule = cls(pm, "WeatherRule (+M2)", ["action: alert|task", "task_definition?", "task_mode: create|suggest"], 40, 840, 240)
cls(pm, "Household (+M2)", ["now_list_size = 7", "now_weights jsonb", "quiet_from/until = 21:00/07:00"], 320, 840, 240)
assoc(pm, rule, dfn, "*", "0..1", kind="dependency")

# Page 3: Complete (UML 2 sequence) — completing a rotation chore from the dashboard
ps = Page("complete", "Complete sequence", 1400, 700)
ps.box("Tasks: complete a rotation chore on the dashboard (UML 2 sequence), DRAFT v1", 20, 10, 900, 30, TITLE)
ps.box("sd complete", 20, 50, 1340, 600, "shape=umlFrame;whiteSpace=wrap;html=1;width=260;height=24;verticalAlign=top;align=left;spacingLeft=8;")
lanes = ["dashboard", "fa-api", "fa-tasks", "fa-store / Postgres", "fa-notify", "ntfy"]
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
msg("dashboard", "fa-api", 160, "POST /tasks/{id}/complete {member} (device token)")
msg("fa-api", "fa-tasks", 210, "complete(task, member, device)")
msg("fa-tasks", "fa-tasks", 260, "policy check (D-013, D-019 override)")
msg("fa-tasks", "fa-store / Postgres", 310, "BEGIN · task done · cursor per advance_on (D-039) · last_done_at · next_due_at")
msg("fa-tasks", "fa-store / Postgres", 360, "audit · ChangeBus tasks.updated · COMMIT")
msg("fa-tasks", "fa-notify", 420, "outbox: assigned → next member (D-044)")
msg("fa-notify", "ntfy", 480, "notify.send: batched 60 s, after quiet hours")
msg("fa-api", "dashboard", 540, "200 + SSE tasks.updated", True)

write("arch/Tasks.drawio", [pc, pm, ps])

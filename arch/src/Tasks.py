import sys; sys.path.insert(0, "tools")
from drawio import Page, write, COMP, EXT, TITLE, PKG, NOTE, ENUM, cls, assoc

# Page 1: Components (UML 2 component diagram)
pc = Page("components", "Components", 1400, 760)
b, e = pc.box, pc.edge
b("Tasks: lists, recurrence, assignment, notifications (UML 2 component), DRAFT v0 (M2)", 20, 10, 1000, 30, TITLE)
api = b("fa-api\n/tasks /lists /definitions /notify-prefs + SSE", 460, 60, 320, 60, COMP)
wrk = b("fa-server worker\n(fa-jobs scheduler)", 900, 60, 240, 60, COMP)
tk = b("fa-tasks\nrecurrence · strategies · policy · score · routines", 300, 220, 340, 70, COMP)
nt = b("fa-notify\nNotifier · prefs · quiet hours · outbox", 760, 220, 300, 70, COMP)
wx = b("fa-weather\ntask rules → TaskSink", 40, 220, 220, 70, COMP)
dom = b("fa-domain\nTaskSink trait", 40, 380, 220, 50, COMP)
con = b("fa-connectors\nntfy client (no DB)", 760, 380, 300, 60, COMP)
st = b("fa-store\nM2 repos · ChangeBus", 380, 520, 340, 60, COMP)
ntfy = b("«container» ntfy\nbinwiederhier/ntfy", 760, 520, 300, 60, EXT)
ph = b("«external» ntfy Android app\n(member's phone, tailnet)", 1100, 520, 260, 60, EXT)
for s, t, l in [(api, tk, "CRUD, complete"), (api, nt, "prefs, test"), (wrk, tk, "tasks.materialize / tick"),
                (wrk, nt, "notify.send"), (tk, nt, "enqueue notification"), (wx, dom, "uses"), (tk, dom, "implements"),
                (tk, st, ""), (nt, st, "outbox"), (nt, con, ""), (wx, st, "")]:
    e(s, t, l)
e(con, ntfy, "HTTPS publish + token", False)
e(ntfy, ph, "subscribe", False)
b("fa-weather never depends on fa-tasks: fa-server wires fa-tasks in as the TaskSink", 40, 470, 300, 60, NOTE)
b("v0: behaviour marked Q-035 … Q-045 waits for batch 14 answers", 1100, 220, 260, 60, NOTE)

# Page 2: Model (UML 2 class)
pm = Page("model", "Model", 1500, 900)
pm.box("Tasks: M2 tables (UML 2 class), DRAFT v0", 20, 10, 700, 30, TITLE)
pm.box("Tasks (M2)", 20, 50, 1000, 560, PKG)
lst = cls(pm, "TaskList", ["kind: chores|routine|homework|general", "completion_policy: assignee|adult_only", "visible_on_dashboard", "flags (Q-043, Q-044)"], 40, 90, 260)
dfn = cls(pm, "TaskDefinition", ["title, steps_template[]", "schedule: rrule | after_done (Q-035)", "lead_time, on_missed (Q-036)", "strategy: fixed|rotation|pool", "rotation_cursor", "priority_flag, season_window? (Q-041)", "last_done_at?, next_due_at"], 360, 90, 260)
tsk = cls(pm, "Task", ["title, due_at?, visible_from?", "status: open|done|skipped", "origin: manual|system|weather", "priority_flag", "completed_by?, completed_via_device?", "claimed_by?/claimed_until? (Q-038)"], 360, 330, 260)
stp = cls(pm, "TaskStep", ["ord, text", "done_at?, done_by?"], 700, 360)
mem = cls(pm, "Member", ["display_name, role"], 40, 360)
away = cls(pm, "MemberAway (Q-037)", ["from, until, note?"], 40, 480)
strat = cls(pm, "Strategy", ["fixed", "rotation", "pool", "smart (M6)"], 700, 90, 140, "enumeration", ENUM)
assoc(pm, lst, dfn, "1", "*", kind="composition"); assoc(pm, lst, tsk, "1", "*", kind="composition")
assoc(pm, dfn, tsk, "0..1", "*", "occurrences"); assoc(pm, tsk, stp, "1", "*", kind="composition")
assoc(pm, dfn, mem, "*", "1..* {ordered}", "candidates"); assoc(pm, tsk, mem, "*", "*", "assignees")
assoc(pm, mem, away, "1", "*", kind="composition"); assoc(pm, dfn, strat, "", "", kind="dependency")
pm.box("Notify (M2)", 1040, 50, 440, 560, PKG)
pref = cls(pm, "NotifyPref", ["channel: ntfy", "topic, token (Secret)", "triggers[] (Q-042)", "quiet_from?/until?", "copy_to_member?"], 1060, 90, 240)
ntf = cls(pm, "Notification", ["trigger, dedupe_key", "payload", "sent_at?, error?"], 1060, 330, 240)
assoc(pm, pref, ntf, "1", "*", "outbox")
pm.box("Weather (M1, Feeds.md)", 20, 630, 600, 160, PKG)
rule = cls(pm, "WeatherRule", ["action: alert|task", "task_definition?", "task_mode (Q-041)"], 40, 670, 240)

# Page 3: Complete (UML 2 sequence) — completing a rotation chore from the dashboard
ps = Page("complete", "Complete sequence", 1400, 700)
ps.box("Tasks: complete a rotation chore on the dashboard (UML 2 sequence), DRAFT v0", 20, 10, 900, 30, TITLE)
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
msg("fa-tasks", "fa-tasks", 260, "policy check (D-013, override)")
msg("fa-tasks", "fa-store / Postgres", 310, "BEGIN · task done · cursor++ (Q-037) · last_done_at · next_due_at")
msg("fa-tasks", "fa-store / Postgres", 360, "audit · ChangeBus tasks.updated · COMMIT")
msg("fa-tasks", "fa-notify", 420, "enqueue(next assignee, assigned)")
msg("fa-notify", "ntfy", 480, "notify.send (batched, quiet hours)")
msg("fa-api", "dashboard", 540, "200 + SSE tasks.updated", True)

write("arch/Tasks.drawio", [pc, pm, ps])

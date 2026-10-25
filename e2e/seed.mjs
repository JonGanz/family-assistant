// Seeds sources, calendars, a paired device and the first sync through the API (fictional data).
// Env: FA_E2E_BASE_URL, FA_E2E_USER, FA_E2E_PASSWORD, FA_E2E_FIXTURES (http://127.0.0.1:18091),
// FA_E2E_RADICALE (http://127.0.0.1:18092). Writes the ids to the file in argv[2].
import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";

const base = process.env.FA_E2E_BASE_URL;
const fixtures = process.env.FA_E2E_FIXTURES;
const radicale = process.env.FA_E2E_RADICALE;
let cookie = "";

async function call(method, path, body, headers = {}) {
    const res = await fetch(base + path, {
        method,
        headers: {
            "content-type": "application/json",
            "x-fa-request": "1",
            ...(cookie ? { cookie } : {}),
            ...headers,
        },
        body: body === undefined ? undefined : JSON.stringify(body),
    });
    const text = await res.text();
    if (!res.ok) throw new Error(`${method} ${path} -> ${res.status} ${text}`);
    const sc = res.headers.get("set-cookie");
    if (sc) cookie = sc.split(";")[0];
    return {
        json: text ? JSON.parse(text) : null,
        etag: res.headers.get("etag"),
    };
}

await call("POST", "/api/v1/auth/login", {
    username: process.env.FA_E2E_USER,
    password: process.env.FA_E2E_PASSWORD,
});
// Adults manage only their own notification prefs, so Sam needs a session of their own.
const adultCookie = cookie;
await call("POST", "/api/v1/auth/login", {
    username: process.env.FA_E2E_SAM_USER,
    password: process.env.FA_E2E_SAM_PASSWORD,
});
const samCookie = cookie;
cookie = adultCookie;
const members = (await call("GET", "/api/v1/members")).json;
const list = Array.isArray(members)
    ? members
    : (members.items ?? members.members);
const id = (name) => list.find((m) => m.display_name === name).id;

// [source name, new-source body, members of its calendar]
const wanted = [
    [
        "School",
        { kind: "ics", url: `${fixtures}/ics/school.ics` },
        [id("Riley")],
    ],
    ["Standup", { kind: "ics", url: `${fixtures}/ics/dst.ics` }, []],
    ["Adult private", { kind: "ics", url: `${fixtures}/ics/private.ics` }, []],
    [
        "Music",
        { kind: "ics", url: `${fixtures}/ics/jordan.ics` },
        [id("Jordan")],
    ],
    [
        "Family CalDAV",
        {
            kind: "caldav",
            url: `${radicale}/e2e/`,
            username: "e2e",
            password: "e2e",
        },
        [],
    ],
];

const sources = {};
for (const [name, body, who] of wanted) {
    const created = (
        await call("POST", "/api/v1/calendar-sources", {
            ...body,
            name,
            owner: id("Alex"),
        })
    ).json;
    for (const c of created.calendars) {
        await call(
            "PATCH",
            `/api/v1/calendars/${c.id}`,
            { enabled: true, members: who },
            {
                "if-match": `"${c.version}"`,
            },
        );
    }
    await call("POST", `/api/v1/calendar-sources/${created.source.id}/sync`);
    sources[name] = created.source.id;
}

// Wait for the worker to finish every first sync.
const deadline = Date.now() + 90_000;
for (;;) {
    const mine = new Set(Object.values(sources));
    // The example household also imports a disabled "School calendar" source that never syncs.
    const all = (await call("GET", "/api/v1/calendar-sources")).json.filter(
        (s) => mine.has(s.id),
    );
    const pending = all.filter((s) => !s.last_synced_at);
    const failed = all.filter((s) => s.last_error);
    if (failed.length)
        throw new Error(`sync failed: ${JSON.stringify(failed)}`);
    if (!pending.length) break;
    if (Date.now() > deadline)
        throw new Error(`first sync timed out: ${pending.map((s) => s.name)}`);
    await new Promise((r) => setTimeout(r, 1000));
}

// ... and for the weather evaluation (the CLI fetch queued it for the worker).
for (;;) {
    const adv = (await call("GET", "/api/v1/advisories")).json;
    if (adv.length) break;
    if (Date.now() > deadline + 30_000)
        throw new Error("no advisories after the weather fetch");
    await new Promise((r) => setTimeout(r, 1000));
}

// ---- M2: notification prefs, lists, definitions, routine, weather rule (fictional) ----
const NY = "America/New_York";
const localDate = (offsetDays = 0) =>
    new Intl.DateTimeFormat("en-CA", { timeZone: NY }).format(
        new Date(Date.now() + offsetDays * 86_400_000),
    );
const today = localDate();
const byday = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"][
    new Date(`${today}T12:00:00Z`).getUTCDay()
];

// Prefs first: notify.send drops rows of members that have none. Riley's assignments are copied to Alex.
const prefs = {};
for (const name of ["Alex", "Sam", "Riley", "Jordan"]) {
    prefs[name] = (
        await call(
            "GET",
            `/api/v1/members/${id(name)}/notify-prefs`,
            undefined,
            name === "Sam" ? { cookie: samCookie } : {},
        )
    ).json;
}
await call(
    "PUT",
    `/api/v1/members/${id("Riley")}/notify-prefs`,
    { copy_to_member: id("Alex") },
    { "if-match": `"${prefs.Riley.version}"` },
);

const mkList = async (name, kind, extra = {}) =>
    (
        await call("POST", "/api/v1/task-lists", {
            name,
            kind,
            completion_policy: "assignee",
            child_can_add: "none",
            dashboard_add: false,
            calendar_default: "hidden",
            sort: 0,
            visible_on_dashboard: true,
            ...extra,
        })
    ).json;
const chores = await mkList("Chores", "chores");
const homework = await mkList("Homework", "homework", {
    calendar_default: "always",
});
const errands = await mkList("Errands", "general", {
    child_can_add: "own",
    dashboard_add: true,
    calendar_default: "until_done",
});

const mkDef = async (body) =>
    (
        await call("POST", "/api/v1/task-definitions", {
            list: chores.id,
            start_date: today,
            lead_days: 0,
            steps_template: [],
            ...body,
        })
    ).json;
const defs = {
    rotation: await mkDef({
        title: "Water the plants",
        schedule: { kind: "after_done", interval_days: 3 },
        strategy: "rotation",
        candidates: [id("Alex"), id("Sam")],
    }),
    weekly: await mkDef({
        title: "Feed the cat",
        schedule: { kind: "rrule", rrule: `FREQ=WEEKLY;BYDAY=${byday}` },
        strategy: "fixed",
        candidates: [id("Riley")],
        steps_template: ["Fill the bowl", "Fresh water"],
    }),
    pool: await mkDef({
        title: "Unload the dishwasher",
        schedule: { kind: "after_done", interval_days: 1 },
        strategy: "pool",
        candidates: [id("Alex"), id("Sam"), id("Riley")],
    }),
    // Only the weather rule below creates tasks from this one.
    weather: await mkDef({
        title: "Wrap up warm",
        schedule: { kind: "after_done", interval_days: 7 },
        strategy: "fixed",
        candidates: [id("Alex")],
        paused: true,
    }),
};

const keyDate = new Date(Date.now() + 3 * 86_400_000).toISOString();
const homeworkTask = (
    await call("POST", "/api/v1/tasks", {
        list: homework.id,
        title: "Volcano model due",
        due_at: keyDate,
        assignees: [id("Riley")],
        steps: [],
    })
).json;

const routine = (
    await call("POST", "/api/v1/routines", {
        name: "Morning routine",
        members: [id("Riley")],
        starts_at: "00:00",
        done_by: "23:59",
        day_filter: "daily",
        steps: [
            { text: "Brush teeth" },
            { text: "Get dressed" },
            { text: "Pack school bag" },
        ],
    })
).json;

const rule = (
    await call("POST", "/api/v1/weather-rules", {
        name: "E2E wrap up warm",
        condition: "min(temp_c, today) < 5",
        action: "task",
        severity: "info",
        message: "Cold today",
        task_mode: "create",
        task_definition: { definition: defs.weather.id },
    })
).json;

// Materialise now instead of waiting for the minute jobs.
execFileSync(process.env.FA_E2E_BIN, ["tasks", "run-now"], {
    stdio: "inherit",
});

const device = (await call("POST", "/api/v1/devices", { name: "e2e-kitchen" }))
    .json;
writeFileSync(
    process.argv[2],
    JSON.stringify({
        sources,
        device,
        members: Object.fromEntries(list.map((m) => [m.display_name, m.id])),
        lists: { chores: chores.id, homework: homework.id, errands: errands.id },
        defs: Object.fromEntries(Object.entries(defs).map(([k, v]) => [k, v.id])),
        homeworkTask: homeworkTask.id,
        routine: routine.id,
        weatherRule: rule.id,
        today,
    }),
);
console.log("seeded", Object.keys(sources).join(", "));

// Seeds sources, calendars, a paired device and the first sync through the API (fictional data).
// Env: FA_E2E_BASE_URL, FA_E2E_USER, FA_E2E_PASSWORD, FA_E2E_FIXTURES (http://127.0.0.1:18091),
// FA_E2E_RADICALE (http://127.0.0.1:18092). Writes the ids to the file in argv[2].
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

const device = (await call("POST", "/api/v1/devices", { name: "e2e-kitchen" }))
    .json;
writeFileSync(
    process.argv[2],
    JSON.stringify({
        sources,
        device,
        members: Object.fromEntries(list.map((m) => [m.display_name, m.id])),
    }),
);
console.log("seeded", Object.keys(sources).join(", "));

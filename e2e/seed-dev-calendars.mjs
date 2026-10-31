// Adds the busy sample calendars (e2e/fixtures/ics/dev-*.ics) to the dev household: one ICS source
// per member plus a shared family one, with events across the week for testing the dashboard and
// the Calendar page (recurring, all-day, multi-day, overnight, UTC, past, a crowded day).
// Run by scripts/dev-seed.sh after seed-household.mjs; the fixture server must be up so the
// worker can fetch the files (the dates in them are relative to today).
// Env: FA_E2E_BASE_URL, FA_E2E_USER, FA_E2E_PASSWORD, FA_E2E_FIXTURES (http://127.0.0.1:18091).
const base = process.env.FA_E2E_BASE_URL;
const fixtures = process.env.FA_E2E_FIXTURES;
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
    return text ? JSON.parse(text) : null;
}

await call("POST", "/api/v1/auth/login", {
    username: process.env.FA_E2E_USER,
    password: process.env.FA_E2E_PASSWORD,
});
const members = await call("GET", "/api/v1/members");
const list = Array.isArray(members) ? members : (members.items ?? members.members);
const id = (name) => list.find((m) => m.display_name === name).id;

// [source name, fixture, members of its calendar (none = everyone)]
const wanted = [
    ["Family", "dev-family", []],
    ["Alex", "dev-alex", [id("Alex")]],
    ["Sam", "dev-sam", [id("Sam")]],
    ["Riley", "dev-riley", [id("Riley")]],
    ["Jordan", "dev-jordan", [id("Jordan")]],
];

const ids = [];
for (const [name, file, who] of wanted) {
    const created = await call("POST", "/api/v1/calendar-sources", {
        kind: "ics",
        url: `${fixtures}/ics/${file}.ics`,
        name: `${name} (sample)`,
        owner: id("Alex"),
    });
    for (const c of created.calendars) {
        await call(
            "PATCH",
            `/api/v1/calendars/${c.id}`,
            { enabled: true, members: who },
            { "if-match": `"${c.version}"` },
        );
    }
    await call("POST", `/api/v1/calendar-sources/${created.source.id}/sync`);
    ids.push(created.source.id);
}

// The worker does the first sync; wait for it so the dashboard has events straight away.
const deadline = Date.now() + 60_000;
for (;;) {
    const mine = (await call("GET", "/api/v1/calendar-sources")).filter((s) => ids.includes(s.id));
    const failed = mine.filter((s) => s.last_error);
    if (failed.length) throw new Error(`sync failed: ${JSON.stringify(failed)}`);
    if (mine.every((s) => s.last_synced_at)) break;
    if (Date.now() > deadline) throw new Error("first calendar sync timed out");
    await new Promise((r) => setTimeout(r, 1000));
}
console.log(`sample calendars added: ${wanted.map((w) => w[0]).join(", ")}`);

// Seeds the fictional example household through the setup API (no secrets, no real data) and
// completes the first-run setup. Used by scripts/e2e.sh and scripts/demo-reset.sh.
// Env: FA_E2E_BASE_URL, FA_E2E_USER, FA_E2E_PASSWORD, optional FA_E2E_PIN (owner PIN).
const base = process.env.FA_E2E_BASE_URL;
let cookie = "";

async function call(method, path, body) {
    const res = await fetch(base + path, {
        method,
        headers: {
            "content-type": "application/json",
            "x-fa-request": "1",
            ...(cookie ? { cookie } : {}),
        },
        body: body === undefined ? undefined : JSON.stringify(body),
    });
    const text = await res.text();
    if (!res.ok) throw new Error(`${method} ${path} -> ${res.status} ${text}`);
    const sc = res.headers.get("set-cookie");
    if (sc) cookie = sc.split(";")[0];
    return text ? JSON.parse(text) : null;
}

await call("POST", "/api/v1/setup/household", {
    name: "The Example Family",
    tz: "America/New_York",
    units: "metric",
    currency: "USD",
    location: { label: "Springfield (fictional)", lat: 40.0, lon: -75.0 },
    base_url: base,
});
await call("POST", "/api/v1/setup/owner", {
    display_name: "Alex",
    color: "purple",
    username: process.env.FA_E2E_USER,
    password: process.env.FA_E2E_PASSWORD,
    ...(process.env.FA_E2E_PIN ? { pin: process.env.FA_E2E_PIN } : {}),
});

const member = (body) => call("POST", "/api/v1/members", body);
await member({
    display_name: "Sam",
    role: "adult",
    birthdate: "1987-09-30",
    interests: ["gardening", "podcasts"],
    hobbies: ["pottery"],
    responsibilities: ["meal planning", "medical appointments"],
});
const riley = await member({
    display_name: "Riley",
    role: "child",
    color: "green",
    birthdate: "2016-06-03",
    grade: "3",
    interests: ["dinosaurs", "soccer"],
    hobbies: ["drawing"],
    responsibilities: ["feed the cat", "pack school bag"],
    aliases: ["Ri"],
    gender: "female",
    relevance_tags: ["soccer"],
});
await member({
    display_name: "Jordan",
    role: "child",
    birthdate: "2022-02-18",
    interests: ["trucks", "music"],
    responsibilities: ["tidy toys"],
    gender: "male",
});

await call("POST", "/api/v1/weather-rules", {
    name: "Cold morning coats",
    condition: "min(temp_c, tonight) < 2",
    action: "alert",
    severity: "info",
    message: "Frosty start: coats and hats for school.",
});

const school = await call("POST", "/api/v1/orgs", {
    name: "Example Elementary",
    kind: "school",
    domains: ["example.edu"],
    senders: ["office@example.edu"],
    contact_mode: "suggest",
});
const teacher = await call("POST", `/api/v1/orgs/${school.id}/contacts`, {
    name: "Ms. Teacher",
    email: "teacher@example.edu",
    role: "teacher",
});
await call("POST", `/api/v1/members/${riley.id}/enrollments`, {
    org: school.id,
    grade: "3",
    class_name: "3B",
    from: "2026-09-01",
    contacts: [teacher.id],
});
await call("POST", "/api/v1/orgs", {
    name: "Example Utilities",
    kind: "biller",
    domains: ["example.com"],
    senders: ["billing@example.com"],
    bill_template: {
        amount: "Amount due:\\s*(\\$[0-9.,]+)",
        due: "Due date:\\s*([A-Za-z]+ [0-9]{1,2}, [0-9]{4})",
        account: "Account ending in\\s*([0-9]{4})",
    },
});

await call("POST", "/api/v1/setup/complete");
console.log("example household seeded");

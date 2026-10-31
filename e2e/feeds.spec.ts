import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import {
  expect,
  test,
  type APIRequestContext,
  type Playwright,
} from "@playwright/test";

// Needs the stack from scripts/e2e.sh: fixture feeds (fictional, example.com), Radicale, the
// Open-Meteo stub, api and worker as separate processes. All dates are relative to today.
const BASE = process.env.FA_E2E_BASE_URL ?? "http://127.0.0.1:18080";
const seed = JSON.parse(
  readFileSync(process.env.FA_E2E_SEED ?? "", "utf8"),
) as {
  device: { token: string };
};
const NY = "America/New_York";

interface Ev {
  title: string;
  start_at: string;
  all_day: boolean;
  visibility: string;
  location?: string | null;
  description?: string | null;
}

const iso = (offsetDays: number) =>
  new Date(Date.now() + offsetDays * 86_400_000).toISOString();

/** `YYYY-MM-DD HH:mm` of the instant in New York. */
function ny(instant: string) {
  const p = Object.fromEntries(
    new Intl.DateTimeFormat("en-CA", {
      timeZone: NY,
      hourCycle: "h23",
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    })
      .formatToParts(new Date(instant))
      .map((x) => [x.type, x.value]),
  );
  return `${p.year}-${p.month}-${p.day} ${p.hour}:${p.minute}`;
}

/** The fixture date N days from today, as the fixture server renders it (America/New_York date). */
const fixtureDay = (n: number) => {
  // Calendar-day arithmetic: adding n * 24 h is a day off across a DST change.
  const [y, m, d] = new Intl.DateTimeFormat("en-CA", { timeZone: "America/New_York" })
    .format(new Date())
    .split("-")
    .map(Number);
  return new Date(Date.UTC(y, m - 1, d + n)).toISOString().slice(0, 10);
};

async function login(pw: Playwright, user: string, password: string) {
  const ctx = await pw.request.newContext({
    baseURL: BASE,
    extraHTTPHeaders: { "X-FA-Request": "1" },
  });
  const res = await ctx.post("/api/v1/auth/login", {
    data: { username: user, password },
  });
  expect(res.ok()).toBeTruthy();
  return ctx;
}

const events = async (ctx: APIRequestContext, from: number, to: number) => {
  const res = await ctx.get(`/api/v1/events?from=${iso(from)}&to=${iso(to)}`);
  expect(res.ok()).toBeTruthy();
  return (await res.json()) as Ev[];
};

let alex: APIRequestContext;
let riley: APIRequestContext;

test.beforeAll(async ({ playwright }) => {
  alex = await login(
    playwright,
    process.env.FA_E2E_USER ?? "alex@example.com",
    process.env.FA_E2E_PASSWORD ?? "",
  );
  riley = await login(
    playwright,
    process.env.FA_E2E_CHILD_USER ?? "riley@example.com",
    process.env.FA_E2E_CHILD_PASSWORD ?? "",
  );
});

test("ICS sync: weekly rule with an EXDATE and a moved occurrence", async () => {
  const soccer = (await events(alex, 1, 40))
    .filter((e) => e.title.startsWith("Soccer Practice"))
    .map((e) => `${ny(e.start_at)} ${e.title}`)
    .sort();
  expect(soccer).toEqual([
    `${fixtureDay(7)} 18:30 Soccer Practice (moved)`,
    `${fixtureDay(21)} 17:00 Soccer Practice`,
    `${fixtureDay(28)} 17:00 Soccer Practice`,
    `${fixtureDay(35)} 17:00 Soccer Practice`,
  ]);
  // No occurrence on the EXDATE.
  expect(soccer.some((s) => s.startsWith(fixtureDay(14)))).toBe(false);
});

test("ICS sync: all-day events", async () => {
  const holiday = (await events(alex, 0, 10)).find(
    (e) => e.title === "School Holiday",
  );
  expect(holiday?.all_day).toBe(true);
});

test("ICS sync: a weekly 08:00 New York event keeps its local time across a DST change", async () => {
  const standups = (await events(alex, 0, 399)).filter(
    (e) => e.title === "Weekly Standup",
  );
  expect(standups.length).toBeGreaterThan(50);
  for (const e of standups) expect(ny(e.start_at).slice(11)).toBe("08:00");
  // 08:00 New York is 12:00Z in summer time and 13:00Z in winter time: both must occur.
  const utcHours = new Set(
    standups.map((e) => new Date(e.start_at).getUTCHours()),
  );
  expect([...utcHours].sort()).toEqual([12, 13]);
});

test("CalDAV sync (Radicale)", async () => {
  const titles = (await events(alex, 0, 20)).map((e) => e.title);
  for (const t of ["Book Club", "Pottery Night", "Vet Visit"])
    expect(titles).toContain(t);
  const club = (await events(alex, 0, 20)).find((e) => e.title === "Book Club");
  expect(club?.start_at).toContain("T19:00:00");
});

test("/weather returns the fixture forecast", async () => {
  const res = await alex.get("/api/v1/weather");
  expect(res.ok()).toBeTruthy();
  const w = await res.json();
  expect(w.current.temperature_2m).toBe(-3);
  expect(w.hourly.temperature_2m).toHaveLength(48);
  expect(w.daily.time).toHaveLength(7);
});

test("advisories exist and are deduped across two fetches", async () => {
  const texts = async () =>
    (await (await alex.get("/api/v1/advisories")).json()) as {
      id: string;
      text: string;
    }[];
  const before = await texts();
  expect(before.some((a) => a.text.startsWith("Frost tonight"))).toBe(true);
  expect(before.some((a) => a.text.startsWith("Rain likely today"))).toBe(true);

  const fetchedAt = async () =>
    (await (await alex.get("/api/v1/weather")).json()).fetched_at;
  const first = await fetchedAt();
  execFileSync(process.env.FA_E2E_BIN ?? "fa-server", ["weather", "fetch"], {
    stdio: "pipe",
  });
  await expect.poll(fetchedAt).not.toBe(first);
  await new Promise((r) => setTimeout(r, 4000)); // let the worker evaluate the rules again
  const after = await texts();
  expect(after.map((a) => a.id).sort()).toEqual(before.map((a) => a.id).sort());
});

test("a child sees only permitted events, adults see everything", async () => {
  const child = await events(riley, 0, 10);
  const titles = child.map((e) => e.title);
  expect(titles.some((t) => t.startsWith("Soccer Practice"))).toBe(true); // Riley's calendar
  expect(titles).not.toContain("Toddler Music Class"); // Jordan's calendar
  expect(titles).not.toContain("Dentist Checkup"); // an adult's private event ...
  expect(child.some((e) => e.title === "Busy" && e.visibility === "busy")).toBe(
    true,
  ); // ... as Busy

  const adult = (await events(alex, 0, 10)).map((e) => e.title);
  expect(adult).toContain("Toddler Music Class");
  expect(adult).toContain("Dentist Checkup");
});

test("the device sees masked busy events", async ({ request }) => {
  const res = await request.get("/api/v1/dashboard", {
    headers: { Authorization: `Bearer ${seed.device.token}` },
  });
  expect(res.ok()).toBeTruthy();
  const d = await res.json();
  const evs = d.events as Ev[];
  expect(evs.map((e) => e.title)).not.toContain("Dentist Checkup");
  const busy = evs.filter((e) => e.visibility === "busy");
  expect(busy.length).toBeGreaterThan(0);
  for (const b of busy) {
    expect(b.title).toBe("Busy");
    expect(b.location ?? null).toBeNull();
    expect(b.description ?? null).toBeNull();
  }
  expect(d.weather).not.toBeNull();
  expect(d.advisories.length).toBeGreaterThan(0);
});

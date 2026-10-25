import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import {
  expect,
  test,
  type APIRequestContext,
  type Playwright,
} from "@playwright/test";

// M2 API specs. They need the stack from scripts/e2e.sh: api and worker as separate processes,
// a real ntfy container, and the data from e2e/seed.mjs (fictional household, lists, chores, a
// routine and a weather rule). The Open-Meteo stub is always freezing and rainy.
const BASE = process.env.FA_E2E_BASE_URL ?? "http://127.0.0.1:18080";
const NTFY = process.env.FA_E2E_NTFY ?? "http://127.0.0.1:18093";
const seed = JSON.parse(
  readFileSync(process.env.FA_E2E_SEED ?? "", "utf8"),
) as {
  device: { token: string };
  members: Record<string, string>;
  lists: Record<string, string>;
  defs: Record<string, string>;
  homeworkTask: string;
  routine: string;
  today: string;
};
const M = seed.members;
const NY = "America/New_York";

/* eslint-disable @typescript-eslint/no-explicit-any */
type J = any;

const ymd = (instant: string | Date) =>
  new Intl.DateTimeFormat("en-CA", { timeZone: NY }).format(new Date(instant));
const hhmm = (offsetMinutes: number) =>
  new Intl.DateTimeFormat("en-GB", {
    timeZone: NY,
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).format(new Date(Date.now() + offsetMinutes * 60_000));

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

let alex: APIRequestContext;
let sam: APIRequestContext;
let riley: APIRequestContext;
let device: APIRequestContext;

test.beforeAll(async ({ playwright }) => {
  alex = await login(
    playwright,
    process.env.FA_E2E_USER ?? "alex@example.com",
    process.env.FA_E2E_PASSWORD ?? "",
  );
  sam = await login(
    playwright,
    process.env.FA_E2E_SAM_USER ?? "sam@example.com",
    process.env.FA_E2E_SAM_PASSWORD ?? "",
  );
  riley = await login(
    playwright,
    process.env.FA_E2E_CHILD_USER ?? "riley@example.com",
    process.env.FA_E2E_CHILD_PASSWORD ?? "",
  );
  device = await playwright.request.newContext({
    baseURL: BASE,
    extraHTTPHeaders: {
      "X-FA-Request": "1",
      Authorization: `Bearer ${seed.device.token}`,
    },
  });
});
test.afterAll(async () => {
  await Promise.all([
    alex.dispose(),
    sam.dispose(),
    riley.dispose(),
    device.dispose(),
  ]);
});

const tasks = async (
  ctx: APIRequestContext,
  query = "view=all&status=any",
): Promise<J[]> => {
  const res = await ctx.get(`/api/v1/tasks?${query}`);
  expect(res.ok()).toBeTruthy();
  return res.json();
};
const byTitle = async (ctx: APIRequestContext, title: string, query?: string) =>
  (await tasks(ctx, query)).filter((t) => t.title === title);
const one = async (ctx: APIRequestContext, title: string) => {
  const found = await byTitle(ctx, title);
  expect(found, `tasks titled ${title}`).toHaveLength(1);
  return found[0];
};
const definition = async (id: string) => {
  const res = await alex.get("/api/v1/task-definitions");
  expect(res.ok()).toBeTruthy();
  return ((await res.json()) as J[]).find((d) => d.id === id);
};
const post = (ctx: APIRequestContext, path: string, data?: unknown) =>
  ctx.post(`/api/v1/${path}`, data === undefined ? {} : { data });

/** Flips the household override (D-013) the way a user does: PATCH /household as an adult. */
async function setOverride(on: boolean) {
  const url = "/api/v1/household";
  const cur = await (await alex.get(url)).json();
  const res = await alex.patch(url, {
    data: { dashboard_completion_override: on },
    headers: { "If-Match": String(cur.version) },
  });
  expect(res.ok()).toBeTruthy();
  expect((await res.json()).dashboard_completion_override).toBe(on);
}

async function poll<T>(
  what: string,
  fn: () => Promise<T | undefined | false>,
  timeoutMs = 20_000,
  everyMs = 1_000,
): Promise<T> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const v = await fn();
    if (v) return v;
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await new Promise((r) => setTimeout(r, everyMs));
  }
}

test.describe.configure({ mode: "serial" });

test.describe("materialisation", () => {
  test("each definition created one task, assigned by its strategy", async () => {
    const water = await one(alex, "Water the plants");
    expect(water.origin).toBe("system");
    expect(water.assignees).toHaveLength(1);
    expect([M.Alex, M.Sam]).toContain(water.assignees[0]);
    expect(ymd(water.due_at)).toBe(seed.today);

    const cat = await one(alex, "Feed the cat");
    expect(cat.assignees).toEqual([M.Riley]);
    expect(cat.steps.map((s: J) => s.text)).toEqual([
      "Fill the bowl",
      "Fresh water",
    ]);

    const dishes = await one(alex, "Unload the dishwasher");
    expect([...dishes.assignees].sort()).toEqual(
      [M.Alex, M.Sam, M.Riley].sort(),
    );
  });

  test("another pass creates no duplicates", async () => {
    execFileSync(process.env.FA_E2E_BIN ?? "fa-server", ["tasks", "run-now"], {
      stdio: "pipe",
    });
    for (const title of [
      "Water the plants",
      "Feed the cat",
      "Unload the dishwasher",
    ]) {
      expect(await byTitle(alex, title)).toHaveLength(1);
    }
  });
});

test.describe("completion", () => {
  test("rotation advances on completion and undo restores the definition", async () => {
    const task = await one(alex, "Water the plants");
    const before = await definition(seed.defs.rotation);
    expect(before.next_assignee).toBeTruthy();

    const done = await post(alex, `tasks/${task.id}/complete`, {});
    expect(done.ok()).toBeTruthy();
    const completed = await done.json();
    expect(completed.status).toBe("done");
    expect(completed.undo_until).toBeTruthy();

    const after = await definition(seed.defs.rotation);
    expect(after.last_done_at).toBeTruthy();
    expect(after.next_assignee).not.toBe(before.next_assignee);
    const expected = new Date(`${seed.today}T12:00:00Z`);
    expected.setUTCDate(expected.getUTCDate() + 3);
    expect(ymd(after.next_due_at)).toBe(expected.toISOString().slice(0, 10));

    const undo = await post(alex, `tasks/${task.id}/reopen`);
    expect(undo.ok()).toBeTruthy();
    expect((await undo.json()).status).toBe("open");
    const restored = await definition(seed.defs.rotation);
    expect(restored.next_assignee).toBe(before.next_assignee);
    expect(restored.next_due_at).toBe(before.next_due_at);
    expect(restored.last_done_at ?? null).toBe(before.last_done_at ?? null);
  });

  test("steps tick without completing the task", async () => {
    const cat = await one(alex, "Feed the cat");
    for (const s of cat.steps) {
      const res = await alex.patch(`/api/v1/tasks/${cat.id}/steps/${s.id}`, {
        data: { done: true },
      });
      expect(res.ok()).toBeTruthy();
    }
    const after = await one(alex, "Feed the cat");
    expect(after.status).toBe("open");
    expect(after.steps.every((s: J) => s.done_at)).toBeTruthy();
  });

  test("a pool task can be claimed by one member at a time", async () => {
    const dishes = await one(alex, "Unload the dishwasher");
    const mine = await post(riley, `tasks/${dishes.id}/claim`);
    expect(mine.ok()).toBeTruthy();
    expect((await mine.json()).claimed_by).toBe(M.Riley);

    expect((await post(alex, `tasks/${dishes.id}/claim`)).status()).toBe(409);
    expect((await post(riley, `tasks/${dishes.id}/release`)).ok()).toBeTruthy();
    expect((await one(alex, "Unload the dishwasher")).claimed_by ?? null).toBe(
      null,
    );
    expect((await post(alex, `tasks/${dishes.id}/claim`)).ok()).toBeTruthy();
    expect((await post(alex, `tasks/${dishes.id}/release`)).ok()).toBeTruthy();
  });

  test("a child sees and completes only their own tasks", async () => {
    const mine = await tasks(riley);
    const titles = mine.map((t) => t.title);
    expect(titles).toContain("Feed the cat");
    expect(titles).not.toContain("Water the plants");

    const water = await one(alex, "Water the plants");
    expect((await post(riley, `tasks/${water.id}/complete`, {})).status()).toBe(
      403,
    );
    expect(
      (
        await post(riley, "task-lists", {
          name: "Nope",
          kind: "general",
          completion_policy: "assignee",
          child_can_add: "none",
          dashboard_add: false,
          calendar_default: "hidden",
          sort: 0,
          visible_on_dashboard: true,
        })
      ).status(),
    ).toBe(403);
    const cat = await one(alex, "Feed the cat");
    expect((await post(riley, `tasks/${cat.id}/skip`)).status()).toBe(403);
    const ok = await post(riley, `tasks/${cat.id}/complete`, {});
    expect(ok.ok()).toBeTruthy();
    expect((await post(riley, `tasks/${cat.id}/reopen`)).ok()).toBeTruthy();
  });

  test("the dashboard device completes with a member, only when the override is on", async () => {
    const dishes = await one(alex, "Unload the dishwasher");
    await setOverride(false);
    expect(
      (
        await post(device, `tasks/${dishes.id}/complete`, { member: M.Riley })
      ).status(),
    ).toBe(403);
    await setOverride(true);
    try {
      expect(
        (await post(device, `tasks/${dishes.id}/complete`, {})).status(),
      ).toBe(422);
      const res = await post(device, `tasks/${dishes.id}/complete`, {
        member: M.Riley,
      });
      expect(res.ok()).toBeTruthy();
      const done = await res.json();
      expect(done.status).toBe("done");
      expect(done.completed_by).toBe(M.Riley);
      expect((await post(alex, `tasks/${dishes.id}/reopen`)).ok()).toBeTruthy();
    } finally {
      await setOverride(false);
    }
  });
});

test.describe("views", () => {
  test("the Now list is capped, open and ordered by score", async () => {
    const now = await tasks(alex, "view=now");
    const household = await (await alex.get("/api/v1/household")).json();
    expect(now.length).toBeGreaterThan(0);
    expect(now.length).toBeLessThanOrEqual(household.now_list_size);
    expect(now.every((t) => t.status === "open")).toBeTruthy();
    const scores = now.map((t) => t.score);
    expect([...scores].sort((a, b) => b - a)).toEqual(scores);
  });

  test("the calendar range returns tasks whose display is not hidden", async () => {
    const from = new Date(Date.now() - 86_400_000).toISOString();
    const to = new Date(Date.now() + 7 * 86_400_000).toISOString();
    const inRange = await tasks(
      alex,
      `from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`,
    );
    const titles = inRange.map((t) => t.title);
    expect(titles).toContain("Volcano model due");
    expect(titles).not.toContain("Water the plants"); // Chores are hidden on the calendar
    expect(inRange.every((t) => t.calendar_display !== "hidden")).toBeTruthy();
  });
});

test.describe("routines", () => {
  test("ticking every step finishes the run, unticking reopens it", async () => {
    const runs = await (
      await riley.get(`/api/v1/routines/today?member=${M.Riley}`)
    ).json();
    const run = (runs as J[]).find((r) => r.routine === seed.routine);
    expect(run, "today's run of the morning routine").toBeTruthy();
    expect(run.total).toBe(3);
    expect(run.status).toBe("open");

    const tick = (stepId: string, done: boolean) =>
      riley.patch(`/api/v1/routine-runs/${run.id}/steps/${stepId}`, {
        data: { done },
      });
    const state = async () =>
      (
        (await (
          await riley.get(`/api/v1/routines/today?member=${M.Riley}`)
        ).json()) as J[]
      ).find((r) => r.id === run.id);

    for (const s of run.steps)
      expect((await tick(s.step, true)).ok()).toBeTruthy();
    let now = await state();
    expect(now.status).toBe("done");
    expect(now.done).toBe(3);

    expect((await tick(run.steps[0].step, false)).ok()).toBeTruthy();
    now = await state();
    expect(now.status).toBe("open");
    expect(now.done).toBe(2);
    expect((await tick(run.steps[0].step, true)).ok()).toBeTruthy();
    expect((await state()).status).toBe("done");
  });
});

test.describe("weather", () => {
  test("a task rule creates one task for today", async () => {
    const created = await poll(
      "the weather task",
      async () => {
        const found = await byTitle(alex, "Wrap up warm");
        return found.length ? found : undefined;
      },
      60_000,
    );
    expect(created).toHaveLength(1);
    expect(created[0].origin).toBe("weather");
    expect(created[0].status).toBe("open");
    expect(ymd(created[0].due_at)).toBe(seed.today);

    // Evaluating again must not add a second one for the same day.
    execFileSync(process.env.FA_E2E_BIN ?? "fa-server", ["weather", "fetch"], {
      stdio: "pipe",
    });
    await new Promise((r) => setTimeout(r, 8_000));
    expect(await byTitle(alex, "Wrap up warm")).toHaveLength(1);
  });
});

// ---- notifications, read back from the real ntfy ------------------------------------------

interface Msg {
  title?: string;
  message: string;
  priority?: number;
  click?: string;
}
// Adults manage only their own prefs (and children's), so Sam's calls use Sam's session.
const prefsCtx = (member: string) => (member === M.Sam ? sam : alex);
const topicOf = async (member: string): Promise<string> => {
  const res = await prefsCtx(member).get(
    `/api/v1/members/${member}/notify-prefs`,
  );
  expect(res.ok()).toBeTruthy();
  const prefs = await res.json();
  expect(prefs.server_configured).toBe(true);
  expect(prefs.topic).toMatch(/^[a-z0-9]{24}$/);
  return prefs.topic;
};
const messages = async (topic: string): Promise<Msg[]> => {
  // ntfy rate-limits readers (burst 60, one more every 5 s): back off on 429.
  let res = await fetch(`${NTFY}/${topic}/json?poll=1&since=all`);
  for (let i = 0; i < 6 && res.status === 429; i++) {
    await new Promise((r) => setTimeout(r, 5_000));
    res = await fetch(`${NTFY}/${topic}/json?poll=1&since=all`);
  }
  expect(res.ok).toBeTruthy();
  return (await res.text())
    .split("\n")
    .filter(Boolean)
    .map((l) => JSON.parse(l))
    .filter((m) => m.event === "message");
};
const text = (m: Msg) => `${m.title ?? ""}\n${m.message}`;
const waitFor = (topic: string, what: string, pred: (m: Msg) => boolean) =>
  poll(what, async () => (await messages(topic)).find(pred), 150_000, 4_000);
const newTask = async (
  title: string,
  assignee: string,
  list = seed.lists.errands,
) => {
  const res = await post(alex, "tasks", {
    list,
    title,
    assignees: [assignee],
    steps: [],
  });
  expect(res.status()).toBe(201);
  return res.json();
};
const setPrefs = async (member: string, body: J) => {
  const ctx = prefsCtx(member);
  const url = `/api/v1/members/${member}/notify-prefs`;
  const cur = await (await ctx.get(url)).json();
  return ctx.put(url, {
    data: body,
    headers: { "If-Match": `"${cur.version}"` },
  });
};

test.describe("notifications", () => {
  // Each message waits a minute (batching) and then for the next minute job: allow two of those.
  test.setTimeout(420_000);

  test("an assigned task reaches its member and a copy reaches the adult", async () => {
    const [rileyTopic, alexTopic] = await Promise.all([
      topicOf(M.Riley),
      topicOf(M.Alex),
    ]);
    const direct = await waitFor(rileyTopic, "Riley's assigned message", (m) =>
      text(m).includes("Feed the cat"),
    );
    expect(direct.click ?? "").toContain("/tasks");
    const copy = await waitFor(
      alexTopic,
      "the copy on Alex's topic",
      (m) => /Riley/.test(text(m)) && /Feed the cat/.test(text(m)),
    );
    expect(copy).toBeTruthy();
  });

  test("several updates are batched, and quiet hours hold messages back", async () => {
    const [samTopic, jordanTopic] = await Promise.all([
      topicOf(M.Sam),
      topicOf(M.Jordan),
    ]);
    // Jordan is in quiet hours for the next two hours; Sam is the control.
    const quiet = await setPrefs(M.Jordan, {
      quiet_from: hhmm(-60),
      quiet_until: hhmm(60),
    });
    expect(quiet.ok()).toBeTruthy();

    const stamp = Date.now();
    const a = `Sort the recycling ${stamp}`;
    const b = `Wipe the table ${stamp}`;
    const held = `Tidy the toys ${stamp}`;
    await newTask(held, M.Jordan);
    await newTask(a, M.Sam);
    await newTask(b, M.Sam);

    const batch = await waitFor(samTopic, "Sam's batched message", (m) =>
      text(m).includes(stamp.toString()),
    );
    expect(batch.title).toMatch(/^\d+ updates$/);
    expect(batch.message).toContain(a);
    expect(batch.message).toContain(b);
    expect(batch.click ?? "").toContain("/tasks");

    // Same send cycle as Sam's message: nothing for Jordan.
    expect(
      (await messages(jordanTopic)).filter((m) => text(m).includes(held)),
    ).toHaveLength(0);

    // Clearing the quiet hours lets later messages through.
    const cleared = await setPrefs(M.Jordan, {
      quiet_from: null,
      quiet_until: null,
    });
    expect(cleared.ok()).toBeTruthy();
    const prefs = await (
      await alex.get(`/api/v1/members/${M.Jordan}/notify-prefs`)
    ).json();
    expect(prefs.quiet_from ?? null).toBeNull();
    const later = `Feed the fish ${stamp}`;
    await newTask(later, M.Jordan);
    const got = await waitFor(
      jordanTopic,
      "Jordan's message after quiet hours",
      (m) => text(m).includes(later),
    );
    expect(text(got)).not.toContain(held);
  });

  test("the test endpoint publishes immediately", async () => {
    const topic = await topicOf(M.Sam);
    const before = (await messages(topic)).length;
    const res = await post(sam, `members/${M.Sam}/notify-prefs/test`);
    expect(res.status()).toBe(204);
    await poll(
      "the test message",
      async () => ((await messages(topic)).length > before ? true : undefined),
      30_000,
      3_000,
    );
  });
});

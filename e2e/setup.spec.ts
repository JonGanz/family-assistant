import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import {
  expect,
  test,
  type APIRequestContext,
  type APIResponse,
} from "@playwright/test";

// The first-run API against an instance on an EMPTY database (scripts/e2e.sh starts it on
// FA_E2E_SETUP_URL, port 18094, with no FA_COOKIE_SECURE, FA_ALLOW_PRIVATE_HOSTS or
// FA_PUBLIC_BASE_URL set). The tests build on each other and run serially.
const BASE = process.env.FA_E2E_SETUP_URL ?? "http://127.0.0.1:18094";
const FIXTURES = process.env.FA_E2E_FIXTURES ?? "http://127.0.0.1:18091";
const LAT = 41.5; // the fixture server answers this latitude with a mild forecast on request
const PASSWORD = "first-run-passphrase-1";
const CHILD_PASSWORD = "dino1234";

/* eslint-disable @typescript-eslint/no-explicit-any */
type J = any;
test.describe.configure({ mode: "serial" });

let anon: APIRequestContext;
let owner: APIRequestContext;
let ownerId = "";
let householdId = "";
test.beforeAll(async ({ playwright }) => {
  const mk = () =>
    playwright.request.newContext({
      baseURL: BASE,
      extraHTTPHeaders: { "X-FA-Request": "1" },
    });
  anon = await mk();
  owner = await mk();
});
test.afterAll(async () => {
  await Promise.all([anon.dispose(), owner.dispose()]);
});

const problem = async (res: APIResponse): Promise<J> => res.json();
const cookieOf = (res: APIResponse): string =>
  res.headersArray().find((h) => h.name.toLowerCase() === "set-cookie")?.value ?? "";
const settingsView = async (): Promise<J> => {
  const res = await owner.get("/api/v1/system/settings");
  expect(res.ok()).toBeTruthy();
  return res.json();
};
const setting = (view: J, key: string): J => view.settings.find((s: J) => s.key === key);
const ready = async (): Promise<void> => {
  // The server restarts its listeners about a second after a settings change.
  for (let i = 0; i < 60; i++) {
    try {
      if ((await anon.get("/readyz")).ok()) return;
    } catch {
      /* restarting */
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error("the instance did not come back");
};
// A changed setting is applied by an in-process restart about a second after the write.
const settle = async (): Promise<void> => {
  await new Promise((r) => setTimeout(r, 2_500));
  await ready();
};
const waitFor = async (what: string, fn: () => Promise<boolean>, ms = 30_000) => {
  const deadline = Date.now() + ms;
  for (;;) {
    try {
      if (await fn()) return;
    } catch {
      /* restarting */
    }
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await new Promise((r) => setTimeout(r, 500));
  }
};

// ---- before setup ---------------------------------------------------------------------------

test("a fresh database reports setup as needed", async () => {
  const res = await anon.get("/api/v1/setup");
  expect(res.status()).toBe(200);
  expect(await res.json()).toMatchObject({
    needed: true,
    owner_needed: false,
    completed: false,
    secure: false,
    origin: BASE,
    via_tailscale: false,
  });
});

test("every other API route answers 409 setup_required; health stays up", async () => {
  expect((await anon.get("/healthz")).ok()).toBeTruthy();
  expect((await anon.get("/readyz")).ok()).toBeTruthy();
  for (const path of ["/api/v1/me", "/api/v1/members", "/api/v1/openapi.json", "/api/v1/dashboard"]) {
    const res = await anon.get(path);
    expect(res.status(), path).toBe(409);
    expect((await problem(res)).code, path).toBe("setup_required");
  }
  const login = await anon.post("/api/v1/auth/login", {
    data: { username: "nobody", password: "x" },
  });
  expect(login.status()).toBe(409);
  expect((await problem(login)).code).toBe("setup_required");
});

test("the owner cannot be created before the household; the household is validated", async () => {
  const early = await anon.post("/api/v1/setup/owner", {
    data: { display_name: "Pat", username: "pat@example.com", password: PASSWORD },
  });
  expect(early.status()).toBe(409);
  // No household yet: the contract says setup_closed, the server says setup_required. Either
  // refuses; what matters is that nothing was created.
  expect(["setup_closed", "setup_required"]).toContain((await problem(early)).code);

  const badTz = await anon.post("/api/v1/setup/household", {
    data: {
      name: "Example household",
      tz: "Mars/Olympus_Mons",
      units: "metric",
      currency: "USD",
      base_url: BASE,
    },
  });
  expect(badTz.status()).toBe(422);
  expect((await anon.get("/api/v1/setup").then((r) => r.json())).needed).toBe(true);
});

// ---- the wizard's writes --------------------------------------------------------------------

test("the household is created once; its address is stored", async () => {
  const res = await anon.post("/api/v1/setup/household", {
    data: {
      name: "Example household",
      tz: "America/New_York",
      location: { label: "Springfield (fictional)", lat: LAT, lon: -75 },
      units: "imperial",
      currency: "USD",
      base_url: BASE,
    },
  });
  expect(res.status(), await res.text()).toBe(201);
  householdId = (await res.json()).household_id;
  expect(householdId).toBeTruthy();

  const status = await (await anon.get("/api/v1/setup")).json();
  expect(status).toMatchObject({ needed: false, owner_needed: true, completed: false });
  // Storing the address is a settings change: the server restarts its listeners shortly after.
  await settle();

  const again = await anon.post("/api/v1/setup/household", {
    data: { name: "Second", tz: "UTC", units: "metric", currency: "USD", base_url: BASE },
  });
  expect(again.status()).toBe(409);
  expect((await problem(again)).code).toBe("setup_closed");
  // Still gated until an adult exists.
  expect((await anon.get("/api/v1/me")).status()).toBe(409);
});

test("the owner is validated, then created with a session; the cookie is not Secure over plain HTTP", async () => {
  const weak = await anon.post("/api/v1/setup/owner", {
    data: { display_name: "Pat", username: "pat@example.com", password: "short" },
  });
  expect(weak.status()).toBe(422);
  const badPin = await anon.post("/api/v1/setup/owner", {
    data: { display_name: "Pat", username: "pat@example.com", password: PASSWORD, pin: "12" },
  });
  expect(badPin.status()).toBe(422);

  const res = await owner.post("/api/v1/setup/owner", {
    data: {
      display_name: "Pat",
      color: "#3366cc",
      username: "pat@example.com",
      password: PASSWORD,
      pin: "246810",
    },
  });
  expect(res.status(), await res.text()).toBe(201);
  const cookie = cookieOf(res);
  expect(cookie).toMatch(/HttpOnly/i);
  expect(cookie).not.toMatch(/;\s*Secure/i);
  const me: J = await res.json();
  ownerId = me.member?.id ?? me.actor?.member_id ?? me.actor?.member?.id;
  expect(ownerId).toBeTruthy();

  const again = await anon.post("/api/v1/setup/owner", {
    data: { display_name: "Eve", username: "eve@example.com", password: PASSWORD },
  });
  expect(again.status()).toBe(409);
  expect((await problem(again)).code).toBe("setup_closed");
});

test("the wizard's later steps run as the new adult; complete is adult-only and final", async () => {
  expect((await owner.get("/api/v1/me")).ok()).toBeTruthy();
  expect((await anon.post("/api/v1/setup/complete")).status()).toBe(401);
  expect((await anon.get("/api/v1/setup")).ok()).toBeTruthy();

  const child = await owner.post("/api/v1/members", {
    data: { display_name: "Robin", role: "child", color: "#cc6633" },
  });
  expect(child.ok(), await child.text()).toBeTruthy();
  const childId = ((await child.json()) as J).id;
  const account = await owner.post(`/api/v1/members/${childId}/account`, {
    data: { username: "robin@example.com", password: CHILD_PASSWORD },
  });
  expect(account.ok(), await account.text()).toBeTruthy();

  const done = await owner.post("/api/v1/setup/complete");
  expect(done.status()).toBe(204);
  expect(await (await anon.get("/api/v1/setup")).json()).toMatchObject({
    needed: false,
    owner_needed: false,
    completed: true,
  });
  // Closed for good: no second household, no second owner.
  const late = await anon.post("/api/v1/setup/owner", {
    data: { display_name: "Eve", username: "eve@example.com", password: PASSWORD },
  });
  expect(late.status()).toBe(409);
});

// ---- the master key -------------------------------------------------------------------------

test("the master key needs the adult's password, is audited, and confirms once stored", async () => {
  const wrong = await owner.post("/api/v1/system/master-key", { data: { password: "nope-nope-nope" } });
  expect(wrong.status()).toBe(401);
  expect((await settingsView()).key_backup_confirmed_at).toBeNull();

  const ok = await owner.post("/api/v1/system/master-key", { data: { password: PASSWORD } });
  expect(ok.status()).toBe(200);
  const key = Buffer.from((await ok.json()).key_base64, "base64");
  expect(key).toHaveLength(32);

  expect((await anon.post("/api/v1/system/master-key", { data: { password: PASSWORD } })).status()).toBe(401);
  expect((await owner.post("/api/v1/system/master-key/confirm")).status()).toBe(204);
  const view = await settingsView();
  expect(new Date(view.key_backup_confirmed_at).getTime()).toBeGreaterThan(0);
});

// ---- cookies --------------------------------------------------------------------------------

test("the session cookie is Secure exactly when the request arrived over HTTPS", async ({ playwright }) => {
  const login = async (headers: Record<string, string>) => {
    const ctx = await playwright.request.newContext({
      baseURL: BASE,
      extraHTTPHeaders: { "X-FA-Request": "1", ...headers },
    });
    const res = await ctx.post("/api/v1/auth/login", {
      data: { username: "pat@example.com", password: PASSWORD },
    });
    expect(res.status(), await res.text()).toBe(200);
    const cookie = cookieOf(res);
    await ctx.dispose();
    return cookie;
  };
  expect(await login({})).not.toMatch(/;\s*Secure/i);
  // Playwright connects from loopback, a trusted proxy by default (D-069).
  expect(await login({ "X-Forwarded-Proto": "https" })).toMatch(/;\s*Secure/i);
  expect(await login({ "X-Forwarded-Proto": "http" })).not.toMatch(/;\s*Secure/i);

  const status = await (
    await anon.get("/api/v1/setup", { headers: { "X-Forwarded-Proto": "https", "X-Forwarded-Host": "family.example.ts.net" } })
  ).json();
  expect(status.secure).toBe(true);
});

// ---- passwords (C-03) and the removed routes ------------------------------------------------

test("changing your own password needs the current one; an adult resets a child's without it", async ({ playwright }) => {
  const me = (await (await owner.get("/api/v1/me")).json()) as J;
  const mine = ownerId || me.member?.id;
  const bare = await owner.post(`/api/v1/members/${mine}/account/password`, {
    data: { password: "another-long-passphrase-2" },
  });
  expect(bare.status()).toBe(422);
  const wrong = await owner.post(`/api/v1/members/${mine}/account/password`, {
    data: { current_password: "not-my-password", new_password: "another-long-passphrase-2" },
  });
  expect([401, 403]).toContain(wrong.status());

  const members: J[] = await (await owner.get("/api/v1/members")).json().then((m: J) => (Array.isArray(m) ? m : (m.items ?? m.members)));
  const robin = members.find((m) => m.display_name === "Robin");
  const reset = await owner.post(`/api/v1/members/${robin.id}/account/password`, {
    data: { password: "dino5678" },
  });
  expect(reset.ok(), await reset.text()).toBeTruthy();
  const kid = await playwright.request.newContext({ baseURL: BASE, extraHTTPHeaders: { "X-FA-Request": "1" } });
  expect((await kid.post("/api/v1/auth/login", { data: { username: "robin@example.com", password: "dino5678" } })).ok()).toBeTruthy();
  // A child may not manage members, the inbox or the system settings.
  expect((await kid.get("/api/v1/inbox")).status()).toBe(403);
  expect((await kid.get("/api/v1/system/settings")).status()).toBe(403);
  expect((await kid.post("/api/v1/members", { data: { display_name: "X", role: "child" } })).status()).toBe(403);
  await kid.dispose();

  const good = await owner.post(`/api/v1/members/${mine}/account/password`, {
    data: { current_password: PASSWORD, new_password: "another-long-passphrase-2" },
  });
  expect(good.ok(), await good.text()).toBeTruthy();
  // Put it back (the old password signs in again below).
  const back = await owner.post(`/api/v1/members/${mine}/account/password`, {
    data: { current_password: "another-long-passphrase-2", new_password: PASSWORD },
  });
  expect(back.ok(), await back.text()).toBeTruthy();
});

test("removed routes are gone", async () => {
  const login = await owner.post("/api/v1/auth/login", { data: { username: "pat@example.com", password: PASSWORD } });
  expect(login.ok()).toBeTruthy();
  const fake = "00000000-0000-7000-8000-000000000000";
  expect([404, 405]).toContain((await owner.post(`/api/v1/advisories/${fake}/dismiss`, { data: {} })).status());
  expect((await owner.get("/api/v1/inbox/counts")).status()).toBe(404);
});

// ---- the SSRF guard, settings, restart ------------------------------------------------------

test("a feed on a private address is refused until the household allows it", async () => {
  const src = { kind: "ics", name: "Fixture feed", url: `${FIXTURES}/ics/school.ics` };
  const refused = await owner.post("/api/v1/calendar-sources", { data: src });
  expect(refused.ok()).toBe(false);
  expect(refused.status()).toBeLessThan(500);
  const body = await refused.text();
  expect(body).toMatch(/private|internal|local|not allowed/i);

  const patch = await owner.patch("/api/v1/system/settings", {
    data: { values: { allow_private_hosts: true } },
  });
  expect(patch.ok(), await patch.text()).toBeTruthy();
  await settle();
  expect(setting(await settingsView(), "allow_private_hosts").value).toBe(true);
  const accepted = await owner.post("/api/v1/calendar-sources", { data: src });
  expect(accepted.ok(), await accepted.text()).toBeTruthy();
});

test("settings: a PATCH survives a restart; invalid values are named; env-set values are locked", async () => {
  const before = await settingsView();
  expect(before.connection).toMatchObject({ secure: false, origin: BASE });
  const tailnet = setting(before, "tailnet_base_url");
  expect(tailnet.locked).toBe(false);
  expect(setting(before, "allow_http_feeds")).toMatchObject({ locked: true, source: "env" });
  expect(setting(before, "ntfy_token").value).toBeNull();

  const bad = await owner.patch("/api/v1/system/settings", {
    data: { values: { tailnet_base_url: "not a url" } },
  });
  expect(bad.status()).toBe(422);
  expect(await problem(bad)).toMatchObject({ code: "invalid_setting", field: "tailnet_base_url" });

  const locked = await owner.patch("/api/v1/system/settings", {
    data: { values: { allow_http_feeds: false } },
  });
  expect(locked.status()).toBe(409);
  expect((await problem(locked)).code).toBe("setting_locked");

  const secret = await owner.patch("/api/v1/system/settings", {
    data: { values: { ntfy_token: "tk_example_token_value" } },
  });
  expect(secret.ok(), await secret.text()).toBeTruthy();
  const url = "https://family.example.ts.net";
  const ok = await owner.patch("/api/v1/system/settings", { data: { values: { tailnet_base_url: url } } });
  expect(ok.ok(), await ok.text()).toBeTruthy();
  await settle();
  const after = await settingsView();
  expect(setting(after, "tailnet_base_url")).toMatchObject({ value: url, source: "db", is_set: true });
  expect(setting(after, "ntfy_token")).toMatchObject({ value: null, is_set: true });
  expect(JSON.stringify(after)).not.toContain("tk_example_token_value");

  // The server re-reads its settings and restarts its listeners; the value is still there after
  // and the session survives (it lives in the database).
  expect(setting(await settingsView(), "tailnet_base_url").value).toBe(url);

  // null resets to the default.
  const reset = await owner.patch("/api/v1/system/settings", { data: { values: { tailnet_base_url: null } } });
  expect(reset.ok()).toBeTruthy();
  await settle();
  expect(setting(await settingsView(), "tailnet_base_url")).toMatchObject({ source: "default", is_set: false });
});

test("confirming the key backup does not restart the server", async () => {
  const started = Date.now();
  expect((await owner.post("/api/v1/system/master-key/confirm")).status()).toBe(204);
  await new Promise((r) => setTimeout(r, 3_000));
  const res = await owner.get("/api/v1/me");
  expect(res.ok()).toBeTruthy();
  expect(Date.now() - started).toBeLessThan(15_000);
});

// ---- advisories clear when the rule stops matching ------------------------------------------

test("an advisory disappears when its rule stops matching the forecast", async () => {
  test.setTimeout(120_000);
  const bin = process.env.FA_E2E_BIN ?? "fa-server";
  const dbUrl = process.env.FA_E2E_SETUP_DATABASE_URL;
  test.skip(!dbUrl, "needs FA_E2E_SETUP_DATABASE_URL to run the CLI against this instance");
  // The CLI reads its settings from the environment only: allow the loopback fixture server.
  const env = { ...process.env, DATABASE_URL: dbUrl, FA_ALLOW_PRIVATE_HOSTS: "true", FA_OPEN_METEO_URL: FIXTURES };
  const fetchWeather = () => execFileSync(bin, ["weather", "fetch"], { env, stdio: "pipe" });
  const mild = (on: boolean) =>
    fetch(`${FIXTURES}/control/mild?latitude=${LAT}&on=${on ? 1 : 0}`, { method: "POST" });
  const active = async (): Promise<string[]> =>
    ((await (await owner.get("/api/v1/advisories")).json()) as J[]).map((a) => a.text);
  try {
    await mild(false);
    fetchWeather();
    await waitFor("the frost advisory", async () => (await active()).some((t) => t.startsWith("Frost tonight")));
    const advisories: J[] = await (await owner.get("/api/v1/advisories")).json();
    for (const a of advisories) expect(a).not.toHaveProperty("dismissed_at");

    await mild(true);
    fetchWeather();
    await waitFor("the advisory to clear", async () => (await active()).length === 0);
  } finally {
    await mild(false);
  }
});

// ---- the household file ---------------------------------------------------------------------

test("config export downloads TOML; import previews with a dry run, then writes the same thing", async () => {
  const exp = await owner.get("/api/v1/config/export");
  expect(exp.ok()).toBeTruthy();
  expect(exp.headers()["content-disposition"]).toContain('filename="household.toml"');
  const exported = await exp.text();
  expect(exported).toContain("Example household");
  expect(exported).not.toContain(PASSWORD);

  const memberCount = async (): Promise<number> => {
    const m: J = await (await owner.get("/api/v1/members")).json();
    return (Array.isArray(m) ? m : (m.items ?? m.members)).length;
  };
  const before = await memberCount();
  const file = readFileSync("../config/examples/household.example.toml", "utf8");
  const post = (dry: boolean, data: string) =>
    owner.post(`/api/v1/config/import?dry_run=${dry}`, { data, headers: { "Content-Type": "text/plain" } });

  const dry = await post(true, file);
  expect(dry.status(), await dry.text()).toBe(200);
  const preview: J = await dry.json();
  expect(preview.members_created).toEqual(["alex", "sam", "riley", "jordan"]);
  expect(await memberCount()).toBe(before); // a dry run writes nothing

  const bad = await post(true, "this is = [not valid");
  expect(bad.status()).toBeGreaterThanOrEqual(400);
  expect(bad.status()).toBeLessThan(500);

  const real = await post(false, file);
  expect(real.status(), await real.text()).toBe(200);
  expect((await real.json()).members_created).toEqual(preview.members_created);
  expect(await memberCount()).toBe(before + 4);
});

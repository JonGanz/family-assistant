import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import {
  expect,
  test,
  type APIRequestContext,
  type Playwright,
} from "@playwright/test";

import { sendRaw } from "./mail-send.ts";

// D-078 and D-068 on the seeded stack: an inbox holds only items from sources the adult may see,
// the dashboard carries a pending count per adult, devices never get the inbox, and the removed
// routes are gone. A second, owners-only mailbox (Alex) is added for the scoping checks and
// removed again, so the M3 specs see the inbox they expect.
const BASE = process.env.FA_E2E_BASE_URL ?? "http://127.0.0.1:18080";
const seed = JSON.parse(
  readFileSync(process.env.FA_E2E_SEED ?? "", "utf8"),
) as {
  device: { token: string };
  members: Record<string, string>;
  imapPort: number;
  smtpPort: number;
};
const M = seed.members;
const api = "/api/v1";
const MAILBOX = "private@example.com";

/* eslint-disable @typescript-eslint/no-explicit-any */
type J = any;
test.describe.configure({ mode: "serial" });

async function login(pw: Playwright, user: string, password: string) {
  const ctx = await pw.request.newContext({
    baseURL: BASE,
    extraHTTPHeaders: { "X-FA-Request": "1" },
  });
  const res = await ctx.post(`${api}/auth/login`, {
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
  await Promise.all([alex.dispose(), sam.dispose(), riley.dispose(), device.dispose()]);
});

// The dashboard is for paired devices only.
const pending = async (member: string): Promise<number> => {
  const res = await device.get(`${api}/dashboard`);
  expect(res.ok()).toBeTruthy();
  const view: J = await res.json();
  expect(view).not.toHaveProperty("inbox");
  expect(Array.isArray(view.pending_approvals)).toBe(true);
  return view.pending_approvals.find((p: J) => p.member_id === member)?.count ?? 0;
};
const inboxEvents = async (ctx: APIRequestContext): Promise<J[]> => {
  const res = await ctx.get(`${api}/inbox`);
  expect(res.ok()).toBeTruthy();
  return (await res.json()).items.filter((i: J) => i.kind === "event").map((i: J) => i.event);
};
const poll = async <T>(what: string, fn: () => Promise<T | undefined | false>, ms = 60_000) => {
  const deadline = Date.now() + ms;
  for (;;) {
    const v = await fn();
    if (v) return v;
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await new Promise((r) => setTimeout(r, 1_000));
  }
};

test("devices are refused the inbox, elevated or not", async () => {
  expect((await device.get(`${api}/inbox`)).status()).toBe(403);
  expect((await riley.get(`${api}/inbox`)).status()).toBe(403);
  const approve = await device.post(`${api}/inbox/event/${M.Alex}/approve`, { data: {} });
  expect(approve.status()).toBe(403);
});

test("the dashboard has no inbox and no dismiss; it carries a per-adult count", async () => {
  const view: J = await (await device.get(`${api}/dashboard`)).json();
  expect(view).not.toHaveProperty("inbox");
  expect(view.pending_approvals.every((p: J) => typeof p.count === "number")).toBe(true);
  // Only adults are counted.
  for (const p of view.pending_approvals) expect([M.Alex, M.Sam]).toContain(p.member_id);
  // The dismiss route and the old /members web route are gone for good.
  const adv: J[] = await (await alex.get(`${api}/advisories`)).json();
  const id = adv[0]?.id ?? M.Alex;
  expect([404, 405]).toContain((await alex.post(`${api}/advisories/${id}/dismiss`, { data: {} })).status());
  for (const a of adv) expect(a).not.toHaveProperty("dismissed_at");
});

test("an owners-only source's proposal is in the owner's inbox and count, not in the other adult's", async () => {
  test.setTimeout(180_000);
  const before = { alex: await pending(M.Alex), sam: await pending(M.Sam) };

  const created = await alex.post(`${api}/mail-sources`, {
    data: {
      provider: "imap",
      name: "Private mailbox",
      address: MAILBOX,
      owners: [M.Alex],
      visibility: "owners",
      read_only: true,
      backfill_days: 30,
      imap: { host: "127.0.0.1", port: seed.imapPort, username: MAILBOX, password: "pw" },
    },
  });
  expect(created.ok(), await created.text()).toBeTruthy();
  const source: J = await created.json();
  const sourceId = source.id ?? source.source?.id;

  try {
    const stamp = Date.now();
    const title = `Owners only recital ${stamp}`;
    const d = new Date(Date.now() + 6 * 86_400_000).toISOString().slice(0, 10).replaceAll("-", "");
    await sendRaw({
      port: seed.smtpPort,
      to: MAILBOX,
      raw: [
        `From: "Ms. Teacher" <teacher@example.edu>`,
        `To: ${MAILBOX}`,
        `Subject: Recital ${stamp}`,
        `Date: ${new Date().toUTCString()}`,
        `Message-ID: <owners-only-${stamp}@example.edu>`,
        `MIME-Version: 1.0`,
        `Content-Type: multipart/alternative; boundary="b1"`,
        ``,
        `--b1`,
        `Content-Type: text/plain; charset=utf-8`,
        ``,
        `Dear families, the recital is on. The invite is attached.`,
        `--b1`,
        `Content-Type: text/calendar; charset=utf-8; method=REQUEST`,
        ``,
        `BEGIN:VCALENDAR`,
        `VERSION:2.0`,
        `PRODID:-//example.edu//EN`,
        `METHOD:REQUEST`,
        `BEGIN:VEVENT`,
        `UID:owners-only-${stamp}@example.edu`,
        `DTSTAMP:20261026T090000Z`,
        `DTSTART:${d}T150000Z`,
        `DTEND:${d}T170000Z`,
        `SUMMARY:${title}`,
        `END:VEVENT`,
        `END:VCALENDAR`,
        `--b1--`,
        ``,
      ].join("\n"),
    });
    const bin = process.env.FA_E2E_BIN ?? "fa-server";
    const proposal: J = await poll("the owners-only proposal in Alex's inbox", async () => {
      execFileSync(bin, ["mail", "sync-now"], { stdio: "pipe" });
      execFileSync(bin, ["mail", "process-now"], { stdio: "pipe" });
      return (await inboxEvents(alex)).find((e) => e.title === title);
    });

    // Sam may not see it, list it, count it or decide it.
    expect((await inboxEvents(sam)).some((e) => e.title === title)).toBe(false);
    const samTry = await sam.post(`${api}/inbox/event/${proposal.id}/approve`, { data: {} });
    expect([403, 404]).toContain(samTry.status());
    expect((await inboxEvents(alex)).some((e) => e.title === title)).toBe(true);

    expect(await pending(M.Alex)).toBe(before.alex + 1);
    expect(await pending(M.Sam)).toBe(before.sam);

    // Alex rejects it: the count goes back.
    const rejected = await alex.post(`${api}/inbox/event/${proposal.id}/reject`, { data: {} });
    expect(rejected.status()).toBe(204);
    expect(await pending(M.Alex)).toBe(before.alex);
  } finally {
    await alex.delete(`${api}/mail-sources/${sourceId}`);
  }
});

test("the household can turn the per-adult count off", async () => {
  const get = async (): Promise<J> => (await alex.get(`${api}/household`)).json();
  const hh = await get();
  expect(hh.dashboard_pending_approvals).toBe(true);
  const patch = async (on: boolean, version: number) => {
    const res = await alex.patch(`${api}/household`, {
      data: { dashboard_pending_approvals: on },
      headers: { "If-Match": `"${version}"` },
    });
    expect(res.ok()).toBeTruthy();
    return res.json();
  };
  const off = await patch(false, hh.version);
  try {
    expect(off.dashboard_pending_approvals).toBe(false);
    const view: J = await (await device.get(`${api}/dashboard`)).json();
    expect(view.pending_approvals).toEqual([]);
  } finally {
    await patch(true, (await get()).version);
  }
});

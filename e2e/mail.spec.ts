import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { connect } from "node:net";
import {
  expect,
  test,
  type APIRequestContext,
  type Playwright,
} from "@playwright/test";

import { sendRaw } from "./mail-send.ts";

// M3 API specs. They need the stack from scripts/e2e.sh: GreenMail (IMAP + SMTP), a writable
// Radicale calendar, ntfy, api and worker as separate processes, and the data from e2e/seed.mjs
// (fictional org, enrollments, an IMAP source with seven fixture mails from e2e/fixtures/mail).
// The tests build on each other, so they run serially.
const BASE = process.env.FA_E2E_BASE_URL ?? "http://127.0.0.1:18080";
const NTFY = process.env.FA_E2E_NTFY ?? "http://127.0.0.1:18093";
const RADICALE = process.env.FA_E2E_RADICALE ?? "http://127.0.0.1:18092";
const seed = JSON.parse(
  readFileSync(process.env.FA_E2E_SEED ?? "", "utf8"),
) as {
  device: { token: string };
  members: Record<string, string>;
  sources: Record<string, string>;
  mailSource: string;
  mailbox: string;
  imapPort: number;
  smtpPort: number;
};
const M = seed.members;

/* eslint-disable @typescript-eslint/no-explicit-any */
type J = any;
test.describe.configure({ mode: "serial" });

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

async function poll<T>(
  what: string,
  fn: () => Promise<T | undefined | false>,
  timeoutMs = 30_000,
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

const api = "/api/v1";
const messages = async (ctx: APIRequestContext = alex): Promise<J[]> => {
  const res = await ctx.get(`${api}/messages?limit=100&include_ignored=true`);
  expect(res.ok()).toBeTruthy();
  return (await res.json()).items;
};
const msg = async (subject: string, ctx: APIRequestContext = alex) => {
  const found = (await messages(ctx)).filter((m) => m.subject === subject);
  expect(found, `messages titled ${subject}`).toHaveLength(1);
  return found[0];
};
const detail = async (id: string, query = "", ctx = alex): Promise<J> => {
  const res = await ctx.get(`${api}/messages/${id}${query}`);
  expect(res.ok()).toBeTruthy();
  return res.json();
};
const inbox = async (ctx: APIRequestContext = alex): Promise<J> => {
  const res = await ctx.get(`${api}/inbox`);
  expect(res.ok()).toBeTruthy();
  return res.json();
};
const inboxItems = async (kind: string, ctx = alex): Promise<J[]> =>
  (await inbox(ctx)).items
    .filter((i: J) => i.kind === kind)
    .map((i: J) => i[kind]);
const decide = (
  ctx: APIRequestContext,
  kind: string,
  id: string,
  verb: string,
  data: J = {},
) => ctx.post(`${api}/inbox/${kind}/${id}/${verb}`, { data });
const runMail = () => {
  const bin = process.env.FA_E2E_BIN ?? "fa-server";
  execFileSync(bin, ["mail", "sync-now"], { stdio: "pipe" });
  execFileSync(bin, ["mail", "process-now"], { stdio: "pipe" });
};

const SUBJECT = {
  invite: "Science fair for grade 3 (class 3B)",
  news: "Example Elementary weekly news",
  bill: "Your Example Utilities statement is ready",
  paid: "Payment received, thank you",
  slip: "Museum trip permission slip, grade 3",
  promo: "50% off everything this weekend only!",
  newhire: "Hello from the new class 3B assistant",
};

/** Flags of the INBOX messages of the GreenMail mailbox, keyed by subject (plain IMAP). */
async function imapFlags(): Promise<Record<string, string>> {
  const lines: string[] = await new Promise((resolve, reject) => {
    const sock = connect(seed.imapPort, "127.0.0.1");
    let buf = "";
    const cmds = [
      `a1 LOGIN ${seed.mailbox} pw`,
      "a2 SELECT INBOX",
      "a3 FETCH 1:* (FLAGS BODY.PEEK[HEADER.FIELDS (SUBJECT)])",
      "a4 LOGOUT",
    ];
    let i = 0;
    sock.setTimeout(15_000, () => reject(new Error("imap timeout")));
    sock.on("error", reject);
    sock.on("data", (c) => {
      buf += c.toString("utf8");
      if (new RegExp(`^a${i + 1} (OK|NO|BAD)`, "m").test(buf)) {
        i += 1;
        if (i === cmds.length) return resolve(buf.split("\r\n"));
        sock.write(`${cmds[i]}\r\n`);
      }
    });
    sock.write(`${cmds[0]}\r\n`);
  });
  const out: Record<string, string> = {};
  let flags = "";
  for (const l of lines) {
    const f = /FLAGS \(([^)]*)\)/.exec(l);
    if (f) flags = f[1];
    const s = /^Subject: (.*)$/i.exec(l);
    if (s) out[s[1].trim()] = flags;
  }
  return out;
}

// ---- sync, classification, links ------------------------------------------------------------

test("the IMAP source synced the seven fixture mails and processed them all", async () => {
  const list = await messages();
  expect(list).toHaveLength(7);
  for (const m of list) expect(m.processing_state).toBe("processed");
  const sources = await (await alex.get(`${api}/mail-sources`)).json();
  const src = sources.find((s: J) => s.id === seed.mailSource);
  expect(src.provider).toBe("imap");
  expect(src.state).toBe("ok");
  expect(src.read_only).toBe(false);
  expect(src.message_count).toBe(7);
  // The password never comes back.
  expect(JSON.stringify(src)).not.toContain('"pw"');
});

test("categories, bill flag and member links follow the rules", async () => {
  const cat = async (s: string) => (await msg(s)).category;
  expect(await cat(SUBJECT.invite)).toBe("school");
  expect(await cat(SUBJECT.news)).toBe("school");
  expect(await cat(SUBJECT.slip)).toBe("school");
  expect(await cat(SUBJECT.newhire)).toBe("school");
  expect(await cat(SUBJECT.bill)).toBe("bills");
  expect(await cat(SUBJECT.paid)).toBe("bills");
  expect(await cat(SUBJECT.promo)).toBe("newsletters"); // List-Unsubscribe makes bulk mail a newsletter
  expect((await msg(SUBJECT.bill)).is_bill).toBe(true);
  expect((await msg(SUBJECT.paid)).is_bill).toBe(false);
  // Grade 3, class 3B at Example Elementary: Riley.
  for (const s of [SUBJECT.invite, SUBJECT.news, SUBJECT.slip])
    expect((await msg(s)).members).toContain(M.Riley);
  // Only Riley, never the other child.
  expect((await msg(SUBJECT.invite)).members).not.toContain(M.Jordan);
  // The decisions say how it was decided.
  const d = await detail((await msg(SUBJECT.news)).id);
  const category = d.decisions.find(
    (x: J) => x.decision_point === "email.category",
  );
  expect(category).toMatchObject({
    value: "school",
    stage: "rules",
    is_final: true,
  });
});

test("a newsletter is split into sections with relevance for the child", async () => {
  const id = (await msg(SUBJECT.news)).id;
  const d = await detail(id, `?member=${M.Riley}`);
  const rel = (title: string) =>
    d.sections.find((s: J) => s.title === title)?.relevance;
  expect(rel("Grade 3")).toBe("relevant");
  expect(rel("Grade 5")).not.toBe("relevant");
});

test("a second sync pass adds nothing; a new mail arrives incrementally", async () => {
  runMail();
  expect(await messages()).toHaveLength(7);
  await sendRaw({
    port: seed.smtpPort,
    to: seed.mailbox,
    raw: `From: Example Shop <news@shop.example.org>\nTo: ${seed.mailbox}\nSubject: Spring catalogue\nDate: ${new Date().toUTCString()}\nMessage-ID: <catalogue-1@shop.example.org>\nContent-Type: text/plain; charset=utf-8\n\nNew arrivals. Unsubscribe any time.\n`,
  });
  const res = await alex.post(`${api}/mail-sources/${seed.mailSource}/sync`);
  expect(res.status()).toBe(202);
  await poll("the new mail", async () =>
    (await messages()).find(
      (m) =>
        m.subject === "Spring catalogue" && m.processing_state === "processed",
    ),
  );
  expect(await messages()).toHaveLength(8);
});

// ---- event proposals ----------------------------------------------------------------------

test("the teacher's invite is a proposal in the inbox, never written on its own", async () => {
  const events = await inboxItems("event");
  const fair = events.filter((e) => e.title === "Grade 3 Science Fair");
  expect(fair).toHaveLength(1);
  expect(fair[0]).toMatchObject({
    location: "School gym",
    confidence: 0.95,
    pending_cancel: false,
  });
  expect(fair[0].members).toContain(M.Riley);
  expect(fair[0].suggested_calendar).toBeTruthy();
  const d = await detail((await msg(SUBJECT.invite)).id);
  expect(d.events.map((e: J) => e.title)).toContain("Grade 3 Science Fair");
  // Not in the calendar yet.
  const from = new Date().toISOString();
  const to = new Date(Date.now() + 14 * 86_400_000).toISOString();
  const cal = await (
    await alex.get(`${api}/events?from=${from}&to=${to}`)
  ).json();
  expect(cal.filter((e: J) => e.title === "Grade 3 Science Fair")).toHaveLength(
    0,
  );
});

const radicaleHrefs = async (): Promise<string> => {
  const res = await fetch(`${RADICALE}/e2e/family/`, {
    method: "PROPFIND",
    headers: {
      Authorization: `Basic ${Buffer.from("e2e:e2e").toString("base64")}`,
      Depth: "1",
    },
  });
  return res.text();
};

test("approving the event writes it to CalDAV, and the next sync shows it once", async () => {
  const proposal = (await inboxItems("event")).find(
    (e) => e.title === "Grade 3 Science Fair",
  );
  const res = await decide(alex, "event", proposal.id, "approve");
  expect(res.status()).toBe(204);
  await poll(
    "the CalDAV write",
    async () => (await radicaleHrefs()).includes(`fa-${proposal.id}`),
    60_000,
  );
  expect((await inboxItems("event")).some((e) => e.id === proposal.id)).toBe(
    false,
  );

  // The CLI syncs now; the API's "Sync now" is skipped when a scheduled sync ran in the same slot.
  const sync = async () => {
    execFileSync(
      process.env.FA_E2E_BIN ?? "fa-server",
      ["calendar", "sync", seed.sources["Family CalDAV"]],
      { stdio: "pipe" },
    );
  };
  const fairs = async () => {
    const from = new Date().toISOString();
    const to = new Date(Date.now() + 14 * 86_400_000).toISOString();
    const cal: J[] = await (
      await alex.get(`${api}/events?from=${from}&to=${to}`)
    ).json();
    return cal.filter((e) => e.title === "Grade 3 Science Fair");
  };
  await sync();
  await poll(
    "the event in the calendar",
    async () => (await fairs()).length > 0,
    30_000,
  );
  await new Promise((r) => setTimeout(r, 3_000));
  await sync();
  await new Promise((r) => setTimeout(r, 5_000));
  expect(await fairs()).toHaveLength(1);
});

test("rejecting a proposal removes it and writes nothing", async () => {
  const proposal = (await inboxItems("event")).find(
    (e) => e.title === SUBJECT.bill,
  );
  expect(
    proposal,
    "the date phrase of the bill mail is proposed as an event",
  ).toBeTruthy();
  const res = await decide(alex, "event", proposal.id, "reject");
  expect(res.status()).toBe(204);
  expect((await inboxItems("event")).some((e) => e.id === proposal.id)).toBe(
    false,
  );
  expect(await radicaleHrefs()).not.toContain(proposal.id);
});

// ---- bills ----------------------------------------------------------------------------------

test("a bill becomes a pay task on the adult pool; completing it pays the bill and reopening reopens it", async () => {
  const bills: J[] = await (await alex.get(`${api}/bills`)).json();
  expect(bills).toHaveLength(1);
  const bill = bills[0];
  expect(bill).toMatchObject({
    payee: "Example Utilities",
    amount_minor: 8420,
    currency: "USD",
    account_hint: "4821",
    status: "open",
  });
  expect(bill.due_on).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  expect(bill.task).toBeTruthy();

  const task = await (await alex.get(`${api}/tasks/${bill.task}`)).json();
  expect(task.title).toBe("Pay Example Utilities");
  expect(task.list_name).toBe("Bills");
  // The pool is every adult; Alex or Sam can complete it, a child cannot.
  expect(
    task.assignees.length === 0 ||
      task.assignees.every((a: string) => [M.Alex, M.Sam].includes(a)),
  ).toBe(true);
  expect(
    (await riley.post(`${api}/tasks/${bill.task}/complete`, { data: {} })).ok(),
  ).toBe(false);

  const done = await alex.post(`${api}/tasks/${bill.task}/complete`, {
    data: {},
  });
  expect(done.ok()).toBeTruthy();
  const paid = (await (
    await alex.get(`${api}/bills?status=paid`)
  ).json()) as J[];
  expect(paid.map((b) => b.id)).toContain(bill.id);
  expect(paid.find((b) => b.id === bill.id).task_done).toBe(true);

  const reopen = await alex.post(`${api}/tasks/${bill.task}/reopen`);
  expect(reopen.ok()).toBeTruthy();
  const open = (await (
    await alex.get(`${api}/bills?status=open`)
  ).json()) as J[];
  expect(open.map((b) => b.id)).toContain(bill.id);
});

test("the payment-received mail makes no bill and no task", async () => {
  const d = await detail((await msg(SUBJECT.paid)).id);
  expect(d.bills).toHaveLength(0);
  expect(d.tasks).toHaveLength(0);
});

// ---- suggested tasks and contacts -------------------------------------------------------

test("a suggested task is accepted or rejected from the inbox", async () => {
  const tasks = await inboxItems("task");
  expect(tasks.length).toBeGreaterThanOrEqual(1);
  const slip =
    tasks.find((t) => t.title.startsWith("Please sign and return")) ?? tasks[0];
  const ok = await decide(alex, "task", slip.id, "approve");
  expect(ok.status()).toBe(204);
  const accepted = await (await alex.get(`${api}/tasks/${slip.id}`)).json();
  expect(accepted.status).toBe("open");
  expect(accepted.origin).toBe("email");
  for (const other of tasks.filter((t) => t.id !== slip.id)) {
    expect((await decide(alex, "task", other.id, "reject")).status()).toBe(204);
    expect(
      (await (await alex.get(`${api}/tasks/${other.id}`)).json()).status,
    ).toBe("skipped");
  }
  expect(await inboxItems("task")).toHaveLength(0);
});

test("an unknown sender at the school's domain is a contact suggestion; accepting adds the contact", async () => {
  const suggestions = await inboxItems("contact");
  const hire = suggestions.find((c) => c.email === "newhire@example.edu");
  expect(hire).toMatchObject({
    org_name: "Example Elementary",
    name: "Mr. Newhire",
  });
  const res = await decide(alex, "contact", hire.id, "approve", {
    role: "teacher",
  });
  expect(res.status()).toBe(204);
  const orgs: J[] = await (await alex.get(`${api}/orgs`)).json();
  const school = orgs.find((o) => o.name === "Example Elementary");
  const added = school.contacts.find(
    (c: J) => c.email === "newhire@example.edu",
  );
  expect(added).toMatchObject({ name: "Mr. Newhire", role: "teacher" });
  expect((await inboxItems("contact")).some((c) => c.id === hire.id)).toBe(
    false,
  );
});

// ---- labelling ------------------------------------------------------------------------------

test("fixing a label records a human decision and updates the message", async () => {
  const id = (await msg(SUBJECT.news)).id;
  const res = await alex.post(`${api}/messages/${id}/labels`, {
    data: { decision_point: "email.category", value: "newsletters" },
  });
  expect(res.ok()).toBeTruthy();
  const d = await detail(id);
  expect(d.summary.category).toBe("newsletters");
  const human = d.decisions.filter(
    (x: J) =>
      x.decision_point === "email.category" &&
      x.stage === "human" &&
      x.decided_by,
  );
  expect(human.length).toBeGreaterThanOrEqual(1);
  expect(human[0]).toMatchObject({ value: "newsletters", is_final: true });
  expect(human[0].decided_by).toBe(M.Alex);
  // Put it back so later specs see the school category again.
  const back = await alex.post(`${api}/messages/${id}/labels`, {
    data: { decision_point: "email.category", value: "school" },
  });
  expect(back.ok()).toBeTruthy();
});

test("the labelling queue serves messages with progress, and the eval-sample toggle counts", async () => {
  const next = await alex.get(`${api}/labelling/next`);
  expect(next.status()).toBe(200);
  const item = await next.json();
  expect(item.progress).toHaveProperty("target");
  const id = item.message.summary.id;
  const toggle = await alex.post(`${api}/messages/${id}/eval-sample`, {
    data: { in_sample: true },
  });
  expect(toggle.status()).toBe(204);
  expect((await detail(id)).summary.in_eval_sample).toBe(true);
  const after = await (await alex.get(`${api}/labelling/next`)).json();
  expect(after.progress.in_sample).toBeGreaterThanOrEqual(1);
  await alex.post(`${api}/messages/${id}/eval-sample`, {
    data: { in_sample: false },
  });
});

// ---- actions on the mailbox -----------------------------------------------------------------

test("the automatic label action is on the mailbox (IMAP keyword) and can be undone", async () => {
  const id = (await msg(SUBJECT.invite)).id;
  const d = await poll("the automatic label action", async () => {
    const x = await detail(id);
    return x.actions.some((a: J) => a.automatic && a.action === "label") && x;
  });
  const label = d.actions.find(
    (a: J) => a.automatic && a.action === "label" && !a.undone_at,
  );
  expect(label.label).toBe("FA/School");
  expect(label.error ?? null).toBeNull();
  const keyword = (flags: string) =>
    flags.split(" ").some((f) => /^FA_.*School$/.test(f));
  expect(keyword((await imapFlags())[SUBJECT.invite])).toBe(true);

  const undo = await alex.post(`${api}/mail-actions/${label.id}/undo`);
  expect(undo.ok()).toBeTruthy();
  expect((await undo.json()).undone_at).toBeTruthy();
  expect(keyword((await imapFlags())[SUBJECT.invite])).toBe(false);
});

test("a manual archive-free action: mark_read sets \\Seen on the mailbox", async () => {
  const id = (await msg(SUBJECT.promo)).id;
  const res = await alex.post(`${api}/messages/${id}/actions`, {
    data: { action: "mark_read" },
  });
  expect(res.ok()).toBeTruthy();
  expect((await imapFlags())[SUBJECT.promo]).toContain("\\Seen");
});

test("a read-only source refuses actions", async () => {
  const sources: J[] = await (await alex.get(`${api}/mail-sources`)).json();
  const src = sources.find((s) => s.id === seed.mailSource);
  const patch = (readOnly: boolean, version: number) =>
    alex.patch(`${api}/mail-sources/${seed.mailSource}`, {
      data: { read_only: readOnly },
      headers: { "If-Match": `"${version}"` },
    });
  const ro = await patch(true, src.version);
  expect(ro.ok()).toBeTruthy();
  const id = (await msg(SUBJECT.news)).id;
  const res = await alex.post(`${api}/messages/${id}/actions`, {
    data: { action: "mark_read" },
  });
  expect(res.status()).toBe(409);
  const now: J = (await (await alex.get(`${api}/mail-sources`)).json()).find(
    (s: J) => s.id === seed.mailSource,
  );
  expect((await patch(false, now.version)).ok()).toBeTruthy();
});

// ---- visibility -----------------------------------------------------------------------------

test("visibility: household sources are readable by both adults, never by a child or a device", async () => {
  expect((await messages(sam)).length).toBeGreaterThanOrEqual(7);
  expect((await riley.get(`${api}/messages`)).status()).toBe(403);
  expect(
    (
      await riley.get(`${api}/messages/${(await msg(SUBJECT.news)).id}`)
    ).status(),
  ).toBe(403);
  expect((await riley.get(`${api}/mail-sources`)).status()).toBe(403);
  expect((await riley.get(`${api}/inbox`)).status()).toBe(403);
  expect([401, 403]).toContain((await device.get(`${api}/messages`)).status());
  // A device without elevation gets counts only.
  const view = await (await device.get(`${api}/inbox`)).json();
  expect(view.items).toHaveLength(0);
  expect(view.counts).toHaveProperty("total");
});

test("visibility: an 'owners' source is hidden from the other adult", async () => {
  const src: J = (await (await alex.get(`${api}/mail-sources`)).json()).find(
    (s: J) => s.id === seed.mailSource,
  );
  const set = async (visibility: string, version: number) => {
    const r = await alex.patch(`${api}/mail-sources/${seed.mailSource}`, {
      data: { visibility },
      headers: { "If-Match": `"${version}"` },
    });
    expect(r.ok()).toBeTruthy();
    return r.json();
  };
  const owned = await set("owners", src.version);
  const samList = await messages(sam);
  expect(samList.filter((m) => m.source === seed.mailSource)).toHaveLength(0);
  const id = (await msg(SUBJECT.news)).id;
  // Hidden, not just forbidden: the id does not leak that the message exists.
  expect([403, 404]).toContain(
    (await sam.get(`${api}/messages/${id}`)).status(),
  );
  expect((await messages(alex)).length).toBeGreaterThanOrEqual(7);
  await set("household", owned.version ?? (owned as J).source?.version);
  expect((await messages(sam)).length).toBeGreaterThanOrEqual(7);
});

// ---- notifications --------------------------------------------------------------------------

const topicOf = async (
  ctx: APIRequestContext,
  member: string,
): Promise<string> => {
  const res = await ctx.get(`${api}/members/${member}/notify-prefs`);
  expect(res.ok()).toBeTruthy();
  return (await res.json()).topic;
};
const ntfy = async (topic: string): Promise<J[]> => {
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
// Approval rows are titled by what they ask for. A batch that also holds rows of another
// trigger (the M2 specs assign tasks to Alex at the same time) opens /tasks instead of /inbox, so
// the match is on the text, and the link only has to be right for a pure approval message.
const isApproval = (m: J) =>
  /Event suggested|Task suggested|contact needs your confirmation/i.test(
    `${m.title ?? ""} ${m.message}`,
  );

test("approval_needed reaches ntfy for a batched adult and nothing for a digest adult", async () => {
  // A batched message waits a minute and then for the next minute job.
  test.setTimeout(240_000);
  expect(
    (await (await alex.get(`${api}/members/${M.Alex}/notify-prefs`)).json())
      .approval_mode,
  ).toBe("batched");
  expect(
    (await (await sam.get(`${api}/members/${M.Sam}/notify-prefs`)).json())
      .approval_mode,
  ).toBe("digest");
  const alexTopic = await topicOf(alex, M.Alex);
  const samTopic = await topicOf(sam, M.Sam);
  const got = await poll(
    "an approval notification for Alex",
    async () => (await ntfy(alexTopic)).find(isApproval),
    180_000,
    5_000,
  );
  expect(String(got.click ?? "")).toMatch(/\/(inbox|tasks)$/);
  expect((await ntfy(samTopic)).filter(isApproval)).toHaveLength(0);
});

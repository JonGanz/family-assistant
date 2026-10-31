import { execFileSync } from 'node:child_process'
import { expect, test, type BrowserContext, type Page } from '@playwright/test'
import { sendRaw } from '../../../e2e/mail-send.ts'

// UI specs for M3 (arch/Frontend.md section 3b): Settings -> Mail, the Mail list and message,
// the Inbox, the labelling view and the dashboard `inbox` panel. They run against the stack from
// scripts/e2e.sh (GreenMail on E2E_IMAP_PORT/E2E_SMTP_PORT, a writable Radicale calendar) and
// create their own mailbox and mails, so they do not depend on the API specs' data.
// Env: E2E_BASE_URL, E2E_USERNAME/E2E_PASSWORD (adult), E2E_IMAP_PORT, E2E_SMTP_PORT, FA_E2E_BIN
// and the server environment that scripts/e2e.sh exports (for `mail sync-now`).
const adult = {
  user: process.env.E2E_USERNAME ?? 'alex@example.com',
  pass: process.env.E2E_PASSWORD ?? '',
}
const BIN = process.env.FA_E2E_BIN ?? 'fa-server'
const IMAP_PORT = Number(process.env.E2E_IMAP_PORT ?? 18143)
const SMTP_PORT = Number(process.env.E2E_SMTP_PORT ?? 18025)
const MAILBOX = 'ui@example.com'
const SOURCE = 'UI mailbox'

const day = (n: number) => {
  // Calendar-day arithmetic on the New York date: n * 24 h is a day off across a DST change.
  const [y, m, d] = new Intl.DateTimeFormat('en-CA', { timeZone: 'America/New_York' })
    .format(new Date())
    .split('-')
    .map(Number)
  return new Date(Date.UTC(y, m - 1, d + n)).toISOString().slice(0, 10).replaceAll('-', '')
}

function invite(summary: string, uid: string, inDays: number) {
  return `From: "Ms. Teacher" <teacher@example.edu>
To: ${MAILBOX}
Subject: ${summary} for grade 3 (class 3B)
Date: ${new Date().toUTCString()}
Message-ID: <${uid}@example.edu>
MIME-Version: 1.0
Content-Type: multipart/alternative; boundary="b1"

--b1
Content-Type: text/plain; charset=utf-8

Dear families of grade 3, class 3B: the invite is attached.
--b1
Content-Type: text/calendar; charset=utf-8; method=REQUEST

BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//example.edu//EN
METHOD:REQUEST
BEGIN:VEVENT
UID:${uid}@example.edu
DTSTAMP:${day(0)}T090000Z
DTSTART:${day(inDays)}T180000Z
DTEND:${day(inDays)}T190000Z
SUMMARY:${summary}
END:VEVENT
END:VCALENDAR
--b1--
`
}

async function uiLogin(page: Page, who: { user: string; pass: string }) {
  await page.goto('/login')
  await page.getByLabel('Username').fill(who.user)
  await page.getByLabel('Password').fill(who.pass)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/$/)
}

const syncNow = () => {
  execFileSync(BIN, ['mail', 'sync-now', '--source', SOURCE], { stdio: 'pipe' })
  execFileSync(BIN, ['mail', 'process-now'], { stdio: 'pipe' })
}

/** The inbox list item whose title field holds `title` (the field is a v-model, not an attribute). */
async function eventItem(page: Page, title: string) {
  const items = page.locator('li.item')
  await expect(items.first()).toBeVisible()
  for (let i = 0; i < (await items.count()); i++) {
    const item = items.nth(i)
    if (
      (await item
        .getByLabel('Title')
        .inputValue()
        .catch(() => '')) === title
    )
      return item
  }
  throw new Error(`no inbox item titled ${title}`)
}

test.describe.configure({ mode: 'serial' })

// One login for the whole file: the server rate-limits logins per IP and the other UI specs log in
// as well, so six more logins from the same address would starve them.
let ctx: BrowserContext
let page: Page
test.beforeAll(async ({ browser }) => {
  ctx = await browser.newContext({ extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.3' } })
  page = await ctx.newPage()
  await uiLogin(page, adult)
})
test.afterAll(async () => {
  await ctx.close()
})

test('Settings -> Mail: add an IMAP source with Test connection', async () => {
  await page.goto('/settings/mail')
  const form = page.getByTestId('add-mail-source')
  await form.getByLabel('Type').selectOption('imap')
  await form.getByLabel('Name', { exact: true }).fill(SOURCE)
  await form.getByLabel('Email address (user name)', { exact: true }).fill(MAILBOX)
  await form.getByLabel('Server', { exact: true }).fill('127.0.0.1')
  await form.getByLabel('Port', { exact: true }).fill(String(IMAP_PORT))
  await form.getByLabel('App password', { exact: true }).fill('pw')
  await form.getByRole('button', { name: 'Test connection' }).click()
  await expect(page.getByTestId('imap-test')).toContainText('Connected')
  await form.getByRole('button', { name: 'Add source' }).click()
  await expect(page.getByText(SOURCE, { exact: true }).first()).toBeVisible()

  // Editing a row sends If-Match (pause and resume are PATCHes).
  const row = page.getByTestId(`mail-source-${SOURCE}`)
  await row.getByRole('button', { name: 'Pause' }).click()
  await expect(row.getByTestId('state')).toHaveText('paused')
  await row.getByRole('button', { name: 'Resume' }).click()
  await expect(row.getByTestId('state')).toHaveText('ok')

  // Deliver mail to the new mailbox and sync it.
  await sendRaw({ port: SMTP_PORT, to: MAILBOX, raw: invite('Spring Concert', 'ui-concert-1', 6) })
  await sendRaw({ port: SMTP_PORT, to: MAILBOX, raw: invite('Winter Recital', 'ui-recital-1', 8) })
  for (const [subject, body] of [
    [
      'Grade 3 news from class 3B',
      'Hello grade 3 families.\n\nGrade 3\nPumpkin art on Thursday.\n',
    ],
    ['Grade 3 reading week', 'Reading week starts on Monday for grade 3, class 3B.\n'],
  ])
    await sendRaw({
      port: SMTP_PORT,
      to: MAILBOX,
      raw: `From: "Ms. Teacher" <teacher@example.edu>\nTo: ${MAILBOX}\nSubject: ${subject}\nDate: ${new Date().toUTCString()}\nMessage-ID: <${Date.now()}-${subject.length}@example.edu>\nContent-Type: text/plain; charset=utf-8\n\n${body}`,
    })
  syncNow()
})

test('Mail list: filter by child, open a message', async () => {
  await page.goto('/mail')
  await page.getByLabel('Member').selectOption({ label: 'Riley' })
  await page.getByRole('button', { name: 'Search' }).click()
  const row = page.getByRole('link', { name: /Spring Concert for grade 3/ })
  await expect(row).toBeVisible()
  await expect(row.getByText('School')).toBeVisible()
  await page.getByLabel('Member').selectOption({ label: 'Jordan' })
  await page.getByRole('button', { name: 'Search' }).click()
  await expect(page.getByRole('link', { name: /Spring Concert for grade 3/ })).toHaveCount(0)
  await expect(page.getByText('No messages.')).toBeVisible()

  await page.getByLabel('Member').selectOption({ label: 'Riley' })
  await page.getByRole('button', { name: 'Search' }).click()
  await page.getByRole('link', { name: /Grade 3 news from class 3B/ }).click()
  await expect(page).toHaveURL(/\/mail\/[0-9a-f-]+$/)
  await expect(page.getByRole('heading', { name: /Grade 3 news from class 3B/ })).toBeVisible()
  await expect(page.getByText('Pumpkin art on Thursday.').first()).toBeVisible()
})

test('Inbox: approve an event proposal', async () => {
  await page.goto('/inbox')
  await page.getByRole('tab', { name: /^Events/ }).click()
  const item = await eventItem(page, 'Spring Concert')
  const badge = page.getByRole('link', { name: /Inbox/ })
  await expect(badge).toContainText(/\d/)
  await item.getByRole('button', { name: /^Approve/ }).click()
  await expect(page.locator('li.item').filter({ hasText: 'Spring Concert' })).toHaveCount(0)
  // The other proposal is still waiting.
  await eventItem(page, 'Winter Recital')
})

test('Labelling view: one-tap confirm and progress', async () => {
  await page.goto('/mail/label')
  const progress = page.locator('label.progress')
  await expect(progress).toContainText(/\d+ of \d+ labelled/)
  const labelled = async () =>
    Number(/(\d+) of \d+ labelled/.exec((await progress.textContent()) ?? '')?.[1])
  const before = await labelled()
  const subject = await page.locator('article.card h2').textContent()
  // Progress counts the evaluation-sample messages a member has labelled.
  const sampled = page.waitForResponse((r) => r.url().includes('/eval-sample') && r.ok())
  await page.getByLabel('In the evaluation sample').check()
  await sampled
  await page.getByRole('button', { name: 'Looks right' }).click()
  await expect.poll(labelled).toBeGreaterThan(before)
  await expect(page.locator('article.card h2')).not.toHaveText(subject ?? '')
  await page.getByRole('button', { name: 'Skip' }).click()
})

test("Dashboard: no inbox, no dismiss; the members strip shows each adult's pending count", async ({
  browser,
}) => {
  const members = await (await page.request.get('/api/v1/members')).json()
  const list = Array.isArray(members) ? members : (members.items ?? members.members)
  const alex = list.find((m: { display_name: string }) => m.display_name === 'Alex')

  // A fresh proposal, so that Alex has something pending whatever the earlier specs decided.
  const stamp = Date.now()
  await sendRaw({
    port: SMTP_PORT,
    to: MAILBOX,
    raw: invite(`Badge check ${stamp}`, `ui-badge-${stamp}`, 9),
  })
  syncNow()

  const res = await page.request.post('/api/v1/devices', {
    data: { name: `e2e-m3-${Date.now()}` },
    headers: { 'X-FA-Request': '1' },
  })
  expect(res.ok()).toBeTruthy()
  const paired = await res.json()
  const dev = await (await page.request.get(`/api/v1/devices/${paired.id}`)).json()
  const patched = await page.request.patch(`/api/v1/devices/${paired.id}`, {
    headers: { 'X-FA-Request': '1', 'If-Match': `"${dev.version}"` },
    data: {
      settings: {
        theme: 'auto',
        agenda_days: 3,
        members: 'all',
        panels: [{ kind: 'clock' }, { kind: 'members' }, { kind: 'advisories' }],
      },
    },
  })
  expect(patched.ok()).toBeTruthy()
  const kiosk = await browser.newContext({ viewport: { width: 1920, height: 1080 } })
  const screen = await kiosk.newPage()
  await screen.goto(paired.pair_url)

  // Alex is the owner of the mail sources, so the seeded proposals count for them (D-078).
  const badge = screen.getByTestId(`approvals-${alex.id}`)
  await expect(badge).toBeVisible()
  expect(Number((await badge.textContent()) ?? '0')).toBeGreaterThanOrEqual(1)

  // D-068: the dashboard shows no inbox at all and offers no way to dismiss an advisory.
  await expect(screen.getByTestId('inbox-total')).toHaveCount(0)
  await expect(screen.getByTestId('inbox-counts')).toHaveCount(0)
  await expect(screen.getByRole('button', { name: 'Approve' })).toHaveCount(0)
  await expect(screen.getByRole('button', { name: /dismiss/i })).toHaveCount(0)
  // The device is refused the inbox itself, with or without elevation.
  const inbox = await kiosk.request.get('/api/v1/inbox')
  expect([401, 403]).toContain(inbox.status())
  await kiosk.close()
})

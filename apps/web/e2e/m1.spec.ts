import { expect, test, type Browser, type Page } from '@playwright/test'

// Logins are limited per client address. The server trusts X-Forwarded-For from loopback, so each
// file logs in from its own made-up address and the suite does not share one budget.
test.use({ extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.1' } })

// UI specs for M1 (arch/Frontend.md section 7). They run against the stack from scripts/e2e.sh:
// api and worker as separate processes, fixture feeds and Radicale. Env: E2E_BASE_URL,
// E2E_USERNAME/E2E_PASSWORD (adult), E2E_CHILD_USERNAME/E2E_CHILD_PASSWORD, E2E_FIXTURES.
const adult = {
  user: process.env.E2E_USERNAME ?? 'alex@example.com',
  pass: process.env.E2E_PASSWORD ?? '',
}
const child = {
  user: process.env.E2E_CHILD_USERNAME ?? 'riley@example.com',
  pass: process.env.E2E_CHILD_PASSWORD ?? '',
}
const FIXTURES = process.env.E2E_FIXTURES ?? 'http://127.0.0.1:18091'

async function uiLogin(page: Page, who: { user: string; pass: string }) {
  await page.goto('/login')
  await page.getByLabel('Username').fill(who.user)
  await page.getByLabel('Password').fill(who.pass)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/$/)
}

/** Pairs a device through the API and opens its dashboard in a fresh context (the kiosk). */
async function openKiosk(page: Page, browser: Browser) {
  await uiLogin(page, adult)
  const res = await page.request.post('/api/v1/devices', {
    data: { name: `e2e-${Date.now()}` },
    headers: { 'X-FA-Request': '1' },
  })
  expect(res.ok()).toBeTruthy()
  const { pair_url: pairUrl } = await res.json()
  const ctx = await browser.newContext()
  const screen = await ctx.newPage()
  await screen.goto(pairUrl)
  return { screen, ctx }
}

test('dashboard renders the agenda, the weather and an advisory', async ({ page, browser }) => {
  const { screen, ctx } = await openKiosk(page, browser)
  await expect(screen.getByRole('region', { name: 'Agenda' })).toContainText('Spirit Day')
  await expect(screen.getByTestId('temp')).toBeVisible()
  await expect(screen.getByRole('region', { name: 'Advisories' })).toContainText('Frost tonight')
  await ctx.close()
})

test('a rule evaluated by the worker shows up on the dashboard without a reload', async ({
  page,
  browser,
}) => {
  const { screen, ctx } = await openKiosk(page, browser)
  const advisories = screen.getByRole('region', { name: 'Advisories' })
  await expect(advisories).toContainText('Frost tonight')
  const text = `Live update ${Date.now()}`
  const res = await page.request.post('/api/v1/weather-rules', {
    data: {
      name: `E2E live ${Date.now()}`,
      condition: 'max(temp_c, today) < 100',
      action: 'alert',
      severity: 'info',
      message: text,
    },
    headers: { 'X-FA-Request': '1' },
  })
  expect(res.status()).toBe(201) // the worker evaluates it and the api pushes advisory.updated
  await expect(advisories).toContainText(text, { timeout: 20_000 })
  await ctx.close()
})

test('a child login sees only permitted events', async ({ page }) => {
  await uiLogin(page, child)
  await page.goto('/calendar')
  await page.getByRole('button', { name: 'agenda' }).click()
  await expect(page.getByText('Spirit Day').first()).toBeVisible() // Riley's school calendar
  // Check this week and the next (the fixture events are a couple of days out).
  for (let week = 0; week < 2; week++) {
    await expect(page.getByText('Toddler Music Class')).toHaveCount(0) // Jordan's calendar
    await expect(page.getByText('Dentist Checkup')).toHaveCount(0) // an adult's private event
    await page.getByRole('button', { name: 'Next' }).click()
  }
})

test('settings: add an ICS source and enable its calendar', async ({ page }) => {
  await uiLogin(page, adult)
  await page.goto('/settings/calendars')
  await page.getByRole('button', { name: 'ICS link' }).click()
  await page.getByLabel('Name').fill('UI Added Feed')
  await page.getByLabel('Calendar link').fill(`${FIXTURES}/ics/jordan.ics`)
  await page.getByRole('button', { name: 'Test' }).click()
  await expect(page.getByTestId('test-result')).toContainText('It works')
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await expect(page.getByTestId('source-UI Added Feed')).toBeVisible()
  // The worker's first sync bumps the calendar's version; toggling with a stale version is a
  // 409. Wait for it, then reload so the page holds the current versions.
  await expect
    .poll(async () => {
      const res = await page.request.get('/api/v1/calendar-sources')
      const rows = (await res.json()) as { name: string; last_synced_at?: string | null }[]
      return rows.find((r) => r.name === 'UI Added Feed')?.last_synced_at ?? null
    })
    .not.toBeNull()
  await page.reload()
  const source = page.getByTestId('source-UI Added Feed')
  await expect(source).toBeVisible()
  const show = source.getByRole('checkbox', { name: /^Show / })
  await expect(show).toBeChecked() // discovered calendars start enabled
  const patched = () =>
    page.waitForResponse(
      (r) => r.url().includes('/api/v1/calendars/') && r.request().method() === 'PATCH',
    )
  let saved = patched()
  await show.uncheck()
  expect((await saved).status()).toBe(200)
  saved = patched()
  await show.check()
  expect((await saved).status()).toBe(200)
  await page.reload()
  await expect(
    page.getByTestId('source-UI Added Feed').getByRole('checkbox', { name: /^Show / }),
  ).toBeChecked()
})

test('settings: a weather rule with an invalid condition shows an error, then a preview', async ({
  page,
}) => {
  await uiLogin(page, adult)
  await page.goto('/settings/weather')
  await page.getByRole('button', { name: 'Add a rule' }).click()
  await page.getByLabel('Name').fill('E2E invalid then valid')
  await page.getByLabel('Condition', { exact: true }).fill('max(temp_c, today) >')
  await expect(page.getByTestId('rule-error')).toBeVisible()
  await page.getByLabel('Condition', { exact: true }).fill('max(temp_c, today) > -50')
  await expect(page.getByTestId('rule-preview')).toContainText('would fire')
  await page.getByLabel('Message').fill('Warm enough, e2e')
  await page.getByRole('button', { name: 'Save' }).click()
  await expect(page.getByTestId('rule-E2E invalid then valid')).toBeVisible()
})

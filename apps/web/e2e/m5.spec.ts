import { expect, test, type BrowserContext, type Page } from '@playwright/test'

// UI specs for M5 on the seeded stack from scripts/e2e.sh: recipes (import from a fixture page),
// the meal plan, the shopping list (offline included) and the kiosk Food view. Everything is
// fictional (D-035). Desktop width: the `food-tab-*` links exist only at medium and expanded
// widths. Env: E2E_BASE_URL, E2E_USERNAME/E2E_PASSWORD (adult), E2E_CHILD_USERNAME/E2E_CHILD_PASSWORD,
// E2E_FIXTURES (the fixture server, which serves the recipe pages).
const FIXTURES = process.env.E2E_FIXTURES ?? 'http://127.0.0.1:18091'
const adult = {
  user: process.env.E2E_USERNAME ?? 'alex@example.com',
  pass: process.env.E2E_PASSWORD ?? '',
}
const child = {
  user: process.env.E2E_CHILD_USERNAME ?? 'riley@example.com',
  pass: process.env.E2E_CHILD_PASSWORD ?? '',
}
const NY = 'America/New_York'

/** The New York calendar date N days from today. */
const day = (n: number) => {
  const [y, m, d] = new Intl.DateTimeFormat('en-CA', { timeZone: NY })
    .format(new Date())
    .split('-')
    .map(Number)
  return new Date(Date.UTC(y!, m! - 1, d! + n)).toISOString().slice(0, 10)
}

async function uiLogin(page: Page, who: { user: string; pass: string }) {
  await page.goto('/login')
  await page.getByLabel('Username').fill(who.user)
  await page.getByLabel('Password').fill(who.pass)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/$/)
}

test.describe.configure({ mode: 'serial' })
test.use({ viewport: { width: 1280, height: 900 } })

let adultCtx: BrowserContext
let childCtx: BrowserContext
let adultPage: Page
let childPage: Page
test.beforeAll(async ({ browser }) => {
  // One login per person for the whole file: logins are rate-limited per address.
  const ctx = {
    viewport: { width: 1280, height: 900 },
    extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.5' },
  }
  adultCtx = await browser.newContext(ctx)
  adultPage = await adultCtx.newPage()
  await uiLogin(adultPage, adult)
  childCtx = await browser.newContext(ctx)
  childPage = await childCtx.newPage()
  await uiLogin(childPage, child)
})
test.afterAll(async () => {
  // Leave the shared stack as found.
  const api = adultPage.request
  const hdr = { 'X-FA-Request': '1' }
  const plan = await api
    .get(`/api/v1/meal-plan?from=${day(-7)}&to=${day(50)}`)
    .then((r) => r.json())
  for (const e of plan) await api.delete(`/api/v1/meal-plan/${e.id}`, { headers: hdr })
  const g = await api.get('/api/v1/grocery').then((r) => r.json())
  for (const i of g.items) await api.delete(`/api/v1/grocery/items/${i.id}`, { headers: hdr })
  const rs = await api.get('/api/v1/recipes?q=Fictional').then((r) => r.json())
  for (const r of rs.items) await api.delete(`/api/v1/recipes/${r.id}`, { headers: hdr })
  const ts = await api.get('/api/v1/meal-templates').then((r) => r.json())
  for (const t of ts) await api.delete(`/api/v1/meal-templates/${t.id}`, { headers: hdr })
  await adultCtx.close()
  await childCtx.close()
})

const rows = (page: Page) => page.getByTestId(/^grocery-item-/)

/** What the server holds for an item of this text: its checked state (undefined when it is not there). */
async function serverChecked(page: Page, text: string) {
  const g = await (await page.request.get('/api/v1/grocery')).json()
  const it = g.items.find((i: { text: string }) => i.text === text)
  return it ? !!it.checked_at : undefined
}

test('an adult imports a recipe page and saves it', async () => {
  const page = adultPage
  await page.goto('/food/recipes')
  await page.getByTestId('recipe-new').click()
  await page.getByTestId('recipe-import-url').click()
  await page.getByLabel('Recipe link').fill(`${FIXTURES}/recipes/jsonld.html`)
  await page.getByTestId('recipe-import-submit').click()
  await expect(page).toHaveURL(/\/food\/recipes\/new/)
  // The draft lives in memory: no reload between import and save.
  await expect(page.getByLabel('Title')).toHaveValue('Fictional Lentil Stew')
  await expect(page.getByTestId('recipe-line-0')).toBeVisible()
  await expect(page.getByTestId('recipe-line-5')).toBeVisible()
  await page.getByTestId('recipe-save').getByRole('button', { name: 'Save' }).click()
  await expect(page).toHaveURL(/\/food\/recipes\/[0-9a-f-]{36}$/)
  await expect(page.getByRole('heading', { name: 'Fictional Lentil Stew' })).toBeVisible()
  await expect(page.getByTestId('recipe-line-0')).toBeVisible()
  await expect(page.getByTestId('recipe-rating')).toBeVisible()
})

test("an adult adds it to tomorrow's dinner", async () => {
  const page = adultPage
  await page.goto('/food/plan')
  await expect(page.getByTestId('food-tab-plan')).toBeVisible()
  await page.getByTestId(`plan-slot-add-${day(1)}-dinner`).click()
  const sheet = page.getByTestId('entry-sheet')
  await sheet.getByLabel('Search recipes').fill('Lentil')
  await page
    .getByTestId(/^pick-result-/)
    .first()
    .click()
  await page.getByTestId('entry-save').click()
  await expect(page.getByTestId(`plan-cell-${day(1)}-dinner`)).toContainText(
    'Fictional Lentil Stew',
  )
})

test('make the list for the coming week: items are grouped', async () => {
  const page = adultPage
  await page.getByTestId('plan-make-list').click()
  await expect(page.getByTestId('make-list-summary')).toContainText(/1 meal/)
  // Cover tomorrow whatever the default range is.
  const sheet = page.getByTestId('make-list-sheet')
  await sheet.getByLabel('From').fill(day(0))
  await sheet.getByLabel('To', { exact: true }).fill(day(7))
  await page.getByTestId('make-list-confirm').click()
  await page.getByTestId('food-tab-list').click()
  await expect(page).toHaveURL(/\/food\/list/)
  await expect(
    rows(page)
      .filter({ hasText: /lentil/i })
      .first(),
  ).toBeVisible()
  await expect(
    rows(page)
      .filter({ hasText: /carrot/i })
      .first(),
  ).toBeVisible()
  // Grouped: aisle headings separate the items.
  expect(await page.getByRole('heading', { level: 3 }).count()).toBeGreaterThanOrEqual(1)
})

test('applying a template over a free-text entry with Add keeps both', async () => {
  const page = adultPage
  const api = page.request
  const hdr = { 'X-FA-Request': '1' }
  const slots = await (await api.get('/api/v1/meals/slots')).json()
  const dinner = slots.find((s: { name: string }) => s.name === 'Dinner')
  const target = day(3)
  const made = await api.post('/api/v1/meal-plan', {
    headers: hdr,
    data: { date: target, slot_id: dinner.id, free_text: 'Fictional leftovers' },
  })
  expect(made.ok()).toBeTruthy()
  const tpl = await api.post('/api/v1/meal-templates', {
    headers: hdr,
    data: {
      name: 'Fictional one-day plan',
      days: 1,
      cells: [{ day: 1, slot_id: dinner.id, ord: 0, free_text: 'Fictional template dinner' }],
    },
  })
  expect(tpl.ok()).toBeTruthy()

  await page.goto('/food/plan')
  await expect(page.getByTestId(`plan-cell-${target}-dinner`)).toContainText('Fictional leftovers')
  await page.getByTestId('plan-apply-template').click()
  await page.getByTestId('apply-sheet').getByLabel('Start date').fill(target)
  await page.getByTestId('apply-preview').click()
  const conflict = page.getByTestId(/^template-conflict-/)
  await expect(conflict).toBeVisible()
  // Free text is there, so Add is the default (D-120); choose it explicitly.
  await page.getByTestId(/^template-add-/).check()
  await page.getByTestId('template-apply-confirm').click()
  const cell = page.getByTestId(`plan-cell-${target}-dinner`)
  await expect(cell).toContainText('Fictional leftovers')
  await expect(cell).toContainText('Fictional template dinner')
})

test('deleting a planned meal asks about the list and answers it', async () => {
  const page = adultPage
  await page.goto('/food/plan')
  const cell = page.getByTestId(`plan-cell-${day(1)}-dinner`)
  await cell.getByRole('button', { name: /Fictional Lentil Stew/ }).click()
  await page.getByTestId('entry-delete').click()
  const dialog = page.getByTestId('impact-dialog')
  await expect(dialog).toBeVisible()
  await expect(page.getByTestId('impact-item').first()).toBeVisible()
  // The dialog closes before the request is answered: wait for the reconcile itself.
  const reconciled = page.waitForResponse((r) => r.url().includes('/api/v1/grocery/reconcile'))
  await page.getByTestId('impact-update').click()
  expect((await reconciled).ok()).toBeTruthy()
  await expect(dialog).toBeHidden()
  await expect(cell).not.toContainText('Fictional Lentil Stew')
  // The list no longer holds what that meal needed.
  await page.goto('/food/list')
  await expect(page.getByTestId('grocery-add-input')).toBeVisible()
  await expect(rows(page).filter({ hasText: /lentil/i })).toHaveCount(0)
})

test('a child adds and checks an item, and cannot edit or delete', async () => {
  const page = childPage
  await page.goto('/food/list')
  await page.getByLabel('Add an item').fill('xyzzy oat')
  await page.getByLabel('Add an item').press('Enter')
  const row = rows(page).filter({ hasText: 'xyzzy oat' })
  await expect(row).toBeVisible()
  // A checked item leaves the open list for the "checked" section, so the server is the witness.
  await row.getByRole('checkbox').click()
  await expect.poll(() => serverChecked(page, 'xyzzy oat')).toBe(true)
  await expect(page.getByRole('button', { name: /^Delete / })).toHaveCount(0)
  await expect(page.getByRole('button', { name: /^Edit / })).toHaveCount(0)
})

test('offline: items are added and checked, the note shows, and the server catches up', async () => {
  const page = adultPage
  await page.goto('/food/list')
  await expect(page.getByTestId('grocery-add-input')).toBeVisible()
  await expect(page.getByTestId('grocery-offline-note')).toHaveCount(0)

  await adultCtx.setOffline(true)
  const add = page.getByLabel('Add an item')
  await add.fill('xyzzy offline')
  await add.press('Enter')
  await expect(page.getByTestId('grocery-offline-note')).toBeVisible()
  const row = rows(page).filter({ hasText: 'xyzzy offline' })
  await expect(row).toBeVisible()
  await row.getByRole('checkbox').click()
  // Still queued: the server knows nothing yet.
  expect(await serverChecked(page, 'xyzzy offline').catch(() => undefined)).toBeUndefined()

  await adultCtx.setOffline(false)
  await expect(page.getByTestId('grocery-offline-note')).toHaveCount(0, { timeout: 30_000 })
  await expect.poll(() => serverChecked(page, 'xyzzy offline'), { timeout: 30_000 }).toBe(true)
})

test("the kiosk Food view adds an item without a PIN, and the meals panel shows today's dinner", async ({
  browser,
}) => {
  const page = adultPage
  const api = page.request
  const hdr = { 'X-FA-Request': '1' }
  const slots = await (await api.get('/api/v1/meals/slots')).json()
  const dinner = slots.find((s: { name: string }) => s.name === 'Dinner')
  const made = await api.post('/api/v1/meal-plan', {
    headers: hdr,
    data: { date: day(0), slot_id: dinner.id, free_text: 'Fictional kiosk dinner' },
  })
  expect(made.ok()).toBeTruthy()

  const res = await api.post('/api/v1/devices', {
    data: { name: `e2e-m5-${Date.now()}` },
    headers: hdr,
  })
  expect(res.ok()).toBeTruthy()
  const paired = await res.json()
  const dev = await (await api.get(`/api/v1/devices/${paired.id}`)).json()
  const patched = await api.patch(`/api/v1/devices/${paired.id}`, {
    headers: { ...hdr, 'If-Match': `"${dev.version}"` },
    data: {
      settings: {
        theme: 'auto',
        agenda_days: 3,
        members: 'all',
        food_view: true,
        panels: [{ kind: 'clock' }, { kind: 'meals' }],
      },
    },
  })
  expect(patched.ok()).toBeTruthy()

  const kiosk = await browser.newContext({ viewport: { width: 1920, height: 1080 } })
  const screen = await kiosk.newPage()
  await screen.goto(paired.pair_url)
  await expect(screen.getByTestId('panel-meals')).toBeVisible()
  await expect(
    screen.getByTestId('meal-row').filter({ hasText: 'Fictional kiosk dinner' }),
  ).toBeVisible()

  await screen.getByTestId('kiosk-view-food').click()
  // The rolling plan: seven days, today the third row, with today's dinner in it.
  await expect(screen.getByTestId('food-roll').locator('tbody tr')).toHaveCount(7)
  await expect(screen.locator(`[data-testid="food-roll"] tr:nth-child(3)`)).toHaveClass(/today/)
  await expect(screen.getByTestId(`roll-cell-${day(0)}-dinner`)).toContainText(
    'Fictional kiosk dinner',
  )
  await screen.getByTestId('food-tab-shopping').click()
  // An add typed before the list has loaded is dropped, so wait for the list first.
  await expect(
    screen
      .getByText('Nothing on the list.')
      .or(screen.getByRole('button', { name: /^Checked \(/ }))
      .or(rows(screen).first()),
  ).toBeVisible()
  const add = screen.getByLabel('Add an item')
  await add.fill('xyzzy kiosk')
  await add.press('Enter')
  await expect(rows(screen).filter({ hasText: 'xyzzy kiosk' })).toBeVisible()
  // No PIN was asked for, and the server has it.
  await expect(screen.getByRole('dialog')).toHaveCount(0)
  await expect
    .poll(async () => {
      const g = await (await api.get('/api/v1/grocery')).json()
      return g.items.some((i: { text: string }) => i.text === 'xyzzy kiosk')
    })
    .toBe(true)
  await kiosk.close()
})

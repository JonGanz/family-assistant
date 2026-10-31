import { execFileSync } from 'node:child_process'
import { expect, test, type Browser, type Page } from '@playwright/test'

// Logins are limited per client address. The server trusts X-Forwarded-For from loopback, so each
// file logs in from its own made-up address and the suite does not share one budget.
test.use({ extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.2' } })

// UI specs for M2 (arch/Frontend.md sections 3a and 4). They run against the stack from
// scripts/e2e.sh: api and worker as separate processes, a real ntfy, and the data from
// e2e/seed.mjs (the lists Chores, Homework and Errands; the "Morning routine" for Riley).
// Env: E2E_BASE_URL, E2E_USERNAME/E2E_PASSWORD (adult) plus FA_E2E_BIN and the server
// environment that scripts/e2e.sh exports (for `tasks run-now`).
const adult = {
  user: process.env.E2E_USERNAME ?? 'alex@example.com',
  pass: process.env.E2E_PASSWORD ?? '',
}
// The household is in America/New_York: a UTC date is a day ahead in the evening.
const localDay = () =>
  new Intl.DateTimeFormat('en-CA', { timeZone: 'America/New_York' }).format(new Date())
const BIN = process.env.FA_E2E_BIN ?? 'fa-server'

async function uiLogin(page: Page, who: { user: string; pass: string }) {
  await page.goto('/login')
  await page.getByLabel('Username').fill(who.user)
  await page.getByLabel('Password').fill(who.pass)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/$/)
}

const runNow = () => execFileSync(BIN, ['tasks', 'run-now'], { stdio: 'pipe' })

/** Switches the household's dashboard completion override through the API (adult session). */
async function setOverride(page: Page, on: boolean) {
  const h = await (await page.request.get('/api/v1/household')).json()
  const res = await page.request.patch('/api/v1/household', {
    headers: { 'X-FA-Request': '1', 'If-Match': `"${h.version}"` },
    data: { dashboard_completion_override: on },
  })
  expect(res.ok()).toBeTruthy()
}

/** Pairs a device through the API and opens its dashboard in a fresh context (the kiosk). */
async function openKiosk(
  page: Page,
  browser: Browser,
  viewport = { width: 1920, height: 1080 },
  routinePanel: Record<string, unknown> = { kind: 'routine', pinned: true },
  preset: 'hub' | 'board' = 'hub',
) {
  const res = await page.request.post('/api/v1/devices', {
    data: { name: `e2e-m2-${Date.now()}` },
    headers: { 'X-FA-Request': '1' },
  })
  expect(res.ok()).toBeTruthy()
  const paired = await res.json()
  // The default layout has no task panels: use the presets 'Family hub' (Now and the routine) or
  // 'Chores board' (member tasks), as the layout editor offers them.
  const dev = await (await page.request.get(`/api/v1/devices/${paired.id}`)).json()
  const patched = await page.request.patch(`/api/v1/devices/${paired.id}`, {
    headers: { 'X-FA-Request': '1', 'If-Match': `"${dev.version}"` },
    data: {
      settings: {
        theme: 'auto',
        agenda_days: 3,
        members: 'all',
        panels:
          preset === 'board'
            ? [{ kind: 'clock' }, { kind: 'weather', hourly: false }, { kind: 'member_tasks' }]
            : [
                { kind: 'clock' },
                { kind: 'weather', hourly: true },
                { kind: 'advisories' },
                { kind: 'agenda' },
                { kind: 'now' },
                routinePanel,
              ],
      },
    },
  })
  expect(patched.ok()).toBeTruthy()
  const pairUrl = paired.pair_url
  const ctx = await browser.newContext({ viewport })
  const screen = await ctx.newPage()
  await screen.goto(pairUrl)
  return { screen, ctx }
}

test.describe.configure({ mode: 'serial' })

test('a chore made in Settings shows up as a task, completes and undoes', async ({ page }) => {
  await uiLogin(page, adult)
  const title = `Wipe the counters ${Date.now()}`

  await page.goto('/settings/tasks?tab=definitions')
  await page.getByRole('button', { name: 'Add a chore or schedule' }).click()
  await page.getByLabel('Title').fill(title)
  await page.locator('select[name=list]').selectOption({ label: 'Chores' })
  await page.getByRole('checkbox', { name: 'Alex' }).check()
  await page.getByLabel("Every N days after it's done").check()
  await page.locator('input[name=interval_days]').fill('2')
  await page.getByRole('button', { name: 'Save' }).click()
  await expect(page.getByTestId(`def-${title}`)).toBeVisible()

  runNow() // materialise now instead of waiting for the minute job
  await page.goto('/tasks')
  await page.getByRole('tab', { name: 'Lists' }).click()
  const box = page.getByRole('checkbox', { name: `Complete ${title}` })
  await expect(box).toBeVisible()
  await expect(box).not.toBeChecked()

  await box.check()
  const toast = page.getByRole('status').filter({ hasText: /done|completed/i })
  await expect(toast).toBeVisible()
  await expect(box).toHaveCount(0) // done tasks leave the open list
  await toast.getByRole('button', { name: 'Undo' }).click()
  await expect(box).toBeVisible()
  await expect(box).not.toBeChecked()
})

test('the dashboard Now panel completes a task through the member picker', async ({
  page,
  browser,
}) => {
  await uiLogin(page, adult)
  const title = `Water the herbs ${Date.now()}`
  const lists = await (await page.request.get('/api/v1/task-lists')).json()
  const chores = lists.find((l: { name: string }) => l.name === 'Chores')
  const made = await page.request.post('/api/v1/tasks', {
    // High and long overdue (the score caps at 2), so it tops the Now list whatever else is open.
    data: {
      list: chores.id,
      title,
      assignees: [],
      steps: [],
      priority_flag: 'high',
      due_at: new Date(Date.now() - 6 * 86_400_000).toISOString(),
    },
    headers: { 'X-FA-Request': '1' },
  })
  expect(made.status()).toBe(201)

  await setOverride(page, true)
  const { screen, ctx } = await openKiosk(page, browser)
  try {
    const now = screen.getByRole('region', { name: 'Now' })
    const box = now.getByRole('checkbox', { name: `Complete ${title}` })
    await expect(box).toBeVisible()
    await box.check()
    const picker = screen.getByRole('dialog', { name: 'Who did it?' })
    await expect(picker).toBeVisible()
    await picker.getByRole('button', { name: 'Riley' }).click()
    await expect(picker).toBeHidden()
    await expect(now.getByRole('checkbox', { name: `Complete ${title}` })).toHaveCount(0)
    await screen.getByRole('button', { name: 'Undo' }).click()
    await expect(now.getByRole('checkbox', { name: `Complete ${title}` })).toBeVisible()
  } finally {
    await ctx.close()
    await setOverride(page, false)
  }
})

test('the routine panel fills its progress bar to 100%', async ({ page, browser }) => {
  await uiLogin(page, adult)
  await setOverride(page, true)
  const { screen, ctx } = await openKiosk(page, browser)
  try {
    const routine = screen.getByRole('region', { name: 'Routine' })
    await expect(routine).toContainText('Morning routine')
    const bar = routine.getByRole('progressbar').first()
    const steps = ['Brush teeth', 'Get dressed', 'Pack school bag']
    // Start from an empty run (the API specs may have ticked it), then tick every step.
    for (const step of steps) {
      const btn = routine.getByRole('button', { name: step })
      if ((await btn.getAttribute('aria-pressed')) === 'true') await btn.click()
      await expect(btn).toHaveAttribute('aria-pressed', 'false')
    }
    await expect(bar).toHaveAttribute('aria-valuenow', '0')
    for (const step of steps) {
      const btn = routine.getByRole('button', { name: step })
      await btn.click()
      await expect(btn).toHaveAttribute('aria-pressed', 'true')
    }
    await expect(bar).toHaveAttribute('aria-valuetext', '3 of 3') // 100%
    await expect(bar).toHaveAttribute('aria-valuenow', '3')
    await expect(bar).toHaveAttribute('aria-valuemax', '3')
    await expect(routine.getByRole('status')).toBeVisible()
  } finally {
    await ctx.close()
    await setOverride(page, false)
  }
})

test('an unpinned routine panel shows the run inside its window', async ({ page, browser }) => {
  await uiLogin(page, adult)
  const { screen, ctx } = await openKiosk(page, browser, undefined, { kind: 'routine' })
  try {
    await expect(screen.getByRole('region', { name: 'Routine' })).toContainText('Morning routine')
  } finally {
    await ctx.close()
  }
})

type Rect = { x: number; y: number; width: number; height: number }
const hit = (a: Rect, b: Rect) =>
  a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height

/** Panels must not sit on top of each other and nothing may overflow the screen sideways. */
async function checkKiosk(
  page: Page,
  browser: Browser,
  name: string,
  size: { width: number; height: number },
  preset: 'hub' | 'board',
) {
  const { screen, ctx } = await openKiosk(page, browser, size, undefined, preset)
  try {
    const regions =
      preset === 'hub'
        ? {
            Now: screen.getByRole('region', { name: 'Now' }),
            Routine: screen.getByRole('region', { name: 'Routine' }),
          }
        : { Columns: screen.locator('[data-member]').first() }
    for (const r of Object.values(regions)) await expect(r).toBeVisible()
    const overflow = await screen.evaluate(
      () => document.documentElement.scrollWidth - window.innerWidth,
    )
    await screen.screenshot({ path: `test-results/kiosk-${preset}-${name}.png` })
    expect(overflow, `${name} horizontal overflow`).toBeLessThanOrEqual(1)
    const boxes = await Promise.all(Object.values(regions).map((r) => r.boundingBox()))
    const rects = boxes.filter((b): b is Rect => !!b)
    const clash = rects.flatMap((a, i) => rects.slice(i + 1).filter((b) => hit(a, b)))
    expect(clash, `${name}: overlapping panels`).toHaveLength(0)
    if (preset === 'board') {
      // Every member column must fit on the screen.
      const cols = await screen
        .locator('[data-member]')
        .evaluateAll((els) => els.map((e) => e.getBoundingClientRect().right - window.innerWidth))
      expect(Math.max(...cols), `${name}: a member column runs off the screen`).toBeLessThanOrEqual(
        1,
      )
    }
  } finally {
    await ctx.close()
  }
}

async function fillNow(page: Page) {
  const lists = await (await page.request.get('/api/v1/task-lists')).json()
  const chores = lists.find((l: { name: string }) => l.name === 'Chores')
  for (let i = 0; i < 8; i++) {
    const made = await page.request.post('/api/v1/tasks', {
      data: {
        list: chores.id,
        title: `Filler ${i} ${Date.now()}`,
        assignees: [],
        steps: [],
        priority_flag: 'high',
        due_at: new Date().toISOString(),
      },
      headers: { 'X-FA-Request': '1' },
    })
    expect(made.status()).toBe(201)
  }
}

test('Family hub lays out Now and the routine at 1920x1080', async ({ page, browser }) => {
  await uiLogin(page, adult)
  await fillNow(page)
  await checkKiosk(page, browser, 'landscape', { width: 1920, height: 1080 }, 'hub')
})

test('Family hub lays out Now and the routine in portrait', async ({ page, browser }) => {
  await uiLogin(page, adult)
  await checkKiosk(page, browser, 'portrait', { width: 1080, height: 1920 }, 'hub')
})

test('Chores board fits its member columns at 1920x1080', async ({ page, browser }) => {
  await uiLogin(page, adult)
  await checkKiosk(page, browser, 'landscape', { width: 1920, height: 1080 }, 'board')
})

test('the dashboard + button adds a task through the member picker', async ({ page, browser }) => {
  await uiLogin(page, adult)
  const title = `Buy milk ${Date.now()}`
  const { screen, ctx } = await openKiosk(page, browser)
  try {
    await screen.getByRole('button', { name: 'Add a task' }).click()
    await screen.getByTestId('add-title').fill(title)
    await screen.getByTestId('add-list').selectOption({ label: 'Errands' })
    await screen.getByTestId('add-submit').click()
    const picker = screen.getByRole('dialog', { name: 'Who is adding it?' })
    await expect(picker).toBeVisible()
    await picker.getByRole('button', { name: 'Riley' }).click()
    await expect(screen.getByTestId('add-form')).toBeHidden()
    const tasks = await (await page.request.get('/api/v1/tasks?view=all&status=open')).json()
    const made = tasks.find((t: { title: string }) => t.title === title)
    expect(made, 'the new task').toBeTruthy()
    expect(made.list_name).toBe('Errands')
  } finally {
    await ctx.close()
  }
})

test('Settings, Absences: add and remove one', async ({ page }) => {
  await uiLogin(page, adult)
  const note = `Camp ${Date.now()}`
  await page.goto('/settings/tasks?tab=absences')
  await page.locator('select[name=member]').selectOption({ label: 'Jordan' })
  const day = localDay()
  await page.locator('input[name=from]').fill(day)
  await page.locator('input[name=until]').fill(day)
  await page.locator('input[name=note]').fill(note)
  await page.getByRole('button', { name: 'Add absence' }).click()
  const row = page.getByRole('listitem').filter({ hasText: note })
  await expect(row).toBeVisible()
  await row.getByRole('button', { name: 'Remove' }).click()
  await expect(page.getByText(note)).toHaveCount(0)
})

test('a suggested weather task is accepted from the Suggested tab', async ({ page }) => {
  await uiLogin(page, adult)
  const title = `Take in the washing ${Date.now()}`
  const api = { headers: { 'X-FA-Request': '1' } }
  const lists = await (await page.request.get('/api/v1/task-lists')).json()
  const chores = lists.find((l: { name: string }) => l.name === 'Chores')
  const members = await (await page.request.get('/api/v1/members')).json()
  const alex = (Array.isArray(members) ? members : members.items).find(
    (m: { display_name: string }) => m.display_name === 'Alex',
  )
  const def = await page.request.post('/api/v1/task-definitions', {
    ...api,
    data: {
      list: chores.id,
      title,
      schedule: { kind: 'after_done', interval_days: 7 },
      start_date: localDay(),
      strategy: 'fixed',
      candidates: [alex.id],
      paused: true, // only the weather rule below may make tasks from it
    },
  })
  expect(def.status()).toBe(201)
  const rule = await page.request.post('/api/v1/weather-rules', {
    ...api,
    data: {
      name: `E2E suggest ${Date.now()}`,
      condition: 'min(temp_c, today) < 5',
      action: 'task',
      severity: 'info',
      message: 'Cold today',
      task_mode: 'suggest',
      task_definition: { definition: (await def.json()).id },
    },
  })
  expect(rule.status()).toBe(201)

  await expect(async () => {
    const found = await (await page.request.get('/api/v1/tasks?view=all&status=suggested')).json()
    expect(found.some((t: { title: string }) => t.title === title)).toBe(true)
  }).toPass({ timeout: 60_000 })

  await page.goto('/tasks')
  await page.getByRole('tab', { name: 'Suggested' }).click()
  const row = page.getByRole('listitem').filter({ hasText: title })
  await expect(row).toBeVisible()
  await row.getByRole('button', { name: 'Accept' }).click()
  await expect(row).toHaveCount(0)
  const open = await (await page.request.get('/api/v1/tasks?view=all&status=open')).json()
  expect(open.some((t: { title: string }) => t.title === title)).toBe(true)
})

test('Settings, Notifications: Send test succeeds', async ({ page }) => {
  await uiLogin(page, adult)
  await page.goto('/settings/notifications')
  await expect(page.getByTestId('not-configured')).toHaveCount(0)
  await expect(page.getByTestId('subscribe-url')).toContainText('/')
  await page.getByRole('button', { name: 'Send test' }).click()
  const result = page.getByTestId('test-result')
  await expect(result).toBeVisible()
  await expect(result).toHaveClass(/text-success/)
})

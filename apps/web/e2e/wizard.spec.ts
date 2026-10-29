import { expect, test, type Page } from '@playwright/test'

// The first-run wizard end to end, on the EMPTY instance that scripts/e2e.sh starts (E2E_WIZARD_URL).
// scripts/e2e.sh serves it on all interfaces and passes the host's LAN address, so the browser sees
// plain HTTP from a non-localhost origin: not a secure context (no service worker, no
// crypto.subtle, no clipboard), which is the path a real home install takes (D-072).
// Env: E2E_NTFY_URL, E2E_WIZARD_URL (default http://127.0.0.1:18095, then a secure context and the check is skipped).
const WIZARD = process.env.E2E_WIZARD_URL ?? 'http://127.0.0.1:18095'
const NTFY = process.env.E2E_NTFY_URL ?? 'http://127.0.0.1:18093'
const OWNER = {
  name: 'Pat',
  user: 'pat@example.com',
  pass: 'first-run-passphrase-1',
  pin: '246810',
}

test.describe.configure({ mode: 'serial' })

let page: Page

test.beforeAll(async ({ browser }) => {
  const ctx = await browser.newContext({ baseURL: WIZARD })
  page = await ctx.newPage()
})
test.afterAll(async () => {
  await page.context().close()
})

const step = (name: string) => page.getByTestId(`step-${name}`)
// Saving the household's address changes a runtime setting, so the server restarts its listeners
// about a second later; a request in that window is refused. Retry until it is back.
const reloadWhenUp = async () => {
  for (let i = 0; ; i++) {
    try {
      await page.reload()
      return
    } catch (e) {
      if (i >= 15) throw e
      await page.waitForTimeout(1000)
    }
  }
}
// Wait out the restart (it starts about a second after the write), then until /readyz answers.
const settle = async () => {
  await page.waitForTimeout(2500)
  for (let i = 0; i < 30; i++) {
    try {
      if ((await page.request.get('/readyz')).ok()) return
    } catch {
      /* restarting */
    }
    await page.waitForTimeout(500)
  }
  throw new Error('the server did not come back')
}
const next = async () => page.getByRole('button', { name: 'Next' }).click()

test('every route goes to the wizard; the app loads over plain HTTP without a secure context', async () => {
  await page.goto('/')
  await expect(page).toHaveURL(/\/setup$/)
  await expect(page.getByRole('heading', { name: 'Set up Family Assistant' })).toBeVisible()
  await expect(page.getByTestId('welcome-facts')).toContainText('plain HTTP')

  const secure = await page.evaluate(() => window.isSecureContext)
  if (!new URL(WIZARD).hostname.match(/^(127\.|localhost)/)) {
    expect(secure).toBe(false)
    // No service worker can register here, and the app still works.
    expect(await page.evaluate(() => 'serviceWorker' in navigator)).toBe(false)
  }
  await page.goto('/login')
  await expect(page).toHaveURL(/\/setup$/)
  await page.goto('/members')
  await expect(page).toHaveURL(/\/setup$/)
  await page.getByRole('button', { name: 'Start' }).click()
})

test('household: validated, then saved', async () => {
  await expect(step('household')).toBeVisible()
  await page.getByLabel('Household name').fill('Example household')
  await page.getByLabel('Time zone').fill('America/New_York')
  await page.getByLabel('Label').fill('Springfield (fictional)')
  await page.getByLabel('Latitude').fill('north')
  await page.getByLabel('Longitude').fill('-75')
  await next()
  await expect(page.getByRole('alert').first()).toBeVisible() // not a number
  await page.getByLabel('Latitude').fill('41.5')
  await page.getByLabel('Units').selectOption('imperial')
  await page.getByLabel('Currency (ISO code)').fill('USD')
  await next()
  await expect(step('owner')).toBeVisible()
  await settle()
})

test('you: the first adult, who is signed in', async () => {
  await page.getByLabel('Your name').fill(OWNER.name)
  await page.getByLabel('Username').fill(OWNER.user)
  await page.getByLabel('Password (12 characters or more)').fill('short')
  await page.getByLabel('Password again').fill('short')
  await next()
  await expect(page.getByRole('alert').first()).toBeVisible() // too short
  await page.getByLabel('Password (12 characters or more)').fill(OWNER.pass)
  await page.getByLabel('Password again').fill(OWNER.pass)
  await page.locator('input[name=pin]').fill(OWNER.pin)
  await next()
  await expect(step('family')).toBeVisible()
  const me = await page.request.get('/api/v1/me')
  expect(me.ok()).toBeTruthy()
})

test('family: add a child with a login; leaving and coming back resumes the step', async () => {
  await expect(page.getByTestId('member-Pat')).toBeVisible()
  const form = page.getByTestId('add-member')
  await form.getByLabel('Name', { exact: true }).fill('Robin')
  await form.getByLabel('Role').selectOption('child')
  await form.getByLabel('Username').fill('robin@example.com')
  await form.getByLabel('Password', { exact: true }).fill('dino1234')
  await form.getByRole('button', { name: 'Add' }).click()
  await expect(page.getByTestId('member-Robin')).toBeVisible()

  // The step is remembered (localStorage fa.setup.step) and the server decides how far back to go.
  await reloadWhenUp()
  await expect(page).toHaveURL(/\/setup$/)
  await expect(step('family')).toBeVisible()
  await expect(page.getByTestId('member-Robin')).toBeVisible()
  await page.getByRole('button', { name: 'Next' }).click()
  await expect(step('access')).toBeVisible()
})

test('access: the detected address is stored; a notifications server can be set', async () => {
  await expect(page.getByLabel('Public address')).toHaveValue(new RegExp(`^${WIZARD}/?$`))
  await page.getByLabel('Notifications server (ntfy)').fill(NTFY)
  await next()
  await expect(step('key')).toBeVisible()
  await settle()
})

test('master key: shown only with the password, and the backup must be confirmed', async () => {
  await expect(page.getByRole('button', { name: 'Next' })).toBeDisabled()
  await page.getByLabel('Your login password').fill('not-my-password')
  await page.getByRole('button', { name: 'Show the key' }).click()
  await expect(page.getByRole('alert')).toBeVisible()
  await page.getByLabel('Your login password').fill(OWNER.pass)
  await page.getByRole('button', { name: 'Show the key' }).click()
  const key = page.getByTestId('master-key')
  await expect(key).toBeVisible()
  expect(Buffer.from(await key.inputValue(), 'base64')).toHaveLength(32)
  // Copy works or says why not; it never throws in an insecure context.
  await page.getByRole('button', { name: 'Copy' }).click()
  await expect(
    page.getByRole('status').filter({ hasText: /Copied|Copying is not available/ }),
  ).toBeVisible()
  await expect(page.getByRole('button', { name: 'Confirm backup' })).toBeDisabled()
  await page.getByLabel('I stored it somewhere safe').check()
  await page.getByRole('button', { name: 'Confirm backup' }).click()
  await expect(page.getByText('The backup is confirmed.')).toBeVisible()
  await next()
  await expect(step('start')).toBeVisible()
})

test('start: finishing leaves a logged-in adult on the app, and the wizard is closed', async () => {
  await page.getByRole('button', { name: 'Finish' }).click()
  await expect(page).toHaveURL(/\/$/)
  await expect(page.getByRole('navigation', { name: 'Primary' })).toBeVisible()
  await page.goto('/setup')
  await expect(page).not.toHaveURL(/\/setup$/)

  const status = await (await page.request.get('/api/v1/setup')).json()
  expect(status).toMatchObject({ needed: false, owner_needed: false, completed: true })

  // The key is confirmed, so System no longer warns; the connection is stated as a fact.
  await page.goto('/settings/system')
  await expect(page.getByTestId('backup-done')).toBeVisible()
  await expect(page.getByTestId('backup-warning')).toHaveCount(0)
  await expect(page.getByTestId('connection-text')).toContainText(/HTTP/)
  await expect(page.getByTestId('setting-ntfy_url').locator('input')).toHaveValue(NTFY)
})

test('the new household has members, the child can sign in, and no household file was needed', async ({
  browser,
}) => {
  const members = await (await page.request.get('/api/v1/members')).json()
  const list = Array.isArray(members) ? members : (members.items ?? members.members)
  expect(list.map((m: { display_name: string }) => m.display_name).sort()).toEqual(['Pat', 'Robin'])
  const kid = await browser.newContext({ baseURL: WIZARD })
  const login = await kid.request.post('/api/v1/auth/login', {
    data: { username: 'robin@example.com', password: 'dino1234' },
    headers: { 'X-FA-Request': '1' },
  })
  expect(login.ok()).toBeTruthy()
  await kid.close()
})

test('Settings, System: save a setting, wait for the restart, and the value stays', async () => {
  const url = 'https://family.example.ts.net'
  await page.goto('/settings/system')
  // The environment of the e2e server sets FA_ALLOW_HTTP_FEEDS, so that one is locked (D-075).
  await expect(page.getByTestId('setting-allow_http_feeds').locator('input')).toBeDisabled()
  const field = page.getByTestId('setting-tailnet_base_url').locator('input')
  await field.fill(url)
  await page.getByRole('button', { name: 'Save' }).click()
  await expect(page.getByText('Saved.')).toBeVisible({ timeout: 30_000 })
  await page.reload()
  await expect(page.getByTestId('setting-tailnet_base_url').locator('input')).toHaveValue(url)
  // A bad value is named, not swallowed.
  await page
    .getByTestId('setting-tailnet_base_url')
    .locator('input')
    .fill('ftp://family.example.com')
  await page.getByRole('button', { name: 'Save' }).click()
  await expect(page.getByRole('alert').first()).toBeVisible({ timeout: 30_000 })
})

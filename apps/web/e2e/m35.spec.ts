import { expect, test, type BrowserContext, type Page } from '@playwright/test'

// UI specs for M3.5 on the seeded stack from scripts/e2e.sh: Settings, System, the members move
// (D-070) and who may reach what. The wizard has its own file (wizard.spec.ts).
// Env: E2E_BASE_URL, E2E_USERNAME/E2E_PASSWORD (adult), E2E_CHILD_USERNAME/E2E_CHILD_PASSWORD.
const adult = {
  user: process.env.E2E_USERNAME ?? 'alex@example.com',
  pass: process.env.E2E_PASSWORD ?? '',
}
const child = {
  user: process.env.E2E_CHILD_USERNAME ?? 'riley@example.com',
  pass: process.env.E2E_CHILD_PASSWORD ?? '',
}

async function uiLogin(page: Page, who: { user: string; pass: string }) {
  await page.goto('/login')
  await page.getByLabel('Username').fill(who.user)
  await page.getByLabel('Password').fill(who.pass)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/$/)
}

test.describe.configure({ mode: 'serial' })

// One login per person for the whole file: logins are rate-limited per address and the other UI
// specs log in as well.
let adultCtx: BrowserContext
let childCtx: BrowserContext
let adultPage: Page
let childPage: Page
test.beforeAll(async ({ browser }) => {
  adultCtx = await browser.newContext({ extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.4' } })
  adultPage = await adultCtx.newPage()
  await uiLogin(adultPage, adult)
  childCtx = await browser.newContext({ extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.4' } })
  childPage = await childCtx.newPage()
  await uiLogin(childPage, child)
})
test.afterAll(async () => {
  await adultCtx.close()
  await childCtx.close()
})

test('Settings index: groups, and System and Members are there for an adult', async () => {
  const page = adultPage
  await page.goto('/')
  await page.getByRole('link', { name: 'Settings' }).click()
  await expect(page).toHaveURL(/\/settings$/)
  await expect(page.getByRole('heading', { name: 'Settings', level: 1 })).toBeVisible()
  const nav = page.getByRole('navigation', { name: 'Settings' })
  for (const name of ['Account', 'Household', 'Members', 'System', 'Devices']) {
    await expect(nav.getByRole('link', { name })).toBeVisible()
  }
  await expect(
    page.getByRole('navigation', { name: 'Primary' }).getByRole('link', { name: 'Members' }),
  ).toHaveCount(0)
})

test('System: the connection is stated as a fact; a server-set value is locked; the household file previews', async () => {
  const page = adultPage
  await page.goto('/settings/system')
  await expect(page.getByRole('heading', { name: 'System', level: 1 })).toBeVisible()
  await expect(page.getByTestId('connection-text')).toContainText(/HTTP/)
  // FA_PUBLIC_BASE_URL is set in the environment of the e2e server, so the UI cannot change it.
  const pub = page.getByTestId('setting-public_base_url')
  await expect(pub.locator('input')).toBeDisabled()
  await expect(pub).toContainText(/Set by the server/)
  // A key that nobody set is not locked.
  await expect(page.getByTestId('setting-tailnet_base_url').locator('input')).toBeEnabled()

  const [download] = await Promise.all([
    page.waitForEvent('download'),
    page.getByRole('button', { name: 'Export household file' }).click(),
  ])
  expect(download.suggestedFilename()).toBe('household.toml')

  // A dry-run preview of the example file: nothing is written until Apply, which this test never presses.
  await page
    .locator('input[name=household_file]')
    .setInputFiles('../../config/examples/household.example.toml')
  const preview = page.getByTestId('import-preview')
  await expect(preview).toContainText('Preview')
  await expect(preview.getByRole('button', { name: 'Apply' })).toBeVisible()
})

test('Settings, Household: the per-adult dashboard count is a setting', async () => {
  const page = adultPage
  await page.goto('/settings/household')
  await expect(page.locator('input[name=dashboard_pending_approvals]')).toBeChecked()
})

test('Settings, Account: the current password is required to change your own', async () => {
  const page = adultPage
  await page.goto('/settings/account')
  await page.locator('input[name=current_password]').fill('not-my-password-at-all')
  await page.locator('input[name=new_password]').fill('a-brand-new-passphrase-1')
  await page.locator('input[name=new_password_again]').fill('a-brand-new-passphrase-1')
  await page.getByRole('button', { name: 'Change password' }).click()
  await expect(page.getByRole('alert')).toBeVisible()
  // Still signed in with the old password.
  expect((await page.request.get('/api/v1/me')).ok()).toBeTruthy()
})

test('a child cannot reach Settings, Members; /members is gone', async () => {
  const page = childPage
  await page.goto('/settings/members')
  await expect(page).not.toHaveURL(/\/settings\/members/)
  expect((await page.request.get('/api/v1/system/settings')).status()).toBe(403)
  await page.goto('/settings')
  await expect(page.getByRole('link', { name: 'Members' })).toHaveCount(0)
  await expect(page.getByRole('link', { name: 'System' })).toHaveCount(0)
  await page.goto('/members')
  await expect(page.locator('ul.members')).toHaveCount(0)
  await expect(page.getByTestId('add-member')).toHaveCount(0)
  // The adult's route table has no /members either: the old page does not come back.
  await adultPage.goto('/members')
  await expect(adultPage.locator('ul.members')).toHaveCount(0)
  await expect(adultPage.getByTestId('add-member')).toHaveCount(0)
})

import { expect, test } from '@playwright/test'

// Logins are limited per client address. The server trusts X-Forwarded-For from loopback, so each
// file logs in from its own made-up address and the suite does not share one budget.
test.use({ extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.5' } })

// Env: E2E_BASE_URL (default http://localhost:18080), E2E_USERNAME, E2E_PASSWORD (an adult account).
const username = process.env.E2E_USERNAME ?? 'alex@example.com'
const password = process.env.E2E_PASSWORD ?? ''

test('login, members, pair a device, dashboard shows the heartbeat', async ({ page, browser }) => {
  await page.goto('/login')
  await page.getByLabel('Username').fill(username)
  await page.getByLabel('Password').fill(password)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/$/)

  // Members left the top navigation (D-070): they live under Settings, which is adult-only.
  await expect(page.getByRole('navigation').getByRole('link', { name: 'Members' })).toHaveCount(0)
  await page.getByRole('link', { name: 'Settings' }).click()
  await expect(page).toHaveURL(/\/settings$/)
  const settingsNav = page.getByRole('navigation', { name: 'Settings' })
  await settingsNav.getByRole('link', { name: 'Members' }).click()
  await expect(page).toHaveURL(/\/settings\/members$/)
  await expect(page.locator('ul.members li').first()).toBeVisible()

  await settingsNav.getByRole('link', { name: 'Devices' }).click()
  await page.getByLabel('Device name').fill(`e2e-${Date.now()}`)
  await page.getByRole('button', { name: 'Pair a device' }).click()
  await expect(page.getByAltText('QR code for the pair URL')).toBeVisible()
  const pairUrl = (await page.getByTestId('pair-url').textContent())?.trim()
  expect(pairUrl).toContain('/dashboard?device=')

  // A separate browser context stands in for the kiosk screen (no user session).
  const kiosk = await browser.newContext()
  const screen = await kiosk.newPage()
  await screen.goto(pairUrl!)
  await expect(screen.getByTestId('household')).not.toBeEmpty()
  await expect(screen.getByTestId('connection')).toHaveAttribute('data-status', 'open')
  await expect(screen.getByTestId('heartbeat')).toBeVisible()
  await kiosk.close()
})

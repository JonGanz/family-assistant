import { expect, test, type Page } from '@playwright/test'

// UI spec for M3.6 (themes): the choice is per device (localStorage `fa.theme`), set in
// Settings, Appearance, and survives a reload. Env as in m35.spec.ts.
const adult = {
  user: process.env.E2E_USERNAME ?? 'alex@example.com',
  pass: process.env.E2E_PASSWORD ?? '',
}

async function uiLogin(page: Page) {
  await page.goto('/login')
  await page.getByLabel('Username').fill(adult.user)
  await page.getByLabel('Password').fill(adult.pass)
  await page.getByRole('button', { name: 'Log in' }).click()
  await expect(page).toHaveURL(/\/$/)
}

test('Appearance: a picked theme sticks across a reload and data-theme-mode matches', async ({
  browser,
}) => {
  const ctx = await browser.newContext({ extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.5' } })
  const page = await ctx.newPage()
  await uiLogin(page)
  await page.goto('/settings/appearance')

  // Nord ships a dark variant only, so the mode is dark whatever the device prefers.
  await page.locator('[data-family="nord"] button.pick').click()
  const html = page.locator('html')
  await expect(html).toHaveAttribute('data-theme-mode', 'dark')
  const bg = await html.evaluate((el) => el.style.getPropertyValue('--color-bg'))
  expect(bg).toMatch(/^#[0-9a-f]{6}$/i)
  const saved = await page.evaluate(() => localStorage.getItem('fa.theme'))
  expect(JSON.parse(saved ?? '{}').family).toBe('nord')

  await page.reload()
  await expect(page.locator('[data-family="nord"]')).toHaveClass(/current/)
  await expect(html).toHaveAttribute('data-theme-mode', 'dark')
  expect(await html.evaluate((el) => el.style.getPropertyValue('--color-bg'))).toBe(bg)

  // A family with both modes follows the explicit mode choice.
  await page.locator('[data-family="solarized"] button.pick').click()
  await page.getByTestId('mode').selectOption('light')
  await expect(html).toHaveAttribute('data-theme-mode', 'light')
  await page.reload()
  await expect(page.getByTestId('mode')).toHaveValue('light')
  await expect(html).toHaveAttribute('data-theme-mode', 'light')
  await ctx.close()
})

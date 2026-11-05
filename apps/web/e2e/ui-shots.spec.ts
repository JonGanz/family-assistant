import { mkdirSync } from 'node:fs'
import { expect, test } from '@playwright/test'

// 2x pixel density so the review images stay sharp on high-DPI screens.
test.use({ deviceScaleFactor: 2 })

// Review screenshots (M4). Run through scripts/ui-shots.sh, never in the default e2e run.
// Output: apps/web/test-results/ui-shots/<page>-<width>-<light|dark>.png

/** App routes captured with `--all` (UI_SHOTS_ALL=1). Edit this list when views are added or split. */
const APP_ROUTES = [
  '/',
  '/calendar',
  '/tasks',
  '/routines',
  '/mail',
  '/inbox',
  '/bills',
  '/settings',
  '/settings/account',
  '/settings/appearance',
  '/settings/notifications',
  '/settings/household',
  '/settings/members',
  '/settings/tasks',
  '/settings/calendars',
  '/settings/weather',
  '/settings/mail',
  '/settings/orgs',
  '/settings/system',
  '/settings/devices',
  '/settings/about',
  '/settings/status',
]

const WIDTHS = [360, 800, 1280]
const MODES = ['light', 'dark'] as const
const OUT = 'test-results/ui-shots'
const all = !!process.env.UI_SHOTS_ALL

// Same arrangement as the other UI specs: a made-up client address so logins have their own budget.
test.use({ extraHTTPHeaders: { 'X-Forwarded-For': '10.99.0.7' }, reducedMotion: 'reduce' })

const slug = (route: string) => (route === '/' ? 'home' : route.slice(1).replace(/\//g, '-'))

test.beforeAll(() => mkdirSync(OUT, { recursive: true }))

for (const mode of MODES) {
  for (const width of WIDTHS) {
    test(`@shots gallery ${width} ${mode}`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 })
      await page.goto(`/ui-gallery?theme=catppuccin&mode=${mode}`)
      await expect(page.getByRole('heading', { name: 'UI gallery' })).toBeVisible()
      await expect(page.locator('html')).toHaveAttribute('data-theme-mode', mode)
      await page.screenshot({ path: `${OUT}/ui-gallery-${width}-${mode}.png`, fullPage: true })
    })
  }
}

test.describe('app routes', () => {
  test.skip(!all, 'set UI_SHOTS_ALL=1 (scripts/ui-shots.sh --all)')
  for (const mode of MODES) {
    for (const width of WIDTHS) {
      test(`@shots app ${width} ${mode}`, async ({ page }) => {
        test.setTimeout(180_000)
        await page.addInitScript(
          (m) =>
            localStorage.setItem('fa.theme', JSON.stringify({ family: 'catppuccin', mode: m })),
          mode,
        )
        await page.setViewportSize({ width, height: 900 })
        await page.goto('/login')
        await page.getByLabel('Username').fill(process.env.E2E_USERNAME ?? 'alex@example.com')
        await page.getByLabel('Password').fill(process.env.E2E_PASSWORD ?? '')
        await page.getByRole('button', { name: 'Log in' }).click()
        await expect(page).toHaveURL(/\/$/)
        for (const route of APP_ROUTES) {
          await page.goto(route)
          await page.waitForLoadState('networkidle')
          await page.screenshot({
            path: `${OUT}/${slug(route)}-${width}-${mode}.png`,
            fullPage: true,
          })
        }
      })
    }
  }
})

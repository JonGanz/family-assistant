import { mkdirSync } from 'node:fs'
import { expect, test } from '@playwright/test'
import mail from './ui-shots.routes/mail'
import settings1 from './ui-shots.routes/settings1'
import settings2 from './ui-shots.routes/settings2'
import tasks from './ui-shots.routes/tasks'
import today from './ui-shots.routes/today'
import type { ShotRoute } from './ui-shots.routes/types'

/** The shell itself: the phone nav drawer and the collapsed desktop drawer (main session). */
const shell: ShotRoute[] = [
  {
    route: '/',
    name: 'shell-nav-drawer',
    widths: [360],
    before: async (page) => {
      await page.getByRole('button', { name: 'Open navigation' }).click()
      await page.waitForTimeout(300)
    },
  },
  {
    route: '/',
    name: 'shell-rail-collapsed',
    widths: [1280],
    before: async (page) => {
      await page.getByRole('button', { name: 'Collapse navigation' }).click()
      await page.waitForTimeout(300)
    },
  },
]

// 2x pixel density so the review images stay sharp on high-DPI screens.
test.use({ deviceScaleFactor: 2 })

// Review screenshots (M4). Run through scripts/ui-shots.sh, never in the default e2e run.
// Output: apps/web/test-results/ui-shots/<page>-<width>-<light|dark>.png

/** App routes captured with `--all` (UI_SHOTS_ALL=1), one file per area so wave-3 tasks own their own list.
 *  UI_SHOTS_AREAS=tasks,mail narrows the run. */
const AREAS: Record<string, ShotRoute[]> = { today, tasks, mail, settings1, settings2, shell }
const areaFilter = (process.env.UI_SHOTS_AREAS ?? '').split(',').filter(Boolean)
const APP_ROUTES: ShotRoute[] = Object.entries(AREAS)
  .filter(([a]) => areaFilter.length === 0 || areaFilter.includes(a))
  .flatMap(([, rs]) => rs)

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
      // The gallery is a dev-only route; the --all run serves the production build.
      test.skip(all, 'gallery shots run without --all (dev server only)')
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
        for (const shot of APP_ROUTES) {
          if (shot.widths && !shot.widths.includes(width)) continue
          await page.goto(shot.route)
          // Not 'networkidle': the live update stream keeps a request open forever.
          await page.waitForLoadState('load')
          await page
            .locator('.ui-skeleton')
            .first()
            .waitFor({ state: 'detached', timeout: 10_000 })
            .catch(() => {})
          await page.waitForTimeout(400)
          if (shot.before) await shot.before(page)
          await page.screenshot({
            path: `${OUT}/${shot.name ?? slug(shot.route)}-${width}-${mode}.png`,
            fullPage: true,
          })
        }
      })
    }
  }
})

import type { ShotRoute } from './types'

/** Review screenshots for the mail area (owned by M4 T-12). Add detail routes, tabs or sheets here. */
export default [
  { route: '/mail' },
  {
    route: '/mail',
    name: 'mail-message',
    before: async (page) => {
      await page.locator('a[href^="/mail/"]:not([href="/mail/label"])').first().click()
      await page.waitForURL(/\/mail\/[0-9a-f-]+/)
      await page.waitForTimeout(600)
    },
  },
  { route: '/mail/label' },
  { route: '/inbox' },
  { route: '/bills' },
] satisfies ShotRoute[]

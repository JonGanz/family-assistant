import type { ShotRoute } from './types'

/** Review screenshots for the tasks area (owned by M4 T-11). Add detail routes, tabs or sheets here. */
export default [
  { route: '/tasks' },
  {
    route: '/tasks',
    name: 'tasks-detail',
    before: async (page) => {
      await page.locator('button.body').first().click()
      await page.waitForURL(/\/tasks\/[0-9a-f-]+/)
      await page.waitForTimeout(400)
    },
  },
  {
    route: '/tasks',
    name: 'tasks-new-sheet',
    before: async (page) => {
      await page.getByRole('button', { name: 'New task' }).click()
      await page.waitForTimeout(400)
    },
  },
] satisfies ShotRoute[]

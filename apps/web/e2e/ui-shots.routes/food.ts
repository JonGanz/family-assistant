import type { ShotRoute } from './types'

/** Review screenshots for Food (owned by M5 T-12). The recipe page opens the first card of the library
 *  (the shot run's household needs at least one recipe). The kiosk Food view needs a paired device, which
 *  the app-route run has no session for: it is covered by m5.spec.ts instead. */
export default [
  { route: '/food/plan' },
  { route: '/food/list' },
  { route: '/food/recipes' },
  {
    route: '/food/recipes',
    name: 'food-recipe',
    before: async (page) => {
      const card = page.getByTestId(/^recipe-card-/).first()
      if ((await card.count()) === 0) return
      await card.click()
      await page.waitForURL(/\/food\/recipes\/[0-9a-f-]{36}/)
      await page.waitForTimeout(400)
    },
  },
  { route: '/food/templates' },
  { route: '/settings/meals' },
] satisfies ShotRoute[]

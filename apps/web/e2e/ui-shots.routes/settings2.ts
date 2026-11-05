import type { ShotRoute } from './types'

/** Review screenshots for the settings2 area (owned by M4 T-14). Add detail routes, tabs or sheets here. */
export default [
  { route: '/settings/notifications' },
  { route: '/settings/tasks' },
  { route: '/settings/calendars' },
  {
    route: '/settings/calendars',
    name: 'settings-calendars-add-source',
    before: async (page) => {
      await page.getByRole('button', { name: 'Add a source' }).last().click()
      await page.waitForTimeout(400)
    },
  },
  { route: '/settings/weather' },
  { route: '/settings/mail' },
  { route: '/settings/orgs' },
  { route: '/settings/system' },
  { route: '/settings/status' },
] satisfies ShotRoute[]

import type { ShotRoute } from './types'

/** Review screenshots for the settings1 area (owned by M4 T-13). Add detail routes, tabs or sheets here. */
export default [
  { route: '/settings' },
  { route: '/settings/account' },
  { route: '/settings/appearance' },
  { route: '/settings/household' },
  { route: '/settings/members' },
  { route: '/settings/devices' },
  { route: '/settings/about' },
] satisfies ShotRoute[]

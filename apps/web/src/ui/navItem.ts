import { useRoute } from 'vue-router'
import { isActivePath } from './nav'

/** The path the nav marks active: the explicit override, else the current route's. */
export function useActivePath(override: () => string | undefined) {
  const route = useRoute() as { path: string } | undefined
  return (to: string) => isActivePath(override() ?? route?.path ?? '', to)
}

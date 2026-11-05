import type { IconName } from './icons'

export interface NavDestination {
  id: string
  label: string
  icon: IconName
  to: string
  badge?: number
  group?: string
  /** Runs instead of navigating (the compact bar's More item); the item renders as a button. */
  action?: () => void
  /** Marks an `action` item active (a path cannot). */
  current?: boolean
}

/** True when `path` is the destination or below it (`/tasks/3` is under `/tasks`; `/` matches only itself). */
export function isActivePath(path: string, to: string): boolean {
  if (to === '/') return path === '/'
  return path === to || path.startsWith(`${to}/`)
}

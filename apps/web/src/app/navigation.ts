import { computed } from 'vue'
import { useInboxStore } from '../stores/inbox'
import { useSessionStore } from '../stores/session'
import type { NavDestination } from '../ui'

export interface AppDestination extends NavDestination {
  /** Hidden from children (D-063). */
  adultOnly?: boolean
  /** Shown in the compact bottom bar; the rest sit behind More. */
  primary: boolean
}

/** Order and grouping are data: edit this list to reorder the nav (plan/M4.md, Navigation). */
export const DESTINATIONS: AppDestination[] = [
  { id: 'today', label: 'Today', icon: 'home', to: '/', primary: true },
  { id: 'calendar', label: 'Calendar', icon: 'calendar', to: '/calendar', primary: true },
  { id: 'tasks', label: 'Tasks', icon: 'tasks', to: '/tasks', primary: true },
  { id: 'inbox', label: 'Inbox', icon: 'inbox', to: '/inbox', primary: true, adultOnly: true },
  { id: 'routines', label: 'Routines', icon: 'routines', to: '/routines', primary: false },
  { id: 'mail', label: 'Mail', icon: 'mail', to: '/mail', primary: false, adultOnly: true },
  { id: 'bills', label: 'Bills', icon: 'bills', to: '/bills', primary: false, adultOnly: true },
  { id: 'settings', label: 'Settings', icon: 'settings', to: '/settings', primary: false },
]

export interface NavModel {
  /** Everything this user may open, in order (rail and drawer). */
  all: AppDestination[]
  /** The bottom bar's destinations. */
  primary: AppDestination[]
  /** What the More sheet lists. */
  more: AppDestination[]
}

/** Pure part of `useNavigation`, for tests: filter by role, fill the inbox badge, split primary and more. */
export function buildNavigation(isAdult: boolean, inboxCount: number): NavModel {
  const all = DESTINATIONS.filter((d) => isAdult || !d.adultOnly).map((d) =>
    d.id === 'inbox' && inboxCount > 0 ? { ...d, badge: inboxCount } : { ...d },
  )
  return {
    all,
    primary: all.filter((d) => d.primary),
    more: all.filter((d) => !d.primary),
  }
}

export function useNavigation() {
  const session = useSessionStore()
  const inbox = useInboxStore()
  return computed(() => buildNavigation(session.isAdult, inbox.total))
}

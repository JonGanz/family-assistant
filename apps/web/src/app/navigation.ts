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
  { id: 'food', label: 'Food', icon: 'food', to: '/food', primary: true },
  { id: 'inbox', label: 'Inbox', icon: 'inbox', to: '/inbox', primary: true, adultOnly: true },
  { id: 'routines', label: 'Routines', icon: 'routines', to: '/routines', primary: false },
  { id: 'mail', label: 'Mail', icon: 'mail', to: '/mail', primary: false, adultOnly: true },
  { id: 'bills', label: 'Bills', icon: 'bills', to: '/bills', primary: false, adultOnly: true },
  { id: 'settings', label: 'Settings', icon: 'settings', to: '/settings', primary: false },
]

/** The Food tabs (Frontend.md §3f): the expanded drawer lists them under a "Food" group instead of the one Food entry. */
export const FOOD_ENTRIES: AppDestination[] = [
  {
    id: 'food-plan',
    label: 'Meal plan',
    icon: 'food',
    to: '/food/plan',
    group: 'Food',
    primary: false,
  },
  {
    id: 'food-list',
    label: 'Shopping list',
    icon: 'cart',
    to: '/food/list',
    group: 'Food',
    primary: false,
  },
  {
    id: 'food-recipes',
    label: 'Recipes',
    icon: 'recipe',
    to: '/food/recipes',
    group: 'Food',
    primary: false,
  },
  {
    id: 'food-templates',
    label: 'Templates',
    icon: 'template',
    to: '/food/templates',
    group: 'Food',
    primary: false,
    adultOnly: true,
  },
]

export interface NavModel {
  /** Everything this user may open, in order (rail and drawer). */
  all: AppDestination[]
  /** The bottom bar's destinations. */
  primary: AppDestination[]
  /** What the More sheet lists. */
  more: AppDestination[]
  /** The drawer's list: `all` with Food replaced by its four tabs (group "Food"). */
  drawer: AppDestination[]
}

/** Pure part of `useNavigation`, for tests: filter by role, fill the inbox badge, split primary and more. */
export function buildNavigation(isAdult: boolean, inboxCount: number): NavModel {
  const all = DESTINATIONS.filter((d) => isAdult || !d.adultOnly).map((d) =>
    d.id === 'inbox' && inboxCount > 0 ? { ...d, badge: inboxCount } : { ...d },
  )
  const entries = FOOD_ENTRIES.filter((d) => isAdult || !d.adultOnly).map((d) => ({ ...d }))
  return {
    all,
    primary: all.filter((d) => d.primary),
    more: all.filter((d) => !d.primary),
    drawer: all.flatMap((d) => (d.id === 'food' ? entries : [d])),
  }
}

export function useNavigation() {
  const session = useSessionStore()
  const inbox = useInboxStore()
  return computed(() => buildNavigation(session.isAdult, inbox.total))
}

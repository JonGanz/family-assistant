import type { IconName } from '../ui'

export interface SettingsPage {
  to: string
  label: string
  /** One line under the label (SettingsLayout shows this). */
  blurb: string
  /** The same line under its newer name, for lists that read `summary`. */
  summary: string
  icon: IconName
  adult?: boolean
}

export interface SettingsGroup {
  title: string
  pages: SettingsPage[]
}

/** The settings map (Frontend.md §3c): one list drives the sidebar, the chips and the index. */
type PageDef = Omit<SettingsPage, 'summary'>
const page = (p: PageDef): SettingsPage => ({ ...p, summary: p.blurb })

const RAW: { title: string; pages: PageDef[] }[] = [
  {
    title: 'Personal',
    pages: [
      { to: '/settings/account', icon: 'person', label: 'Account', blurb: 'Change your password.' },
      {
        to: '/settings/appearance',
        icon: 'palette',
        label: 'Appearance',
        blurb: 'Colour theme for this device.',
      },
      {
        to: '/settings/notifications',
        icon: 'notifications',
        label: 'Notifications',
        blurb: 'What you are told about, and how.',
      },
    ],
  },
  {
    title: 'Household',
    pages: [
      {
        to: '/settings/household',
        icon: 'home',
        label: 'Household',
        blurb: 'Name, time zone, location, units and defaults.',
        adult: true,
      },
      {
        to: '/settings/members',
        icon: 'group',
        label: 'Members',
        blurb: 'Adults and children, their logins and PINs.',
        adult: true,
      },
      {
        to: '/settings/meals',
        icon: 'food',
        label: 'Meals',
        blurb: 'Meal slots, stores, aisles and ingredients',
        adult: true,
      },
      {
        to: '/settings/tasks',
        icon: 'tasks',
        label: 'Tasks',
        blurb: 'Lists, chores and schedules, routines, absences.',
        adult: true,
      },
    ],
  },
  {
    title: 'Sources and rules',
    pages: [
      {
        to: '/settings/calendars',
        icon: 'calendar',
        label: 'Calendars',
        blurb: 'Google, CalDAV and ICS calendars.',
        adult: true,
      },
      {
        to: '/settings/weather',
        icon: 'weather',
        label: 'Weather',
        blurb: 'Advisory rules.',
        adult: true,
      },
      {
        to: '/settings/mail',
        icon: 'mail',
        label: 'Mail',
        blurb: 'Mail sources, categories and rules.',
        adult: true,
      },
      {
        to: '/settings/ai',
        icon: 'ai',
        label: 'AI',
        blurb: 'Local language model for email',
        adult: true,
      },
      {
        to: '/settings/orgs',
        icon: 'label',
        label: 'Organizations',
        blurb: 'Schools and clubs the mail comes from.',
        adult: true,
      },
    ],
  },
  {
    title: 'System',
    pages: [
      {
        to: '/settings/system',
        icon: 'settings',
        label: 'System',
        blurb: 'Addresses, notifications server, master key, household file.',
        adult: true,
      },
      {
        to: '/settings/devices',
        icon: 'devices',
        label: 'Devices',
        blurb: 'Pair wall screens and edit their layout.',
        adult: true,
      },
      {
        to: '/settings/status',
        icon: 'sync',
        label: 'Status',
        blurb: 'What is syncing and when.',
        adult: true,
      },
      {
        to: '/settings/about',
        icon: 'info',
        label: 'About',
        blurb: 'Version and commit of the server and of this page.',
        adult: true,
      },
    ],
  },
]

export const SETTINGS_GROUPS: SettingsGroup[] = RAW.map((g) => ({
  title: g.title,
  pages: g.pages.map(page),
}))

/** Groups a user may see: children get the pages that are not adult-only. */
export function visibleGroups(isAdult: boolean): SettingsGroup[] {
  return SETTINGS_GROUPS.map((g) => ({
    title: g.title,
    pages: g.pages.filter((p) => isAdult || !p.adult),
  })).filter((g) => g.pages.length > 0)
}

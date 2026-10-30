export interface SettingsPage {
  to: string
  label: string
  blurb: string
  adult?: boolean
}

export interface SettingsGroup {
  title: string
  pages: SettingsPage[]
}

/** The settings map (Frontend.md §3c): one list drives the sidebar, the chips and the index. */
export const SETTINGS_GROUPS: SettingsGroup[] = [
  {
    title: 'Personal',
    pages: [
      { to: '/settings/account', label: 'Account', blurb: 'Change your password.' },
      {
        to: '/settings/appearance',
        label: 'Appearance',
        blurb: 'Colour theme for this device.',
      },
      {
        to: '/settings/notifications',
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
        label: 'Household',
        blurb: 'Name, time zone, location, units and defaults.',
        adult: true,
      },
      {
        to: '/settings/members',
        label: 'Members',
        blurb: 'Adults and children, their logins and PINs.',
        adult: true,
      },
      {
        to: '/settings/tasks',
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
        label: 'Calendars',
        blurb: 'Google, CalDAV and ICS calendars.',
        adult: true,
      },
      { to: '/settings/weather', label: 'Weather', blurb: 'Advisory rules.', adult: true },
      {
        to: '/settings/mail',
        label: 'Mail',
        blurb: 'Mail sources, categories and rules.',
        adult: true,
      },
      {
        to: '/settings/orgs',
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
        label: 'System',
        blurb: 'Addresses, notifications server, master key, household file.',
        adult: true,
      },
      {
        to: '/settings/devices',
        label: 'Devices',
        blurb: 'Pair wall screens and edit their layout.',
        adult: true,
      },
      { to: '/settings/status', label: 'Status', blurb: 'What is syncing and when.', adult: true },
    ],
  },
]

/** Groups a user may see: children get the pages that are not adult-only. */
export function visibleGroups(isAdult: boolean): SettingsGroup[] {
  return SETTINGS_GROUPS.map((g) => ({
    title: g.title,
    pages: g.pages.filter((p) => isAdult || !p.adult),
  })).filter((g) => g.pages.length > 0)
}

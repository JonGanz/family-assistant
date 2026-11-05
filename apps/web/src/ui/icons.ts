// The only place icons are imported (R-005, D-095). Views use names from this map, never `~icons/...`.
// Style: Material Symbols Rounded, weight 400. The SVGs use fill=currentColor.
import I_home from '~icons/material-symbols/home-rounded'
import I_today from '~icons/material-symbols/today-rounded'
import I_calendar from '~icons/material-symbols/calendar-month-rounded'
import I_tasks from '~icons/material-symbols/task-alt-rounded'
import I_routines from '~icons/material-symbols/event-repeat-rounded'
import I_inbox from '~icons/material-symbols/inbox-rounded'
import I_mail from '~icons/material-symbols/mail-rounded'
import I_bills from '~icons/material-symbols/receipt-long-rounded'
import I_settings from '~icons/material-symbols/settings-rounded'
import I_more from '~icons/material-symbols/more-horiz-rounded'
import I_add from '~icons/material-symbols/add-rounded'
import I_edit from '~icons/material-symbols/edit-rounded'
import I_delete from '~icons/material-symbols/delete-rounded'
import I_back from '~icons/material-symbols/arrow-back-rounded'
import I_close from '~icons/material-symbols/close-rounded'
import I_check from '~icons/material-symbols/check-rounded'
import I_undo from '~icons/material-symbols/undo-rounded'
import I_search from '~icons/material-symbols/search-rounded'
import I_filter from '~icons/material-symbols/filter-list-rounded'
import I_sort from '~icons/material-symbols/sort-rounded'
import I_expand from '~icons/material-symbols/expand-more-rounded'
import I_collapse from '~icons/material-symbols/expand-less-rounded'
import I_chevron_right from '~icons/material-symbols/chevron-right-rounded'
import I_menu from '~icons/material-symbols/menu-rounded'
import I_person from '~icons/material-symbols/person-rounded'
import I_group from '~icons/material-symbols/group-rounded'
import I_devices from '~icons/material-symbols/devices-rounded'
import I_palette from '~icons/material-symbols/palette-rounded'
import I_notifications from '~icons/material-symbols/notifications-rounded'
import I_weather from '~icons/material-symbols/partly-cloudy-day-rounded'
import I_label from '~icons/material-symbols/label-rounded'
import I_link from '~icons/material-symbols/link-rounded'
import I_open_external from '~icons/material-symbols/open-in-new-rounded'
import I_warning from '~icons/material-symbols/warning-rounded'
import I_error from '~icons/material-symbols/error-rounded'
import I_info from '~icons/material-symbols/info-rounded'
import I_success from '~icons/material-symbols/check-circle-rounded'
import I_ai from '~icons/material-symbols/auto-awesome-rounded'
import I_refresh from '~icons/material-symbols/refresh-rounded'
import I_sync from '~icons/material-symbols/sync-rounded'
import I_lock from '~icons/material-symbols/lock-rounded'
import I_key from '~icons/material-symbols/key-rounded'
import I_logout from '~icons/material-symbols/logout-rounded'
import I_arrow_upward from '~icons/material-symbols/arrow-upward-rounded'
import I_arrow_downward from '~icons/material-symbols/arrow-downward-rounded'
import I_menu_open from '~icons/material-symbols/menu-open-rounded'
import I_visibility from '~icons/material-symbols/visibility-rounded'
import I_visibility_off from '~icons/material-symbols/visibility-off-rounded'

import I_home_outline from '~icons/material-symbols/home-outline-rounded'
import I_today_outline from '~icons/material-symbols/today-outline-rounded'
import I_calendar_outline from '~icons/material-symbols/calendar-month-outline-rounded'
import I_tasks_outline from '~icons/material-symbols/task-alt-outline-rounded'
import I_routines_outline from '~icons/material-symbols/event-repeat-outline-rounded'
import I_inbox_outline from '~icons/material-symbols/inbox-outline-rounded'
import I_mail_outline from '~icons/material-symbols/mail-outline-rounded'
import I_bills_outline from '~icons/material-symbols/receipt-long-outline-rounded'
import I_settings_outline from '~icons/material-symbols/settings-outline-rounded'
import I_more_outline from '~icons/material-symbols/more-horiz-outline-rounded'

export const icons = {
  home: I_home,
  today: I_today,
  calendar: I_calendar,
  tasks: I_tasks,
  routines: I_routines,
  inbox: I_inbox,
  mail: I_mail,
  bills: I_bills,
  settings: I_settings,
  more: I_more,
  add: I_add,
  edit: I_edit,
  delete: I_delete,
  back: I_back,
  close: I_close,
  check: I_check,
  undo: I_undo,
  search: I_search,
  filter: I_filter,
  sort: I_sort,
  expand: I_expand,
  collapse: I_collapse,
  chevron_right: I_chevron_right,
  menu: I_menu,
  person: I_person,
  group: I_group,
  devices: I_devices,
  palette: I_palette,
  notifications: I_notifications,
  weather: I_weather,
  label: I_label,
  link: I_link,
  open_external: I_open_external,
  warning: I_warning,
  error: I_error,
  info: I_info,
  success: I_success,
  ai: I_ai,
  refresh: I_refresh,
  sync: I_sync,
  lock: I_lock,
  key: I_key,
  logout: I_logout,
  arrow_upward: I_arrow_upward,
  arrow_downward: I_arrow_downward,
  menu_open: I_menu_open,
  visibility: I_visibility,
  visibility_off: I_visibility_off,
  home_outline: I_home_outline,
  today_outline: I_today_outline,
  calendar_outline: I_calendar_outline,
  tasks_outline: I_tasks_outline,
  routines_outline: I_routines_outline,
  inbox_outline: I_inbox_outline,
  mail_outline: I_mail_outline,
  bills_outline: I_bills_outline,
  settings_outline: I_settings_outline,
  more_outline: I_more_outline,
} as const

export type IconName = keyof typeof icons

/** The glyph for a nav item: filled while active, outline otherwise (when the set has an outline pair). */
export function navGlyph(name: IconName, active: boolean): IconName {
  const outline = `${name}_outline` as IconName
  return !active && outline in icons ? outline : name
}

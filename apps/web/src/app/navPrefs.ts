export const NAV_COLLAPSED_KEY = 'fa.nav.collapsed'

/** Whether this device keeps the expanded-window drawer collapsed to the rail. */
export function getNavCollapsed(): boolean {
  try {
    return localStorage.getItem(NAV_COLLAPSED_KEY) === '1'
  } catch {
    return false
  }
}

export function setNavCollapsed(collapsed: boolean) {
  try {
    if (collapsed) localStorage.setItem(NAV_COLLAPSED_KEY, '1')
    else localStorage.removeItem(NAV_COLLAPSED_KEY)
  } catch {
    // Storage can be blocked; the choice then lasts until reload.
  }
}

const STORAGE_KEY = 'fa.deviceToken'

export function getDeviceToken(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY)
  } catch {
    return null
  }
}

/**
 * Pick up `#device=<token>` from the URL fragment, persist it, and clear the fragment.
 * Returns the active token (fresh from the hash, else previously stored).
 */
export function captureDeviceToken(): string | null {
  const params = new URLSearchParams(location.hash.replace(/^#/, ''))
  const fromHash = params.get('device')
  if (!fromHash) return getDeviceToken()
  try {
    localStorage.setItem(STORAGE_KEY, fromHash)
  } catch {
    /* storage unavailable: token is still usable for this page load via the return value */
  }
  history.replaceState(null, '', location.pathname + location.search)
  return fromHash
}

export function clearDeviceToken(): void {
  try {
    localStorage.removeItem(STORAGE_KEY)
  } catch {
    /* ignore */
  }
}

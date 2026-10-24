const STORAGE_KEY = 'fa.deviceToken'

export function getDeviceToken(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY)
  } catch {
    return null
  }
}

/**
 * Pick up `?device=<token>` from the URL, persist it, and strip it from the address bar right away
 * (so it doesn't linger in history or get bookmarked). Returns the active token (fresh from the
 * URL, else previously stored).
 */
export function captureDeviceToken(): string | null {
  const params = new URLSearchParams(location.search)
  const fromUrl = params.get('device')
  if (!fromUrl) return getDeviceToken()
  try {
    localStorage.setItem(STORAGE_KEY, fromUrl)
  } catch {
    /* storage unavailable: token is still usable for this page load via the return value */
  }
  params.delete('device')
  const rest = params.toString()
  history.replaceState(null, '', location.pathname + (rest ? `?${rest}` : '') + location.hash)
  return fromUrl
}

export function clearDeviceToken(): void {
  try {
    localStorage.removeItem(STORAGE_KEY)
  } catch {
    /* ignore */
  }
}

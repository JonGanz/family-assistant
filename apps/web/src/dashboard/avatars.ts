import { getDeviceToken } from '../kiosk/deviceToken'

const blobs = new Map<string, Promise<string | null>>()

/**
 * `<img>` cannot send the device's bearer token, so on a paired screen avatars are fetched
 * with it and shown through a blob URL (the service worker caches the response for offline).
 * Without a device token (settings preview) the plain URL works through the session cookie.
 */
export function avatarSrc(url: string | null | undefined): Promise<string | null> {
  if (!url) return Promise.resolve(null)
  const token = getDeviceToken()
  if (!token) return Promise.resolve(url)
  let p = blobs.get(url)
  if (!p) {
    p = fetch(url, { headers: { Authorization: `Bearer ${token}` } })
      .then(async (r) => (r.ok ? URL.createObjectURL(await r.blob()) : null))
      .catch(() => null)
    blobs.set(url, p)
  }
  return p
}

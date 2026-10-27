/**
 * Registers the service worker, but only in a secure context: over plain HTTP on a LAN address
 * there is none, and the app must load from the network without it (D-072). The generated worker
 * (`/sw.js`, vite-plugin-pwa `autoUpdate`) skips waiting and claims clients, so a bare register
 * is enough.
 */
export async function registerServiceWorker(
  secure: boolean = typeof window !== 'undefined' && window.isSecureContext,
): Promise<boolean> {
  if (!secure || typeof navigator === 'undefined' || !('serviceWorker' in navigator)) return false
  try {
    await navigator.serviceWorker.register('/sw.js')
    return true
  } catch {
    return false
  }
}

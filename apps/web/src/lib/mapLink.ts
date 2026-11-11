/**
 * A link that opens an event's location in a map. Apple devices get the `maps:` scheme (Apple Maps),
 * Android gets `geo:` (the chooser of installed map apps); everything else opens Google Maps in a new
 * tab. Google Maps is the only web provider for now (no setting).
 */
export interface MapLink {
  href: string
  /** True when the link should open in a new tab (the web fallback). */
  external: boolean
}

export function mapLink(
  location: string,
  userAgent: string = globalThis.navigator?.userAgent ?? '',
): MapLink | null {
  const q = location.trim()
  if (!q) return null
  const enc = encodeURIComponent(q)
  if (/iPhone|iPad|iPod|Macintosh.*Mobile/.test(userAgent)) {
    return { href: `maps:?q=${enc}`, external: false }
  }
  if (/Android/.test(userAgent)) return { href: `geo:0,0?q=${enc}`, external: false }
  return { href: `https://www.google.com/maps/search/?api=1&query=${enc}`, external: true }
}

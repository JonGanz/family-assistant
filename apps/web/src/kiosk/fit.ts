// The kiosk is designed at 1920x1080 (landscape) or 1080x1920 (portrait) CSS pixels with a 16px
// root. Smaller screens (tablets, scaled displays, a browser bar) get a smaller root, so every
// rem-sized part shrinks together and the whole dashboard still fits: the page never scrolls.

export const DESIGN_LONG = 1920
export const DESIGN_SHORT = 1080
export const MIN_SCALE = 0.45

/** The scale for a `w` x `h` viewport: 1 at the design size or larger, never below MIN_SCALE. */
export function kioskScale(w: number, h: number): number {
  if (!(w > 0 && h > 0)) return 1
  const s = Math.min(Math.max(w, h) / DESIGN_LONG, Math.min(w, h) / DESIGN_SHORT)
  return Math.min(1, Math.max(MIN_SCALE, s))
}

/** The root font size in px for the viewport. */
export const kioskRootPx = (w: number, h: number) => Math.round(16 * kioskScale(w, h) * 100) / 100

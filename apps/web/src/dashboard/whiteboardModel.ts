import type { Hue } from '../lib/hues'
import { isHue } from '../lib/hues'

/** `ink` is black on a light theme and white on a dark one; anything else is a theme hue. */
export type Ink = 'ink' | Hue
export interface Stroke {
  c: Ink
  w: number
  p: [number, number][]
}

export const WHITEBOARD_KEY = 'fa.dashboard.whiteboard'

export function loadStrokes(): Stroke[] {
  try {
    const raw = JSON.parse(localStorage.getItem(WHITEBOARD_KEY) ?? '[]') as unknown
    if (!Array.isArray(raw)) return []
    return raw.filter(
      (s): s is Stroke =>
        !!s &&
        (s.c === 'ink' || isHue(s.c)) &&
        typeof s.w === 'number' &&
        Array.isArray(s.p) &&
        s.p.length > 0,
    )
  } catch {
    return []
  }
}

export function saveStrokes(strokes: Stroke[]): void {
  try {
    localStorage.setItem(WHITEBOARD_KEY, JSON.stringify(strokes))
  } catch {
    /* storage full or unavailable: the drawing stays on screen only */
  }
}

/** SVG path of a stroke; a single point becomes a dot (zero-length segment, round cap). */
export function strokePath(p: [number, number][]): string {
  const [first, ...rest] = p
  if (!first) return ''
  const pts = rest.length ? rest : [first]
  return `M${first[0]} ${first[1]}${pts.map(([x, y]) => `L${x} ${y}`).join('')}`
}

import type { Hue } from '../lib/hues'
import { isHue } from '../lib/hues'

/** `ink` is black on a light theme and white on a dark one; anything else is a theme hue. */
export type Ink = 'ink' | Hue
export const SHAPES = ['rect', 'circle', 'triangle', 'hexagon'] as const
export type Shape = (typeof SHAPES)[number]
type Pt = [number, number]
/** Freehand: `p` is the path. A shape (`s`): `p` is the two corners of its bounding box. */
export interface Stroke {
  c: Ink
  w: number
  p: Pt[]
  s?: Shape
}

export const WHITEBOARD_KEY = 'fa.dashboard.whiteboard'

const isShape = (x: unknown): x is Shape => (SHAPES as readonly unknown[]).includes(x)

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
        (s.s === undefined ? s.p.length > 0 : isShape(s.s) && s.p.length === 2),
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

/** SVG path of a freehand stroke; a single point becomes a dot (zero-length segment, round cap). */
export function strokePath(p: Pt[]): string {
  const [first, ...rest] = p
  if (!first) return ''
  const pts = rest.length ? rest : [first]
  return `M${first[0]} ${first[1]}${pts.map(([x, y]) => `L${x} ${y}`).join('')}`
}

const ELLIPSE_STEPS = 64

/** Outline vertices of a shape in the box between two corners (a circle is a sampled ellipse). */
export function shapePoints(shape: Shape, a: Pt, b: Pt): Pt[] {
  const l = Math.min(a[0], b[0])
  const r = Math.max(a[0], b[0])
  const t = Math.min(a[1], b[1])
  const m = Math.max(a[1], b[1])
  const w = r - l
  const cx = (l + r) / 2
  const cy = (t + m) / 2
  switch (shape) {
    case 'rect':
      return [
        [l, t],
        [r, t],
        [r, m],
        [l, m],
      ]
    case 'triangle':
      return [
        [cx, t],
        [r, m],
        [l, m],
      ]
    case 'hexagon':
      return [
        [l + w / 4, t],
        [r - w / 4, t],
        [r, cy],
        [r - w / 4, m],
        [l + w / 4, m],
        [l, cy],
      ]
    case 'circle':
      return Array.from({ length: ELLIPSE_STEPS }, (_, i): Pt => {
        const th = (2 * Math.PI * i) / ELLIPSE_STEPS
        return [cx + (w / 2) * Math.cos(th), cy + ((m - t) / 2) * Math.sin(th)]
      })
  }
}

/** The vertices to draw a stroke through, closed (first point repeated) for shapes. */
export function outline(s: Stroke): Pt[] {
  if (!s.s) return s.p
  const pts = shapePoints(s.s, s.p[0]!, s.p[1]!)
  return [...pts, pts[0]!]
}

/** SVG path of any stroke. */
export function pathOf(s: Stroke): string {
  if (!s.s) return strokePath(s.p)
  return `${strokePath(outline(s))}Z`
}

function segDist([x, y]: Pt, [x0, y0]: Pt, [x1, y1]: Pt): number {
  const dx = x1 - x0
  const dy = y1 - y0
  const len2 = dx * dx + dy * dy
  const t = len2 === 0 ? 0 : Math.max(0, Math.min(1, ((x - x0) * dx + (y - y0) * dy) / len2))
  return Math.hypot(x - (x0 + t * dx), y - (y0 + t * dy))
}

/** Distance from a point to the nearest part of a stroke's line. */
export function distanceTo(s: Stroke, pt: Pt): number {
  const o = outline(s)
  if (o.length === 1) return Math.hypot(pt[0] - o[0]![0], pt[1] - o[0]![1])
  let best = Infinity
  for (let i = 1; i < o.length; i++) best = Math.min(best, segDist(pt, o[i - 1]!, o[i]!))
  return best
}

// Live scaling of a recipe line (D-125). Only the leading amount of `raw_text` is rewritten.
import type { RecipeLineView } from '../api/food'

const GLYPHS: [number, string][] = [
  [1 / 4, '¼'],
  [1 / 3, '⅓'],
  [1 / 2, '½'],
  [2 / 3, '⅔'],
  [3 / 4, '¾'],
]
const GLYPH_VALUE: Record<string, number> = {
  '¼': 0.25,
  '½': 0.5,
  '¾': 0.75,
  '⅓': 1 / 3,
  '⅔': 2 / 3,
  '⅛': 0.125,
  '⅜': 0.375,
  '⅝': 0.625,
  '⅞': 0.875,
}

/** Formats a quantity with a fraction glyph when it is within 0.02 of one, else up to two decimals. */
export function formatQty(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return '0'
  let whole = Math.floor(n)
  const frac = n - whole
  if (frac > 0.98) whole += 1
  else if (frac >= 0.02) {
    const g = GLYPHS.find(([v]) => Math.abs(frac - v) <= 0.02)
    if (g) return (whole ? String(whole) : '') + g[1]
    return String(Math.round(n * 100) / 100)
  }
  return String(whole)
}

const AMOUNT = '(\\d+\\s+\\d+\\/\\d+|\\d+\\/\\d+|(?:\\d+\\s?)?[¼½¾⅓⅔⅛⅜⅝⅞]|\\d+(?:[.,]\\d+)?)'
const LEADING = new RegExp(`^(\\s*)${AMOUNT}(?:(\\s*(?:-|–|to)\\s*)${AMOUNT})?`)

function parseAmount(s: string): number {
  const t = s.trim()
  let m = /^(\d+)\s+(\d+)\/(\d+)$/.exec(t)
  if (m) return Number(m[1]) + Number(m[2]) / Number(m[3])
  m = /^(\d+)\/(\d+)$/.exec(t)
  if (m) return Number(m[1]) / Number(m[2])
  m = /^(?:(\d+)\s?)?([¼½¾⅓⅔⅛⅜⅝⅞])$/.exec(t)
  if (m) return Number(m[1] ?? 0) + GLYPH_VALUE[m[2]!]!
  return Number(t.replace(',', '.'))
}

/** The line text for `factor`: unchanged at 1× or without a parsed quantity. */
export function scaleLine(line: RecipeLineView, factor: number): string {
  if (factor === 1 || line.qty == null) return line.raw_text
  const m = LEADING.exec(line.raw_text)
  if (m) {
    const a = parseAmount(m[2]!) * factor
    let out = formatQty(a)
    if (m[3] && m[4]) out += m[3] + formatQty(parseAmount(m[4]) * factor)
    if (Number.isFinite(a)) return m[1]! + out + line.raw_text.slice(m[0].length)
  }
  const unit = line.unit ? ` ${line.unit}` : ''
  return `${formatQty(line.qty * factor)}${unit} ${line.item ?? line.raw_text}`
}

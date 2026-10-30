/** Slot geometry of a Week-view cell, in rem (so it follows the kiosk's font scale). */
export const SLOT_REM = 3.2
export const GAP_REM = 0.25
export const MIN_COLUMN_REM = 14

export interface Fit {
  /** Items drawn. */
  shown: number
  /** Items folded into the "+N" slot (0 when everything fits). */
  more: number
  cols: number
}

/**
 * How many of `total` items a cell of `w` x `h` px draws. One column, or two when each column
 * would still be at least `MIN_COLUMN_REM` wide; when the items do not all fit, the last slot is
 * the "+N" button. A size of 0 (not measured yet) draws everything.
 */
export function fitCell(total: number, w: number, h: number, remPx: number): Fit {
  if (w <= 0 || h <= 0) return { shown: total, more: 0, cols: 1 }
  const gap = GAP_REM * remPx
  const rows = Math.max(1, Math.floor((h + gap) / (SLOT_REM * remPx + gap)))
  const cols = w >= (2 * MIN_COLUMN_REM + GAP_REM) * remPx ? 2 : 1
  const slots = rows * cols
  if (total <= slots) return { shown: total, more: 0, cols }
  return { shown: slots - 1, more: total - slots + 1, cols }
}

/** Slot geometry of a Week-view cell, in rem (so it follows the kiosk's font scale). */
export const SLOT_REM = 3.4
export const GAP_REM = 0.25
export const MIN_COLUMN_REM = 9

export interface Fit {
  /** Items drawn. */
  shown: number
  /** Items folded into the "+N" slot (0 when everything fits). */
  more: number
  cols: number
}

/**
 * How many of `total` items a cell of `w` x `h` px draws. Items go in one column; only when they
 * do not fit that way are they split into two half-width columns (and only if each half stays at
 * least `MIN_COLUMN_REM` wide, else the cell keeps its full-width column). When they still do not
 * all fit, the last slot (the bottom row's final half) is the "+N" button. A size of 0 (not
 * measured yet) draws everything.
 */
export function fitCell(
  total: number,
  w: number,
  h: number,
  remPx: number,
  slotRem: number = SLOT_REM,
): Fit {
  if (w <= 0 || h <= 0) return { shown: total, more: 0, cols: 1 }
  const gap = GAP_REM * remPx
  const rows = Math.max(1, Math.floor((h + gap) / (slotRem * remPx + gap)))
  const canSplit = w >= (2 * MIN_COLUMN_REM + GAP_REM) * remPx
  const cols = total > rows && canSplit ? 2 : 1
  const slots = rows * cols
  if (total <= slots) return { shown: total, more: 0, cols }
  return { shown: slots - 1, more: total - slots + 1, cols }
}

/** Landscape slots: up to 3.4rem, shrinking to the text's height to fit 2 rows (1920x1080). */
export const LANDSCAPE = { max: SLOT_REM, min: 3.1, rows: 2 }
/** Portrait slots: up to 4.2rem, shrinking to the text's height to fit 3 rows (1080x1920). */
export const PORTRAIT = { max: 4.2, min: 3.35, rows: 3 }

export interface SlotRange {
  max: number
  min: number
  rows: number
}

/** The slot height (rem) for a cell `h` px tall: tall if there is room, else just enough for `rows`. */
export function adaptiveSlotRem(h: number, remPx: number, r: SlotRange): number {
  if (h <= 0) return r.max
  const rem = (h / remPx - (r.rows - 1) * GAP_REM) / r.rows
  // Rounded down so float error never costs the last row.
  return Math.min(r.max, Math.max(r.min, Math.floor(rem * 100) / 100))
}

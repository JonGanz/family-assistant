/** Slot geometry of a Week-view cell, in rem (so it follows the kiosk's font scale). */
export const SLOT_REM = 3.2
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

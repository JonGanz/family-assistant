/** How many one-line items fit under a day number in a cell of the given height (all in px). */
export function capacity(cellHeight: number, lineHeight: number, headerHeight: number): number {
  if (!(lineHeight > 0)) return 0
  return Math.max(0, Math.floor((cellHeight - headerHeight) / lineHeight))
}

/** Items to show of `total` given room for `cap` lines: all if they fit, else cap-1 plus a "+N" line. */
export function visibleCount(total: number, cap: number): number {
  return total <= cap ? total : Math.max(0, cap - 1)
}

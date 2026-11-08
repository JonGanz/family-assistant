/** Moves `list[i]` by `delta` (-1 up, +1 down) and returns a new array; out-of-range moves return the same order. */
export function moved<T>(list: readonly T[], i: number, delta: number): T[] {
  const j = i + delta
  const out = [...list]
  if (j < 0 || j >= out.length) return out
  const [x] = out.splice(i, 1)
  out.splice(j, 0, x as T)
  return out
}

/** Adds or removes `id` in a list, in place (for checkbox groups bound to an array). */
export function toggleIn<T>(list: T[], id: T, on: boolean): void {
  const i = list.indexOf(id)
  if (on && i < 0) list.push(id)
  else if (!on && i >= 0) list.splice(i, 1)
}

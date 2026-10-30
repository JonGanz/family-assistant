/** True when both builds say which commit they came from and the commits differ. */
export function buildsDiffer(server?: string | null, web?: string | null): boolean {
  return !!server && !!web && server !== web
}

/** A build time for display in the household time zone; the raw text when it is not a date. */
export function builtAtText(iso: string | null | undefined, tz: string): string {
  if (!iso) return 'unknown'
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleString(undefined, { timeZone: tz, dateStyle: 'medium', timeStyle: 'short' })
}

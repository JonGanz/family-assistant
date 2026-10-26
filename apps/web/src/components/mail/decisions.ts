import type { DecisionPointKey, DecisionView } from '../../api/client'

/** The four decision points a person labels (event routing is handled in the inbox). */
export const LABEL_POINTS: DecisionPointKey[] = [
  'email.category',
  'email.time_sensitive',
  'email.member_link',
  'email.is_bill',
]

export const POINT_TITLES: Record<DecisionPointKey, string> = {
  'email.category': 'Category',
  'email.time_sensitive': 'Time-sensitive',
  'email.member_link': 'About',
  'email.is_bill': 'Is a bill',
  'event.calendar_route': 'Calendar',
}

/** The decision that currently counts: the final one, else the latest. */
export function currentDecision(
  decisions: DecisionView[],
  key: DecisionPointKey,
): DecisionView | undefined {
  const mine = decisions.filter((d) => d.decision_point === key)
  return mine.find((d) => d.is_final) ?? mine[mine.length - 1]
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i

/**
 * The members of the final `email.member_link` decisions (rules record one decision per member,
 * a person records one list), or undefined when none is final. The non-final default is the
 * word "owners", which is not a member id.
 */
export function memberLinkIds(decisions: DecisionView[]): string[] | undefined {
  const final = decisions.filter((d) => d.decision_point === 'email.member_link' && d.is_final)
  if (!final.length) return undefined
  const ids = final
    .flatMap((d) => (Array.isArray(d.value) ? d.value : [d.value]))
    .filter((v): v is string => typeof v === 'string' && UUID.test(v))
  return [...new Set(ids)]
}

/**
 * Value to record for "Looks right": the current value, or a neutral default. `members` are the
 * message's linked members, used when the link was only a default.
 */
export function valueOrDefault(
  decisions: DecisionView[],
  key: DecisionPointKey,
  members: string[] = [],
): unknown {
  if (key === 'email.member_link') return memberLinkIds(decisions) ?? members
  const v = currentDecision(decisions, key)?.value
  if (v !== undefined && v !== null) return v
  if (key === 'email.category') return null
  return false
}

export function describeValue(
  key: DecisionPointKey,
  value: unknown,
  names: { member: (id: string) => string; category: (key: string) => string },
): string {
  if (value === undefined || value === null) return 'none'
  if (key === 'email.member_link') {
    const ids = Array.isArray(value) ? (value as string[]) : []
    return ids.length ? ids.map(names.member).join(', ') : 'nobody'
  }
  if (key === 'email.category') return names.category(String(value))
  if (typeof value === 'boolean') return value ? 'yes' : 'no'
  return String(value)
}

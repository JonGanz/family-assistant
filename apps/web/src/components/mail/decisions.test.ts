import { describe, expect, it } from 'vitest'
import type { DecisionView } from '../../api/client'
import { currentDecision, describeValue, memberLinkIds, valueOrDefault } from './decisions'

const d = (k: string, value: unknown, is_final: boolean): DecisionView =>
  ({ decision_point: k, value, is_final }) as DecisionView

describe('decisions', () => {
  it('prefers the final decision', () => {
    const ds = [d('email.is_bill', true, true), d('email.is_bill', false, false)]
    expect(currentDecision(ds, 'email.is_bill')?.value).toBe(true)
  })
  it('falls back to neutral defaults', () => {
    expect(valueOrDefault([], 'email.member_link')).toEqual([])
    expect(valueOrDefault([], 'email.is_bill')).toBe(false)
    expect(valueOrDefault([], 'email.category')).toBeNull()
  })
  it('reads member links from per-member decisions and ignores the default word', () => {
    const a = '01a176ca-e85e-7758-973e-44ee044ef609'
    const b = '01a176ca-e85c-718e-b923-969fc6a065a2'
    const rules = [d('email.member_link', a, true), d('email.member_link', b, true)]
    expect(memberLinkIds(rules)).toEqual([a, b])
    expect(valueOrDefault(rules, 'email.member_link')).toEqual([a, b])
    const byUser = [d('email.member_link', [a], true), d('email.member_link', 'owners', false)]
    expect(memberLinkIds(byUser)).toEqual([a])
    const onlyDefault = [d('email.member_link', 'owners', false)]
    expect(memberLinkIds(onlyDefault)).toBeUndefined()
    expect(valueOrDefault(onlyDefault, 'email.member_link', [b])).toEqual([b])
  })
  it('describes values', () => {
    const names = { member: (i: string) => `M${i}`, category: (k: string) => k.toUpperCase() }
    expect(describeValue('email.member_link', ['1', '2'], names)).toBe('M1, M2')
    expect(describeValue('email.member_link', [], names)).toBe('nobody')
    expect(describeValue('email.time_sensitive', true, names)).toBe('yes')
    expect(describeValue('email.category', 'school', names)).toBe('SCHOOL')
  })
})

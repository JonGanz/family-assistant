import { describe, expect, it } from 'vitest'
import { describeConditions, emptyRuleForm, fromView, splitValues, toInput } from './ruleForm'

describe('rule form', () => {
  it('splits values on commas and newlines', () => {
    expect(splitValues('a.com, b.com\n c.com ,')).toEqual(['a.com', 'b.com', 'c.com'])
  })

  it('builds a category rule input', () => {
    const f = emptyRuleForm()
    f.name = ' School mail '
    f.value = 'school'
    f.conditions[0]!.values = 'example.edu'
    expect(toInput(f)).toEqual({
      decision_point: 'email.category',
      name: 'School mail',
      priority: 100,
      conditions: [{ field: 'from_domain', op: 'in', values: ['example.edu'] }],
      value: 'school',
      confidence: 0.9,
      enabled: true,
    })
  })

  it('sends booleans for yes/no decision points and a header name for header conditions', () => {
    const f = emptyRuleForm()
    f.decision_point = 'email.time_sensitive'
    f.value = 'true'
    f.conditions = [{ field: 'header', op: 'exists', header: 'List-Id', values: 'ignored' }]
    const i = toInput(f)
    expect(i.value).toBe(true)
    expect(i.conditions).toEqual([{ field: 'header', op: 'exists', header: 'List-Id', values: [] }])
  })

  it('round-trips a view and describes it', () => {
    const f = fromView({
      id: 'r1',
      decision_point: 'email.is_bill',
      name: 'Bills',
      priority: 5,
      conditions: [{ field: 'subject', op: 'contains_any', values: ['invoice', 'bill'] }],
      value: true,
      confidence: 0.8,
      enabled: false,
      version: 1,
    })
    expect(f.value).toBe('true')
    expect(f.conditions[0]!.values).toBe('invoice, bill')
    expect(describeConditions(toInput(f).conditions)).toBe('Subject contains any of invoice, bill')
  })
})

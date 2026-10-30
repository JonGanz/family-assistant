import { describe, expect, it } from 'vitest'
import { resumeStep } from '../../stores/setup'
import { validateNewMember } from '../parts/memberForm'
import { emptyHousehold, emptyOwner, validateHousehold, validateOwner } from './validate'

const status = (over: Record<string, unknown> = {}) =>
  ({
    needed: false,
    owner_needed: false,
    completed: false,
    secure: false,
    origin: 'http://h',
    via_tailscale: false,
    ...over,
  }) as never

describe('household step', () => {
  const ok = () => ({ ...emptyHousehold(), name: 'Example Household', currency: 'EUR' })

  it('needs a name, a time zone and a three-letter currency', () => {
    expect(validateHousehold(ok())).toEqual({})
    expect(validateHousehold({ ...ok(), name: ' ' }).name).toBeTruthy()
    expect(validateHousehold({ ...ok(), tz: '' }).tz).toBeTruthy()
    expect(validateHousehold({ ...ok(), currency: 'EURO' }).currency).toBeTruthy()
  })

  it('takes latitude and longitude together, in range', () => {
    expect(validateHousehold({ ...ok(), lat: '48.1', lon: '11.5' })).toEqual({})
    expect(validateHousehold({ ...ok(), lat: '48.1' }).lat).toBeTruthy()
    expect(validateHousehold({ ...ok(), lat: '95', lon: '11.5' }).lat).toBeTruthy()
    expect(validateHousehold({ ...ok(), lat: '48', lon: '200' }).lon).toBeTruthy()
  })
})

describe('owner step', () => {
  const ok = () => ({
    ...emptyOwner(),
    display_name: 'Alex',
    username: 'alex',
    password: 'a long enough pass',
    again: 'a long enough pass',
  })

  it('requires a 12-character password that matches', () => {
    expect(validateOwner(ok())).toEqual({})
    expect(validateOwner({ ...ok(), password: 'short', again: 'short' }).password).toBeTruthy()
    expect(validateOwner({ ...ok(), again: 'different one here' }).again).toBeTruthy()
  })

  it('checks username and PIN', () => {
    expect(validateOwner({ ...ok(), username: 'a b' }).username).toBeTruthy()
    expect(validateOwner({ ...ok(), pin: '12' }).pin).toBeTruthy()
    expect(validateOwner({ ...ok(), pin: '1234' })).toEqual({})
  })
})

describe('family step', () => {
  const base = { display_name: 'Sam', role: 'child', username: '', password: '', pin: '' }

  it('login and PIN are optional, and a child password may be 4 characters', () => {
    expect(validateNewMember(base)).toEqual({})
    expect(validateNewMember({ ...base, username: 'sam', password: 'abcd' })).toEqual({})
    expect(
      validateNewMember({ ...base, role: 'adult', username: 'sam', password: 'abcd' }).password,
    ).toBeTruthy()
    expect(validateNewMember({ ...base, username: 'sam' }).password).toBeTruthy()
  })
})

describe('resume', () => {
  it('goes back to the server-known step and on to the stored one', () => {
    expect(resumeStep(status({ needed: true, owner_needed: true }), 5)).toBe(2)
    expect(resumeStep(status({ owner_needed: true }), 1)).toBe(3)
    expect(resumeStep(status(), 1)).toBe(4)
    expect(resumeStep(status(), 6)).toBe(6)
    expect(resumeStep(status(), 99)).toBe(7)
  })
})

describe('owner colour', () => {
  it('defaults to blue and accepts only hue names', () => {
    const o = { ...emptyOwner(), display_name: 'Pat', username: 'pat', password: 'x'.repeat(12) }
    o.again = o.password
    expect(o.color).toBe('blue')
    expect(validateOwner(o).color).toBeUndefined()
    expect(validateOwner({ ...o, color: '#2f6fed' as never }).color).toBeDefined()
  })
})

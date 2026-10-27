import { describe, expect, it } from 'vitest'
import { newId } from './ids'

describe('newId', () => {
  it('makes distinct v4 UUIDs', () => {
    const a = newId()
    expect(a).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/)
    expect(newId()).not.toBe(a)
  })
})

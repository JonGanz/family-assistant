import { beforeEach, describe, expect, it } from 'vitest'
import {
  CHECKS_PREFIX,
  CHECKS_TTL_MS,
  emptyChecks,
  loadChecks,
  pruneChecks,
  saveChecks,
  toggleCheck,
} from './recipeChecks'

const NOW = 1_000_000_000_000
beforeEach(() => localStorage.clear())

describe('recipeChecks', () => {
  it('toggles on and off and stamps the time', () => {
    let c = toggleCheck(emptyChecks(), 'lines', 'b', NOW)
    c = toggleCheck(c, 'lines', 'a', NOW + 1)
    expect(c.lines).toEqual(['b', 'a'])
    expect(c.at).toBe(NOW + 1)
    c = toggleCheck(c, 'lines', 'b', NOW + 2)
    expect(c.lines).toEqual(['a'])
  })

  it('round-trips through storage', () => {
    saveChecks('r1', toggleCheck(emptyChecks(), 'steps', 's1', NOW))
    expect(loadChecks('r1', NOW + 1000).steps).toEqual(['s1'])
  })

  it('treats a change older than 12 hours as reset and clears it', () => {
    saveChecks('r1', toggleCheck(emptyChecks(), 'steps', 's1', NOW))
    expect(loadChecks('r1', NOW + CHECKS_TTL_MS - 1).steps).toEqual(['s1'])
    expect(loadChecks('r1', NOW + CHECKS_TTL_MS + 1).steps).toEqual([])
    expect(localStorage.getItem(CHECKS_PREFIX + 'r1')).toBeNull()
  })

  it('ignores non-string ids and drops old index-keyed entries on prune', () => {
    localStorage.setItem(
      CHECKS_PREFIX + 'r1',
      JSON.stringify({ lines: [1, 'a'], steps: [], at: NOW }),
    )
    expect(loadChecks('r1', NOW).lines).toEqual(['a'])
    localStorage.setItem('fa.recipe.checks.r1', JSON.stringify({ lines: [1], steps: [], at: NOW }))
    pruneChecks(NOW)
    expect(localStorage.getItem('fa.recipe.checks.r1')).toBeNull()
    expect(localStorage.getItem(CHECKS_PREFIX + 'r1')).not.toBeNull()
  })

  it('survives corrupt JSON', () => {
    localStorage.setItem(CHECKS_PREFIX + 'r1', '{nope')
    expect(loadChecks('r1', NOW)).toEqual(emptyChecks())
    localStorage.setItem(CHECKS_PREFIX + 'r2', '{"at":1}')
    expect(loadChecks('r2', NOW)).toEqual(emptyChecks())
  })

  it('prunes expired and corrupt entries only', () => {
    saveChecks('old', toggleCheck(emptyChecks(), 'lines', 'l0', NOW - CHECKS_TTL_MS - 5))
    saveChecks('new', toggleCheck(emptyChecks(), 'lines', 'l0', NOW))
    localStorage.setItem(CHECKS_PREFIX + 'bad', 'x')
    localStorage.setItem('other', 'keep')
    pruneChecks(NOW)
    expect(localStorage.getItem(CHECKS_PREFIX + 'old')).toBeNull()
    expect(localStorage.getItem(CHECKS_PREFIX + 'bad')).toBeNull()
    expect(localStorage.getItem(CHECKS_PREFIX + 'new')).not.toBeNull()
    expect(localStorage.getItem('other')).toBe('keep')
  })
})

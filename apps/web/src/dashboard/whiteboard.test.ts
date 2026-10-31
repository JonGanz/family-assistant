import { beforeEach, describe, expect, it } from 'vitest'
import { loadStrokes, saveStrokes, strokePath, WHITEBOARD_KEY } from './whiteboardModel'

describe('whiteboard model', () => {
  beforeEach(() => localStorage.clear())
  it('round-trips strokes with hue names, not colours', () => {
    saveStrokes([
      { c: 'red', w: 4, p: [[1, 2]] },
      {
        c: 'ink',
        w: 2,
        p: [
          [0, 0],
          [3, 3],
        ],
      },
    ])
    expect(localStorage.getItem(WHITEBOARD_KEY)).not.toMatch(/#|rgb/)
    expect(loadStrokes()).toHaveLength(2)
  })
  it('drops junk and survives bad JSON', () => {
    localStorage.setItem(WHITEBOARD_KEY, JSON.stringify([{ c: '#000', w: 1, p: [[0, 0]] }]))
    expect(loadStrokes()).toEqual([])
    localStorage.setItem(WHITEBOARD_KEY, '{')
    expect(loadStrokes()).toEqual([])
  })
  it('draws a single point as a dot', () => {
    expect(strokePath([[5, 6]])).toBe('M5 6L5 6')
    expect(
      strokePath([
        [0, 0],
        [1, 1],
      ]),
    ).toBe('M0 0L1 1')
  })
})

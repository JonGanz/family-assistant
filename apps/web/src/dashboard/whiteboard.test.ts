import { beforeEach, describe, expect, it } from 'vitest'
import {
  distanceTo,
  loadStrokes,
  pathOf,
  saveStrokes,
  shapePoints,
  strokePath,
  WHITEBOARD_KEY,
  type Stroke,
} from './whiteboardModel'

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

describe('whiteboard shapes', () => {
  it('keeps shapes with two corners and drops malformed ones', () => {
    saveStrokes([
      {
        c: 'blue',
        w: 4,
        s: 'hexagon',
        p: [
          [0, 0],
          [10, 10],
        ],
      },
    ])
    expect(loadStrokes()).toHaveLength(1)
    localStorage.setItem(
      WHITEBOARD_KEY,
      JSON.stringify([
        {
          c: 'blue',
          w: 4,
          s: 'star',
          p: [
            [0, 0],
            [1, 1],
          ],
        },
        { c: 'blue', w: 4, s: 'rect', p: [[0, 0]] },
      ]),
    )
    expect(loadStrokes()).toEqual([])
  })
  it('builds the outline of each shape', () => {
    expect(shapePoints('rect', [10, 20], [0, 0])).toHaveLength(4)
    expect(shapePoints('triangle', [0, 0], [10, 10])[0]).toEqual([5, 0])
    expect(shapePoints('hexagon', [0, 0], [8, 8])).toHaveLength(6)
    const c = shapePoints('circle', [0, 0], [10, 10])
    expect(c[0]).toEqual([10, 5])
  })
  it('erases by the line, not the box: the middle of a rectangle is far from it', () => {
    const r: Stroke = {
      c: 'ink',
      w: 2,
      s: 'rect',
      p: [
        [0, 0],
        [100, 100],
      ],
    }
    expect(distanceTo(r, [50, 50])).toBe(50)
    expect(distanceTo(r, [50, 3])).toBe(3)
    expect(distanceTo({ c: 'ink', w: 2, p: [[5, 5]] }, [8, 9])).toBe(5)
  })
  it('closes a shape path', () => {
    expect(
      pathOf({
        c: 'ink',
        w: 2,
        s: 'triangle',
        p: [
          [0, 0],
          [10, 10],
        ],
      }),
    ).toMatch(/Z$/)
  })
})

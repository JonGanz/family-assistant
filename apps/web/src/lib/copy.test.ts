import { afterEach, describe, expect, it, vi } from 'vitest'
import { copyText } from './copy'

afterEach(() => vi.unstubAllGlobals())

describe('copyText', () => {
  it('uses the async clipboard when present', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { clipboard: { writeText } })
    expect(await copyText('x')).toBe(true)
    expect(writeText).toHaveBeenCalledWith('x')
  })

  it('falls back to execCommand without the async clipboard', async () => {
    vi.stubGlobal('navigator', {})
    document.execCommand = vi.fn().mockReturnValue(true)
    expect(await copyText('x')).toBe(true)
    expect(document.execCommand).toHaveBeenCalledWith('copy')
  })

  it('reports failure when nothing works', async () => {
    vi.stubGlobal('navigator', {})
    document.execCommand = vi.fn().mockReturnValue(false)
    expect(await copyText('x')).toBe(false)
  })
})

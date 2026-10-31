import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import Modal from './Modal.vue'

describe('Modal', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('closes itself after 60 s without a touch', () => {
    const w = mount(Modal, { props: { title: 'x' } })
    vi.advanceTimersByTime(59_000)
    expect(w.emitted('close')).toBeUndefined()
    vi.advanceTimersByTime(1000)
    expect(w.emitted('close')).toHaveLength(1)
  })

  it('a touch inside restarts the countdown', async () => {
    const w = mount(Modal, { props: { title: 'x' } })
    vi.advanceTimersByTime(50_000)
    await w.find('.backdrop').trigger('pointerdown')
    vi.advanceTimersByTime(50_000)
    expect(w.emitted('close')).toBeUndefined()
    vi.advanceTimersByTime(10_000)
    expect(w.emitted('close')).toHaveLength(1)
  })
})

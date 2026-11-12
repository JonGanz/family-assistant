import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PinPad from './PinPad.vue'

const press = async (w: ReturnType<typeof mount>, ...keys: string[]) => {
  for (const k of keys) {
    await w
      .findAll('button')
      .find((b) => b.text() === k)!
      .trigger('click')
  }
}
const count = (w: ReturnType<typeof mount>) =>
  w.get('[data-testid=pin-display]').findAll('.dot.on').length

describe('PinPad', () => {
  it('emits the PIN once at least 4 digits are entered and clears', async () => {
    const w = mount(PinPad)
    await press(w, '1', '2', '3')
    expect(w.find('.ok').attributes('disabled')).toBeDefined()
    await press(w, '4')
    await w.find('.ok').trigger('click')
    expect(w.emitted('submit')).toEqual([['1234']])
    expect(count(w)).toBe(0)
  })

  it('caps at 6 digits and supports backspace', async () => {
    const w = mount(PinPad)
    await press(w, '1', '2', '3', '4', '5', '6', '7')
    expect(count(w)).toBe(6)
    await w.get('[aria-label=Backspace]').trigger('click')
    await w.find('.ok').trigger('click')
    expect(w.emitted('submit')).toEqual([['12345']])
  })
})

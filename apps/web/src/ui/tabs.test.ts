import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import UiTabs from './UiTabs.vue'

const options = [
  { value: 'a', label: 'Alpha' },
  { value: 'b', label: 'Beta' },
]

describe('UiTabs', () => {
  it('emits the chosen value and marks the current one', async () => {
    const w = mount(UiTabs, { props: { modelValue: 'a', options, label: 'Demo' } })
    const items = w.findAll('[role=radio], [role=tab]')
    expect(items).toHaveLength(2)
    await items[1]!.trigger('click')
    expect(w.emitted('update:modelValue')![0]).toEqual(['b'])
  })
})

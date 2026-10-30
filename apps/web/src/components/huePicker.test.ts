import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import HuePicker from './HuePicker.vue'

describe('HuePicker', () => {
  it('is a radio group of eight named hues', async () => {
    const w = mount(HuePicker, { props: { modelValue: 'blue', name: 'c' } })
    const radios = w.findAll('input[type=radio]')
    expect(radios).toHaveLength(8)
    expect((w.find('input[value=blue]').element as HTMLInputElement).checked).toBe(true)
    expect(w.text()).toContain('Purple')
    await w.find('input[value=purple]').setValue(true)
    expect(w.emitted('update:modelValue')?.[0]).toEqual(['purple'])
  })
})

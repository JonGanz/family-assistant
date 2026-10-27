import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import Tabs from './Tabs.vue'

const tabs = [
  { key: 'a', label: 'A' },
  { key: 'b', label: 'B' },
  { key: 'c', label: 'C' },
] as const

describe('Tabs', () => {
  it('uses a roving tabindex and arrow keys', async () => {
    const w = mount(Tabs, {
      props: {
        tabs,
        modelValue: 'a',
        'onUpdate:modelValue': (v: string) => w.setProps({ modelValue: v }),
      },
      attachTo: document.body,
    })
    const idx = () => w.findAll('[role=tab]').map((t) => t.attributes('tabindex'))
    expect(idx()).toEqual(['0', '-1', '-1'])
    await w.get('[role=tablist]').trigger('keydown', { key: 'ArrowRight' })
    expect(idx()).toEqual(['-1', '0', '-1'])
    await w.get('[role=tablist]').trigger('keydown', { key: 'ArrowLeft' })
    await w.get('[role=tablist]').trigger('keydown', { key: 'ArrowLeft' })
    expect(idx()).toEqual(['-1', '-1', '0'])
    await w.get('[role=tablist]').trigger('keydown', { key: 'Home' })
    expect(idx()).toEqual(['0', '-1', '-1'])
    w.unmount()
  })
})

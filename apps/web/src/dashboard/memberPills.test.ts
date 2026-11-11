import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import MemberPills from './MemberPills.vue'

describe('MemberPills', () => {
  it('renders rounded app-variant chips and nothing when empty', () => {
    const w = mount(MemberPills, {
      props: { members: [{ id: 'a', name: 'Ann', color: '#336699' }] },
    })
    expect(w.find('.member-chip--app').text()).toBe('Ann')
    expect(
      mount(MemberPills, { props: { members: [] } })
        .find('.member-pills')
        .exists(),
    ).toBe(false)
  })
})

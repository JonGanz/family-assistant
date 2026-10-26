import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PageHeader from './PageHeader.vue'
import StateBlock from './StateBlock.vue'

const RouterLink = { props: ['to'], template: '<a :href="String(to)"><slot /></a>' }

describe('PageHeader', () => {
  it('renders title, subtitle, eyebrow and actions', () => {
    const w = mount(PageHeader, {
      props: { title: 'Bills', subtitle: 'Unpaid first', eyebrow: 'Settings' },
      slots: { actions: '<button>Refresh</button>' },
      global: { stubs: { RouterLink } },
    })
    expect(w.find('h1').text()).toBe('Bills')
    expect(w.find('.page-subtitle').text()).toBe('Unpaid first')
    expect(w.find('.page-eyebrow').text()).toBe('Settings')
    expect(w.find('.page-actions button').text()).toBe('Refresh')
    expect(w.find('.page-back').exists()).toBe(false)
  })

  it('renders a back link when given', () => {
    const w = mount(PageHeader, {
      props: { title: 'Task', back: '/tasks' },
      global: { stubs: { RouterLink } },
    })
    expect(w.find('.page-back').attributes('href')).toBe('/tasks')
    expect(w.find('.page-actions').exists()).toBe(false)
  })
})

describe('StateBlock', () => {
  it('shows loading, then empty, and never both', () => {
    const w = mount(StateBlock, { props: { loading: true, empty: true, emptyText: 'No bills.' } })
    expect(w.find('[role="status"]').text()).toBe('Loading…')
    expect(w.text()).not.toContain('No bills.')
  })

  it('shows the empty text once loaded and an error banner when set', async () => {
    const w = mount(StateBlock, { props: { loading: false, empty: true, emptyText: 'No bills.' } })
    expect(w.text()).toContain('No bills.')
    await w.setProps({ error: 'boom' })
    expect(w.find('[role="alert"]').text()).toBe('boom')
    expect(w.text()).not.toContain('No bills.')
  })
})

import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useInboxStore } from '../stores/inbox'
import { useMailStore } from '../stores/mail'
import { resetSnackbar, useSnackbar } from '../ui/composables/useSnackbar'
import AiChip from '../components/mail/AiChip.vue'
import DecisionFix from '../components/mail/DecisionFix.vue'
import BillItem from './parts/inbox/BillItem.vue'
import InboxView from './InboxView.vue'
import LabelView from './LabelView.vue'
import MailView from './MailView.vue'

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
  useRoute: () => ({ query: {}, params: {} }),
  RouterLink: { props: ['to'], template: '<a><slot /></a>' },
  onBeforeRouteLeave: vi.fn(),
}))
vi.mock('../api/client', () => ({
  api: {
    messages: vi.fn(),
    categories: vi.fn(),
    mailSources: vi.fn(),
    members: vi.fn(),
    household: vi.fn(),
    inbox: vi.fn(),
    approveInbox: vi.fn(),
    rejectInbox: vi.fn(),
    calendars: vi.fn(),
    sources: vi.fn(),
    labellingNext: vi.fn(),
    setEvalSample: vi.fn(),
    labelMessage: vi.fn(),
  },
}))
import { api } from '../api/client'
const m = vi.mocked(api)

const stubs = { RouterLink: { props: ['to'], template: '<a><slot /></a>' } }

beforeEach(() => {
  setActivePinia(createPinia())
  vi.resetAllMocks()
  resetSnackbar()
  m.members.mockResolvedValue([] as never)
  m.categories.mockResolvedValue([] as never)
  m.mailSources.mockResolvedValue([] as never)
  m.calendars.mockResolvedValue([] as never)
  m.sources.mockResolvedValue([] as never)
})

describe('Mail filters', () => {
  it('selects a category chip and returns to All on a second tap', async () => {
    m.messages.mockResolvedValue({ items: [], next_before: null } as never)
    m.categories.mockResolvedValue([{ id: 'c1', key: 'school', label: 'School' }] as never)
    const w = mount(MailView, { global: { stubs } })
    await flushPromises()
    const chip = () => w.findAll('[aria-label="Category"] button')[1]!
    expect(chip().attributes('aria-pressed')).toBe('false')
    await chip().trigger('click')
    expect(useMailStore().filters.category).toEqual(['school'])
    expect(chip().attributes('aria-pressed')).toBe('true')
    await chip().trigger('click')
    expect(useMailStore().filters.category).toEqual([])
  })
})

describe('Inbox', () => {
  const bill = {
    id: 'b1',
    payee: 'Water Co',
    amount_minor: 4200,
    currency: 'USD',
    due_on: null,
    confidence: 0.9,
    message: null,
  }
  it('shows counts in the tabs and confirms before rejecting', async () => {
    m.inbox.mockResolvedValue({
      items: [{ kind: 'bill', bill }],
      counts: { event: 0, bill: 1, task: 0, contact: 0, total: 1 },
      next_before: null,
    } as never)
    m.rejectInbox.mockResolvedValue(undefined as never)
    const w = mount(InboxView, { global: { stubs }, attachTo: document.body })
    await flushPromises()
    expect(w.text()).toContain('Bills (1)')
    await w.get('[role="radio"]:nth-child(2), [role="tab"]:nth-child(2)').trigger('click')
    const reject = w.findAll('button').find((b) => b.text() === 'Reject')!
    await reject.trigger('click')
    expect(m.rejectInbox).not.toHaveBeenCalled()
    const dialogReject = [...document.body.querySelectorAll('button')].filter(
      (b) => b.textContent?.trim() === 'Reject',
    )
    dialogReject[dialogReject.length - 1]!.click()
    await flushPromises()
    expect(m.rejectInbox).toHaveBeenCalledWith('bill', 'b1')
    expect(useInboxStore().items).toHaveLength(0)
    w.unmount()
  })
  it('approves with only the edited fields', async () => {
    const w = mount(BillItem, { props: { bill: bill as never }, global: { stubs } })
    await w.get('input').setValue('Gas Co')
    await w
      .findAll('button')
      .find((b) => b.text() === 'Approve')!
      .trigger('click')
    expect(w.emitted('approve')![0]).toEqual([{ payee: 'Gas Co' }])
  })
})

describe('Labelling', () => {
  it('toggles the evaluation sample instantly with an Undo', async () => {
    const message = {
      summary: {
        id: 'x1',
        subject: 'Hello',
        from_addr: 'a@example.com',
        received_at: '2026-01-01T10:00:00Z',
        in_eval_sample: false,
        members: [],
      },
      decisions: [],
      body_text: 'hi',
    }
    m.labellingNext.mockResolvedValue({
      message,
      progress: { labelled: 1, target: 10 },
    } as never)
    m.setEvalSample.mockResolvedValue(undefined as never)
    const w = mount(LabelView, { global: { stubs } })
    await flushPromises()
    const sw = () => w.get('[role="switch"]')
    expect(sw().attributes('aria-checked')).toBe('false')
    await sw().trigger('click')
    await flushPromises()
    expect(m.setEvalSample).toHaveBeenCalledWith('x1', true)
    const snack = useSnackbar().state.current
    expect(snack?.action?.label).toBe('Undo')
    await snack!.action!.run()
    expect(m.setEvalSample).toHaveBeenLastCalledWith('x1', false)
  })
})

describe('AI provenance', () => {
  const mk = (id: string, by: 'rules' | 'llm', confidence: number) => ({
    kind: 'bill',
    bill: {
      id,
      payee: `P-${id}`,
      amount_minor: 100,
      currency: 'USD',
      due_on: null,
      confidence,
      message: null,
      extracted_by: by,
    },
  })
  async function inboxWith() {
    m.inbox.mockResolvedValue({
      items: [mk('b1', 'llm', 0.824), mk('b2', 'rules', 0.9)],
      counts: { event: 0, bill: 2, task: 0, contact: 0, total: 2 },
      next_before: null,
    } as never)
    const w = mount(InboxView, { global: { stubs }, attachTo: document.body })
    await flushPromises()
    await w.get('[role="radio"]:nth-child(2), [role="tab"]:nth-child(2)').trigger('click')
    return w
  }
  it('shows the chip with a rounded percentage only on llm items, and filters', async () => {
    const w = await inboxWith()
    expect(w.findAll('.ai-chip')).toHaveLength(1)
    expect(w.get('.ai-chip').text()).toContain('AI · 82%')
    expect(w.findAll('li.item')).toHaveLength(2)
    await w.get('[aria-label="Filter"] [role="checkbox"]').trigger('click')
    expect(w.findAll('li.item')).toHaveLength(1)
    w.unmount()
  })
  it('has an accessible name', () => {
    const w = mount(AiChip, { props: { confidence: 0.82 } })
    expect(w.attributes('aria-label')).toBe('Suggested by the local language model, 82% confident')
  })
  it('shows the chip on an llm decision only', () => {
    const base = { decision_point: 'email.category', value: 'school', confidence: 0.7 }
    const props = { point: 'email.category', categories: [], members: [] }
    const llm = mount(DecisionFix, {
      props: { ...props, decisions: [{ ...base, stage: 'llm', is_final: true }] } as never,
    })
    expect(llm.find('.ai-chip').exists()).toBe(true)
    const rules = mount(DecisionFix, {
      props: { ...props, decisions: [{ ...base, stage: 'rules', is_final: true }] } as never,
    })
    expect(rules.find('.ai-chip').exists()).toBe(false)
  })
})

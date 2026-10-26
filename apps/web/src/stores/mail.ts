import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  api,
  type CategoryView,
  type LabelInput,
  type LabelQueueItem,
  type MailActionInput,
  type MailSourceView,
  type MakeTaskInput,
  type MessageDetail,
  type MessageQuery,
  type MessageSummary,
} from '../api/client'

export interface MailFilters {
  category: string
  member: string
  source: string
  q: string
  includeIgnored: boolean
}

export const emptyFilters = (): MailFilters => ({
  category: '',
  member: '',
  source: '',
  q: '',
  includeIgnored: false,
})

const msg = (e: unknown) => (e instanceof Error ? e.message : String(e))

export function filtersToQuery(f: MailFilters, before?: string): MessageQuery {
  return {
    category: f.category || undefined,
    member: f.member || undefined,
    source: f.source || undefined,
    q: f.q.trim() || undefined,
    include_ignored: f.includeIgnored || undefined,
    before,
  }
}

export const useMailStore = defineStore('mail', () => {
  const items = ref<MessageSummary[]>([])
  const nextBefore = ref<string | null>(null)
  const filters = ref<MailFilters>(emptyFilters())
  const categories = ref<CategoryView[]>([])
  const sources = ref<MailSourceView[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  const detail = ref<MessageDetail | null>(null)
  const detailError = ref<string | null>(null)

  const label = ref<LabelQueueItem | null>(null)
  const labelEmpty = ref(false)
  const labelError = ref<string | null>(null)

  async function loadFilters(): Promise<void> {
    try {
      const [c, s] = await Promise.all([api.categories(), api.mailSources()])
      categories.value = c
      sources.value = s
    } catch (e) {
      error.value = msg(e)
    }
  }

  async function load(): Promise<void> {
    loading.value = true
    try {
      const page = await api.messages(filtersToQuery(filters.value))
      items.value = page.items
      nextBefore.value = page.next_before ?? null
      error.value = null
    } catch (e) {
      error.value = msg(e)
    } finally {
      loading.value = false
    }
  }

  async function more(): Promise<void> {
    if (!nextBefore.value || loading.value) return
    loading.value = true
    try {
      const page = await api.messages(filtersToQuery(filters.value, nextBefore.value))
      items.value = [...items.value, ...page.items]
      nextBefore.value = page.next_before ?? null
    } catch (e) {
      error.value = msg(e)
    } finally {
      loading.value = false
    }
  }

  async function loadDetail(id: string, member?: string): Promise<void> {
    try {
      detail.value = await api.message(id, member)
      detailError.value = null
    } catch (e) {
      detail.value = null
      detailError.value = msg(e)
    }
  }

  async function act(id: string, b: MailActionInput, member?: string): Promise<void> {
    await api.messageAction(id, b)
    await loadDetail(id, member)
  }

  async function undo(id: string, actionId: string, member?: string): Promise<void> {
    await api.undoMailAction(actionId)
    await loadDetail(id, member)
  }

  /** One-tap fix of a decision point; the server answers with the refreshed message. */
  async function fix(id: string, b: LabelInput): Promise<void> {
    detail.value = await api.labelMessage(id, b)
  }

  async function makeTask(id: string, b: MakeTaskInput = {}, member?: string): Promise<void> {
    await api.makeTask(id, b)
    await loadDetail(id, member)
  }

  async function nextToLabel(after?: string): Promise<void> {
    try {
      const r = await api.labellingNext(after)
      label.value = r ?? null
      labelEmpty.value = !r
      labelError.value = null
    } catch (e) {
      labelError.value = msg(e)
    }
  }

  /** Records one decision point; keeps the refreshed message in the labelling queue item. */
  async function labelPoint(b: LabelInput): Promise<void> {
    const cur = label.value
    if (!cur) return
    const d = await api.labelMessage(cur.message.summary.id, b)
    label.value = { ...cur, message: d }
  }

  async function setEvalSample(id: string, on: boolean): Promise<void> {
    await api.setEvalSample(id, on)
    const cur = label.value
    if (cur && cur.message.summary.id === id) {
      label.value = {
        ...cur,
        message: { ...cur.message, summary: { ...cur.message.summary, in_eval_sample: on } },
      }
    }
  }

  return {
    items,
    nextBefore,
    filters,
    categories,
    sources,
    loading,
    error,
    detail,
    detailError,
    label,
    labelEmpty,
    labelError,
    loadFilters,
    load,
    more,
    loadDetail,
    act,
    undo,
    fix,
    makeTask,
    nextToLabel,
    labelPoint,
    setEvalSample,
  }
})

import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  mailApi,
  type CategoryInput,
  type CategoryPatch,
  type CategoryView,
  type MailRuleInput,
  type MailRulePatch,
  type MailRuleView,
  type MailSourcePatch,
  type MailSourceView,
  type NewMailSource,
  type SignalView,
} from '../api/mailSettings'
import { isConflict } from '../api/http'

export const useMailSettingsStore = defineStore('mailSettings', () => {
  const sources = ref<MailSourceView[]>([])
  const categories = ref<CategoryView[]>([])
  const rules = ref<MailRuleView[]>([])
  const signals = ref<SignalView[]>([])
  const error = ref<string | null>(null)
  const conflict = ref(false)
  const loaded = ref(false)

  async function load(): Promise<void> {
    try {
      const [s, c, r, g] = await Promise.all([
        mailApi.sources(),
        mailApi.categories(),
        mailApi.rules(),
        mailApi.signals(),
      ])
      sources.value = s
      categories.value = c
      rules.value = r
      signals.value = g
      error.value = null
      conflict.value = false
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loaded.value = true
    }
  }

  /** Runs a versioned write; a 409/428 sets `conflict` and returns false. */
  async function guarded(fn: () => Promise<unknown>): Promise<boolean> {
    try {
      await fn()
      conflict.value = false
      await load()
      return true
    } catch (e) {
      if (isConflict(e)) {
        conflict.value = true
        return false
      }
      throw e
    }
  }

  const createSource = (b: NewMailSource) => guarded(() => mailApi.createSource(b))
  const patchSource = (s: MailSourceView, p: MailSourcePatch) =>
    guarded(() => mailApi.patchSource(s.id, s.version, p))
  const removeSource = (id: string) => guarded(() => mailApi.deleteSource(id))
  const pause = (s: MailSourceView) => patchSource(s, { state: 'paused' })
  const resume = (s: MailSourceView) => patchSource(s, { state: 'ok' })
  async function syncSource(id: string): Promise<void> {
    await mailApi.syncSource(id)
  }

  const createCategory = (b: CategoryInput) => guarded(() => mailApi.createCategory(b))
  const patchCategory = (c: CategoryView, p: CategoryPatch) =>
    guarded(() => mailApi.patchCategory(c.id, c.version, p))
  const archiveCategory = (c: CategoryView) => patchCategory(c, { archived: true })

  const createRule = (b: MailRuleInput) => guarded(() => mailApi.createRule(b))
  const patchRule = (r: MailRuleView, p: MailRulePatch) =>
    guarded(() => mailApi.patchRule(r.id, r.version, p))
  const removeRule = (id: string) => guarded(() => mailApi.deleteRule(id))

  const setSignal = (key: string, enabled: boolean) =>
    guarded(() => mailApi.patchSignal(key, enabled))

  return {
    sources,
    categories,
    rules,
    signals,
    error,
    conflict,
    loaded,
    load,
    createSource,
    patchSource,
    removeSource,
    pause,
    resume,
    syncSource,
    createCategory,
    patchCategory,
    archiveCategory,
    createRule,
    patchRule,
    removeRule,
    setSignal,
  }
})

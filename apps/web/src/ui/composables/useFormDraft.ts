import { computed, getCurrentInstance, onScopeDispose, ref, watch } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'

/** DTOs are JSON, so a JSON round trip is a safe deep clone (it also strips reactive proxies). */
function clone<T>(v: T): T {
  return v === undefined ? v : (JSON.parse(JSON.stringify(v)) as T)
}

export function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true
  if (typeof a !== 'object' || typeof b !== 'object' || !a || !b) return false
  if (Array.isArray(a) !== Array.isArray(b)) return false
  const ka = Object.keys(a)
  const kb = Object.keys(b)
  return (
    ka.length === kb.length &&
    ka.every((k) => deepEqual((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k]))
  )
}

export interface FormDraftOptions {
  /** Ask before leaving with unsaved changes; default `window.confirm`. */
  confirmLeave?: () => boolean | Promise<boolean>
}

/**
 * Multi-field forms (D-096): edit a deep copy, show the Save/Discard bar while it differs from
 * the source, and guard leaving (router and tab close) while dirty. A change of the source
 * replaces the draft only while the draft is clean.
 */
export function useFormDraft<T>(
  source: () => T,
  save: (draft: T) => Promise<void>,
  opts: FormDraftOptions = {},
) {
  const base = ref(clone(source())) as { value: T }
  const draft = ref(clone(source())) as { value: T }
  const saving = ref(false)
  const error = ref<string | null>(null)
  const dirty = computed(() => !deepEqual(draft.value, base.value))

  watch(
    source,
    (v) => {
      if (!dirty.value) {
        base.value = clone(v)
        draft.value = clone(v)
      }
    },
    { deep: true },
  )

  async function doSave(): Promise<boolean> {
    if (!dirty.value || saving.value) return false
    saving.value = true
    error.value = null
    try {
      await save(clone(draft.value))
      base.value = clone(draft.value)
      return true
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Could not save'
      return false
    } finally {
      saving.value = false
    }
  }

  function discard() {
    draft.value = clone(base.value)
    error.value = null
  }

  const confirmLeave = opts.confirmLeave ?? (() => window.confirm('Discard unsaved changes?'))
  if (getCurrentInstance()) {
    onBeforeRouteLeave(async () => (dirty.value ? await confirmLeave() : true))
  }
  const beforeUnload = (e: BeforeUnloadEvent) => {
    e.preventDefault()
    e.returnValue = ''
  }
  watch(
    dirty,
    (d) => {
      if (typeof window === 'undefined') return
      if (d) window.addEventListener('beforeunload', beforeUnload)
      else window.removeEventListener('beforeunload', beforeUnload)
    },
    { immediate: true },
  )
  onScopeDispose(() => window.removeEventListener('beforeunload', beforeUnload))

  return { draft, dirty, saving, error, save: doSave, discard }
}

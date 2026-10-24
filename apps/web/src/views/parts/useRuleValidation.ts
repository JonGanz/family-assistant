import { onScopeDispose, ref, watch, type Ref } from 'vue'
import { api, type ValidateRuleResponse } from '../../api/client'

export const VALIDATE_DEBOUNCE_MS = 300

/** Validates the condition on the server 300 ms after the last keystroke; stale replies are dropped. */
export function useRuleValidation(condition: Ref<string>, message: Ref<string>) {
  const result = ref<ValidateRuleResponse | null>(null)
  const failure = ref('')
  const pending = ref(false)
  let timer: ReturnType<typeof setTimeout> | undefined
  let seq = 0

  async function run() {
    const mine = ++seq
    try {
      const r = await api.validateRule(condition.value, message.value || undefined)
      if (mine !== seq) return
      result.value = r
      failure.value = ''
    } catch (e) {
      if (mine !== seq) return
      result.value = null
      failure.value = e instanceof Error ? e.message : 'Could not check the rule.'
    } finally {
      if (mine === seq) pending.value = false
    }
  }

  watch([condition, message], () => {
    clearTimeout(timer)
    seq++
    if (!condition.value.trim()) {
      result.value = null
      failure.value = ''
      pending.value = false
      return
    }
    pending.value = true
    timer = setTimeout(run, VALIDATE_DEBOUNCE_MS)
  })

  onScopeDispose(() => clearTimeout(timer))
  return { result, failure, pending }
}

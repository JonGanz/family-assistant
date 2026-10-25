import { onScopeDispose, ref, watch, type Ref } from 'vue'
import { api, type ValidateDefinitionResponse } from '../../api/client'

export const DEFINITION_VALIDATE_DEBOUNCE_MS = 300

/** Validates the RRULE and season condition 300 ms after the last change; stale replies are dropped. */
export function useDefinitionValidation(
  rrule: Ref<string | null>,
  condition: Ref<string>,
  startDate: Ref<string>,
) {
  const result = ref<ValidateDefinitionResponse | null>(null)
  const failure = ref('')
  const pending = ref(false)
  let timer: ReturnType<typeof setTimeout> | undefined
  let seq = 0

  async function run() {
    const mine = ++seq
    try {
      const r = await api.validateDefinition({
        rrule: rrule.value?.trim() || null,
        season_condition: condition.value.trim() || null,
        start_date: startDate.value || null,
      })
      if (mine !== seq) return
      result.value = r
      failure.value = ''
    } catch (e) {
      if (mine !== seq) return
      result.value = null
      failure.value = e instanceof Error ? e.message : 'Could not check the schedule.'
    } finally {
      if (mine === seq) pending.value = false
    }
  }

  function schedule() {
    clearTimeout(timer)
    seq++
    if (!rrule.value?.trim() && !condition.value.trim()) {
      result.value = null
      failure.value = ''
      pending.value = false
      return
    }
    pending.value = true
    timer = setTimeout(run, DEFINITION_VALIDATE_DEBOUNCE_MS)
  }

  watch([rrule, condition, startDate], schedule, { immediate: true })
  onScopeDispose(() => clearTimeout(timer))
  return { result, failure, pending }
}

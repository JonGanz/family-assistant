import { onScopeDispose, ref } from 'vue'

export function useClock(tickMs = 1000) {
  const now = ref(new Date())
  const id = setInterval(() => {
    now.value = new Date()
  }, tickMs)
  onScopeDispose(() => clearInterval(id))
  return now
}

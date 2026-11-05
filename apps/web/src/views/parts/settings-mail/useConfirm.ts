import { ref } from 'vue'

export interface PendingConfirm {
  title: string
  text: string
  label: string
  run: () => unknown
}

/** One confirm dialog per page: `ask` opens it, `accept` runs the action and closes it. */
export function useConfirm() {
  const pending = ref<PendingConfirm | null>(null)
  function ask(title: string, text: string, run: () => unknown, label = 'Delete') {
    pending.value = { title, text, label, run }
  }
  async function accept() {
    const p = pending.value
    pending.value = null
    await p?.run()
  }
  function cancel() {
    pending.value = null
  }
  return { pending, ask, accept, cancel }
}

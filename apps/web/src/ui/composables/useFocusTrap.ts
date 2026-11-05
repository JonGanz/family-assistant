import { nextTick, onBeforeUnmount, watch, type Ref } from 'vue'

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

/** Moves focus into `el` when `open` becomes true, keeps Tab inside, and returns focus to the opener on close. */
export function useFocusTrap(
  open: Ref<boolean>,
  el: Ref<HTMLElement | null>,
  onEscape: () => void,
) {
  let opener: HTMLElement | null = null

  function onKey(e: KeyboardEvent) {
    const root = el.value
    if (!root) return
    if (e.key === 'Escape') {
      e.stopPropagation()
      onEscape()
      return
    }
    if (e.key !== 'Tab') return
    const items = [...root.querySelectorAll<HTMLElement>(FOCUSABLE)]
    if (!items.length) {
      e.preventDefault()
      root.focus()
      return
    }
    const first = items[0]!
    const last = items[items.length - 1]!
    const active = document.activeElement
    if (e.shiftKey && (active === first || active === root)) {
      e.preventDefault()
      last.focus()
    } else if (!e.shiftKey && active === last) {
      e.preventDefault()
      first.focus()
    }
  }

  watch(
    open,
    async (isOpen) => {
      if (isOpen) {
        opener = document.activeElement as HTMLElement | null
        document.addEventListener('keydown', onKey, true)
        await nextTick()
        const root = el.value
        ;(
          root?.querySelector<HTMLElement>('[autofocus]') ??
          root?.querySelector<HTMLElement>(FOCUSABLE) ??
          root
        )?.focus()
      } else {
        document.removeEventListener('keydown', onKey, true)
        opener?.focus?.()
        opener = null
      }
    },
    { immediate: true, flush: 'post' },
  )
  onBeforeUnmount(() => document.removeEventListener('keydown', onKey, true))
}

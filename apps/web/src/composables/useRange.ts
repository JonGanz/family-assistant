import { computed, ref } from 'vue'
import { addDays, dayKey } from '../lib/time'
import type { RangeMode } from '../lib/types'

/** Range of day keys; `to` is exclusive. */
export interface DayRange {
  from: string
  to: string
}

export function rangeFor(mode: RangeMode, anchor: string): DayRange {
  if (mode === 'day') return { from: anchor, to: addDays(anchor, 1) }
  if (mode === 'week') {
    const dow = new Date(`${anchor}T12:00:00Z`).getUTCDay()
    const from = addDays(anchor, -((dow + 6) % 7)) // Monday start
    return { from, to: addDays(from, 7) }
  }
  const [y, m] = anchor.split('-').map(Number)
  const next = m === 12 ? `${y! + 1}-01-01` : `${y}-${String(m! + 1).padStart(2, '0')}-01`
  return { from: `${anchor.slice(0, 7)}-01`, to: next }
}

export function shiftAnchor(mode: RangeMode, anchor: string, dir: 1 | -1): string {
  if (mode === 'day') return addDays(anchor, dir)
  if (mode === 'week') return addDays(anchor, 7 * dir)
  const [y, m] = anchor.split('-').map(Number)
  const idx = y! * 12 + (m! - 1) + dir
  return `${Math.floor(idx / 12)}-${String((idx % 12) + 1).padStart(2, '0')}-01`
}

export function useRange(tz: string, initial: RangeMode = 'week') {
  const mode = ref<RangeMode>(initial)
  const anchor = ref(dayKey(new Date(), tz))
  const range = computed(() => rangeFor(mode.value, anchor.value))
  return {
    mode,
    anchor,
    range,
    prev: () => (anchor.value = shiftAnchor(mode.value, anchor.value, -1)),
    next: () => (anchor.value = shiftAnchor(mode.value, anchor.value, 1)),
    today: () => (anchor.value = dayKey(new Date(), tz)),
  }
}

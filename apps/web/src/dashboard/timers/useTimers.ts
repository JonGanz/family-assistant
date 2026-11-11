import { computed, ref } from 'vue'
import {
  addTime,
  addTimer,
  anyRinging,
  cancel,
  deserialize,
  dismiss,
  pause,
  remove,
  resume,
  serialize,
  STORAGE_KEY,
  tick,
  type Timer,
} from './timersModel'
import { startAlarm, stopAlarm, unlockAudio } from './sound'

// One shared store per page: timers keep running (and ringing) while another view is open.
const timers = ref<Timer[]>([])
const now = ref(Date.now())
let started = false
let raf = 0
let interval: ReturnType<typeof setInterval> | undefined

function load() {
  try {
    timers.value = deserialize(localStorage.getItem(STORAGE_KEY), Date.now())
  } catch {
    timers.value = []
  }
}
function save() {
  try {
    localStorage.setItem(STORAGE_KEY, serialize(timers.value))
  } catch {
    /* storage unavailable */
  }
}
function sync() {
  const t = Date.now()
  now.value = t
  const next = tick(timers.value, t)
  if (next !== timers.value) {
    timers.value = next
    save()
  }
  if (anyRinging(timers.value)) startAlarm()
  else stopAlarm()
}
function frame() {
  sync()
  raf = requestAnimationFrame(frame)
}
function apply(next: Timer[]) {
  timers.value = next
  save()
  sync()
}

/** Starts the shared ticker on first use. rAF drives the ring; the interval covers hidden tabs. */
export function useTimers() {
  if (!started) {
    started = true
    load()
    sync()
    if (typeof requestAnimationFrame === 'function') raf = requestAnimationFrame(frame)
    interval = setInterval(sync, 500)
  }
  return {
    timers,
    now,
    ringing: computed(() => anyRinging(timers.value)),
    add(name: string, ms: number) {
      unlockAudio()
      apply(addTimer(timers.value, name, ms, Date.now()))
    },
    pause: (id: string) => apply(pause(timers.value, id, Date.now())),
    resume: (id: string) => {
      unlockAudio()
      apply(resume(timers.value, id, Date.now()))
    },
    addTime: (id: string) => apply(addTime(timers.value, id, Date.now())),
    cancel: (id: string) => apply(cancel(timers.value, id)),
    dismiss: (id: string) => apply(dismiss(timers.value, id)),
    remove: (id: string) => apply(remove(timers.value, id)),
  }
}

/** For tests. */
export function resetTimers() {
  if (raf && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(raf)
  if (interval) clearInterval(interval)
  stopAlarm()
  started = false
  timers.value = []
}

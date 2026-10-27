import { defineStore } from 'pinia'
import { reactive, ref } from 'vue'
import { emptyHousehold, emptyOwner } from '../views/setup/validate'
import { api, type SetupStatus } from '../api/client'

export const STEPS = [
  'Welcome',
  'Household',
  'You',
  'Family',
  'Access',
  'Master key',
  'Start',
] as const
export const LAST_STEP = STEPS.length

const KEY = 'fa.setup.step'

/** The step to resume at: the server decides how far back it can be, the stored step how far on. */
export function resumeStep(status: SetupStatus, stored: number): number {
  const s = Number.isInteger(stored) && stored >= 1 ? Math.min(stored, LAST_STEP) : 1
  if (status.needed) return Math.min(s, 2)
  if (status.owner_needed) return 3
  return Math.max(s, 4)
}

function readStored(): number {
  try {
    return Number(localStorage.getItem(KEY)) || 1
  } catch {
    return 1
  }
}

export const useSetupStore = defineStore('setup', () => {
  const status = ref<SetupStatus | null>(null)
  const step = ref(1)
  /** Wizard values, kept here so Back and Next never lose what was typed. */
  const household = reactive(emptyHousehold())
  const owner = reactive(emptyOwner())

  /** Reads `GET /setup`. A failure leaves the last known status (the app must not brick). */
  async function load(force = false): Promise<SetupStatus | null> {
    if (status.value && !force) return status.value
    try {
      status.value = await api.setup()
    } catch {
      /* keep what we had; the API answers 409 setup_required if it matters */
    }
    return status.value
  }

  /** Setup is still to be done: no household, no owner, or an adult has not finished the wizard. */
  const pending = () => !!status.value && (status.value.needed || status.value.owner_needed)

  function goto(n: number): void {
    step.value = Math.max(1, Math.min(LAST_STEP, n))
    try {
      localStorage.setItem(KEY, String(step.value))
    } catch {
      /* private window */
    }
  }

  function resume(): void {
    if (status.value) goto(resumeStep(status.value, readStored()))
  }

  function invalidate(): void {
    status.value = null
  }

  return { status, step, household, owner, load, pending, goto, resume, invalidate }
})

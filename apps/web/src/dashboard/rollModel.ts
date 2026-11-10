import { addDays } from '../lib/time'

/** Days shown before today and after it in the Food view's rolling plan (today is the 3rd). */
export const ROLL_BEFORE = 2
export const ROLL_AFTER = 4

/** The seven `YYYY-MM-DD` days of the rolling plan: two before `today`, today, then four after. */
export function rollingDays(today: string): string[] {
  return Array.from({ length: ROLL_BEFORE + 1 + ROLL_AFTER }, (_, i) =>
    addDays(today, i - ROLL_BEFORE),
  )
}

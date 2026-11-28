/** Weather-driven rain. WMO codes as in Open-Meteo: 51-57 drizzle, 61-67 rain, 80-82 showers,
 * 95-99 thunderstorms. Snow takes precedence (the canvas shows one or the other). */
import { weatherCodes } from './snowModel'

export type RainMode = 'none' | 'drizzle' | 'light' | 'moderate' | 'heavy'

export interface RainParams {
  /** Multiplier on the area-based drop count. */
  density: number
  /** Multiplier on fall speed. */
  speed: number
  /** Hard cap on drops, for a wall tablet. */
  max: number
}

export const RAIN_PARAMS: Record<Exclude<RainMode, 'none'>, RainParams> = {
  drizzle: { density: 0.6, speed: 0.8, max: 90 },
  light: { density: 0.9, speed: 1, max: 130 },
  moderate: { density: 1.5, speed: 1.2, max: 200 },
  heavy: { density: 2.4, speed: 1.5, max: 300 },
}

export function rainMode(code: number | null | undefined): RainMode {
  switch (code) {
    case 51:
    case 53:
    case 55:
    case 56:
    case 57:
      return 'drizzle'
    case 61:
    case 66:
    case 80:
      return 'light'
    case 63:
    case 67:
    case 81:
      return 'moderate'
    case 65:
    case 82:
    case 95:
    case 96:
    case 99:
      return 'heavy'
    default:
      return 'none'
  }
}

const RANK: RainMode[] = ['none', 'drizzle', 'light', 'moderate', 'heavy']

/** Rain mode when the current weather or today's forecast has rain (the stronger wins). */
export function rainModeOf(
  weather: { current?: unknown; daily?: unknown } | null | undefined,
): RainMode {
  return weatherCodes(weather)
    .map(rainMode)
    .reduce((a, b) => (RANK.indexOf(a) >= RANK.indexOf(b) ? a : b), 'none' as RainMode)
}

/** Weather-driven snow. WMO codes as in Open-Meteo: 71 slight, 73 moderate, 75 heavy snow fall,
 * 77 snow grains, 85 slight and 86 heavy snow showers. */
export type SnowMode = 'none' | 'light' | 'moderate' | 'heavy' | 'flurries'

export interface SnowParams {
  /** Multiplier on the area-based flake count. */
  density: number
  /** Multiplier on fall speed. */
  speed: number
  /** Multiplier on sideways sway. */
  drift: number
  /** Constant sideways gust in px/s (flurries only). */
  gust: number
  /** Hard cap on flakes, for a wall tablet. */
  max: number
}

export const SNOW_PARAMS: Record<Exclude<SnowMode, 'none'>, SnowParams> = {
  light: { density: 0.5, speed: 0.8, drift: 0.8, gust: 0, max: 80 },
  moderate: { density: 1, speed: 1, drift: 1, gust: 0, max: 140 },
  heavy: { density: 1.6, speed: 1.2, drift: 1.2, gust: 0, max: 200 },
  flurries: { density: 2.4, speed: 1.9, drift: 1.8, gust: 28, max: 260 },
}

export function snowMode(code: number | null | undefined): SnowMode {
  switch (code) {
    case 71:
    case 77:
      return 'light'
    case 73:
      return 'moderate'
    case 75:
      return 'heavy'
    case 85:
    case 86:
      return 'flurries'
    default:
      return 'none'
  }
}

const SNOW_RANK: SnowMode[] = ['none', 'light', 'moderate', 'heavy', 'flurries']
const stronger = (a: SnowMode, b: SnowMode): SnowMode =>
  SNOW_RANK.indexOf(a) >= SNOW_RANK.indexOf(b) ? a : b

/** The first value of a column-oriented block (`{ weather_code: [..] }`), when it is a number. */
export function firstCode(block: unknown): number | undefined {
  const col = (block as Record<string, unknown> | null | undefined)?.['weather_code']
  const v = Array.isArray(col) ? col[0] : undefined
  return typeof v === 'number' ? v : undefined
}

/** The codes that decide the effects: the current reading and today's daily forecast code. */
export function weatherCodes(
  weather: { current?: unknown; daily?: unknown } | null | undefined,
): number[] {
  const c = weather?.current as Record<string, unknown> | null | undefined
  const cur = c?.['weather_code']
  return [typeof cur === 'number' ? cur : undefined, firstCode(weather?.daily)].filter(
    (x): x is number => x !== undefined,
  )
}

/** Snow mode when the current weather or today's forecast has snow (the stronger wins). */
export function snowModeOf(
  weather: { current?: unknown; daily?: unknown } | null | undefined,
): SnowMode {
  return weatherCodes(weather).map(snowMode).reduce(stronger, 'none')
}

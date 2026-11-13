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

/** Snow mode for the current weather; no current reading means no snow. */
export function snowModeOf(weather: { current?: unknown } | null | undefined): SnowMode {
  const c = weather?.current as Record<string, unknown> | null | undefined
  const code = c?.['weather_code']
  return snowMode(typeof code === 'number' ? code : undefined)
}

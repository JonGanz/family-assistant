export type ThemeSetting = 'auto' | 'light' | 'dark'

/**
 * `auto` is dark between sunset and sunrise (from the weather snapshot). Without sun times it
 * falls back to the browser's colour scheme.
 */
export function resolveTheme(
  theme: ThemeSetting,
  now: Date,
  sunrise?: string | null,
  sunset?: string | null,
  prefersDark = false,
): 'light' | 'dark' {
  if (theme !== 'auto') return theme
  if (!sunrise || !sunset) return prefersDark ? 'dark' : 'light'
  const t = now.getTime()
  const up = Date.parse(sunrise)
  const down = Date.parse(sunset)
  if (Number.isNaN(up) || Number.isNaN(down)) return prefersDark ? 'dark' : 'light'
  // Times are today's: dark before sunrise or from sunset on.
  return t < up || t >= down ? 'dark' : 'light'
}

export function prefersDark(): boolean {
  try {
    return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false
  } catch {
    return false
  }
}

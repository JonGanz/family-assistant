export type Density = 'comfortable' | 'compact'

export const DENSITY_KEY = 'fa.density'

/** The device's density; comfortable (the Material 3 sizes) unless the user chose compact. */
export function getDensity(): Density {
  try {
    return localStorage.getItem(DENSITY_KEY) === 'compact' ? 'compact' : 'comfortable'
  } catch {
    return 'comfortable'
  }
}

export function applyDensity(
  density: Density = getDensity(),
  root: HTMLElement = document.documentElement,
) {
  root.setAttribute('data-density', density)
}

/** Stores the choice for this device and applies it at once. */
export function setDensity(density: Density) {
  try {
    localStorage.setItem(DENSITY_KEY, density)
  } catch {
    // Storage can be blocked; the choice then lasts until reload.
  }
  applyDensity(density)
}

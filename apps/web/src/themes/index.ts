import { ref, watch } from 'vue'
import type { Ref } from 'vue'
import { FAMILIES } from './families'
import { HUES } from './types'
import type { ThemeChoice, ThemeFamily, ThemeVariant } from './types'

export { FAMILIES }
export { HUES } from './types'
export type { Hue, Mode, ThemeChoice, ThemeFamily, ThemeVariant } from './types'

export const DEFAULT_CHOICE: ThemeChoice = { family: 'catppuccin', mode: 'auto' }
const STORAGE_KEY = 'fa.theme'

export function familyOf(id: string): ThemeFamily {
  return (
    FAMILIES.find((f) => f.id === id) ??
    FAMILIES.find((f) => f.id === DEFAULT_CHOICE.family) ??
    FAMILIES[0]!
  )
}

export function resolveVariant(
  choice: ThemeChoice,
  ctx: { prefersDark: boolean; night?: boolean },
): ThemeVariant {
  const family = familyOf(choice.family)
  const modes = new Set(family.variants.map((v) => v.mode))
  let mode: 'light' | 'dark'
  if (modes.size === 1) mode = family.variants[0]!.mode
  else if (choice.mode === 'auto') mode = (ctx.night ?? ctx.prefersDark) ? 'dark' : 'light'
  else mode = choice.mode
  const ofMode = family.variants.filter((v) => v.mode === mode)
  return ofMode.find((v) => v.id === choice.variant) ?? ofMode[0] ?? family.variants[0]!
}

export function applyTheme(
  variant: ThemeVariant,
  el: HTMLElement = document.documentElement,
): void {
  const n = variant.neutrals
  const set = (k: string, v: string) => el.style.setProperty(k, v)
  set('--color-bg', n.bg)
  set('--color-surface', n.surface)
  set('--color-surface-2', n.surface2)
  set('--color-border', n.border)
  set('--color-muted', n.muted)
  set('--color-text', n.text)
  set('--color-on-accent', n.onAccent)
  for (const h of HUES) set(`--hue-${h}`, variant.hues[h])
  set('--color-accent', variant.hues.blue)
  set('--color-focus', variant.hues.blue)
  set('--color-danger', variant.hues.red)
  set('--color-warning', variant.hues.yellow)
  set('--color-success', variant.hues.green)
  el.style.colorScheme = variant.mode
  el.setAttribute('data-theme-mode', variant.mode)
}

export function loadDeviceChoice(): ThemeChoice {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return { ...DEFAULT_CHOICE }
    const v = JSON.parse(raw) as Partial<ThemeChoice> | null
    if (!v || typeof v !== 'object' || typeof v.family !== 'string') return { ...DEFAULT_CHOICE }
    const mode = v.mode === 'light' || v.mode === 'dark' ? v.mode : 'auto'
    return {
      family: v.family,
      mode,
      ...(typeof v.variant === 'string' ? { variant: v.variant } : {}),
    }
  } catch {
    return { ...DEFAULT_CHOICE }
  }
}

export function saveDeviceChoice(choice: ThemeChoice): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(choice))
  } catch {
    /* storage unavailable: the choice lasts for this session only */
  }
}

function systemPrefersDark(): boolean {
  try {
    return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false
  } catch {
    return false
  }
}

/**
 * Holds this device's theme choice, applies it to the document and follows the OS scheme.
 * `enabled` lets the caller skip applying (the kiosk shell takes its theme from device settings).
 */
interface DeviceThemeState {
  choice: Ref<ThemeChoice>
  dark: Ref<boolean>
}
let shared: DeviceThemeState | undefined

/** One choice per page, shared by every caller (the shell and the Appearance page). */
function deviceThemeState(): DeviceThemeState {
  if (shared) return shared
  const choice = ref<ThemeChoice>(loadDeviceChoice())
  const dark = ref(systemPrefersDark())
  try {
    window
      .matchMedia?.('(prefers-color-scheme: dark)')
      ?.addEventListener?.('change', (e) => (dark.value = e.matches))
  } catch {
    // no matchMedia: the scheme just never changes
  }
  watch(choice, (c) => saveDeviceChoice(c), { deep: true })
  shared = { choice, dark }
  return shared
}

/** For tests: forget the shared state so the next caller reloads it from storage. */
export function resetDeviceTheme() {
  shared = undefined
}

export function useDeviceTheme(enabled: () => boolean = () => true) {
  const { choice, dark } = deviceThemeState()
  const apply = () => {
    if (enabled()) applyTheme(resolveVariant(choice.value, { prefersDark: dark.value }))
  }
  apply()
  watch([choice, dark, enabled], apply, { deep: true })

  function setChoice(c: ThemeChoice) {
    choice.value = c
  }
  return { choice, setChoice }
}

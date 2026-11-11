import { ref, watch } from 'vue'
import type { Ref } from 'vue'
import { FAMILIES } from './families'
import { HUES } from './types'
import { contrastRatio } from '../lib/contrast'
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

function pickVariant(
  familyId: string,
  variantId: string | undefined,
  want: 'auto' | 'light' | 'dark',
  autoDark: boolean,
): ThemeVariant {
  const family = familyOf(familyId)
  const modes = new Set(family.variants.map((v) => v.mode))
  let mode: 'light' | 'dark'
  if (modes.size === 1) mode = family.variants[0]!.mode
  else if (want === 'auto') mode = autoDark ? 'dark' : 'light'
  else mode = want
  const ofMode = family.variants.filter((v) => v.mode === mode)
  return ofMode.find((v) => v.id === variantId) ?? ofMode[0] ?? family.variants[0]!
}

/** True when the choice asks for dark right now (night or system dark under `auto`). */
export function wantsDark(
  choice: ThemeChoice,
  ctx: { prefersDark: boolean; night?: boolean },
): boolean {
  return choice.mode === 'dark' || (choice.mode === 'auto' && (ctx.night ?? ctx.prefersDark))
}

export function resolveVariant(
  choice: ThemeChoice,
  ctx: { prefersDark: boolean; night?: boolean },
): ThemeVariant {
  const dark = wantsDark(choice, ctx)
  if (choice.dark) {
    return dark
      ? pickVariant(choice.dark.family, choice.dark.variant, 'dark', true)
      : pickVariant(choice.family, choice.variant, 'light', false)
  }
  return pickVariant(choice.family, choice.variant, choice.mode, ctx.night ?? ctx.prefersDark)
}

function mixHex(a: string, b: string, pctB: number): string {
  const ch = (h: string, i: number) => parseInt(h.slice(1 + i * 2, 3 + i * 2), 16)
  const out = [0, 1, 2].map((i) =>
    Math.round(ch(a, i) * (1 - pctB / 100) + ch(b, i) * (pctB / 100)),
  )
  return '#' + out.map((v) => v.toString(16).padStart(2, '0')).join('')
}

/** The layer, text and selection slots of a variant, with derived values where it has no `extended`. */
export function resolveExtended(variant: ThemeVariant) {
  const n = variant.neutrals
  const x = variant.extended
  const mix = variant.layerMix ?? [5, 10, 15]
  const raw: [string, string, string, string] = x?.layers ?? [
    n.bg,
    mixHex(n.bg, n.text, mix[0]),
    mixHex(n.bg, n.text, mix[1]),
    mixHex(n.bg, n.text, mix[2]),
  ]
  // A published layer that body text cannot read on (R-006: Frappe, Macchiato, Dragon, Solarized
  // dark, Tokyo Night Day, layer 3) is pulled back toward the page until `text` reaches 4.5:1.
  const layers = raw.map((l) => {
    let out = l
    for (let p = 10; p <= 100 && contrastRatio(n.text, out) < 4.5; p += 10) out = mixHex(l, n.bg, p)
    return out
  }) as [string, string, string, string]
  const textSubtle = x?.textSubtle ?? n.muted
  // secondary text per layer: textSubtle, else muted, else text, whichever first reaches 4.5:1
  const onLayerSubtle = layers.map(
    (l) => [textSubtle, n.muted, n.text].find((c) => contrastRatio(c, l) >= 4.5) ?? n.text,
  )
  return {
    layers,
    textStrong: x?.textStrong ?? n.text,
    textSubtle,
    comment: x?.comment ?? n.muted,
    selection: x?.selection ?? mixHex(n.bg, variant.hues.blue, 25),
    highlight: x?.highlight ?? mixHex(n.bg, n.text, 6),
    onLayerSubtle,
  }
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
  const x = resolveExtended(variant)
  x.layers.forEach((l, i) => set(`--color-layer-${i}`, l))
  x.onLayerSubtle.forEach((c, i) => set(`--color-on-layer-${i}-subtle`, c))
  set('--color-text-strong', x.textStrong)
  set('--color-text-subtle', x.textSubtle)
  set('--color-comment', x.comment)
  set('--color-selection', x.selection)
  set('--color-highlight', x.highlight)
  for (const h of HUES) set(`--hue-${h}`, variant.hues[h])
  set('--color-accent', variant.hues.blue)
  set('--color-focus', variant.hues.blue)
  set('--color-danger', variant.hues.red)
  set('--color-warning', variant.hues.yellow)
  set('--color-success', variant.hues.green)
  el.style.colorScheme = variant.mode
  el.setAttribute('data-theme-mode', variant.mode)
}

export function normalizeDark(raw: unknown): { dark?: NonNullable<ThemeChoice['dark']> } {
  const d = raw as { family?: unknown; variant?: unknown } | null | undefined
  if (!d || typeof d !== 'object' || typeof d.family !== 'string') return {}
  return {
    dark: {
      family: d.family,
      ...(typeof d.variant === 'string' ? { variant: d.variant } : {}),
    },
  }
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
      ...normalizeDark(v.dark),
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

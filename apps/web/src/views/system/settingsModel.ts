import type { SettingView } from '../../api/client'

export interface SettingMeta {
  label: string
  help: string
  /** The change only takes effect after the server restarts its listeners. */
  restart?: boolean
}

/** Labels and help for the runtime keys (fa_config::runtime). Unknown keys show the key itself. */
export const SETTING_META: Record<string, SettingMeta> = {
  public_base_url: {
    label: 'Public address',
    help: 'How the family reaches this server (used in pairing links and sign-in redirects).',
  },
  tailnet_base_url: {
    label: 'Tailscale address',
    help: 'The HTTPS address on your tailnet, when the Tailscale profile is on.',
  },
  ntfy_url: { label: 'Notifications server (ntfy)', help: 'Where push notifications are sent.' },
  ntfy_token: { label: 'Notifications token', help: 'Only needed for a protected ntfy topic.' },
  ntfy_public_url: {
    label: 'Notifications address for phones',
    help: 'What phones subscribe to. Empty: the tailnet address on :8443, else this server on :8090.',
  },
  google_client_id: { label: 'Google client ID', help: 'For Google calendar and mail sign-in.' },
  google_client_secret: { label: 'Google client secret', help: 'Stored encrypted.' },
  allow_http_feeds: {
    label: 'Allow plain-HTTP feeds',
    help: 'Let calendar feeds and weather use http:// addresses.',
  },
  allow_private_hosts: {
    label: 'Allow feeds on private addresses',
    help: 'Let feeds point at machines on your home network.',
  },
  trusted_proxies: {
    label: 'Trusted proxies',
    help: 'Networks whose X-Forwarded-* headers are believed, one per line (CIDR).',
    restart: true,
  },
  lan_dashboard_url: {
    label: 'Home-network dashboard address',
    help: 'The address wall screens on your home network use.',
    restart: true,
  },
  lan_allowed_cidrs: {
    label: 'Home-network dashboard: allowed networks',
    help: 'One per line (CIDR). Empty means private and loopback addresses.',
    restart: true,
  },
  lan_allow_pin: {
    label: 'Home-network dashboard: allow PIN elevation',
    help: 'Let the wall screen ask for a PIN over the home-network listener.',
    restart: true,
  },
}

export const metaFor = (key: string): SettingMeta => SETTING_META[key] ?? { label: key, help: '' }

export interface Draft {
  /** Text, bool or list-as-text, according to the setting kind. */
  value: string | boolean
  /** Secrets only: remove the stored value. */
  clear: boolean
}

export const splitList = (s: string): string[] =>
  s
    .split(/[\n,]/)
    .map((x) => x.trim())
    .filter(Boolean)

/** The generated type says `object | null`; JSON values are really any shape. */
const valueOf = (s: SettingView): unknown => s.value

const asList = (v: unknown): string[] => (Array.isArray(v) ? v.map(String) : [])

export function draftFrom(s: SettingView): Draft {
  switch (s.kind) {
    case 'bool':
      return { value: valueOf(s) === true, clear: false }
    case 'cidrs':
      return { value: asList(valueOf(s)).join('\n'), clear: false }
    case 'secret':
      return { value: '', clear: false }
    default:
      return { value: typeof valueOf(s) === 'string' ? (valueOf(s) as string) : '', clear: false }
  }
}

/** Only the editable settings that changed. `null` resets a key to its default. */
export function buildPatch(
  settings: SettingView[],
  drafts: Record<string, Draft>,
): Record<string, unknown> {
  const out: Record<string, unknown> = {}
  for (const s of settings) {
    const d = drafts[s.key]
    if (!d || s.locked) continue
    switch (s.kind) {
      case 'secret':
        if (d.clear) out[s.key] = null
        else if (typeof d.value === 'string' && d.value !== '') out[s.key] = d.value
        break
      case 'bool':
        if (d.value !== (valueOf(s) === true)) out[s.key] = d.value
        break
      case 'cidrs': {
        const next = splitList(String(d.value))
        if (JSON.stringify(next) !== JSON.stringify(asList(valueOf(s)))) out[s.key] = next
        break
      }
      default: {
        const next = String(d.value).trim()
        const cur = typeof valueOf(s) === 'string' ? (valueOf(s) as string) : ''
        if (next !== cur) out[s.key] = next === '' ? null : next
      }
    }
  }
  return out
}

/** A plain statement of how this page was reached; no warning styling (D-072). */
export function connectionText(c: { secure: boolean; origin: string }): string {
  if (!c.secure) return `This page is served over HTTP (${c.origin}).`
  return /\.ts\.net(:\d+)?$/.test(c.origin)
    ? `HTTPS via Tailscale (${c.origin}).`
    : `This page is served over HTTPS (${c.origin}).`
}

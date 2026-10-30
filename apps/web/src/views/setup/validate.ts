import { isHue, type Hue } from '../../lib/hues'
import { ADULT_PASSWORD_MIN, isPin } from '../parts/memberForm'

export interface HouseholdForm {
  name: string
  tz: string
  label: string
  lat: string
  lon: string
  units: 'metric' | 'imperial'
  currency: string
}

export interface OwnerForm {
  display_name: string
  color: Hue
  username: string
  password: string
  again: string
  pin: string
}

export const emptyHousehold = (): HouseholdForm => ({
  name: '',
  tz: (() => {
    try {
      return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
    } catch {
      return 'UTC'
    }
  })(),
  label: '',
  lat: '',
  lon: '',
  units: 'metric',
  currency: 'USD',
})

export const emptyOwner = (): OwnerForm => ({
  display_name: '',
  color: 'blue',
  username: '',
  password: '',
  again: '',
  pin: '',
})

/** Field name → message; empty when valid. Step 2. */
export function validateHousehold(f: HouseholdForm): Record<string, string> {
  const e: Record<string, string> = {}
  if (!f.name.trim()) e.name = 'Enter a name for the household.'
  if (!f.tz.trim()) e.tz = 'Enter a time zone, for example Europe/Berlin.'
  if (!/^[A-Za-z]{3}$/.test(f.currency.trim())) e.currency = 'Use a three-letter currency code.'
  const hasLat = f.lat.trim() !== ''
  const hasLon = f.lon.trim() !== ''
  if (hasLat !== hasLon) e.lat = 'Enter both latitude and longitude, or neither.'
  else if (hasLat) {
    const lat = Number(f.lat)
    const lon = Number(f.lon)
    if (!Number.isFinite(lat) || Math.abs(lat) > 90) e.lat = 'Latitude is between -90 and 90.'
    if (!Number.isFinite(lon) || Math.abs(lon) > 180) e.lon = 'Longitude is between -180 and 180.'
  }
  return e
}

/** Step 3: the first adult. */
export function validateOwner(f: OwnerForm): Record<string, string> {
  const e: Record<string, string> = {}
  if (!f.display_name.trim()) e.display_name = 'Enter your name.'
  const user = f.username.trim()
  if (!user) e.username = 'Choose a username.'
  else if (/\s/.test(user)) e.username = 'A username has no spaces.'
  if (f.password.length < ADULT_PASSWORD_MIN)
    e.password = `Use at least ${ADULT_PASSWORD_MIN} characters.`
  else if (f.password !== f.again) e.again = 'The passwords do not match.'
  if (f.pin && !isPin(f.pin)) e.pin = 'A PIN is 4 to 6 digits.'
  if (!isHue(f.color)) e.color = 'Pick a colour.'
  return e
}

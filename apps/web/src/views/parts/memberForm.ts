/** Client-side checks for member, login and PIN fields; the server re-validates. */

export const ADULT_PASSWORD_MIN = 12
/** Children's passwords may be short (D-031). */
export const CHILD_PASSWORD_MIN = 4

export const passwordMin = (role: string): number =>
  role === 'child' ? CHILD_PASSWORD_MIN : ADULT_PASSWORD_MIN

export const isPin = (s: string): boolean => /^\d{4,6}$/.test(s)

export interface NewMemberFields {
  display_name: string
  role: string
  username: string
  password: string
  pin: string
}

/** Field name → message; empty when the fields are valid. Login and PIN are optional. */
export function validateNewMember(f: NewMemberFields): Record<string, string> {
  const e: Record<string, string> = {}
  if (!f.display_name.trim()) e.display_name = 'Enter a name.'
  const user = f.username.trim()
  if (user && /\s/.test(user)) e.username = 'A username has no spaces.'
  if (user && !f.password) e.password = 'Enter a password for the login.'
  if (!user && f.password) e.username = 'Enter a username for the login.'
  const min = passwordMin(f.role)
  if (f.password && f.password.length < min) e.password = `Use at least ${min} characters.`
  if (f.pin && !isPin(f.pin)) e.pin = 'A PIN is 4 to 6 digits.'
  return e
}

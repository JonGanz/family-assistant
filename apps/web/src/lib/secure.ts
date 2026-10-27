/** True in a secure context (HTTPS or localhost). Google sign-in needs one (D-072). */
export const isSecure = (): boolean => typeof window !== 'undefined' && window.isSecureContext

export const GOOGLE_NEEDS_HTTPS = 'needs HTTPS (turn on Tailscale)'

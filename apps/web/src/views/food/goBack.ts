// Back navigation for the recipe pages: return to the previous page when the app has history (for example the
// meal plan the recipe was opened from), otherwise to a fixed fallback.
interface BackRouter {
  back: () => void
  push: (to: string) => unknown
}

/** True when the router has an earlier entry of this app to go back to. */
export function hasHistory(): boolean {
  return typeof window.history.state?.back === 'string'
}

export function goBack(router: BackRouter, fallback = '/food/recipes'): void {
  if (hasHistory()) router.back()
  else void router.push(fallback)
}

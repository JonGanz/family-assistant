/**
 * Copy text to the clipboard. Uses the async API when the page is in a secure context,
 * otherwise a hidden textarea and `execCommand`. Resolves false when both fail, so the
 * caller can show the text in a selected field with a "press Ctrl+C" hint (D-072).
 */
export async function copyText(text: string): Promise<boolean> {
  try {
    if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
      return true
    }
  } catch {
    /* fall through to the textarea */
  }
  if (typeof document === 'undefined') return false
  const el = document.createElement('textarea')
  el.value = text
  el.setAttribute('readonly', '')
  el.style.position = 'fixed'
  el.style.opacity = '0'
  document.body.appendChild(el)
  el.select()
  try {
    return typeof document.execCommand === 'function' && document.execCommand('copy')
  } catch {
    return false
  } finally {
    document.body.removeChild(el)
  }
}

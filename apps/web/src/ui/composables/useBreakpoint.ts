import { onScopeDispose, ref, type Ref } from 'vue'

export type Breakpoint = 'compact' | 'medium' | 'expanded'

/** Material window classes: compact < 600 px, medium 600-839 px, expanded >= 840 px. */
export function breakpointFor(width: number): Breakpoint {
  if (width < 600) return 'compact'
  if (width < 840) return 'medium'
  return 'expanded'
}

export function useBreakpoint(): Ref<Breakpoint> {
  const bp = ref<Breakpoint>(
    breakpointFor(typeof window === 'undefined' ? 1024 : window.innerWidth),
  )
  if (typeof window === 'undefined') return bp
  const update = () => (bp.value = breakpointFor(window.innerWidth))
  window.addEventListener('resize', update)
  onScopeDispose(() => window.removeEventListener('resize', update))
  update()
  return bp
}

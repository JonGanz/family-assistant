import { inject, provide, reactive, type InjectionKey } from 'vue'

export interface SnackbarOptions {
  text: string
  action?: { label: string; run: () => void | Promise<void> }
  timeoutMs?: number
}

export interface SnackbarItem extends SnackbarOptions {
  id: number
}

export interface SnackbarService {
  /** The message on screen, if any; the rest wait in `queue`. */
  state: { current: SnackbarItem | null; queue: SnackbarItem[] }
  show(opts: SnackbarOptions): void
  dismiss(): void
}

export function createSnackbarService(): SnackbarService {
  const state = reactive<SnackbarService['state']>({ current: null, queue: [] })
  let seq = 0
  let timer: ReturnType<typeof setTimeout> | undefined

  function next() {
    clearTimeout(timer)
    const item = state.queue.shift() ?? null
    state.current = item
    if (item) {
      const ms = item.timeoutMs ?? 5000
      if (ms > 0) timer = setTimeout(next, ms)
    }
  }
  return {
    state,
    show(opts) {
      state.queue.push({ ...opts, id: ++seq })
      if (!state.current) next()
    },
    dismiss: next,
  }
}

const KEY: InjectionKey<SnackbarService> = Symbol('ui-snackbar')
let fallback: SnackbarService | undefined

/** The service the app provides, else a module singleton (so it also works in tests and outside components). */
export function useSnackbar(): SnackbarService {
  return inject(KEY, null) ?? (fallback ??= createSnackbarService())
}

export function provideSnackbar(
  service: SnackbarService = createSnackbarService(),
): SnackbarService {
  provide(KEY, service)
  return service
}

/** For tests: forget the singleton. */
export function resetSnackbar() {
  fallback = undefined
}

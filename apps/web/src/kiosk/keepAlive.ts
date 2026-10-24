// 1x1 silent looping video, generated in code (no asset needed).
function createFallbackVideo(): HTMLVideoElement {
  const v = document.createElement('video')
  v.muted = true
  v.loop = true
  v.playsInline = true
  v.setAttribute('playsinline', '')
  v.width = 1
  v.height = 1
  v.style.cssText =
    'position:fixed;left:0;top:0;width:1px;height:1px;opacity:0.01;pointer-events:none'
  // Canvas capture stream gives a real (blank) video source without a bundled file.
  try {
    const canvas = document.createElement('canvas')
    canvas.width = canvas.height = 1
    canvas.getContext('2d')?.fillRect(0, 0, 1, 1)
    v.srcObject = canvas.captureStream?.(1) ?? null
  } catch {
    /* unsupported: video stays inert */
  }
  return v
}

export interface KeepAliveOptions {
  tickMs?: number
}

/** Starts all keep-alive measures (D-011). Returns a stop function. */
export function startKeepAlive(opts: KeepAliveOptions = {}): () => void {
  let sentinel: WakeLockSentinel | null = null
  let stopped = false
  let video: HTMLVideoElement | null = null

  const startVideo = () => {
    if (video) return
    video = createFallbackVideo()
    document.body.appendChild(video)
    video.play().catch(() => {})
  }

  const acquire = async () => {
    if (stopped || document.visibilityState !== 'visible') return
    if (!('wakeLock' in navigator)) {
      startVideo()
      return
    }
    try {
      sentinel = await navigator.wakeLock.request('screen')
      sentinel.addEventListener('release', () => {
        sentinel = null
      })
    } catch {
      startVideo()
    }
  }

  const onVisibility = () => {
    if (document.visibilityState === 'visible' && !sentinel) void acquire()
  }
  document.addEventListener('visibilitychange', onVisibility)
  void acquire()

  // Periodic DOM tick keeps the page "active" so the browser doesn't idle out.
  const tick = document.createElement('div')
  tick.setAttribute('aria-hidden', 'true')
  tick.style.cssText =
    'position:fixed;left:0;top:0;width:1px;height:1px;opacity:0.01;pointer-events:none'
  document.body.appendChild(tick)
  let n = 0
  const timer = setInterval(() => {
    n = (n + 1) % 2
    tick.style.transform = `translateX(${n}px)`
  }, opts.tickMs ?? 10_000)

  return () => {
    stopped = true
    clearInterval(timer)
    document.removeEventListener('visibilitychange', onVisibility)
    void sentinel?.release().catch(() => {})
    tick.remove()
    video?.remove()
  }
}

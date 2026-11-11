// The alarm: a soft three-note chime synthesised with the Web Audio API (no audio files),
// repeated every ~1.5 s while anything rings. One loop is shared by all ringing timers.

let ctx: AudioContext | null = null
let loop: ReturnType<typeof setInterval> | undefined

const NOTES = [784, 988, 1175] // G5 B5 D6
const REPEAT_MS = 1500

/** Create or resume the AudioContext. Call from a user gesture (autoplay policy). */
export function unlockAudio(): void {
  try {
    if (!ctx) {
      const AC =
        window.AudioContext ??
        (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
      if (!AC) return
      ctx = new AC()
    }
    if (ctx.state === 'suspended') void ctx.resume()
  } catch {
    /* no audio: the card still rings visually */
  }
}

function chime(): void {
  if (!ctx) return
  const t0 = ctx.currentTime
  NOTES.forEach((f, i) => {
    const osc = ctx!.createOscillator()
    const gain = ctx!.createGain()
    osc.type = i === 2 ? 'sine' : 'triangle'
    osc.frequency.value = f
    const s = t0 + i * 0.22
    gain.gain.setValueAtTime(0.0001, s)
    gain.gain.exponentialRampToValueAtTime(0.35, s + 0.03)
    gain.gain.exponentialRampToValueAtTime(0.0001, s + 0.6)
    osc.connect(gain).connect(ctx!.destination)
    osc.start(s)
    osc.stop(s + 0.65)
  })
}

export function startAlarm(): void {
  if (loop) return
  unlockAudio()
  try {
    chime()
  } catch {
    /* ignore */
  }
  loop = setInterval(() => {
    try {
      chime()
    } catch {
      /* ignore */
    }
  }, REPEAT_MS)
}

export function stopAlarm(): void {
  if (loop) clearInterval(loop)
  loop = undefined
}

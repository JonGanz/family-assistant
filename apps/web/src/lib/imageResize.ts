/** Browser-side image downscale + JPEG re-encode so phone photos always fit the server limit. */

export const MAX_EDGE = 2000
export const JPEG_QUALITY = 0.85

/** Target size keeping the aspect ratio, never upscaling; at least 1px per side. */
export function fitWithin(w: number, h: number, max = MAX_EDGE): { width: number; height: number } {
  const scale = Math.min(1, max / Math.max(w, h))
  return {
    width: Math.max(1, Math.round(w * scale)),
    height: Math.max(1, Math.round(h * scale)),
  }
}

async function decode(file: Blob): Promise<{ src: CanvasImageSource; w: number; h: number }> {
  if (typeof createImageBitmap === 'function') {
    try {
      const bmp = await createImageBitmap(file, { imageOrientation: 'from-image' })
      return { src: bmp, w: bmp.width, h: bmp.height }
    } catch {
      /* fall through to <img> */
    }
  }
  const url = URL.createObjectURL(file)
  try {
    const img = new Image()
    img.src = url
    await img.decode()
    return { src: img, w: img.naturalWidth, h: img.naturalHeight }
  } finally {
    URL.revokeObjectURL(url)
  }
}

/** Decode `file`, downscale to `MAX_EDGE` and encode as JPEG. Throws a readable Error. */
export async function resizeToJpeg(file: File): Promise<File> {
  let d
  try {
    d = await decode(file)
  } catch {
    throw new Error(
      'This image could not be read. Choose a JPEG, PNG or WebP photo (or set the phone camera to “most compatible”).',
    )
  }
  const { width, height } = fitWithin(d.w, d.h)
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const ctx = canvas.getContext('2d')
  if (!ctx) throw new Error('This browser cannot process images.')
  ctx.fillStyle = '#fff'
  ctx.fillRect(0, 0, width, height)
  ctx.drawImage(d.src, 0, 0, width, height)
  const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, 'image/jpeg', JPEG_QUALITY))
  if (!blob) throw new Error('The image could not be converted.')
  const name = file.name.replace(/\.[^.]+$/, '') || 'photo'
  return new File([blob], `${name}.jpg`, { type: 'image/jpeg' })
}

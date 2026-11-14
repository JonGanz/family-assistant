import { changelogRaw } from './releaseNotes.generated'

export interface Segment {
  text: string
  kind: 'text' | 'code' | 'bold'
}
export interface ReleaseEntry {
  /** `Unreleased` or a version such as `0.1.31` (no leading v). */
  version: string
  unreleased: boolean
  date: string | null
  bullets: Segment[][]
}

/** Splits one bullet into plain, `code` and **bold** segments; never produces HTML. */
export function inlineSegments(text: string): Segment[] {
  const out: Segment[] = []
  const re = /`([^`]+)`|\*\*([^*]+)\*\*/g
  let last = 0
  for (let m = re.exec(text); m; m = re.exec(text)) {
    if (m.index > last) out.push({ text: text.slice(last, m.index), kind: 'text' })
    out.push(m[1] !== undefined ? { text: m[1], kind: 'code' } : { text: m[2]!, kind: 'bold' })
    last = m.index + m[0].length
  }
  if (last < text.length) out.push({ text: text.slice(last), kind: 'text' })
  return out
}

/** Parses CHANGELOG.md: `## Unreleased` / `## vX.Y.Z — date` headings with `- ` bullets. Anything else is ignored. */
export function parseChangelog(src: string): ReleaseEntry[] {
  const entries: ReleaseEntry[] = []
  let cur: ReleaseEntry | null = null
  for (const raw of src.split(/\r?\n/)) {
    const line = raw.trim()
    if (line.startsWith('## ')) {
      const head = line.slice(3).trim()
      const v = /^v?(\d+\.\d+\.\d+)\s*(?:[—–-]+\s*(\d{4}-\d{2}-\d{2}))?\s*$/.exec(head)
      if (/^unreleased$/i.test(head)) {
        cur = { version: 'Unreleased', unreleased: true, date: null, bullets: [] }
        entries.push(cur)
      } else if (v) {
        cur = { version: v[1]!, unreleased: false, date: v[2] ?? null, bullets: [] }
        entries.push(cur)
      } else {
        cur = null
      }
    } else if (cur && /^[-*]\s+\S/.test(line)) {
      cur.bullets.push(inlineSegments(line.replace(/^[-*]\s+/, '')))
    }
  }
  return entries
}

/** True for the placeholder bullet of an empty section. */
export function isEmptyEntry(e: ReleaseEntry): boolean {
  return (
    e.bullets.length === 0 ||
    (e.bullets.length === 1 && /^nothing yet\.?$/i.test(e.bullets[0]!.map((s) => s.text).join('')))
  )
}

/** Compares a server version (`0.1.31` or `v0.1.31`) with an entry version. */
export function sameVersion(a: string | null | undefined, b: string): boolean {
  return !!a && a.replace(/^v/, '') === b
}

/** What the About page shows: released versions plus Unreleased only when it has content. */
export function releaseNotes(src: string = changelogRaw): ReleaseEntry[] {
  return parseChangelog(src).filter((e) => !(e.unreleased && isEmptyEntry(e)))
}

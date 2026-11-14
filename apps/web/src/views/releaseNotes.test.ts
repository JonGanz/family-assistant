import { describe, expect, it } from 'vitest'
import { parseChangelog, releaseNotes, sameVersion } from './releaseNotes'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { changelogRaw as changelog } from './releaseNotes.generated'

const SRC = `# Changelog

## Unreleased

- Nothing yet.

## v0.1.2 — 2026-11-02

- Added **bold** and \`code\` here.
- Second

## v0.1.1 — 2026-11-01
* Star bullet
`

describe('generated file', () => {
  it('matches CHANGELOG.md', () => {
    const disk = readFileSync(resolve(process.cwd(), '../../CHANGELOG.md'), 'utf8')
    expect(changelog).toBe(disk)
  })
})

describe('parseChangelog', () => {
  it('parses headings, dates and bullets', () => {
    const e = parseChangelog(SRC)
    expect(e.map((x) => x.version)).toEqual(['Unreleased', '0.1.2', '0.1.1'])
    expect(e[1]!.date).toBe('2026-11-02')
    expect(e[1]!.bullets).toHaveLength(2)
    expect(e[1]!.bullets[0]!.map((s) => s.kind)).toEqual(['text', 'bold', 'text', 'code', 'text'])
    expect(e[2]!.bullets).toHaveLength(1)
  })
  it('hides an empty Unreleased section', () => {
    expect(releaseNotes(SRC).map((x) => x.version)).toEqual(['0.1.2', '0.1.1'])
  })
  it('survives malformed input', () => {
    expect(parseChangelog('')).toEqual([])
    expect(parseChangelog('- orphan bullet\n## not a version\n- ignored')).toEqual([])
    const e = parseChangelog('## v1.2.3\n-nospace\n- ok <b>x</b>')
    expect(e[0]!.date).toBeNull()
    expect(e[0]!.bullets).toHaveLength(1)
    expect(e[0]!.bullets[0]![0]!.text).toBe('ok <b>x</b>')
  })
  it('compares versions with or without v', () => {
    expect(sameVersion('0.1.31', '0.1.31')).toBe(true)
    expect(sameVersion('v0.1.31', '0.1.31')).toBe(true)
    expect(sameVersion(null, '0.1.31')).toBe(false)
  })
  it('parses the shipped CHANGELOG.md', () => {
    const e = parseChangelog(changelog)
    expect(e[0]!.unreleased).toBe(true)
    expect(e.filter((x) => !x.unreleased)).toHaveLength((changelog.match(/^## v/gm) ?? []).length)
    expect(e.every((x) => x.unreleased || (x.date && x.bullets.length >= 1))).toBe(true)
  })
})

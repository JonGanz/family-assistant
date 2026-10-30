import type { ThemeFamily } from '../types'

// Rosé Pine. Source: https://github.com/rose-pine/palette (source/index.ts) (MIT).
// Mapping: bg = base background; surface, surface2 = next two levels away; border = subtle line;
// text = main foreground; muted = secondary foreground; onAccent = reads on the blue hue.
// Values marked "derived" were adjusted from the published colour to meet the contrast rules (research/R-002).
const family: ThemeFamily = {
  id: 'rose-pine',
  label: 'Rosé Pine',
  source: 'https://github.com/rose-pine/palette',
  license: 'MIT',
  variants: [
    {
      id: 'dawn',
      label: 'Dawn',
      mode: 'light',
      neutrals: {
        bg: '#faf4ed', // base
        surface: '#fffaf3', // surface
        surface2: '#f2e9e1', // overlay
        border: '#dfdad9', // highlightMed
        muted: '#6a6781', // subtle - derived (published #797593 is 3.67:1)
        text: '#575279', // text
        onAccent: '#232221', // base - derived (published #faf4ed is 3.14:1 on blue)
      },
      hues: {
        red: '#b4637a', // love
        orange: '#c67874', // rose (Rosé Pine has no orange) - derived (published #d7827e is 2.60:1)
        yellow: '#be7f2a', // gold - derived (published #ea9d34 is 2.05:1)
        green: '#286983', // pine (ANSI green)
        cyan: '#56949f', // foam
        blue: '#56949f', // foam (ANSI blue; same as cyan)
        purple: '#907aa9', // iris
        pink: '#b4637a', // love (no pink; same as red)
      },
    },
    {
      id: 'main',
      label: 'Rosé Pine',
      mode: 'dark',
      neutrals: {
        bg: '#191724', // base
        surface: '#1f1d2e', // surface
        surface2: '#26233a', // overlay
        border: '#403d52', // highlightMed
        muted: '#908caa', // subtle
        text: '#e0def4', // text
        onAccent: '#191724', // base
      },
      hues: {
        red: '#eb6f92', // love
        orange: '#ebbcba', // rose (Rosé Pine has no orange)
        yellow: '#f6c177', // gold
        green: '#31748f', // pine (ANSI green)
        cyan: '#9ccfd8', // foam
        blue: '#9ccfd8', // foam (ANSI blue; same as cyan)
        purple: '#c4a7e7', // iris
        pink: '#eb6f92', // love (no pink; same as red)
      },
    },
    {
      id: 'moon',
      label: 'Moon',
      mode: 'dark',
      neutrals: {
        bg: '#232136', // base
        surface: '#2a273f', // surface
        surface2: '#393552', // overlay
        border: '#44415a', // highlightMed
        muted: '#a3a0b8', // subtle - derived (published #908caa is 3.61:1)
        text: '#e0def4', // text
        onAccent: '#232136', // base
      },
      hues: {
        red: '#eb6f92', // love
        orange: '#ea9a97', // rose (Rosé Pine has no orange)
        yellow: '#f6c177', // gold
        green: '#3e8fb0', // pine (ANSI green)
        cyan: '#9ccfd8', // foam
        blue: '#9ccfd8', // foam (ANSI blue; same as cyan)
        purple: '#c4a7e7', // iris
        pink: '#eb6f92', // love (no pink; same as red)
      },
    },
  ],
}

export default family

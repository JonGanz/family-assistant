import type { ThemeFamily } from '../types'

// Solarized. Source: https://github.com/altercation/solarized (README) (MIT).
// Mapping: bg = base background; surface, surface2 = next two levels away; border = subtle line;
// text = main foreground; muted = secondary foreground; onAccent = reads on the blue hue.
// Values marked "derived" were adjusted from the published colour to meet the contrast rules (research/R-002).
const family: ThemeFamily = {
  id: 'solarized',
  label: 'Solarized',
  source: 'https://github.com/altercation/solarized',
  license: 'MIT',
  variants: [
    {
      id: 'light',
      label: 'Solarized Light',
      mode: 'light',
      neutrals: {
        bg: '#fdf6e3', // base3
        surface: '#eee8d5', // base2
        surface2: '#eee8d5', // base2 (Solarized has only two background tones)
        border: '#93a1a1', // base1
        muted: '#566c73', // base01 - derived (published #586e75 is 4.39:1)
        text: '#073642', // base02 (swap: base01 #586e75 was 4.39:1)
        onAccent: '#1e1e1b', // base3 - derived (published #fdf6e3 is 3.41:1 on blue)
      },
      extended: {
        layers: [
          '#fdf6e3', // base3
          '#eee8d5', // base2
          '#CCCDC2', // mix_base01
          '#93A1A1', // base1
        ],
        selection: '#CCCDC2', // mix_base01
        highlight: '#eee8d5', // base2
        textStrong: '#002b36', // base03
        textSubtle: '#586E75', // base01
        comment: '#93A1A1', // base1
      },
      hues: {
        red: '#dc322f', // red
        orange: '#cb4b16', // orange
        yellow: '#b38800', // yellow - derived (published #b58900 is 2.98:1)
        green: '#849700', // green - derived (published #859900 is 2.97:1)
        cyan: '#299e95', // cyan - derived (published #2aa198 is 2.93:1)
        blue: '#268bd2', // blue
        purple: '#6c71c4', // violet
        pink: '#d33682', // magenta
      },
    },
    {
      id: 'dark',
      label: 'Solarized Dark',
      mode: 'dark',
      neutrals: {
        bg: '#002b36', // base03
        surface: '#073642', // base02
        surface2: '#073642', // base02 (Solarized has only two background tones)
        border: '#586e75', // base01
        muted: '#93a1a1', // base1 (swap: base0 #839496 was 4.11:1)
        text: '#eee8d5', // base2 (swap: base1 #93a1a1 text; moved up to stay above muted)
        onAccent: '#00222b', // base03 - derived (published #002b36 is 4.08:1 on blue)
      },
      extended: {
        layers: [
          '#002b36', // base03
          '#073642', // base02
          '#2C4E56', // mix_base1
          '#586E75', // base01
        ],
        selection: '#0B4764', // mix_blue
        highlight: '#073642', // base02
        textStrong: '#fdf6e3', // base3
        textSubtle: '#93A1A1', // base1
        comment: '#586E75', // base01
      },
      hues: {
        red: '#dc322f', // red
        orange: '#cb4b16', // orange
        yellow: '#b58900', // yellow
        green: '#859900', // green
        cyan: '#2aa198', // cyan
        blue: '#268bd2', // blue
        purple: '#6c71c4', // violet
        pink: '#d33682', // magenta
      },
    },
  ],
}

export default family

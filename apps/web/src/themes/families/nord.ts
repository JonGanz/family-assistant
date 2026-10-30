import type { ThemeFamily } from '../types'

// Nord. Source: https://github.com/nordtheme/nord (src/nord.css) (MIT).
// Two variants: the dark Nord (Polar Night backgrounds) and Snow Storm (light). Nord publishes no light UI palette;
// src/nord.css only describes Snow Storm as "semi-light background" colours, so the light variant is our reading:
// nord6, nord5, nord4 as backgrounds and Polar Night (nord0, nord3) as text; Frost and Aurora hues are darkened
// toward black only as far as the contrast rules need (marked "derived"; research/R-003).
// Mapping: bg = base background; surface, surface2 = next two levels away; border = subtle line;
// text = main foreground; muted = secondary foreground; onAccent = reads on the blue hue.
// Values marked "derived" were adjusted from the published colour to meet the contrast rules (research/R-002).
const family: ThemeFamily = {
  id: 'nord',
  label: 'Nord',
  source: 'https://github.com/nordtheme/nord',
  license: 'MIT',
  variants: [
    {
      id: 'nord',
      label: 'Nord',
      mode: 'dark',
      neutrals: {
        bg: '#2e3440', // nord0
        surface: '#3b4252', // nord1
        surface2: '#434c5e', // nord2
        border: '#4c566a', // nord3
        muted: '#d8dee9', // nord4
        text: '#eceff4', // nord6
        onAccent: '#2e3440', // nord0
      },
      hues: {
        red: '#bf616a', // nord11
        orange: '#d08770', // nord12
        yellow: '#ebcb8b', // nord13
        green: '#a3be8c', // nord14
        cyan: '#88c0d0', // nord8
        blue: '#81a1c1', // nord9
        purple: '#b48ead', // nord15
        pink: '#b48ead', // nord15 (Nord has no pink; same as purple)
      },
    },
    {
      id: 'snow-storm',
      label: 'Nord Snow Storm',
      mode: 'light',
      neutrals: {
        bg: '#eceff4', // nord6
        surface: '#e5e9f0', // nord5
        surface2: '#d8dee9', // nord4
        border: '#a4aab6', // nord3 mixed 55% toward nord6 - derived (a Polar Night line would be too heavy)
        muted: '#4c566a', // nord3
        text: '#2e3440', // nord0
        onAccent: '#eceff4', // nord6
      },
      hues: {
        red: '#bf616a', // nord11
        orange: '#b97864', // nord12 - derived (published #d08770 is 2.47:1)
        yellow: '#9b865c', // nord13 - derived (published #ebcb8b is 1.35:1)
        green: '#7c906a', // nord14 - derived (published #a3be8c is 1.77:1)
        cyan: '#66909c', // nord8 - derived (published #88c0d0 is 1.74:1)
        blue: '#506e92', // nord10 - derived (published #5e81ac gives onAccent only 3.50:1)
        purple: '#a07e9a', // nord15 - derived (published #b48ead is 2.46:1)
        pink: '#a07e9a', // nord15 (Nord has no pink; same as purple) - derived
      },
    },
  ],
}

export default family

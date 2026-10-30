import type { ThemeFamily } from '../types'

// Nord. Source: https://github.com/nordtheme/nord (src/nord.css) (MIT).
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
  ],
}

export default family

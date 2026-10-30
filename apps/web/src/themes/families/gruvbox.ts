import type { ThemeFamily } from '../types'

// Gruvbox. Source: https://github.com/morhetz/gruvbox (colors/gruvbox.vim) (MIT).
// Mapping: bg = base background; surface, surface2 = next two levels away; border = subtle line;
// text = main foreground; muted = secondary foreground; onAccent = reads on the blue hue.
// Values marked "derived" were adjusted from the published colour to meet the contrast rules (research/R-002).
const family: ThemeFamily = {
  id: 'gruvbox',
  label: 'Gruvbox',
  source: 'https://github.com/morhetz/gruvbox',
  license: 'MIT',
  variants: [
    {
      id: 'light',
      label: 'Light',
      mode: 'light',
      neutrals: {
        bg: '#fbf1c7', // light0
        surface: '#ebdbb2', // light1
        surface2: '#d5c4a1', // light2
        border: '#bdae93', // light3
        muted: '#504945', // dark2 (fg2; swap: dark3 #665c54 was 3.8:1)
        text: '#3c3836', // dark1 (fg1)
        onAccent: '#fbf1c7', // light0
      },
      hues: {
        red: '#9d0006', // faded_red
        orange: '#af3a03', // faded_orange
        yellow: '#b57614', // faded_yellow
        green: '#79740e', // faded_green
        cyan: '#427b58', // faded_aqua
        blue: '#076678', // faded_blue
        purple: '#8f3f71', // faded_purple
        pink: '#b16286', // neutral_purple (Gruvbox has no pink)
      },
    },
    {
      id: 'dark',
      label: 'Dark',
      mode: 'dark',
      neutrals: {
        bg: '#282828', // dark0
        surface: '#3c3836', // dark1
        surface2: '#504945', // dark2
        border: '#665c54', // dark3
        muted: '#d5c4a1', // light2 (fg2; swap: light3 #bdae93 was 4.05:1)
        text: '#ebdbb2', // light1 (fg1)
        onAccent: '#282828', // dark0
      },
      hues: {
        red: '#fb4934', // bright_red
        orange: '#fe8019', // bright_orange
        yellow: '#fabd2f', // bright_yellow
        green: '#b8bb26', // bright_green
        cyan: '#8ec07c', // bright_aqua (Gruvbox has no cyan)
        blue: '#83a598', // bright_blue
        purple: '#b16286', // neutral_purple
        pink: '#d3869b', // bright_purple (Gruvbox has no pink)
      },
    },
  ],
}

export default family

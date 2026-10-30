import type { ThemeFamily } from '../types'

// Catppuccin. Source: https://github.com/catppuccin/palette (palette.json) (MIT).
// Mapping: bg = base background; surface, surface2 = next two levels away; border = subtle line;
// text = main foreground; muted = secondary foreground; onAccent = reads on the blue hue.
// Values marked "derived" were adjusted from the published colour to meet the contrast rules (research/R-002).
const family: ThemeFamily = {
  id: 'catppuccin',
  label: 'Catppuccin',
  source: 'https://github.com/catppuccin/palette',
  license: 'MIT',
  variants: [
    {
      id: 'latte',
      label: 'Latte',
      mode: 'light',
      neutrals: {
        bg: '#eff1f5', // base
        surface: '#e6e9ef', // mantle
        surface2: '#dce0e8', // crust
        border: '#bcc0cc', // surface1
        muted: '#5c5f77', // subtext1
        text: '#4c4f69', // text
        onAccent: '#ffffff', // white (not in palette)
      },
      hues: {
        red: '#d20f39', // red
        orange: '#c2410c', // peach - derived (published #fe640b is 2.64:1)
        yellow: '#8f5a0a', // yellow - derived (published #df8e1d is 2.31:1)
        green: '#2f7d1f', // green - derived (published #40a02b is 2.96:1)
        cyan: '#179299', // teal
        blue: '#1e66f5', // blue
        purple: '#8839ef', // mauve
        pink: '#b8449a', // pink - derived (published #ea76cb is 2.34:1)
      },
    },
    {
      id: 'mocha',
      label: 'Mocha',
      mode: 'dark',
      neutrals: {
        bg: '#1e1e2e', // base
        surface: '#181825', // mantle
        surface2: '#11111b', // crust
        border: '#45475a', // surface1
        muted: '#a6adc8', // subtext0
        text: '#cdd6f4', // text
        onAccent: '#11111b', // crust
      },
      hues: {
        red: '#f38ba8', // red
        orange: '#fab387', // peach
        yellow: '#f9e2af', // yellow
        green: '#a6e3a1', // green
        cyan: '#94e2d5', // teal
        blue: '#89b4fa', // blue
        purple: '#cba6f7', // mauve
        pink: '#f5c2e7', // pink
      },
    },
    {
      id: 'frappe',
      label: 'Frappé',
      mode: 'dark',
      neutrals: {
        bg: '#303446', // base
        surface: '#292c3c', // mantle
        surface2: '#232634', // crust
        border: '#51576d', // surface1
        muted: '#a5adce', // subtext0
        text: '#c6d0f5', // text
        onAccent: '#232634', // crust
      },
      hues: {
        red: '#e78284', // red
        orange: '#ef9f76', // peach
        yellow: '#e5c890', // yellow
        green: '#a6d189', // green
        cyan: '#81c8be', // teal
        blue: '#8caaee', // blue
        purple: '#ca9ee6', // mauve
        pink: '#f4b8e4', // pink
      },
    },
    {
      id: 'macchiato',
      label: 'Macchiato',
      mode: 'dark',
      neutrals: {
        bg: '#24273a', // base
        surface: '#1e2030', // mantle
        surface2: '#181926', // crust
        border: '#494d64', // surface1
        muted: '#a5adcb', // subtext0
        text: '#cad3f5', // text
        onAccent: '#181926', // crust
      },
      hues: {
        red: '#ed8796', // red
        orange: '#f5a97f', // peach
        yellow: '#eed49f', // yellow
        green: '#a6da95', // green
        cyan: '#8bd5ca', // teal
        blue: '#8aadf4', // blue
        purple: '#c6a0f6', // mauve
        pink: '#f5bde6', // pink
      },
    },
  ],
}

export default family

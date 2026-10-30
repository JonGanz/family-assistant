import type { ThemeFamily } from '../types'

// Tokyo Night. Source: https://github.com/folke/tokyonight.nvim (extras/lua/tokyonight_*.lua) (Apache-2.0).
// Mapping: bg = base background; surface, surface2 = next two levels away; border = subtle line;
// text = main foreground; muted = secondary foreground; onAccent = reads on the blue hue.
// Values marked "derived" were adjusted from the published colour to meet the contrast rules (research/R-002).
const family: ThemeFamily = {
  id: 'tokyo-night',
  label: 'Tokyo Night',
  source: 'https://github.com/folke/tokyonight.nvim',
  license: 'Apache-2.0',
  variants: [
    {
      id: 'day',
      label: 'Day',
      mode: 'light',
      neutrals: {
        bg: '#e1e2e7', // bg
        surface: '#d0d5e3', // bg_dark
        surface2: '#c1c9df', // bg_dark1
        border: '#b4b5b9', // border
        muted: '#475380', // fg_dark - derived (published #6172b0 is 2.79:1)
        text: '#2e51a0', // fg - derived (published #3760bf is 3.54:1)
        onAccent: '#141414', // white (not in palette) - derived (published #ffffff is 4.02:1 on blue)
      },
      hues: {
        red: '#f52a65', // red
        orange: '#b15c00', // orange
        yellow: '#8c6c3e', // yellow
        green: '#587539', // green
        cyan: '#007197', // cyan
        blue: '#2e7de9', // blue
        purple: '#9854f1', // magenta
        pink: '#d20065', // magenta2
      },
    },
    {
      id: 'night',
      label: 'Night',
      mode: 'dark',
      neutrals: {
        bg: '#1a1b26', // bg
        surface: '#16161e', // bg_dark
        surface2: '#0c0e14', // bg_dark1
        border: '#3b4261', // fg_gutter
        muted: '#a9b1d6', // fg_dark
        text: '#c0caf5', // fg
        onAccent: '#16161e', // bg_dark
      },
      hues: {
        red: '#f7768e', // red
        orange: '#ff9e64', // orange
        yellow: '#e0af68', // yellow
        green: '#9ece6a', // green
        cyan: '#7dcfff', // cyan
        blue: '#7aa2f7', // blue
        purple: '#bb9af7', // magenta
        pink: '#ff007c', // magenta2
      },
    },
    {
      id: 'storm',
      label: 'Storm',
      mode: 'dark',
      neutrals: {
        bg: '#24283b', // bg
        surface: '#1f2335', // bg_dark
        surface2: '#1b1e2d', // bg_dark1
        border: '#3b4261', // fg_gutter
        muted: '#a9b1d6', // fg_dark
        text: '#c0caf5', // fg
        onAccent: '#1f2335', // bg_dark
      },
      hues: {
        red: '#f7768e', // red
        orange: '#ff9e64', // orange
        yellow: '#e0af68', // yellow
        green: '#9ece6a', // green
        cyan: '#7dcfff', // cyan
        blue: '#7aa2f7', // blue
        purple: '#bb9af7', // magenta
        pink: '#ff007c', // magenta2
      },
    },
    {
      id: 'moon',
      label: 'Moon',
      mode: 'dark',
      neutrals: {
        bg: '#222436', // bg
        surface: '#1e2030', // bg_dark
        surface2: '#191b29', // bg_dark1
        border: '#3b4261', // fg_gutter
        muted: '#828bb8', // fg_dark
        text: '#c8d3f5', // fg
        onAccent: '#1e2030', // bg_dark
      },
      hues: {
        red: '#ff757f', // red
        orange: '#ff966c', // orange
        yellow: '#ffc777', // yellow
        green: '#c3e88d', // green
        cyan: '#86e1fc', // cyan
        blue: '#82aaff', // blue
        purple: '#c099ff', // magenta
        pink: '#fca7ea', // purple (Moon names this pink-violet "purple")
      },
    },
  ],
}

export default family

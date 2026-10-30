import type { ThemeFamily } from '../types'

// Kanagawa. Source: https://github.com/rebelot/kanagawa.nvim (lua/kanagawa/colors.lua, themes.lua) (MIT).
// Mapping: bg = base background; surface, surface2 = next two levels away; border = subtle line;
// text = main foreground; muted = secondary foreground; onAccent = reads on the blue hue.
// Values marked "derived" were adjusted from the published colour to meet the contrast rules (research/R-002).
const family: ThemeFamily = {
  id: 'kanagawa',
  label: 'Kanagawa',
  source: 'https://github.com/rebelot/kanagawa.nvim',
  license: 'MIT',
  variants: [
    {
      id: 'lotus',
      label: 'Lotus',
      mode: 'light',
      neutrals: {
        bg: '#f2ecbc', // lotusWhite3
        surface: '#e5ddb0', // lotusWhite2
        surface2: '#dcd5ac', // lotusWhite1
        border: '#a09cac', // lotusViolet1
        muted: '#5f5c51', // lotusGray2 - derived (published #716e61 is 3.45:1)
        text: '#545464', // lotusInk1
        onAccent: '#f2ecbc', // lotusWhite3
      },
      hues: {
        red: '#c84053', // lotusRed
        orange: '#cc6d00', // lotusOrange
        yellow: '#836f4a', // lotusYellow2
        green: '#6f894e', // lotusGreen
        cyan: '#5e857a', // lotusAqua2
        blue: '#4d699b', // lotusBlue4
        purple: '#624c83', // lotusViolet4
        pink: '#b35b79', // lotusPink
      },
    },
    {
      id: 'wave',
      label: 'Wave',
      mode: 'dark',
      neutrals: {
        bg: '#1f1f28', // sumiInk3
        surface: '#2a2a37', // sumiInk4
        surface2: '#363646', // sumiInk5
        border: '#54546d', // sumiInk6
        muted: '#c8c093', // oldWhite
        text: '#dcd7ba', // fujiWhite
        onAccent: '#16161d', // sumiInk0
      },
      hues: {
        red: '#e46876', // waveRed
        orange: '#ffa066', // surimiOrange
        yellow: '#e6c384', // carpYellow
        green: '#98bb6c', // springGreen
        cyan: '#7aa89f', // waveAqua2
        blue: '#7e9cd8', // crystalBlue
        purple: '#957fb8', // oniViolet
        pink: '#d27e99', // sakuraPink
      },
    },
    {
      id: 'dragon',
      label: 'Dragon',
      mode: 'dark',
      neutrals: {
        bg: '#181616', // dragonBlack3
        surface: '#282727', // dragonBlack4
        surface2: '#393836', // dragonBlack5
        border: '#625e5a', // dragonBlack6
        muted: '#a6a69c', // dragonGray
        text: '#c5c9c5', // dragonWhite
        onAccent: '#0d0c0c', // dragonBlack0
      },
      hues: {
        red: '#c4746e', // dragonRed
        orange: '#b6927b', // dragonOrange
        yellow: '#c4b28a', // dragonYellow
        green: '#87a987', // dragonGreen
        cyan: '#8ea4a2', // dragonAqua
        blue: '#8ba4b0', // dragonBlue2
        purple: '#8992a7', // dragonViolet
        pink: '#a292a3', // dragonPink
      },
    },
  ],
}

export default family

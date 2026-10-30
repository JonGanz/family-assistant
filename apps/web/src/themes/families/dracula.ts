import type { ThemeFamily } from '../types'

// Dracula. Source: https://draculatheme.com/spec and https://github.com/dracula/dracula-theme (README palette) (MIT).
// Mapping: bg = base background; surface, surface2 = next two levels away; border = subtle line;
// text = main foreground; muted = secondary foreground; onAccent = reads on the blue hue.
// Values marked "derived" were adjusted from the published colour to meet the contrast rules (research/R-002).
const family: ThemeFamily = {
  id: 'dracula',
  label: 'Dracula',
  source: 'https://draculatheme.com/spec',
  license: 'MIT',
  variants: [
    {
      id: 'dracula',
      label: 'Dracula',
      mode: 'dark',
      neutrals: {
        bg: '#282a36', // Background
        surface: '#21222c', // Background Dark (spec)
        surface2: '#191a21', // Background Darker (spec)
        border: '#44475a', // Current Line
        muted: '#8390b7', // Comment - derived (published #6272a4 is 3.03:1)
        text: '#f8f8f2', // Foreground
        onAccent: '#282a36', // Background
      },
      hues: {
        red: '#ff5555', // Red
        orange: '#ffb86c', // Orange
        yellow: '#f1fa8c', // Yellow
        green: '#50fa7b', // Green
        cyan: '#8be9fd', // Cyan
        blue: '#bd93f9', // Purple (spec AnsiBlue; Dracula has no blue)
        purple: '#bd93f9', // Purple
        pink: '#ff79c6', // Pink
      },
    },
    {
      id: 'alucard',
      label: 'Alucard',
      mode: 'light',
      neutrals: {
        bg: '#fffbeb', // Background
        surface: '#ece9df', // Background Lighter (spec)
        surface2: '#dedccf', // Background Light (spec)
        border: '#bcbab3', // Background Darker (spec)
        muted: '#676147', // Comment - derived (published #6c664b is 4.19:1)
        text: '#1f1f1f', // Foreground
        onAccent: '#fffbeb', // Background
      },
      hues: {
        red: '#cb3a2a', // Red
        orange: '#a34d14', // Orange
        yellow: '#846e15', // Yellow
        green: '#14710a', // Green
        cyan: '#036a96', // Cyan
        blue: '#644ac9', // Purple (spec AnsiBlue; Alucard has no blue)
        purple: '#644ac9', // Purple
        pink: '#a3144d', // Pink
      },
    },
  ],
}

export default family

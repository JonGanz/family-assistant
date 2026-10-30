import type { ThemeFamily } from '../types'

// seoul256. Source: https://github.com/junegunn/seoul256.vim (colors/seoul256.vim, colors/seoul256-light.vim) (MIT, per README).
// The plugin has a configurable background (xterm index): dark 233-239 (default 237), light 252-256 (default 253);
// one variant per level, defaults first. Colours are xterm indexes; hex comes from the plugin's own GUI table
// (s:rgb_map, used when g:seoul256_srgb is off), and from the standard xterm-256 table for the few indexes
// that table lacks (232, 242-248, 256 is white).
// Mapping: bg = the background level; text = the plugin's Normal foreground (252 dark, 239 light);
// surface, surface2 = the plugin's own panel levels (dark: bg-1 and bg-2, or 16 below 234, as CursorLine and CocFloating;
// light: bg+1 and bg+2, as Conceal and Todo); border = TabLine background (dark bg+4, light bg-4);
// muted = nearest neutral (xterm 249 dark, 241 light; the plugin has no muted foreground, its Comment is green-grey and fails contrast);
// onAccent = the background, reads on the blue hue.
// Hues come from highlight groups: red Exception/Todo (161 dark, 125 light), orange Define (173 dark, 131 light),
// yellow Type (179 dark, 94 light), green diffAdded (108 dark, 65 light), cyan Constant (73 dark, 23 light),
// blue Repeat/GitGutterChange (68 dark) and SpellCap (25 light), purple Boolean (103 dark) and Identifier (96 light),
// pink Keyword (168). seoul256 has no orange or pink group of its own, so the closest published colour is used.
// Values marked "derived" were adjusted from the published colour to meet the contrast rules (research/R-003).
const family: ThemeFamily = {
  id: 'seoul256',
  label: 'seoul256',
  source: 'https://github.com/junegunn/seoul256.vim',
  license: 'MIT',
  variants: [
    {
      id: 'dark-237',
      label: 'Dark 237',
      mode: 'dark',
      neutrals: {
        bg: '#4b4b4b', // xterm 237
        surface: '#3f3f3f', // xterm 236
        surface2: '#333233', // xterm 235
        border: '#757575', // xterm 241
        muted: '#bfbfbf', // xterm 249
        text: '#d9d9d9', // xterm 252
        onAccent: '#333333', // bg (xterm 237) - derived (published #4b4b4b)
      },
      hues: {
        red: '#eb6ea1', // xterm 161 - derived (published #e12672)
        orange: '#e19972', // xterm 173
        yellow: '#dfbc72', // xterm 179
        green: '#98bc99', // xterm 108
        cyan: '#6fbcbd', // xterm 73
        blue: '#719cdf', // xterm 68
        purple: '#999abd', // xterm 103
        pink: '#e17899', // xterm 168
      },
    },
    {
      id: 'dark-233',
      label: 'Dark 233',
      mode: 'dark',
      neutrals: {
        bg: '#171717', // xterm 233
        surface: '#060606', // xterm 232
        surface2: '#000000', // xterm 16
        border: '#4b4b4b', // xterm 237
        muted: '#bfbfbf', // xterm 249
        text: '#d9d9d9', // xterm 252
        onAccent: '#171717', // bg (xterm 233)
      },
      hues: {
        red: '#e12672', // xterm 161
        orange: '#e19972', // xterm 173
        yellow: '#dfbc72', // xterm 179
        green: '#98bc99', // xterm 108
        cyan: '#6fbcbd', // xterm 73
        blue: '#719cdf', // xterm 68
        purple: '#999abd', // xterm 103
        pink: '#e17899', // xterm 168
      },
    },
    {
      id: 'dark-234',
      label: 'Dark 234',
      mode: 'dark',
      neutrals: {
        bg: '#252525', // xterm 234
        surface: '#171717', // xterm 233
        surface2: '#060606', // xterm 232
        border: '#565656', // xterm 238
        muted: '#bfbfbf', // xterm 249
        text: '#d9d9d9', // xterm 252
        onAccent: '#252525', // bg (xterm 234)
      },
      hues: {
        red: '#e12672', // xterm 161
        orange: '#e19972', // xterm 173
        yellow: '#dfbc72', // xterm 179
        green: '#98bc99', // xterm 108
        cyan: '#6fbcbd', // xterm 73
        blue: '#719cdf', // xterm 68
        purple: '#999abd', // xterm 103
        pink: '#e17899', // xterm 168
      },
    },
    {
      id: 'dark-235',
      label: 'Dark 235',
      mode: 'dark',
      neutrals: {
        bg: '#333233', // xterm 235
        surface: '#252525', // xterm 234
        surface2: '#171717', // xterm 233
        border: '#616161', // xterm 239
        muted: '#bfbfbf', // xterm 249
        text: '#d9d9d9', // xterm 252
        onAccent: '#333233', // bg (xterm 235)
      },
      hues: {
        red: '#e33179', // xterm 161 - derived (published #e12672)
        orange: '#e19972', // xterm 173
        yellow: '#dfbc72', // xterm 179
        green: '#98bc99', // xterm 108
        cyan: '#6fbcbd', // xterm 73
        blue: '#719cdf', // xterm 68
        purple: '#999abd', // xterm 103
        pink: '#e17899', // xterm 168
      },
    },
    {
      id: 'dark-236',
      label: 'Dark 236',
      mode: 'dark',
      neutrals: {
        bg: '#3f3f3f', // xterm 236
        surface: '#333233', // xterm 235
        surface2: '#252525', // xterm 234
        border: '#6b6b6b', // xterm 240
        muted: '#bfbfbf', // xterm 249
        text: '#d9d9d9', // xterm 252
        onAccent: '#333333', // bg (xterm 236) - derived (published #3f3f3f)
      },
      hues: {
        red: '#e75490', // xterm 161 - derived (published #e12672)
        orange: '#e19972', // xterm 173
        yellow: '#dfbc72', // xterm 179
        green: '#98bc99', // xterm 108
        cyan: '#6fbcbd', // xterm 73
        blue: '#719cdf', // xterm 68
        purple: '#999abd', // xterm 103
        pink: '#e17899', // xterm 168
      },
    },
    {
      id: 'dark-238',
      label: 'Dark 238',
      mode: 'dark',
      neutrals: {
        bg: '#565656', // xterm 238
        surface: '#4b4b4b', // xterm 237
        surface2: '#3f3f3f', // xterm 236
        border: '#6c6c6c', // xterm 242
        muted: '#cbcbcb', // xterm 249 - derived (published #bfbfbf)
        text: '#d9d9d9', // xterm 252
        onAccent: '#3c3c3c', // bg (xterm 238) - derived (published #565656)
      },
      hues: {
        red: '#ee85b0', // xterm 161 - derived (published #e12672)
        orange: '#e19972', // xterm 173
        yellow: '#dfbc72', // xterm 179
        green: '#98bc99', // xterm 108
        cyan: '#6fbcbd', // xterm 73
        blue: '#82a8e3', // xterm 68 - derived (published #719cdf)
        purple: '#a3a4c4', // xterm 103 - derived (published #999abd)
        pink: '#e68ca8', // xterm 168 - derived (published #e17899)
      },
    },
    {
      id: 'dark-239',
      label: 'Dark 239',
      mode: 'dark',
      neutrals: {
        bg: '#616161', // xterm 239
        surface: '#565656', // xterm 238
        surface2: '#4b4b4b', // xterm 237
        border: '#767676', // xterm 243
        muted: '#dcdcdc', // xterm 249 - derived (published #bfbfbf)
        text: '#dcdcdc', // xterm 252 - derived (published #d9d9d9)
        onAccent: '#474747', // bg (xterm 239) - derived (published #616161)
      },
      hues: {
        red: '#f29dc0', // xterm 161 - derived (published #e12672)
        orange: '#e5a786', // xterm 173 - derived (published #e19972)
        yellow: '#dfbc72', // xterm 179
        green: '#9bbe9c', // xterm 108 - derived (published #98bc99)
        cyan: '#79c1c2', // xterm 73 - derived (published #6fbcbd)
        blue: '#97b7e8', // xterm 68 - derived (published #719cdf)
        purple: '#b3b3ce', // xterm 103 - derived (published #999abd)
        pink: '#eaa1b8', // xterm 168 - derived (published #e17899)
      },
    },
    {
      id: 'light-253',
      label: 'Light 253',
      mode: 'light',
      neutrals: {
        bg: '#e1e1e1', // xterm 253
        surface: '#e9e9e9', // xterm 254
        surface2: '#f1f1f1', // xterm 255
        border: '#bfbfbf', // xterm 249
        muted: '#636363', // xterm 241 - derived (published #757575)
        text: '#616161', // xterm 239
        onAccent: '#f4f4f4', // bg (xterm 253) - derived (published #e1e1e1)
      },
      hues: {
        red: '#bf2172', // xterm 125
        orange: '#b16d6a', // xterm 131 - derived (published #be7572)
        yellow: '#9a7200', // xterm 94
        green: '#668967', // xterm 65 - derived (published #719872)
        cyan: '#007173', // xterm 23
        blue: '#0074be', // xterm 25
        purple: '#987497', // xterm 96 - derived (published #9a7599)
        pink: '#bd6581', // xterm 168 - derived (published #e17899)
      },
    },
    {
      id: 'light-252',
      label: 'Light 252',
      mode: 'light',
      neutrals: {
        bg: '#d9d9d9', // xterm 252
        surface: '#e1e1e1', // xterm 253
        surface2: '#e9e9e9', // xterm 254
        border: '#a8a8a8', // xterm 248
        muted: '#5f5f5f', // xterm 241 - derived (published #757575)
        text: '#5f5f5f', // xterm 239 - derived (published #616161)
        onAccent: '#f4f4f4', // bg (xterm 252) - derived (published #d9d9d9)
      },
      hues: {
        red: '#bf2172', // xterm 125
        orange: '#ab6967', // xterm 131 - derived (published #be7572)
        yellow: '#9a7200', // xterm 94
        green: '#618362', // xterm 65 - derived (published #719872)
        cyan: '#007173', // xterm 23
        blue: '#0074be', // xterm 25
        purple: '#926f91', // xterm 96 - derived (published #9a7599)
        pink: '#b4607a', // xterm 168 - derived (published #e17899)
      },
    },
    {
      id: 'light-254',
      label: 'Light 254',
      mode: 'light',
      neutrals: {
        bg: '#e9e9e9', // xterm 254
        surface: '#f1f1f1', // xterm 255
        surface2: '#ffffff', // xterm 256
        border: '#c8c8c8', // xterm 250
        muted: '#696969', // xterm 241 - derived (published #757575)
        text: '#616161', // xterm 239
        onAccent: '#f4f4f4', // bg (xterm 254) - derived (published #e9e9e9)
      },
      hues: {
        red: '#bf2172', // xterm 125
        orange: '#b8716f', // xterm 131 - derived (published #be7572)
        yellow: '#9a7200', // xterm 94
        green: '#6a8f6b', // xterm 65 - derived (published #719872)
        cyan: '#007173', // xterm 23
        blue: '#0074be', // xterm 25
        purple: '#9a7599', // xterm 96
        pink: '#c46885', // xterm 168 - derived (published #e17899)
      },
    },
    {
      id: 'light-255',
      label: 'Light 255',
      mode: 'light',
      neutrals: {
        bg: '#f1f1f1', // xterm 255
        surface: '#ffffff', // xterm 256
        surface2: '#ffffff', // xterm 256
        border: '#d1d0d1', // xterm 251
        muted: '#6e6e6e', // xterm 241 - derived (published #757575)
        text: '#616161', // xterm 239
        onAccent: '#f4f4f4', // bg (xterm 255) - derived (published #f1f1f1)
      },
      hues: {
        red: '#bf2172', // xterm 125
        orange: '#be7572', // xterm 131
        yellow: '#9a7200', // xterm 94
        green: '#6e936f', // xterm 65 - derived (published #719872)
        cyan: '#007173', // xterm 23
        blue: '#0074be', // xterm 25
        purple: '#9a7599', // xterm 96
        pink: '#cd6d8b', // xterm 168 - derived (published #e17899)
      },
    },
    {
      id: 'light-256',
      label: 'Light 256',
      mode: 'light',
      neutrals: {
        bg: '#ffffff', // xterm 256
        surface: '#ffffff', // xterm 256
        surface2: '#ffffff', // xterm 256
        border: '#d9d9d9', // xterm 252
        muted: '#757575', // xterm 241
        text: '#616161', // xterm 239
        onAccent: '#ffffff', // bg (xterm 256)
      },
      hues: {
        red: '#bf2172', // xterm 125
        orange: '#be7572', // xterm 131
        yellow: '#9a7200', // xterm 94
        green: '#719872', // xterm 65
        cyan: '#007173', // xterm 23
        blue: '#0074be', // xterm 25
        purple: '#9a7599', // xterm 96
        pink: '#da7494', // xterm 168 - derived (published #e17899)
      },
    },
  ],
}

export default family

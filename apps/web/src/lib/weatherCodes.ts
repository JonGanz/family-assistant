export type WeatherIconName =
  | 'clear'
  | 'clear-night'
  | 'partly-cloudy'
  | 'partly-cloudy-night'
  | 'cloudy'
  | 'fog'
  | 'drizzle'
  | 'rain'
  | 'freezing-rain'
  | 'snow'
  | 'showers'
  | 'snow-showers'
  | 'thunder'

export interface WeatherCodeInfo {
  label: string
  icon: WeatherIconName
  night_icon?: WeatherIconName
}

const W = (label: string, icon: WeatherIconName, night_icon?: WeatherIconName): WeatherCodeInfo =>
  night_icon ? { label, icon, night_icon } : { label, icon }

export const WEATHER_CODES: Record<number, WeatherCodeInfo> = {
  0: W('Clear sky', 'clear', 'clear-night'),
  1: W('Mainly clear', 'clear', 'clear-night'),
  2: W('Partly cloudy', 'partly-cloudy', 'partly-cloudy-night'),
  3: W('Overcast', 'cloudy'),
  45: W('Fog', 'fog'),
  48: W('Rime fog', 'fog'),
  51: W('Light drizzle', 'drizzle'),
  53: W('Drizzle', 'drizzle'),
  55: W('Dense drizzle', 'drizzle'),
  56: W('Light freezing drizzle', 'freezing-rain'),
  57: W('Freezing drizzle', 'freezing-rain'),
  61: W('Light rain', 'rain'),
  63: W('Rain', 'rain'),
  65: W('Heavy rain', 'rain'),
  66: W('Light freezing rain', 'freezing-rain'),
  67: W('Freezing rain', 'freezing-rain'),
  71: W('Light snow', 'snow'),
  73: W('Snow', 'snow'),
  75: W('Heavy snow', 'snow'),
  77: W('Snow grains', 'snow'),
  80: W('Light showers', 'showers'),
  81: W('Showers', 'showers'),
  82: W('Violent showers', 'showers'),
  85: W('Light snow showers', 'snow-showers'),
  86: W('Heavy snow showers', 'snow-showers'),
  95: W('Thunderstorm', 'thunder'),
  96: W('Thunderstorm with hail', 'thunder'),
  99: W('Thunderstorm with heavy hail', 'thunder'),
}

export function weatherInfo(code: number, night = false): { label: string; icon: WeatherIconName } {
  const info = WEATHER_CODES[code] ?? W('Unknown', 'cloudy')
  return { label: info.label, icon: night && info.night_icon ? info.night_icon : info.icon }
}

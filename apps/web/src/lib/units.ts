import type { UnitSystem } from './types'

export type Quantity = 'temp' | 'precip' | 'length' | 'speed'

const round = (n: number, d = 0) => {
  const f = 10 ** d
  return Math.round(n * f) / f
}

export const cToF = (c: number) => (c * 9) / 5 + 32
export const mmToIn = (mm: number) => mm / 25.4
export const cmToIn = (cm: number) => cm / 2.54
export const kmhToMph = (k: number) => k / 1.609344

export function convert(value: number, q: Quantity, units: UnitSystem): number {
  if (units === 'metric') return value
  switch (q) {
    case 'temp':
      return cToF(value)
    case 'precip':
      return mmToIn(value)
    case 'length':
      return cmToIn(value)
    case 'speed':
      return kmhToMph(value)
  }
}

export function unitSuffix(q: Quantity, units: UnitSystem): string {
  const m = units === 'metric'
  switch (q) {
    case 'temp':
      return m ? '°C' : '°F'
    case 'precip':
      return m ? 'mm' : 'in'
    case 'length':
      return m ? 'cm' : 'in'
    case 'speed':
      return m ? 'km/h' : 'mph'
  }
}

export function formatQuantity(value: number, q: Quantity, units: UnitSystem): string {
  const v = convert(value, q, units)
  const digits = q === 'precip' ? (units === 'imperial' ? 2 : 1) : q === 'length' ? 1 : 0
  const s = String(round(v, digits))
  return q === 'temp' ? `${s}${unitSuffix(q, units)}` : `${s} ${unitSuffix(q, units)}`
}

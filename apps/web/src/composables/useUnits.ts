import { computed, type Ref } from 'vue'
import { convert, formatQuantity, unitSuffix, type Quantity } from '../lib/units'
import type { UnitSystem } from '../lib/types'

export function useUnits(units: Ref<UnitSystem>) {
  return {
    format: (v: number, q: Quantity) => formatQuantity(v, q, units.value),
    convert: (v: number, q: Quantity) => convert(v, q, units.value),
    suffix: (q: Quantity) => unitSuffix(q, units.value),
    system: computed(() => units.value),
  }
}

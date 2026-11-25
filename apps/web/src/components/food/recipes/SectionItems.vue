<script setup lang="ts">
import { UiButton, UiIconButton, UiTextField } from '../../../ui'
import type { Place } from './sectionsForm'

const items = defineModel<string[]>('items', { required: true })
const props = defineProps<{
  kind: 'line' | 'step'
  place: Place
  /** Every place an item can go: the top level and each section. */
  targets: { value: Place; label: string }[]
  testPrefix?: string
}>()
const emit = defineEmits<{ move: [index: number, to: Place] }>()

const word = props.kind === 'line' ? 'line' : 'step'
function swap(i: number, d: number) {
  const j = i + d
  if (j < 0 || j >= items.value.length) return
  const l = [...items.value]
  ;[l[i], l[j]] = [l[j]!, l[i]!]
  items.value = l
}
const setAt = (i: number, v: string) => (items.value = items.value.map((x, k) => (k === i ? v : x)))
const removeAt = (i: number) => (items.value = items.value.filter((_, k) => k !== i))
const onMove = (i: number, e: Event) => {
  const sel = e.target as HTMLSelectElement
  const v = sel.value
  // Rows are keyed by index, so the next item reuses this <select>: put it back on the prompt.
  sel.value = ''
  emit('move', i, v === 'top' ? 'top' : Number(v))
}
</script>

<template>
  <div class="items">
    <div v-for="(_, i) in items" :key="i" class="row">
      <div class="grow">
        <UiTextField
          :model-value="items[i]!"
          :label="kind === 'line' ? 'Ingredient line' : `Step ${i + 1}`"
          :multiline="kind === 'step'"
          :data-testid="testPrefix ? `${testPrefix}-${i}` : undefined"
          @update:model-value="setAt(i, $event)"
        />
        <slot name="extra" :index="i" />
        <select
          v-if="targets.length > 1"
          class="moveto"
          :aria-label="`Move ${word} ${i + 1} to`"
          :data-testid="testPrefix ? `${testPrefix}-${i}-moveto` : undefined"
          @change="onMove(i, $event)"
        >
          <option value="" selected disabled>Move to…</option>
          <option
            v-for="t in targets.filter((x) => x.value !== place)"
            :key="String(t.value)"
            :value="String(t.value)"
          >
            {{ t.label }}
          </option>
        </select>
      </div>
      <UiIconButton icon="arrow_upward" :label="`Move ${word} up`" @click="swap(i, -1)" />
      <UiIconButton icon="arrow_downward" :label="`Move ${word} down`" @click="swap(i, 1)" />
      <UiIconButton icon="delete" :label="`Remove ${word}`" @click="removeAt(i)" />
    </div>
    <UiButton variant="tonal" icon="add" @click="items = [...items, '']">Add {{ word }}</UiButton>
  </div>
</template>

<style scoped>
.items {
  display: grid;
  gap: var(--ui-space-2);
}
.row {
  display: flex;
  gap: var(--ui-space-1);
  align-items: flex-start;
}
.grow {
  flex: 1;
  display: grid;
  gap: var(--ui-space-2);
}
.moveto {
  min-height: var(--ui-touch);
  font: inherit;
  color: var(--ui-on-surface);
  background: var(--ui-surface-container, transparent);
  border: 1px solid var(--ui-outline-variant, currentColor);
  border-radius: var(--ui-radius-md);
  padding: 0 var(--ui-space-2);
}
</style>

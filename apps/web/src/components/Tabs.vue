<script setup lang="ts" generic="K extends string">
import { nextTick, useTemplateRef } from 'vue'

const props = defineProps<{ tabs: readonly { key: K; label: string }[]; label?: string }>()
const model = defineModel<K>({ required: true })
const list = useTemplateRef<HTMLElement>('list')

function move(to: number) {
  const n = props.tabs.length
  const t = props.tabs[((to % n) + n) % n]
  if (!t) return
  model.value = t.key
  void nextTick(() => list.value?.querySelector<HTMLElement>('[aria-selected="true"]')?.focus())
}

function onKey(ev: KeyboardEvent) {
  const i = props.tabs.findIndex((t) => t.key === model.value)
  if (ev.key === 'ArrowRight') move(i + 1)
  else if (ev.key === 'ArrowLeft') move(i - 1)
  else if (ev.key === 'Home') move(0)
  else if (ev.key === 'End') move(props.tabs.length - 1)
  else return
  ev.preventDefault()
}
</script>

<template>
  <div ref="list" class="tabs" role="tablist" :aria-label="label" @keydown="onKey">
    <button
      v-for="t in tabs"
      :key="t.key"
      type="button"
      role="tab"
      :aria-selected="model === t.key"
      :tabindex="model === t.key ? 0 : -1"
      :class="{ on: model === t.key }"
      @click="model = t.key"
    >
      {{ t.label }}
    </button>
  </div>
</template>

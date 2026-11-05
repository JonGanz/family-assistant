<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useId, watch } from 'vue'

export interface MenuItem {
  id: string
  label: string
  disabled?: boolean
  danger?: boolean
}
defineProps<{ items: MenuItem[]; label: string }>()
const emit = defineEmits<{ select: [id: string] }>()
const open = ref(false)
const root = ref<HTMLElement | null>(null)
const menuId = useId()

function focusables(): HTMLElement[] {
  return [...(root.value?.querySelectorAll<HTMLElement>('[role="menuitem"]:not([disabled])') ?? [])]
}
function onDoc(e: Event) {
  if (root.value && !root.value.contains(e.target as Node)) open.value = false
}
watch(open, async (o) => {
  if (o) {
    document.addEventListener('pointerdown', onDoc)
    await nextTick()
    focusables()[0]?.focus()
  } else {
    document.removeEventListener('pointerdown', onDoc)
  }
})
onBeforeUnmount(() => document.removeEventListener('pointerdown', onDoc))

function onKey(e: KeyboardEvent) {
  const list = focusables()
  const i = list.indexOf(document.activeElement as HTMLElement)
  if (e.key === 'Escape') {
    open.value = false
    root.value?.querySelector<HTMLElement>('[aria-haspopup]')?.focus()
  } else if (e.key === 'ArrowDown') {
    e.preventDefault()
    list[(i + 1) % list.length]?.focus()
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    list[(i - 1 + list.length) % list.length]?.focus()
  } else if (e.key === 'Tab') {
    open.value = false
  }
}
function pick(id: string) {
  open.value = false
  emit('select', id)
}
</script>

<template>
  <div ref="root" class="ui-menu" @keydown="onKey">
    <slot
      name="activator"
      :toggle="() => (open = !open)"
      :open="open"
      :attrs="{ 'aria-haspopup': 'menu', 'aria-expanded': open, 'aria-controls': menuId }"
    />
    <div v-if="open" :id="menuId" class="ui-menu__list" role="menu" :aria-label="label">
      <button
        v-for="it in items"
        :key="it.id"
        type="button"
        role="menuitem"
        class="ui-menu__item"
        :class="{ 'is-danger': it.danger }"
        :disabled="it.disabled"
        @click="pick(it.id)"
      >
        {{ it.label }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.ui-menu {
  position: relative;
  display: inline-block;
}
.ui-menu__list {
  position: absolute;
  top: 100%;
  right: 0;
  z-index: 40;
  display: grid;
  min-width: 192px;
  padding: var(--ui-space-2) 0;
  border-radius: var(--ui-radius-xs);
  background: var(--ui-surface-container-low);
  box-shadow: var(--ui-elev-2);
  outline: 1px solid var(--ui-outline-variant);
}
.ui-menu__item {
  min-height: var(--ui-touch);
  padding: 0 var(--ui-space-4);
  border: 0;
  background: transparent;
  color: var(--ui-on-surface);
  font: var(--ui-type-label-lg);
  text-align: left;
  cursor: pointer;
}
.ui-menu__item:hover,
.ui-menu__item:focus-visible {
  background: var(--ui-state-hover);
  outline: none;
}
.ui-menu__item:focus-visible {
  background: var(--ui-state-focus);
}
.ui-menu__item.is-danger {
  color: var(--ui-error);
}
.ui-menu__item:disabled {
  opacity: 0.38;
  cursor: not-allowed;
}
</style>

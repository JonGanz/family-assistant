<script setup lang="ts">
import { computed, ref, toRef } from 'vue'
import I_menu_open from '~icons/material-symbols/menu-open-rounded'
import { RouterLink } from 'vue-router'
import UiBadge from './UiBadge.vue'
import UiIcon from './UiIcon.vue'
import { navGlyph } from './icons'
import type { NavDestination } from './nav'
import { useFocusTrap } from './composables/useFocusTrap'
import { useActivePath } from './navItem'

const props = defineProps<{
  destinations: NavDestination[]
  label?: string
  activePath?: string
  /** A modal overlay (compact windows): scrim, focus trap, Esc and scrim click close. Shown while `open`. */
  modal?: boolean
  open?: boolean
  /** Standard drawer only: shows the collapse toggle at the top. */
  collapsible?: boolean
}>()
const emit = defineEmits<{ close: []; collapse: [] }>()
const panel = ref<HTMLElement | null>(null)
useFocusTrap(
  toRef(() => !!props.modal && !!props.open),
  panel,
  () => emit('close'),
)
const path = useActivePath(() => props.activePath)
const active = (d: NavDestination) => (d.action ? !!d.current : path(d.to))
const linkAttrs = (d: NavDestination) =>
  d.action ? { type: 'button', onClick: d.action } : { to: d.to }
const groups = computed(() => {
  const out: { name: string; items: NavDestination[] }[] = []
  for (const d of props.destinations) {
    const name = d.group ?? ''
    let g = out.find((x) => x.name === name)
    if (!g) out.push((g = { name, items: [] }))
    g.items.push(d)
  }
  return out
})
</script>

<template>
  <Teleport v-if="modal" to="body">
    <div v-if="open" class="ui-drawer-modal">
      <div class="ui-drawer-modal__scrim" data-testid="scrim" @click="emit('close')" />
      <div
        ref="panel"
        class="ui-drawer-modal__panel"
        role="dialog"
        aria-modal="true"
        aria-label="Navigation"
        tabindex="-1"
      >
        <nav class="ui-drawer is-modal" :aria-label="label ?? 'Primary'">
          <template v-for="g in groups" :key="g.name">
            <section>
              <h2 v-if="g.name" class="ui-drawer__group">{{ g.name }}</h2>
              <component
                :is="d.action ? 'button' : RouterLink"
                v-for="d in g.items"
                :key="d.id"
                v-bind="linkAttrs(d)"
                class="ui-drawer__item"
                :class="{ 'is-active': active(d) }"
                :aria-current="active(d) ? 'page' : undefined"
              >
                <UiIcon :name="navGlyph(d.icon, active(d))" />
                <span class="ui-drawer__label">{{ d.label }}</span>
                <UiBadge v-if="d.badge" :count="d.badge" />
              </component>
            </section>
          </template>
        </nav>
        <div v-if="$slots.footer" class="ui-drawer__footer"><slot name="footer" /></div>
      </div>
    </div>
  </Teleport>
  <div v-else class="ui-drawer-wrap">
    <nav class="ui-drawer" :aria-label="label ?? 'Primary'">
      <button
        v-if="collapsible"
        type="button"
        class="ui-drawer__toggle"
        aria-label="Collapse navigation"
        title="Collapse navigation"
        aria-expanded="true"
        @click="emit('collapse')"
      >
        <I_menu_open aria-hidden="true" />
      </button>
      <section v-for="g in groups" :key="g.name">
        <h2 v-if="g.name" class="ui-drawer__group">{{ g.name }}</h2>
        <component
          :is="d.action ? 'button' : RouterLink"
          v-for="d in g.items"
          :key="d.id"
          v-bind="linkAttrs(d)"
          class="ui-drawer__item"
          :class="{ 'is-active': active(d) }"
          :aria-current="active(d) ? 'page' : undefined"
        >
          <UiIcon :name="navGlyph(d.icon, active(d))" />
          <span class="ui-drawer__label">{{ d.label }}</span>
          <UiBadge v-if="d.badge" :count="d.badge" />
        </component>
      </section>
    </nav>
    <div v-if="$slots.footer" class="ui-drawer__footer"><slot name="footer" /></div>
  </div>
</template>

<style scoped>
.ui-drawer-wrap {
  display: flex;
  flex-direction: column;
  min-height: 100%;
  background: var(--ui-surface-container-low);
}
.ui-drawer__footer {
  margin-top: auto;
  padding: var(--ui-space-2) var(--ui-space-3) var(--ui-space-4);
}
.ui-drawer__toggle {
  align-self: flex-start;
  display: flex;
  align-items: center;
  justify-content: center;
  width: var(--ui-touch);
  height: var(--ui-touch);
  border: 0;
  border-radius: var(--ui-radius-full);
  background: none;
  color: var(--ui-on-surface-variant);
  font-size: 1.5rem;
  cursor: pointer;
}
.ui-drawer__toggle:hover {
  background: var(--ui-state-hover);
}
.ui-drawer__toggle:focus-visible {
  outline: var(--ui-focus-ring);
}
.ui-drawer-modal {
  position: fixed;
  inset: 0;
  z-index: 50;
}
.ui-drawer-modal__scrim {
  position: absolute;
  inset: 0;
  background: var(--ui-scrim);
}
.ui-drawer-modal__panel {
  position: relative;
  display: flex;
  flex-direction: column;
  width: min(320px, 85vw);
  height: 100%;
  overflow-y: auto;
  border-radius: 0 var(--ui-radius-xl) var(--ui-radius-xl) 0;
  background: var(--ui-surface-container-low);
  padding-top: env(safe-area-inset-top);
  outline: none;
}
.ui-drawer.is-modal {
  width: auto;
  border-radius: 0;
}
.ui-drawer {
  display: flex;
  flex-direction: column;
  gap: var(--ui-space-3);
  width: 296px;
  padding: var(--ui-space-4) var(--ui-space-3);
  border-radius: 0 var(--ui-radius-xl) var(--ui-radius-xl) 0;
  background: var(--ui-surface-container-low);
  color: var(--ui-on-surface-variant);
}
.ui-drawer__group {
  margin: 0;
  padding: var(--ui-space-2) var(--ui-space-4);
  font: var(--ui-type-title-sm);
  color: var(--ui-on-surface-variant);
}
.ui-drawer__item {
  width: 100%;
  text-align: left;
  border: 0;
  background: none;
  cursor: pointer;
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--ui-space-3);
  min-height: 56px;
  padding: 0 var(--ui-space-4);
  border-radius: var(--ui-radius-full);
  color: inherit;
  font: var(--ui-type-label-lg);
  text-decoration: none;
}
.ui-drawer__label {
  flex: 1;
}
.ui-drawer__item:hover {
  background: var(--ui-state-hover);
}
.ui-drawer__item.is-active {
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.ui-drawer__item:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
</style>

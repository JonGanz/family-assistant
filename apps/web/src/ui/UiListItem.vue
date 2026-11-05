<script setup lang="ts">
import { computed, getCurrentInstance } from 'vue'
import { RouterLink } from 'vue-router'

const props = defineProps<{
  headline: string
  supporting?: string
  overline?: string
  to?: string
  href?: string
  disabled?: boolean
}>()
defineEmits<{ click: [e: MouseEvent] }>()
const clickable = !!getCurrentInstance()?.vnode.props?.onClick
const tag = computed(() =>
  props.to ? RouterLink : props.href ? 'a' : clickable ? 'button' : 'div',
)
// An undefined href passed to RouterLink would replace the one it computes.
const linkAttrs = computed(() =>
  props.to ? { to: props.to } : props.href ? { href: props.href } : {},
)
const interactive = computed(() => tag.value !== 'div')
const lines = computed(() => 1 + (props.supporting ? 1 : 0) + (props.overline ? 1 : 0))
</script>

<template>
  <li class="ui-li" :class="`lines-${lines}`">
    <component
      :is="tag"
      class="ui-li__row"
      :class="{ 'is-interactive': interactive }"
      v-bind="linkAttrs"
      :type="tag === 'button' ? 'button' : undefined"
      :disabled="tag === 'button' ? disabled : undefined"
      @click="$emit('click', $event)"
    >
      <span v-if="$slots.leading" class="ui-li__lead"><slot name="leading" /></span>
      <span class="ui-li__text">
        <span v-if="overline" class="ui-li__overline">{{ overline }}</span>
        <span class="ui-li__headline">{{ headline }}</span>
        <span v-if="supporting" class="ui-li__supporting">{{ supporting }}</span>
      </span>
      <span v-if="$slots.trailing" class="ui-li__trail"><slot name="trailing" /></span>
    </component>
  </li>
</template>

<style scoped>
.ui-li {
  display: block;
}
.ui-li__row {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--ui-space-4);
  width: 100%;
  min-height: var(--ui-row-height);
  padding: var(--ui-space-2) var(--ui-space-4);
  border: 0;
  background: transparent;
  color: var(--ui-on-surface);
  font: inherit;
  text-align: left;
  text-decoration: none;
  isolation: isolate;
}
.lines-2 .ui-li__row {
  min-height: calc(var(--ui-row-height) + 16px);
}
.lines-3 .ui-li__row {
  min-height: calc(var(--ui-row-height) + 32px);
}
.is-interactive {
  cursor: pointer;
}
.is-interactive::before {
  content: '';
  position: absolute;
  inset: 0;
  z-index: -1;
}
.is-interactive:hover::before {
  background: var(--ui-state-hover);
}
.is-interactive:active::before {
  background: var(--ui-state-pressed);
}
.is-interactive:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: -2px;
}
.ui-li__lead {
  display: inline-flex;
  flex: none;
  color: var(--ui-on-surface-variant);
}
.ui-li__text {
  display: grid;
  flex: 1;
  min-width: 0;
}
.ui-li__overline {
  font: var(--ui-type-label-sm);
  color: var(--ui-on-surface-variant);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.ui-li__headline {
  font: var(--ui-type-body-lg);
  overflow-wrap: anywhere;
}
.ui-li__supporting {
  font: var(--ui-type-body-md);
  color: var(--ui-on-surface-variant);
  overflow-wrap: anywhere;
}
.ui-li__trail {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: var(--ui-space-2);
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-label-md);
}
</style>

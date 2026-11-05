<script setup lang="ts">
import UiIcon from './UiIcon.vue'
import type { IconName } from './icons'

withDefaults(
  defineProps<{
    icon: IconName
    /** Required: the accessible name of an icon-only control. */
    label: string
    variant?: 'standard' | 'filled' | 'tonal'
    pressed?: boolean
    disabled?: boolean
    type?: 'button' | 'submit'
  }>(),
  { variant: 'standard', type: 'button', pressed: undefined },
)
</script>

<template>
  <button
    class="ui-iconbtn"
    :class="[`ui-iconbtn--${variant}`, { 'is-on': pressed }]"
    :type="type"
    :aria-label="label"
    :aria-pressed="pressed === undefined ? undefined : pressed"
    :title="label"
    :disabled="disabled"
  >
    <UiIcon :name="icon" />
  </button>
</template>

<style scoped>
.ui-iconbtn {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--ui-touch);
  height: var(--ui-touch);
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--ui-on-surface-variant);
  cursor: pointer;
  isolation: isolate;
}
.ui-iconbtn::before {
  content: '';
  position: absolute;
  inset: 4px;
  border-radius: var(--ui-radius-full);
  background: transparent;
  z-index: -1;
}
.ui-iconbtn:hover::before {
  background: var(--ui-state-hover);
}
.ui-iconbtn:focus-visible::before {
  background: var(--ui-state-focus);
}
.ui-iconbtn:active::before {
  background: var(--ui-state-pressed);
}
.ui-iconbtn:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: -2px;
  border-radius: var(--ui-radius-full);
}
.ui-iconbtn--filled {
  color: var(--ui-on-primary);
}
.ui-iconbtn--filled::before {
  background: var(--ui-primary);
}
.ui-iconbtn--filled:hover::before,
.ui-iconbtn--filled:focus-visible::before,
.ui-iconbtn--filled:active::before {
  background: color-mix(in srgb, var(--ui-primary) 88%, var(--ui-on-primary));
}
.ui-iconbtn--tonal,
.ui-iconbtn.is-on {
  color: var(--ui-on-primary-container);
}
.ui-iconbtn--tonal::before,
.ui-iconbtn.is-on::before {
  background: var(--ui-primary-container);
}
.ui-iconbtn:disabled {
  cursor: not-allowed;
  color: color-mix(in srgb, var(--ui-on-surface) 38%, transparent);
}
.ui-iconbtn:disabled::before {
  background: transparent;
}
</style>

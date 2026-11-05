<script setup lang="ts">
import { computed, useAttrs, useId } from 'vue'

defineOptions({ inheritAttrs: false })
const props = defineProps<{
  modelValue: string
  label: string
  supporting?: string
  error?: string
  prefix?: string
  suffix?: string
  type?: string
  multiline?: boolean
  disabled?: boolean
  required?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const attrs = useAttrs()
// class and style stay on the wrapper; everything else goes to the native control.
const isWrapperAttr = (k: string) => k === 'class' || k === 'style'
const wrapperAttrs = computed(() =>
  Object.fromEntries(Object.entries(attrs).filter(([k]) => isWrapperAttr(k))),
)
const inputAttrs = computed(() =>
  Object.fromEntries(Object.entries(attrs).filter(([k]) => !isWrapperAttr(k))),
)
const uid = useId()
const id = computed(() => (attrs.id as string | undefined) ?? uid)
const descId = computed(() => (props.error || props.supporting ? `${id.value}-d` : undefined))
function onInput(e: Event) {
  emit('update:modelValue', (e.target as HTMLInputElement | HTMLTextAreaElement).value)
}
</script>

<template>
  <div
    class="ui-field"
    :class="{ 'has-error': error, 'is-disabled': disabled }"
    v-bind="wrapperAttrs"
  >
    <label :for="id" class="ui-field__label"
      >{{ label }}<span v-if="required" aria-hidden="true"> *</span></label
    >
    <div class="ui-field__box">
      <span v-if="prefix" class="ui-field__fix">{{ prefix }}</span>
      <textarea
        v-if="multiline"
        :id="id"
        class="ui-field__input"
        rows="3"
        :value="modelValue"
        :disabled="disabled"
        :required="required"
        :aria-invalid="error ? true : undefined"
        :aria-describedby="descId"
        v-bind="inputAttrs"
        @input="onInput"
      />
      <input
        v-else
        :id="id"
        class="ui-field__input"
        :type="type ?? 'text'"
        :value="modelValue"
        :disabled="disabled"
        :required="required"
        :aria-invalid="error ? true : undefined"
        :aria-describedby="descId"
        v-bind="inputAttrs"
        @input="onInput"
      />
      <span v-if="suffix" class="ui-field__fix">{{ suffix }}</span>
      <slot name="trailing" />
    </div>
    <p
      v-if="error || supporting"
      :id="descId"
      class="ui-field__help"
      :role="error ? 'alert' : undefined"
    >
      {{ error || supporting }}
    </p>
  </div>
</template>

<style scoped>
.ui-field {
  display: grid;
  gap: var(--ui-space-1);
  min-width: 0;
}
.ui-field__label {
  font: var(--ui-type-label-lg);
  color: var(--ui-on-surface-variant);
}
.ui-field__box {
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
  min-height: var(--ui-touch);
  padding: 0 var(--ui-space-4);
  border: 1px solid var(--ui-outline);
  border-radius: var(--ui-radius-sm);
  background: var(--ui-surface-container-lowest);
  color: var(--ui-on-surface);
}
.ui-field__box:hover {
  border-color: var(--ui-on-surface);
}
.ui-field__box:focus-within {
  border-color: var(--ui-primary);
  outline: 1px solid var(--ui-primary);
}
.ui-field__input {
  flex: 1;
  min-width: 0;
  padding: var(--ui-space-3) 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: inherit;
  font: var(--ui-type-body-lg);
  resize: vertical;
}
.ui-field__fix {
  font: var(--ui-type-body-lg);
  color: var(--ui-on-surface-variant);
}
.ui-field__help {
  margin: 0;
  padding: 0 var(--ui-space-4);
  font: var(--ui-type-body-sm);
  color: var(--ui-on-surface-variant);
}
.has-error .ui-field__box {
  border-color: var(--ui-error);
}
.has-error .ui-field__box:focus-within {
  outline-color: var(--ui-error);
}
.has-error .ui-field__label,
.has-error .ui-field__help {
  color: var(--ui-error);
}
.is-disabled {
  opacity: 0.5;
}
</style>

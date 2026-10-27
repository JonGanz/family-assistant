<script setup lang="ts">
withDefaults(
  defineProps<{
    back?: boolean
    next?: string
    disabled?: boolean
    skip?: boolean
    busy?: boolean
  }>(),
  { back: true, next: 'Next' },
)
defineEmits<{ back: []; skip: [] }>()
</script>

<template>
  <div class="row step-buttons">
    <button v-if="back" type="button" class="btn-ghost" @click="$emit('back')">Back</button>
    <button v-if="skip" type="button" class="btn-ghost" @click="$emit('skip')">Skip for now</button>
    <button type="submit" class="btn-primary" :disabled="disabled || busy">{{ next }}</button>
  </div>
</template>

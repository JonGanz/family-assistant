<script setup lang="ts">
import UiIcon from '../../../ui/UiIcon.vue'

defineProps<{ modelValue: number | null; label: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: number | null] }>()
</script>

<template>
  <div class="stars" role="group" :aria-label="label" data-testid="recipe-rating">
    <button
      v-for="n in 5"
      :key="n"
      type="button"
      class="star"
      :aria-label="`${n} ${n === 1 ? 'star' : 'stars'}`"
      :aria-pressed="modelValue === n"
      :data-testid="`recipe-rating-${n}`"
      @click="emit('update:modelValue', modelValue === n ? null : n)"
    >
      <UiIcon :name="(modelValue ?? 0) >= n ? 'star_filled' : 'star'" :size="24" />
    </button>
  </div>
</template>

<style scoped>
.stars {
  display: flex;
}
.star {
  min-width: 48px;
  min-height: 48px;
  display: grid;
  place-items: center;
  border: 0;
  background: none;
  color: var(--ui-primary);
  cursor: pointer;
}
</style>

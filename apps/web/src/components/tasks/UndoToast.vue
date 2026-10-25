<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'

const props = withDefaults(defineProps<{ message: string; seconds?: number }>(), { seconds: 10 })
const emit = defineEmits<{ undo: []; expire: [] }>()

const left = ref(props.seconds)
let timer: ReturnType<typeof setInterval> | undefined

onMounted(() => {
  timer = setInterval(() => {
    left.value = Math.max(0, left.value - 1)
    if (left.value === 0) {
      clearInterval(timer)
      emit('expire')
    }
  }, 1000)
})
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <div class="toast" role="status" aria-live="polite">
    <span class="msg">{{ message }}</span>
    <span class="left" data-testid="countdown">{{ left }}s</span>
    <button type="button" class="undo" @click="emit('undo')">Undo</button>
  </div>
</template>

<style scoped>
.toast {
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-4);
  background: var(--color-text);
  color: var(--color-bg);
  border-radius: var(--radius-md);
  box-shadow: 0 4px 16px rgb(0 0 0 / 0.25);
}
.left {
  font-variant-numeric: tabular-nums;
  opacity: 0.7;
}
.undo {
  min-height: var(--min-touch, 48px);
  padding: 0 var(--space-3);
  background: none;
  border: 0;
  color: inherit;
  font: inherit;
  font-weight: 700;
  text-decoration: underline;
  cursor: pointer;
}
</style>

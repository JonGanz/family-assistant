<script setup lang="ts">
import { ref } from 'vue'
import { useMailSettingsStore } from '../../../stores/mailSettings'
import { describeError } from '../../shared'

const store = useMailSettingsStore()
const error = ref('')

async function toggle(key: string, enabled: boolean) {
  error.value = ''
  try {
    await store.setSignal(key, enabled)
  } catch (e) {
    error.value = describeError(e, 'Could not save.')
  }
}
</script>

<template>
  <div>
    <p v-if="error" class="text-danger" role="alert">{{ error }}</p>
    <ul class="card-list">
      <li v-for="s in store.signals" :key="s.key">
        <label class="field-inline">
          <input
            type="checkbox"
            :checked="s.enabled"
            :aria-label="s.key"
            @change="toggle(s.key, ($event.target as HTMLInputElement).checked)"
          />
          <span>
            {{ s.description }}
            <span class="muted">({{ s.decision_point }}, confidence {{ s.confidence }})</span>
          </span>
        </label>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.inline {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
</style>

<script setup lang="ts">
import { computed, ref } from 'vue'
import Modal from '../Modal.vue'
import UiButton from '../../ui/UiButton.vue'
import { S } from '../strings'
import { durationMs, PRESET_MINUTES } from './timersModel'

const emit = defineEmits<{ close: []; save: [name: string, ms: number] }>()
const minutes = ref(10)
const seconds = ref(0)
const name = ref('')
const ms = computed(() => durationMs(minutes.value, seconds.value))

function preset(m: number) {
  minutes.value = m
  seconds.value = 0
}
function step(field: 'm' | 's', d: number) {
  if (field === 'm') minutes.value = Math.min(99, Math.max(0, (Number(minutes.value) || 0) + d))
  else {
    const t = Math.max(0, (Number(seconds.value) || 0) + d)
    seconds.value = t % 60
    if (t >= 60 && d > 0) minutes.value = Math.min(99, (Number(minutes.value) || 0) + 1)
  }
}
function save() {
  if (ms.value <= 0) return
  emit('save', name.value, ms.value)
}
</script>

<template>
  <Modal :title="S.timerAdd" @close="emit('close')">
    <form class="form" data-testid="timer-form" @submit.prevent="save">
      <div class="dur">
        <div class="field">
          <UiButton
            variant="tonal"
            icon="add"
            class="step"
            :aria-label="S.timerMinPlus"
            @click="step('m', 1)"
          />
          <input
            v-model.number="minutes"
            type="number"
            inputmode="numeric"
            min="0"
            max="99"
            :aria-label="S.timerMinutes"
            data-testid="timer-minutes"
          />
          <UiButton
            variant="tonal"
            class="step"
            :aria-label="S.timerMinMinus"
            @click="step('m', -1)"
          >
            <span class="minus" aria-hidden="true">–</span>
          </UiButton>
          <span class="unit">{{ S.timerMin }}</span>
        </div>
        <span class="colon">:</span>
        <div class="field">
          <UiButton
            variant="tonal"
            icon="add"
            class="step"
            :aria-label="S.timerSecPlus"
            @click="step('s', 5)"
          />
          <input
            v-model.number="seconds"
            type="number"
            inputmode="numeric"
            min="0"
            max="59"
            :aria-label="S.timerSeconds"
            data-testid="timer-seconds"
          />
          <UiButton
            variant="tonal"
            class="step"
            :aria-label="S.timerSecMinus"
            @click="step('s', -5)"
          >
            <span class="minus" aria-hidden="true">–</span>
          </UiButton>
          <span class="unit">{{ S.timerSec }}</span>
        </div>
      </div>
      <div class="presets" role="group" :aria-label="S.timerPresets">
        <UiButton
          v-for="m in PRESET_MINUTES"
          :key="m"
          variant="tonal"
          :data-testid="`timer-preset-${m}`"
          @click="preset(m)"
        >
          {{ m }} {{ S.timerMin }}
        </UiButton>
      </div>
      <label class="name">
        <span>{{ S.timerName }}</span>
        <input
          v-model="name"
          type="text"
          maxlength="40"
          :placeholder="S.timerDefaultName"
          data-testid="timer-name"
        />
      </label>
      <div class="actions">
        <UiButton variant="tonal" data-testid="timer-cancel" @click="emit('close')">
          {{ S.timerCancelAdd }}
        </UiButton>
        <UiButton type="submit" :disabled="ms <= 0" data-testid="timer-save">
          {{ S.timerSave }}
        </UiButton>
      </div>
    </form>
  </Modal>
</template>

<style scoped>
.form {
  display: grid;
  gap: var(--space-4);
  min-width: min(28rem, 80vw);
}
.dur {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
}
.field {
  display: grid;
  justify-items: center;
  gap: var(--space-2);
}
.field input {
  width: 6rem;
  min-height: var(--min-touch);
  box-sizing: border-box;
  border: 1px solid var(--color-border);
  border-radius: var(--ui-radius-md);
  background: var(--color-layer-1, var(--color-surface));
  color: inherit;
  font: inherit;
  font-size: 2.4rem;
  font-variant-numeric: tabular-nums;
  text-align: center;
}
.colon {
  font-size: 2.4rem;
  font-weight: 700;
}
.unit {
  color: var(--color-muted);
  font-size: var(--font-size-sm, 0.9rem);
}
.step {
  width: 100%;
}
.minus {
  font-size: 1.6rem;
  line-height: 1;
}
.presets {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: var(--space-2);
}
.name {
  display: grid;
  gap: var(--space-1);
}
.name input {
  min-height: var(--min-touch);
  box-sizing: border-box;
  padding-inline: var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--ui-radius-md);
  background: var(--color-layer-1, var(--color-surface));
  color: inherit;
  font: inherit;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-3);
}
</style>

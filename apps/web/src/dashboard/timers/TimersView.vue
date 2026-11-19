<script setup lang="ts">
import { ref } from 'vue'
import UiButton from '../../ui/UiButton.vue'
import { S } from '../strings'
import AddTimerModal from './AddTimerModal.vue'
import TimerCard from './TimerCard.vue'
import { useTimers } from './useTimers'

const tm = useTimers()
const adding = ref(false)
function save(name: string, ms: number) {
  tm.add(name, ms)
  adding.value = false
}
</script>

<template>
  <section class="timers" data-testid="timers-view">
    <UiButton class="add" icon="add" data-testid="timer-add" @click="adding = true">
      {{ S.timerAdd }}
    </UiButton>
    <p v-if="tm.timers.value.length === 0" class="empty">{{ S.timerNone }}</p>
    <div class="cards">
      <TimerCard
        v-for="t in tm.timers.value"
        :key="t.id"
        :timer="t"
        :now="tm.now.value"
        @add-time="tm.addTime(t.id)"
        @pause="tm.pause(t.id)"
        @resume="tm.resume(t.id)"
        @cancel="tm.cancel(t.id)"
        @dismiss="tm.dismiss(t.id)"
        @remove="tm.remove(t.id)"
      />
    </div>
    <AddTimerModal v-if="adding" @close="adding = false" @save="save" />
  </section>
</template>

<style scoped>
.timers {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  display: grid;
  align-content: start;
  gap: var(--space-5);
  padding: var(--space-5);
  position: relative;
}
.add {
  width: 100%;
  height: max(var(--ui-touch, 48px), 2.4em);
  font: inherit;
  font-size: var(--kiosk-font-small);
  font-weight: 600;
}
.add :deep(svg) {
  width: 1.1em;
  height: 1.1em;
}
.empty {
  margin: 0;
  text-align: center;
  color: var(--color-muted);
}
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 19rem), 1fr));
  gap: var(--space-5);
}
</style>

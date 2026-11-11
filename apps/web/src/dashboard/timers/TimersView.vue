<script setup lang="ts">
import { ref } from 'vue'
import UiIcon from '../../ui/UiIcon.vue'
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
    <button type="button" class="add" data-testid="timer-add" @click="adding = true">
      <UiIcon name="add" :size="24" /> {{ S.timerAdd }}
    </button>
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
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  width: 100%;
  min-height: 5rem;
  border: 0;
  border-radius: var(--ui-radius-xl);
  background: var(--color-accent);
  color: var(--color-on-accent);
  font: inherit;
  font-size: var(--font-size-lg);
  font-weight: 700;
  cursor: pointer;
}
.empty {
  margin: 0;
  text-align: center;
  color: var(--color-muted);
}
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr));
  gap: var(--space-5);
}
</style>

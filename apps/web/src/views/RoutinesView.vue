<script setup lang="ts">
import PageHeader from '../components/PageHeader.vue'
import StateBlock from '../components/StateBlock.vue'
import ErrorBanner from '../components/ErrorBanner.vue'
import { computed, onMounted, ref } from 'vue'
import type { RoutineRunView } from '../api/client'
import MemberChip from '../components/MemberChip.vue'
import ProgressBar from '../components/tasks/ProgressBar.vue'
import { useMembersStore } from '../stores/members'
import { useRoutinesStore } from '../stores/routines'
import { describeError } from './shared'

const routines = useRoutinesStore()
const members = useMembersStore()
const error = ref('')

onMounted(() => {
  void members.load()
  void routines.loadToday()
})

const groups = computed(() => {
  const out: { id: string; name: string; color: string; runs: RoutineRunView[] }[] = []
  for (const r of routines.today) {
    let g = out.find((x) => x.id === r.member)
    if (!g) {
      const m = members.items.find((x) => x.id === r.member)
      g = { id: r.member, name: m?.display_name ?? '?', color: m?.color ?? '#888888', runs: [] }
      out.push(g)
    }
    g.runs.push(r)
  }
  return out
})

async function tick(run: RoutineRunView, stepId: string, done: boolean) {
  error.value = ''
  try {
    await routines.tick(run, stepId, done)
  } catch (e) {
    error.value = describeError(e, 'Could not update the step.')
  }
}
</script>

<template>
  <section>
    <PageHeader title="Routines" />
    <StateBlock
      :loading="!routines.todayLoaded && !routines.error"
      :error="routines.error"
      :empty="routines.todayLoaded && groups.length === 0"
      empty-text="No routines today."
    />
    <ErrorBanner v-if="error" :message="error" />
    <section v-for="g in groups" :key="g.id" class="member">
      <h2><MemberChip :name="g.name" :color="g.color" /></h2>
      <div v-for="r in g.runs" :key="r.id" class="run card" :data-testid="`run-${r.routine_name}`">
        <h3>{{ r.routine_name }}</h3>
        <ProgressBar :done="r.done" :total="r.total" :label="`${r.routine_name} progress`" />
        <ul class="steps">
          <li v-for="s in r.steps" :key="s.step" :class="{ done: !!s.done_at }">
            <label>
              <input
                type="checkbox"
                :checked="!!s.done_at"
                @change="tick(r, s.step, ($event.target as HTMLInputElement).checked)"
              />
              {{ s.text }}
              <span v-if="s.target_time" class="muted">by {{ s.target_time.slice(0, 5) }}</span>
            </label>
          </li>
        </ul>
      </div>
    </section>
  </section>
</template>

<style scoped>
.run {
  margin-bottom: var(--space-3);
}
.steps {
  list-style: none;
  padding: 0;
  display: grid;
  gap: var(--space-1);
}
.done label {
  text-decoration: line-through;
  color: var(--color-muted);
}
h3 {
  margin: 0 0 var(--space-2);
}
</style>

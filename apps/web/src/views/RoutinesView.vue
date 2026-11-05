<script setup lang="ts">
import { computed, onMounted } from 'vue'
import type { RoutineRunView } from '../api/client'
import MemberChip from '../components/MemberChip.vue'
import { hueVar } from '../lib/hues'
import { useMembersStore } from '../stores/members'
import { useRoutinesStore } from '../stores/routines'
import {
  UiCard,
  UiCheckbox,
  UiEmptyState,
  UiErrorState,
  UiList,
  UiPage,
  UiProgress,
  UiSkeleton,
  UiTopAppBar,
  useSnackbar,
} from '../ui'
import { describeError } from './shared'

const routines = useRoutinesStore()
const members = useMembersStore()
const snackbar = useSnackbar()

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
      g = { id: r.member, name: m?.display_name ?? '?', color: hueVar(m?.color), runs: [] }
      out.push(g)
    }
    g.runs.push(r)
  }
  return out
})
const loading = computed(() => !routines.todayLoaded && !routines.error)

// Ticking a step is instant with Undo (D-096): Undo ticks it back.
async function tick(run: RoutineRunView, stepId: string, text: string, done: boolean) {
  try {
    await routines.tick(run, stepId, done)
  } catch (e) {
    snackbar.show({ text: describeError(e, 'Could not update the step.') })
    return
  }
  snackbar.show({
    text: done ? `Done: ${text}` : `Unticked: ${text}`,
    action: {
      label: 'Undo',
      run: async () => {
        const current = routines.today.find((r) => r.id === run.id) ?? run
        try {
          await routines.tick(current, stepId, !done)
        } catch (e) {
          snackbar.show({ text: describeError(e, 'Could not undo.') })
        }
      },
    },
  })
}
</script>

<template>
  <UiPage>
    <template #top>
      <UiTopAppBar title="Routines" />
    </template>

    <UiSkeleton v-if="loading" :lines="5" />
    <UiErrorState
      v-else-if="routines.error"
      :message="routines.error"
      @retry="routines.loadToday()"
    />
    <UiEmptyState v-else-if="groups.length === 0" icon="routines" title="No routines today" />

    <section v-for="g in groups" :key="g.id" class="member" :aria-label="g.name">
      <h2 class="who"><MemberChip :name="g.name" :color="g.color" variant="app" /></h2>
      <div class="runs">
        <UiCard
          v-for="r in g.runs"
          :key="r.id"
          variant="filled"
          :data-testid="`run-${r.routine_name}`"
        >
          <template #header>
            <h3>{{ r.routine_name }}</h3>
          </template>
          <UiProgress
            :value="r.total ? (r.done / r.total) * 100 : 0"
            :label="`${r.routine_name} progress`"
          />
          <p class="count">{{ r.done }} of {{ r.total }}</p>
          <UiList :aria-label="`${r.routine_name} steps`">
            <li v-for="s in r.steps" :key="s.step" class="step" :class="{ done: !!s.done_at }">
              <UiCheckbox
                :model-value="!!s.done_at"
                @update:model-value="tick(r, s.step, s.text, $event)"
              >
                {{ s.text }}
                <span v-if="s.target_time" class="by">by {{ s.target_time.slice(0, 5) }}</span>
              </UiCheckbox>
            </li>
          </UiList>
        </UiCard>
      </div>
    </section>
  </UiPage>
</template>

<style scoped>
.member {
  display: grid;
  gap: var(--ui-space-2);
}
.who {
  margin: 0;
}
.runs {
  display: grid;
  gap: var(--ui-space-3);
  grid-template-columns: 1fr;
}
@media (min-width: 600px) {
  .runs {
    grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
  }
}
h3 {
  margin: 0;
  font: var(--ui-type-title-md);
}
.count {
  margin: var(--ui-space-1) 0 0;
  font: var(--ui-type-body-md);
  color: var(--ui-on-surface-variant);
}
.step {
  min-height: var(--ui-touch);
  display: flex;
  align-items: center;
}
.step.done {
  color: var(--ui-on-surface-variant);
  text-decoration: line-through;
}
.by {
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>

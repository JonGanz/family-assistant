<script setup lang="ts">
import type { EventLike, MemberLike } from '../../lib/types'
import { dueLabel } from '../../lib/tasks'
import EventCard from '../../components/EventCard.vue'
import type { TaskView } from '../../api/client'
import { UiCard, UiCheckbox, UiList, UiSection } from '../../ui'
import type { MealSlotCard } from './todayModel'

defineProps<{
  meals: MealSlotCard[]
  tasks: TaskView[]
  events: EventLike[]
  members?: MemberLike[]
  tz: string
  /** Tasks can be completed (today only). */
  completable?: boolean
}>()
const emit = defineEmits<{ complete: [task: TaskView] }>()
</script>

<template>
  <div class="day">
    <UiSection v-for="m in meals" :key="m.slot" :heading="m.slot" data-testid="meal-card">
      <UiCard variant="filled">
        <UiList :aria-label="m.slot">
          <li v-for="i in m.items" :key="i.id" class="row" data-testid="meal-item">
            <RouterLink
              v-if="i.recipeId"
              :to="`/food/recipes/${i.recipeId}`"
              data-testid="meal-recipe"
            >
              {{ i.title }}
            </RouterLink>
            <span v-else>{{ i.title }}</span>
          </li>
        </UiList>
      </UiCard>
    </UiSection>

    <UiSection heading="Tasks" data-testid="tasks-section">
      <UiCard variant="filled">
        <p v-if="!tasks.length" class="empty">Nothing due.</p>
        <UiList v-else aria-label="Tasks">
          <li v-for="t in tasks" :key="t.id" class="row">
            <UiCheckbox
              v-if="completable"
              :model-value="false"
              :label="t.title"
              @update:model-value="emit('complete', t)"
            />
            <template v-else>
              <span>{{ t.title }}</span>
              <span class="muted">{{ dueLabel(t.due_at, new Date(), tz) }}</span>
            </template>
          </li>
        </UiList>
      </UiCard>
    </UiSection>

    <UiSection heading="Calendar" data-testid="events-section">
      <p v-if="!events.length" class="empty">Nothing planned.</p>
      <ul v-else class="events">
        <li v-for="e in events" :key="e.id" data-testid="event-row">
          <EventCard :event="e" :tz="tz" :members="members" />
        </li>
      </ul>
    </UiSection>
  </div>
</template>

<style scoped>
.day {
  display: grid;
  gap: var(--ui-space-4);
}
.row {
  padding: var(--ui-space-1) var(--ui-space-3);
  min-height: var(--ui-touch);
  display: flex;
  align-items: center;
  gap: var(--ui-space-2);
}
.muted,
.empty {
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
  margin: 0;
}
.events {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--ui-space-2);
}
</style>

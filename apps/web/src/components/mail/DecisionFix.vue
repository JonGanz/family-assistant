<script setup lang="ts">
import { computed } from 'vue'
import type { CategoryView, DecisionPointKey, DecisionView, Member } from '../../api/client'
import { currentDecision, describeValue, memberLinkIds, POINT_TITLES } from './decisions'

const props = defineProps<{
  point: DecisionPointKey
  decisions: DecisionView[]
  categories: CategoryView[]
  members: Member[]
  /** Chip is shown open (labelling view) instead of behind a Fix button. */
  open?: boolean
}>()
const emit = defineEmits<{ fix: [value: unknown] }>()

const decision = computed(() => currentDecision(props.decisions, props.point))
const value = computed(() =>
  props.point === 'email.member_link'
    ? (memberLinkIds(props.decisions) ?? [])
    : decision.value?.value,
)
const text = computed(() =>
  describeValue(props.point, value.value, {
    member: (id) => props.members.find((m) => m.id === id)?.display_name ?? '?',
    category: (k) => props.categories.find((c) => c.key === k)?.label ?? k,
  }),
)
const selectedMembers = computed(() =>
  Array.isArray(value.value) ? (value.value as string[]) : [],
)

function toggleMember(id: string) {
  const cur = selectedMembers.value
  emit('fix', cur.includes(id) ? cur.filter((x) => x !== id) : [...cur, id])
}
</script>

<template>
  <div class="decision">
    <div class="head">
      <strong>{{ POINT_TITLES[point] }}</strong>
      <span class="val">{{ text }}</span>
      <span v-if="decision?.is_final && decision.decided_by" class="settings-note"
        >(confirmed)</span
      >
      <span v-else-if="decision" class="settings-note"
        >{{ Math.round(decision.confidence * 100) }}%</span
      >
    </div>
    <div class="choices">
      <template v-if="point === 'email.category'">
        <button
          v-for="c in categories.filter((x) => !x.archived)"
          :key="c.id"
          type="button"
          class="chip"
          :class="{ on: value === c.key }"
          :aria-pressed="value === c.key"
          @click="emit('fix', c.key)"
        >
          {{ c.label }}
        </button>
      </template>
      <template v-else-if="point === 'email.member_link'">
        <button
          v-for="m in members"
          :key="m.id"
          type="button"
          class="chip"
          :class="{ on: selectedMembers.includes(m.id) }"
          :aria-pressed="selectedMembers.includes(m.id)"
          @click="toggleMember(m.id)"
        >
          {{ m.display_name }}
        </button>
      </template>
      <template v-else>
        <button
          type="button"
          class="chip"
          :class="{ on: value === true }"
          :aria-pressed="value === true"
          @click="emit('fix', true)"
        >
          Yes
        </button>
        <button
          type="button"
          class="chip"
          :class="{ on: value === false }"
          :aria-pressed="value === false"
          @click="emit('fix', false)"
        >
          No
        </button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.decision {
  display: grid;
  gap: var(--space-2);
  padding: var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}
.head {
  display: flex;
  gap: var(--space-2);
  align-items: baseline;
  flex-wrap: wrap;
}
.choices {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.chip {
  min-height: 44px;
  padding: 0 var(--space-3);
  border-radius: 999px;
  border: 1px solid var(--color-border);
  background: transparent;
  color: inherit;
}
.chip.on {
  border-color: var(--color-accent, currentColor);
  font-weight: 600;
  outline: 2px solid var(--color-accent, currentColor);
}
</style>

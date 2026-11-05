<script setup lang="ts">
import { computed } from 'vue'
import type { CategoryView, DecisionPointKey, DecisionView, Member } from '../../api/client'
import { UiChip } from '../../ui'
import AiChip from './AiChip.vue'
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
      <AiChip v-if="decision?.stage === 'llm'" :confidence="decision.confidence" />
      <span v-if="decision?.is_final && decision.decided_by" class="muted">(confirmed)</span>
      <span v-else-if="decision" class="muted">{{ Math.round(decision.confidence * 100) }}%</span>
    </div>
    <div class="choices" role="group" :aria-label="POINT_TITLES[point]">
      <template v-if="point === 'email.category'">
        <UiChip
          v-for="c in categories.filter((x) => !x.archived)"
          :key="c.id"
          variant="filter"
          :selected="value === c.key"
          @click="emit('fix', c.key)"
          >{{ c.label }}</UiChip
        >
      </template>
      <template v-else-if="point === 'email.member_link'">
        <UiChip
          v-for="m in members"
          :key="m.id"
          variant="filter"
          :selected="selectedMembers.includes(m.id)"
          @click="toggleMember(m.id)"
          >{{ m.display_name }}</UiChip
        >
      </template>
      <template v-else>
        <UiChip variant="filter" :selected="value === true" @click="emit('fix', true)">Yes</UiChip>
        <UiChip variant="filter" :selected="value === false" @click="emit('fix', false)">No</UiChip>
      </template>
    </div>
  </div>
</template>

<style scoped>
.decision {
  display: grid;
  gap: var(--ui-space-1);
}
.head {
  display: flex;
  gap: var(--ui-space-2);
  align-items: baseline;
  flex-wrap: wrap;
  font: var(--ui-type-body-md);
}
.muted {
  color: var(--ui-on-surface-variant);
}
.choices {
  display: flex;
  flex-wrap: wrap;
  column-gap: var(--ui-space-2);
}
</style>

<script setup lang="ts">
import { computed } from 'vue'
import MemberChip from '../../components/MemberChip.vue'
import { formatRange } from '../../lib/time'
import { toEventLike, toMemberLike } from '../filter'
import Modal from '../Modal.vue'
import { S } from '../strings'
import type { DashboardData } from '../types'

const props = defineProps<{ data: DashboardData; id: string }>()
defineEmits<{ close: [] }>()
const tz = computed(() => props.data.household.tz)
const members = computed(() => props.data.members.map(toMemberLike))
const ev = computed(() => props.data.events.find((e) => e.id === props.id))
const like = computed(() => (ev.value ? toEventLike(ev.value, members.value) : null))
const busy = computed(() => ev.value?.visibility === 'busy')
const who = computed(() => members.value.filter((m) => ev.value?.members.includes(m.id)))
const date = computed(() =>
  ev.value
    ? new Intl.DateTimeFormat(undefined, {
        weekday: 'long',
        month: 'long',
        day: 'numeric',
        timeZone: ev.value.all_day ? 'UTC' : tz.value,
      }).format(
        new Date(
          ev.value.all_day && ev.value.start_date
            ? `${ev.value.start_date}T12:00:00Z`
            : ev.value.start_at,
        ),
      )
    : '',
)
const time = computed(() =>
  like.value ? (like.value.all_day ? S.allDay : formatRange(like.value, tz.value)) : '',
)
</script>

<template>
  <Modal v-if="ev" :title="busy ? S.busy : ev.title" @close="$emit('close')">
    <dl>
      <dt>{{ S.when }}</dt>
      <dd>{{ date }}, {{ time }}</dd>
      <template v-if="!busy && ev.location">
        <dt>{{ S.location }}</dt>
        <dd>{{ ev.location }}</dd>
      </template>
      <template v-if="who.length">
        <dt>{{ S.who }}</dt>
        <dd class="chips">
          <MemberChip v-for="m in who" :key="m.id" :name="m.display_name" :color="m.color" />
        </dd>
      </template>
      <template v-if="!busy && ev.description">
        <dt>{{ S.notes }}</dt>
        <dd class="notes">{{ ev.description }}</dd>
      </template>
      <template v-if="!busy && ev.external_url">
        <dt></dt>
        <dd>
          <a :href="ev.external_url" target="_blank" rel="noopener">{{ S.source }} ↗</a>
        </dd>
      </template>
    </dl>
  </Modal>
</template>

<style scoped>
dl {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: var(--space-2) var(--space-4);
  margin: 0;
}
dt {
  color: var(--color-muted);
}
dd {
  margin: 0;
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}
.notes {
  white-space: pre-wrap;
}
</style>

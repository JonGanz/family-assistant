<script setup lang="ts">
import { computed, reactive } from 'vue'
import type {
  CalendarViewExt,
  EventApproval,
  EventProposalView,
  Member,
  SourceViewExt,
} from '../../../api/client'
import { UiButton, UiCard, UiSelect, UiTextField } from '../../../ui'
import { fmt, fromLocalInput, toLocalInput } from '../../../components/mail/format'

const props = defineProps<{
  ev: EventProposalView
  tz: string
  calendars: CalendarViewExt[]
  sources: SourceViewExt[]
  members: Member[]
  busy?: boolean
}>()
const emit = defineEmits<{ approve: [EventApproval]; reject: []; discard: [] }>()

const writable = computed(() => {
  const ok = new Set(props.sources.filter((s) => s.writable).map((s) => s.id))
  return props.calendars.filter((c) => c.enabled && ok.has(c.source))
})
const calendarOptions = computed(() => [
  { value: '', label: 'Default' },
  ...writable.value.map((c) => ({ value: c.id, label: c.name })),
])
const form = reactive({
  title: props.ev.title,
  start: toLocalInput(props.ev.start_at, props.tz),
  end: toLocalInput(props.ev.end_at, props.tz),
  calendar:
    props.ev.suggested_calendar ?? writable.value.find((c) => c.is_default_target)?.id ?? '',
})
const who = computed(() =>
  props.members.filter((m) => props.ev.members.includes(m.id)).map((m) => m.display_name),
)

function approve() {
  if (props.ev.pending_cancel) return emit('approve', {})
  const edits: EventApproval = {}
  if (form.title !== props.ev.title) edits.title = form.title
  if (form.start && form.start !== toLocalInput(props.ev.start_at, props.tz))
    edits.start_at = fromLocalInput(form.start, props.tz)
  if (form.end && form.end !== toLocalInput(props.ev.end_at, props.tz))
    edits.end_at = fromLocalInput(form.end, props.tz)
  if (form.calendar && form.calendar !== props.ev.suggested_calendar) edits.calendar = form.calendar
  emit('approve', edits)
}
</script>

<template>
  <li class="item">
    <UiCard variant="filled">
      <template #header>
        <div class="head">
          <span>{{ ev.pending_cancel ? 'Cancellation' : 'New event' }}</span>
          <slot name="header-trailing" />
        </div>
      </template>
      <div class="fields">
        <p v-if="ev.pending_cancel" class="note">
          Cancellation: remove <strong>{{ ev.title }}</strong> ({{ fmt(ev.start_at, tz) }}) from the
          calendar.
        </p>
        <template v-else>
          <UiTextField v-model="form.title" label="Title" />
          <template v-if="!ev.all_day">
            <UiTextField v-model="form.start" label="Starts" type="datetime-local" />
            <UiTextField v-model="form.end" label="Ends" type="datetime-local" />
          </template>
          <p v-else class="note">All day, {{ fmt(ev.start_at, tz).split(',')[0] }}</p>
          <UiSelect v-model="form.calendar" label="Calendar" :options="calendarOptions" />
        </template>
        <p class="note">
          <span v-if="ev.from_label">From {{ ev.from_label }}. </span>
          <span v-if="who.length">For {{ who.join(', ') }}. </span>
          Confidence {{ Math.round(ev.confidence * 100) }}%.
          <RouterLink v-if="ev.message" :to="{ name: 'mail-detail', params: { id: ev.message } }"
            >Open message</RouterLink
          >
        </p>
        <p v-if="ev.write_error" class="err" role="alert">
          Writing to the calendar failed: {{ ev.write_error }}. Approve to retry or reject to drop
          it.
        </p>
      </div>
      <template #actions>
        <UiButton
          variant="text"
          :disabled="busy"
          title="Not needed; doesn't count against the AI"
          @click="emit('discard')"
          >Discard</UiButton
        >
        <UiButton
          variant="text"
          :disabled="busy"
          title="The suggestion was wrong (used to measure accuracy)"
          @click="emit('reject')"
          >Reject</UiButton
        >
        <UiButton :disabled="busy" @click="approve">{{
          ev.write_error ? 'Retry' : ev.pending_cancel ? 'Approve removal' : 'Approve'
        }}</UiButton>
      </template>
    </UiCard>
  </li>
</template>

<style scoped>
.item {
  list-style: none;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ui-space-2);
  font: var(--ui-type-title-sm);
}
.fields {
  display: grid;
  gap: var(--ui-space-3);
}
.note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.err {
  margin: 0;
  color: var(--ui-error);
  font: var(--ui-type-body-md);
}
.checks {
  display: grid;
  margin: 0;
  padding: 0;
  border: 0;
}
.checks legend {
  padding: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-label-md);
}
</style>

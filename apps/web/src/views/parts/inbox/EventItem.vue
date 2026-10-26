<script setup lang="ts">
import { computed, reactive } from 'vue'
import type {
  CalendarViewExt,
  EventApproval,
  EventProposalView,
  Member,
  SourceViewExt,
} from '../../../api/client'
import { fmt, fromLocalInput, toLocalInput } from '../../../components/mail/format'

const props = defineProps<{
  ev: EventProposalView
  tz: string
  calendars: CalendarViewExt[]
  sources: SourceViewExt[]
  members: Member[]
  busy?: boolean
}>()
const emit = defineEmits<{ approve: [EventApproval]; reject: [] }>()

const writable = computed(() => {
  const ok = new Set(props.sources.filter((s) => s.writable).map((s) => s.id))
  return props.calendars.filter((c) => c.enabled && ok.has(c.source))
})
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
  <li class="settings-form item">
    <p v-if="ev.pending_cancel" class="settings-note">
      Cancellation: remove <strong>{{ ev.title }}</strong> ({{ fmt(ev.start_at, tz) }}) from the
      calendar.
    </p>
    <template v-else>
      <label>Title <input v-model="form.title" /></label>
      <template v-if="!ev.all_day">
        <label>Starts <input v-model="form.start" type="datetime-local" /></label>
        <label>Ends <input v-model="form.end" type="datetime-local" /></label>
      </template>
      <p v-else class="settings-note">All day, {{ fmt(ev.start_at, tz).split(',')[0] }}</p>
      <label>
        Calendar
        <select v-model="form.calendar">
          <option value="">Default</option>
          <option v-for="c in writable" :key="c.id" :value="c.id">{{ c.name }}</option>
        </select>
      </label>
    </template>
    <p class="settings-note">
      <span v-if="ev.from_label">From {{ ev.from_label }}. </span>
      <span v-if="who.length">For {{ who.join(', ') }}. </span>
      Confidence {{ Math.round(ev.confidence * 100) }}%.
      <RouterLink v-if="ev.message" :to="{ name: 'mail-detail', params: { id: ev.message } }"
        >Open message</RouterLink
      >
    </p>
    <p v-if="ev.write_error" class="settings-error" role="alert">
      Writing to the calendar failed: {{ ev.write_error }}. Approve to retry or reject to drop it.
    </p>
    <div class="actions">
      <button type="button" :disabled="busy" @click="approve">
        {{ ev.write_error ? 'Retry' : ev.pending_cancel ? 'Approve removal' : 'Approve' }}
      </button>
      <button type="button" :disabled="busy" @click="emit('reject')">Reject</button>
    </div>
  </li>
</template>

<style scoped>
.item {
  max-width: none;
}
.actions {
  display: flex;
  gap: var(--space-2);
}
.actions button {
  min-height: 44px;
}
</style>

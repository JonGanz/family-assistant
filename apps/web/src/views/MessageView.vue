<script setup lang="ts">
import ErrorBanner from '../components/ErrorBanner.vue'
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import type { DecisionPointKey } from '../api/client'
import DecisionFix from '../components/mail/DecisionFix.vue'
import { LABEL_POINTS } from '../components/mail/decisions'
import { fmt, fmtDay } from '../components/mail/format'
import { useHouseholdStore } from '../stores/household'
import { useMailStore } from '../stores/mail'
import { formatMoney } from '../stores/bills'
import { useMembersStore } from '../stores/members'
import { describeError } from './shared'
import ActionsPanel from './parts/message/ActionsPanel.vue'
import SectionsPanel from './parts/message/SectionsPanel.vue'
import './parts/settings.css'

const route = useRoute()
const mail = useMailStore()
const household = useHouseholdStore()
const members = useMembersStore()

const id = computed(() => String(route.params.id))
const childId = ref('')
const error = ref('')
const children = computed(() => members.items.filter((m) => m.role === 'child'))
const d = computed(() => mail.detail)

function load() {
  void mail.loadDetail(id.value, childId.value || undefined)
}
onMounted(() => {
  void members.load()
  void mail.loadFilters()
  load()
})
watch([id, childId], load)

async function guard(fn: () => Promise<void>) {
  error.value = ''
  try {
    await fn()
  } catch (e) {
    error.value = describeError(e)
  }
}
const fix = (point: DecisionPointKey, value: unknown) =>
  guard(() => mail.fix(id.value, { decision_point: point, value }))
const act = (a: 'archive' | 'mark_read' | 'label', label?: string) =>
  guard(() => mail.act(id.value, { action: a, label }, childId.value || undefined))
const undo = (aid: string) => guard(() => mail.undo(id.value, aid, childId.value || undefined))
const makeTask = () => guard(() => mail.makeTask(id.value, {}, childId.value || undefined))
</script>

<template>
  <section>
    <p><RouterLink to="/mail">Back to mail</RouterLink></p>
    <ErrorBanner v-if="mail.detailError" :message="mail.detailError" />
    <template v-if="d">
      <h1>{{ d.summary.subject }}</h1>
      <p class="settings-note">
        {{ d.summary.from_name || d.summary.from_addr }} &middot;
        {{ fmt(d.summary.received_at, household.tz) }}
        <template v-if="d.summary.external_url">
          &middot;
          <a :href="d.summary.external_url" target="_blank" rel="noopener noreferrer"
            >open in Gmail ↗</a
          >
        </template>
      </p>
      <ErrorBanner v-if="error" :message="error" />

      <h2>Content</h2>
      <label v-if="children.length" class="inline">
        Show for
        <select v-model="childId">
          <option value="">Everything</option>
          <option v-for="m in children" :key="m.id" :value="m.id">{{ m.display_name }}</option>
        </select>
      </label>
      <p v-if="d.content_purged" class="settings-note">
        The body was removed by the retention policy.
      </p>
      <SectionsPanel
        v-else-if="d.sections.length"
        :sections="d.sections"
        :child-filter="!!childId"
      />
      <pre v-else-if="d.body_text" class="body">{{ d.body_text }}</pre>

      <h2>Decisions</h2>
      <div class="decisions">
        <DecisionFix
          v-for="p in LABEL_POINTS"
          :key="p"
          :point="p"
          :decisions="d.decisions"
          :categories="mail.categories"
          :members="members.items"
          @fix="(v) => fix(p, v)"
        />
      </div>

      <h2>Derived items</h2>
      <ul class="settings-list">
        <li v-for="e in d.events" :key="e.id">
          Event: {{ e.title }}, {{ fmt(e.start_at, household.tz) }}
          <span class="badge">{{ e.status }}</span>
        </li>
        <li v-for="t in d.tasks" :key="t.id">
          Task:
          <RouterLink :to="{ name: 'task-detail', params: { id: t.id } }">{{ t.title }}</RouterLink>
          <span class="badge">{{ t.status }}</span>
        </li>
        <li v-for="b in d.bills" :key="b.id">
          Bill: {{ b.payee }}, {{ formatMoney(b.amount_minor, b.currency) }}
          <span v-if="b.due_on">due {{ fmtDay(b.due_on) }}</span>
          <span class="badge">{{ b.status }}</span>
        </li>
      </ul>
      <p v-if="!d.events.length && !d.tasks.length && !d.bills.length" class="settings-note">
        Nothing derived from this message.
      </p>
      <button type="button" @click="makeTask">Make task</button>

      <h2>Mail actions</h2>
      <ActionsPanel
        :actions="d.actions"
        :can-act="d.can_act"
        :tz="household.tz"
        @act="act"
        @undo="undo"
      />
    </template>
  </section>
</template>

<style scoped>
.decisions {
  display: grid;
  gap: var(--space-3);
}
.body {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font-family: inherit;
}
</style>

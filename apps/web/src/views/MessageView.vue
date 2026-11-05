<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import type { DecisionPointKey } from '../api/client'
import DecisionFix from '../components/mail/DecisionFix.vue'
import AiChip from '../components/mail/AiChip.vue'
import { LABEL_POINTS } from '../components/mail/decisions'
import { fmt, fmtDay } from '../components/mail/format'
import { useHouseholdStore } from '../stores/household'
import { useMailStore } from '../stores/mail'
import { formatMoney } from '../stores/bills'
import { useMembersStore } from '../stores/members'
import {
  UiButton,
  UiCard,
  UiChip,
  UiErrorState,
  UiList,
  UiListItem,
  UiPage,
  UiSection,
  UiSkeleton,
  UiTopAppBar,
  useSnackbar,
} from '../ui'
import { describeError } from './shared'
import ActionsPanel from './parts/message/ActionsPanel.vue'
import SectionsPanel from './parts/message/SectionsPanel.vue'

const route = useRoute()
const router = useRouter()
const mail = useMailStore()
const household = useHouseholdStore()
const members = useMembersStore()
const snackbar = useSnackbar()

const id = computed(() => String(route.params.id))
const childId = ref('')
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

async function guard(fn: () => Promise<void>, done?: string) {
  try {
    await fn()
    if (done) snackbar.show({ text: done })
  } catch (e) {
    snackbar.show({ text: describeError(e) })
  }
}
const fix = (point: DecisionPointKey, value: unknown) =>
  guard(() => mail.fix(id.value, { decision_point: point, value }))
const act = (a: 'archive' | 'mark_read' | 'label', label?: string) =>
  guard(() => mail.act(id.value, { action: a, label }, childId.value || undefined), 'Done')
const undo = (aid: string) => guard(() => mail.undo(id.value, aid, childId.value || undefined))
const makeTask = () =>
  guard(() => mail.makeTask(id.value, {}, childId.value || undefined), 'Task created')
const pickChild = (m: string) => (childId.value = childId.value === m ? '' : m)
</script>

<template>
  <UiPage>
    <template #top>
      <UiTopAppBar :title="d?.summary.subject ?? 'Message'" back @back="router.push('/mail')" />
    </template>

    <UiErrorState v-if="mail.detailError" :message="mail.detailError" @retry="load" />
    <UiSkeleton v-else-if="!d" :lines="6" />
    <template v-else>
      <UiCard variant="filled">
        <template #header>
          <div class="meta">
            <span>{{ d.summary.from_name || d.summary.from_addr }}</span>
            <span>{{ fmt(d.summary.received_at, household.tz) }}</span>
          </div>
        </template>
        <div class="stack">
          <a
            v-if="d.summary.external_url"
            class="ext"
            :href="d.summary.external_url"
            target="_blank"
            rel="noopener noreferrer"
            >open in Gmail ↗</a
          >
          <div class="row">
            <UiButton variant="tonal" icon="add" @click="makeTask">Make task</UiButton>
          </div>
          <ActionsPanel
            :actions="d.actions"
            :can-act="d.can_act"
            :tz="household.tz"
            @act="act"
            @undo="undo"
          />
        </div>
      </UiCard>

      <UiSection heading="Content">
        <UiCard variant="filled">
          <div class="stack">
            <div v-if="children.length" class="chips" role="group" aria-label="Show for">
              <UiChip variant="filter" :selected="!childId" @click="childId = ''"
                >Everything</UiChip
              >
              <UiChip
                v-for="m in children"
                :key="m.id"
                variant="filter"
                :selected="childId === m.id"
                @click="pickChild(m.id)"
                >{{ m.display_name }}</UiChip
              >
            </div>
            <p v-if="d.content_purged" class="note">
              The body was removed by the retention policy.
            </p>
            <SectionsPanel
              v-else-if="d.sections.length"
              :sections="d.sections"
              :child-filter="!!childId"
            />
            <pre v-else-if="d.body_text" class="body">{{ d.body_text }}</pre>
          </div>
        </UiCard>
      </UiSection>

      <UiSection heading="Decisions">
        <UiCard variant="filled">
          <div class="stack">
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
        </UiCard>
      </UiSection>

      <UiSection heading="Derived items">
        <UiCard variant="filled">
          <UiList v-if="d.events.length || d.tasks.length || d.bills.length">
            <UiListItem
              v-for="e in d.events"
              :key="e.id"
              :overline="`Event · ${e.status}`"
              :headline="e.title"
              :supporting="fmt(e.start_at, household.tz)"
            >
              <template v-if="e.extracted_by === 'llm'" #trailing><AiChip /></template>
            </UiListItem>
            <UiListItem
              v-for="t in d.tasks"
              :key="t.id"
              :to="`/tasks/${t.id}`"
              :overline="`Task · ${t.status}`"
              :headline="t.title"
            >
              <template v-if="t.extracted_by === 'llm'" #trailing><AiChip /></template>
            </UiListItem>
            <UiListItem
              v-for="b in d.bills"
              :key="b.id"
              :overline="`Bill · ${b.status}`"
              :headline="`${b.payee}, ${formatMoney(b.amount_minor, b.currency)}`"
              :supporting="b.due_on ? `due ${fmtDay(b.due_on)}` : undefined"
            >
              <template v-if="b.extracted_by === 'llm'" #trailing>
                <AiChip :confidence="b.confidence" />
              </template>
            </UiListItem>
          </UiList>
          <p v-else class="note pad">Nothing derived from this message.</p>
        </UiCard>
      </UiSection>
    </template>
  </UiPage>
</template>

<style scoped>
.meta {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: var(--ui-space-2);
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.stack {
  display: grid;
  gap: var(--ui-space-3);
}
.row {
  display: flex;
  gap: var(--ui-space-2);
}
.ext {
  color: var(--ui-primary);
  font: var(--ui-type-label-lg);
}
.chips {
  display: flex;
  flex-wrap: wrap;
  column-gap: var(--ui-space-2);
}
.note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.body {
  margin: 0;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font: var(--ui-type-body-md);
}
</style>

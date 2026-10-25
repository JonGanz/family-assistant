<script setup lang="ts">
import { computed, ref } from 'vue'
import MemberPicker from '../components/tasks/MemberPicker.vue'
import UndoToast from '../components/tasks/UndoToast.vue'
import { useDashboardStore } from '../stores/dashboard'
import { S } from './strings'
import { tasksOf, toMemberRef } from './tasksModel'

const store = useDashboardStore()
const data = computed(() => store.snapshot)
const members = computed(() => (data.value?.members ?? []).map(toMemberRef))

// Completion: pick who did it, complete, offer an undo.
const completing = ref<{ id: string; pre: string | null } | null>(null)
const undo = ref<{ id: string; member: string; title: string } | null>(null)

function beginComplete(id: string): void {
  const t = tasksOf(data.value!).find((x) => x.id === id)
  if (!t || data.value?.completion_enabled === false) return
  completing.value = { id, pre: t.assignees.length === 1 ? (t.assignees[0] ?? null) : null }
}

async function picked(member: string): Promise<void> {
  const c = completing.value
  if (!c) return
  completing.value = null
  const title = tasksOf(data.value!).find((x) => x.id === c.id)?.title ?? ''
  if (await store.complete(c.id, member)) undo.value = { id: c.id, member, title }
}

function cancelComplete(): void {
  completing.value = null
  store.resetChecks()
}

async function doUndo(): Promise<void> {
  const u = undo.value
  undo.value = null
  if (u) await store.reopen(u.id, u.member)
}

// Adding: the form, then who is adding it.
const adding = ref(false)
const asking = ref(false)
const title = ref('')
const list = ref('')
const assignee = ref('')
const failed = ref(false)
const lists = computed(() => (data.value?.task_lists ?? []).filter((l) => l.dashboard_add))

function beginAdd(): void {
  if (lists.value.length === 0) return
  title.value = ''
  assignee.value = ''
  list.value = lists.value[0]!.id
  failed.value = false
  adding.value = true
  asking.value = false
}

const canSubmit = computed(() => title.value.trim().length > 0 && list.value !== '')

async function addBy(member: string): Promise<void> {
  failed.value = false
  const ok = await store.addTask({
    list: list.value,
    title: title.value.trim(),
    notes: null,
    due_at: null,
    assignees: assignee.value ? [assignee.value] : [],
    steps: [],
    priority_flag: null,
    calendar_display: null,
    member,
  })
  asking.value = false
  if (ok) adding.value = false
  else failed.value = true
}

defineExpose({ beginComplete, beginAdd })
</script>

<template>
  <div class="flow">
    <div v-if="completing" class="modal" data-testid="complete-picker">
      <MemberPicker
        :members="members"
        :preselected="completing.pre"
        :title="S.whoDidIt"
        size="kiosk"
        @pick="picked"
        @cancel="cancelComplete"
      />
    </div>

    <div v-if="adding" class="modal" data-testid="add-form">
      <MemberPicker
        v-if="asking"
        :members="members"
        :title="S.whoAdds"
        size="kiosk"
        @pick="addBy"
        @cancel="asking = false"
      />
      <form v-else class="form" @submit.prevent="canSubmit && (asking = true)">
        <h2>{{ S.addTask }}</h2>
        <label>
          {{ S.taskTitle }}
          <input v-model="title" data-testid="add-title" type="text" autocomplete="off" />
        </label>
        <label>
          {{ S.taskList }}
          <select v-model="list" data-testid="add-list">
            <option v-for="l in lists" :key="l.id" :value="l.id">{{ l.name }}</option>
          </select>
        </label>
        <label>
          {{ S.assignee }}
          <select v-model="assignee" data-testid="add-assignee">
            <option value="">{{ S.anyone }}</option>
            <option v-for="m in members" :key="m.id" :value="m.id">{{ m.name }}</option>
          </select>
        </label>
        <p v-if="failed" role="alert" class="err">{{ S.addFailed }}</p>
        <div class="buttons">
          <button type="button" @click="adding = false">{{ S.cancel }}</button>
          <button type="submit" class="primary" :disabled="!canSubmit" data-testid="add-submit">
            {{ S.create }}
          </button>
        </div>
      </form>
    </div>

    <div v-if="undo" class="toast">
      <UndoToast
        :key="undo.id"
        :message="S.completedToast(undo.title)"
        :seconds="10"
        @undo="doUndo"
        @expire="undo = null"
      />
    </div>
  </div>
</template>

<style scoped>
.modal {
  position: absolute;
  inset: 0;
  z-index: 10;
  display: grid;
  place-items: center;
  background: rgb(0 0 0 / 55%);
}
.toast {
  position: absolute;
  left: 50%;
  bottom: 3.5rem;
  z-index: 11;
  transform: translateX(-50%);
  width: max-content;
  max-width: 90%;
  font-size: var(--kiosk-font-small, 1.5rem);
}
.form {
  display: grid;
  gap: var(--space-3);
  width: min(48rem, 90%);
  padding: var(--space-5);
  background: var(--color-bg);
  color: var(--color-text);
  border-radius: var(--radius-lg);
  font-size: var(--kiosk-font-base, 2rem);
}
h2 {
  margin: 0;
}
label {
  display: grid;
  gap: var(--space-1);
}
input,
select {
  min-height: 4.5rem;
  padding: 0 var(--space-3);
  font: inherit;
  background: var(--color-surface);
  color: inherit;
  border: 2px solid var(--color-border);
  border-radius: var(--radius-md);
}
.buttons {
  display: flex;
  gap: var(--space-3);
  justify-content: flex-end;
}
button {
  min-height: 4.5rem;
  padding: 0 var(--space-5);
  font: inherit;
  background: none;
  color: inherit;
  border: 2px solid var(--color-border);
  border-radius: var(--radius-md);
  cursor: pointer;
}
button.primary {
  background: var(--color-accent);
  border-color: var(--color-accent);
  color: #fff;
}
button:disabled {
  opacity: 0.5;
}
.err {
  margin: 0;
  color: var(--color-danger);
}
</style>

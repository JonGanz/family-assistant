<script setup lang="ts">
import { reactive, ref } from 'vue'
import type {
  AutoAction,
  CategoryBehaviour,
  CategoryView,
  EmailTasksMode,
} from '../../../api/mailSettings'
import { useMailSettingsStore } from '../../../stores/mailSettings'
import { describeError } from '../../shared'

const store = useMailSettingsStore()
const error = ref('')
const ACTIONS: { key: AutoAction; label: string }[] = [
  { key: 'label', label: 'Label in Gmail' },
  { key: 'archive', label: 'Archive' },
  { key: 'mark_read', label: 'Mark read' },
]
const BEHAVIOURS: { key: CategoryBehaviour; label: string }[] = [
  { key: 'ignore', label: 'Ignore (no items made)' },
  { key: 'keep', label: 'Keep (searchable only)' },
  { key: 'extract', label: 'Extract events, bills and tasks' },
]
const TASK_MODES: { key: EmailTasksMode; label: string }[] = [
  { key: 'suggest', label: 'Suggest (you approve)' },
  { key: 'create', label: 'Create directly' },
  { key: 'off', label: 'Off' },
]

const fresh = () => ({
  key: '',
  label: '',
  behaviour: 'keep' as CategoryBehaviour,
  notify: false,
  actions: ['label'] as AutoAction[],
  tasks: 'suggest' as EmailTasksMode,
  color: '#2f6fed',
})
const form = reactive(fresh())

async function run(fn: () => Promise<unknown>, fallback: string) {
  error.value = ''
  try {
    await fn()
  } catch (e) {
    error.value = describeError(e, fallback)
  }
}

const patch = (c: CategoryView, p: Parameters<typeof store.patchCategory>[1]) =>
  run(() => store.patchCategory(c, p), 'Could not save.')

function toggleAction(c: CategoryView, a: AutoAction, on: boolean) {
  const next = on ? [...c.auto_actions, a] : c.auto_actions.filter((x) => x !== a)
  return patch(c, { auto_actions: next })
}

async function add() {
  await run(async () => {
    const ok = await store.createCategory({
      key: form.key.trim(),
      label: form.label.trim(),
      behaviour: form.behaviour,
      notify_time_sensitive: form.notify,
      auto_actions: form.actions,
      email_tasks: form.tasks,
      color: form.color,
    })
    if (ok) Object.assign(form, fresh())
  }, 'Could not add the category.')
}
</script>

<template>
  <div>
    <p v-if="error" class="settings-error" role="alert">{{ error }}</p>
    <table class="cats">
      <thead>
        <tr>
          <th>Category</th>
          <th>What to do</th>
          <th>Notify if time-sensitive</th>
          <th>Automatic actions</th>
          <th>Tasks from email</th>
          <th />
        </tr>
      </thead>
      <tbody>
        <tr v-for="c in store.categories.filter((x) => !x.archived)" :key="c.id">
          <td>
            <span class="dot" :style="{ background: c.color ?? 'var(--color-border)' }" />
            {{ c.label }} <code>{{ c.key }}</code>
          </td>
          <td>
            <select
              :value="c.behaviour"
              :aria-label="`Behaviour for ${c.label}`"
              @change="
                patch(c, {
                  behaviour: ($event.target as HTMLSelectElement).value as CategoryBehaviour,
                })
              "
            >
              <option v-for="b in BEHAVIOURS" :key="b.key" :value="b.key">{{ b.label }}</option>
            </select>
          </td>
          <td>
            <input
              type="checkbox"
              :checked="c.notify_time_sensitive"
              :aria-label="`Notify for ${c.label}`"
              @change="
                patch(c, { notify_time_sensitive: ($event.target as HTMLInputElement).checked })
              "
            />
          </td>
          <td>
            <label v-for="a in ACTIONS" :key="a.key" class="inline">
              <input
                type="checkbox"
                :checked="c.auto_actions.includes(a.key)"
                @change="toggleAction(c, a.key, ($event.target as HTMLInputElement).checked)"
              />
              {{ a.label }}
            </label>
          </td>
          <td>
            <select
              :value="c.email_tasks"
              :aria-label="`Tasks for ${c.label}`"
              @change="
                patch(c, {
                  email_tasks: ($event.target as HTMLSelectElement).value as EmailTasksMode,
                })
              "
            >
              <option v-for="t in TASK_MODES" :key="t.key" :value="t.key">{{ t.label }}</option>
            </select>
          </td>
          <td>
            <button
              type="button"
              @click="run(() => store.archiveCategory(c), 'Could not archive.')"
            >
              Archive
            </button>
          </td>
        </tr>
      </tbody>
    </table>
    <p class="settings-note">
      Auto actions only run on sources that are not read only. Archiving a category keeps its old
      mail labelled.
    </p>

    <form class="settings-form" data-testid="add-category" @submit.prevent="add">
      <h3>Add a category</h3>
      <label>
        Key (lower case letters, digits, underscore)
        <input v-model="form.key" name="key" pattern="[a-z][a-z0-9_]{0,31}" required />
      </label>
      <label>Label <input v-model="form.label" name="label" required /></label>
      <label>
        What to do
        <select v-model="form.behaviour" name="behaviour">
          <option v-for="b in BEHAVIOURS" :key="b.key" :value="b.key">{{ b.label }}</option>
        </select>
      </label>
      <label class="inline">
        <input v-model="form.notify" type="checkbox" /> Notify if time-sensitive
      </label>
      <fieldset>
        <legend>Automatic actions</legend>
        <label v-for="a in ACTIONS" :key="a.key" class="inline">
          <input v-model="form.actions" type="checkbox" :value="a.key" /> {{ a.label }}
        </label>
      </fieldset>
      <label>
        Tasks from email
        <select v-model="form.tasks" name="email_tasks">
          <option v-for="t in TASK_MODES" :key="t.key" :value="t.key">{{ t.label }}</option>
        </select>
      </label>
      <label>Colour <input v-model="form.color" type="color" /></label>
      <button type="submit">Add category</button>
    </form>
  </div>
</template>

<style scoped>
.cats {
  width: 100%;
  border-collapse: collapse;
}
th,
td {
  text-align: left;
  padding: var(--space-1) var(--space-2);
  border-bottom: 1px solid var(--color-border);
  vertical-align: top;
}
.inline {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}
.dot {
  display: inline-block;
  width: 0.75rem;
  height: 0.75rem;
  border-radius: 50%;
}
</style>

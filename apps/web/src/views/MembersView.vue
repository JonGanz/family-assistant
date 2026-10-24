<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import type { Member } from '../api/client'
import Avatar from '../components/Avatar.vue'
import ErrorBanner from '../components/ErrorBanner.vue'
import { describeError } from './shared'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'

const session = useSessionStore()
const members = useMembersStore()

const editing = ref<string | null>(null)
const form = reactive({
  display_name: '',
  color: '#2f6fed',
  avatar: '',
  grade: '',
  interests: '',
  hobbies: '',
  responsibilities: '',
})
const preview = ref<string | null>(null)
const pending = ref<File | null>(null)
const creating = reactive({ display_name: '', role: 'child' })
const error = ref('')

const csv = (s: string) =>
  s
    .split(',')
    .map((x) => x.trim())
    .filter(Boolean)

onMounted(() => members.load())

function startEdit(m: Member) {
  editing.value = m.id
  members.conflictId = null
  error.value = ''
  form.display_name = m.display_name
  form.color = m.color ?? '#2f6fed'
  clearPending()
  form.avatar = m.avatar ?? ''
  form.grade = m.grade ?? ''
  form.interests = (m.interests ?? []).join(', ')
  form.hobbies = (m.hobbies ?? []).join(', ')
  form.responsibilities = (m.responsibilities ?? []).join(', ')
}

function clearPending() {
  if (preview.value) URL.revokeObjectURL(preview.value)
  preview.value = null
  pending.value = null
}

function pickFile(ev: Event) {
  const file = (ev.target as HTMLInputElement).files?.[0]
  clearPending()
  if (!file) return
  const problem = members.checkAvatar(file)
  if (problem) {
    error.value = problem
    ;(ev.target as HTMLInputElement).value = ''
    return
  }
  error.value = ''
  pending.value = file
  preview.value = URL.createObjectURL(file)
}

async function save(m: Member) {
  error.value = ''
  try {
    const ok = await members.update(m, {
      display_name: form.display_name,
      color: form.color || null,
      avatar: form.avatar || null,
      grade: form.grade || null,
      interests: csv(form.interests),
      hobbies: csv(form.hobbies),
      responsibilities: csv(form.responsibilities),
    })
    if (ok) {
      if (pending.value) await members.uploadAvatar(m.id, pending.value)
      clearPending()
      editing.value = null
    }
  } catch (e) {
    error.value = describeError(e, 'Save failed.')
  }
}

async function removeAvatar(m: Member) {
  error.value = ''
  try {
    await members.removeAvatar(m.id)
  } catch (e) {
    error.value = describeError(e, 'Could not remove the picture.')
  }
}

async function reload() {
  await members.load()
  const m = members.items.find((x) => x.id === editing.value)
  if (m) startEdit(m)
}

async function create() {
  error.value = ''
  try {
    await members.create({ display_name: creating.display_name, role: creating.role })
    creating.display_name = ''
  } catch (e) {
    error.value = describeError(e, 'Create failed.')
  }
}
</script>

<template>
  <section>
    <h1>Members</h1>
    <ErrorBanner v-if="members.error" :message="members.error" />
    <ul class="members">
      <li v-for="m in members.items" :key="m.id" :data-testid="`member-${m.display_name}`">
        <template v-if="editing !== m.id">
          <Avatar
            :name="m.display_name"
            :color="m.color ?? '#888888'"
            :src="m.avatar_url"
            size="2.5rem"
          />
          <strong>{{ m.display_name }}</strong>
          <em>{{ m.role }}</em>
          <button v-if="session.isAdult && m.version !== undefined" @click="startEdit(m)">
            Edit
          </button>
        </template>
        <form v-else @submit.prevent="save(m)">
          <label>Name <input v-model="form.display_name" name="display_name" required /></label>
          <label>Colour <input v-model="form.color" name="color" type="color" /></label>
          <div class="avatar-edit">
            <Avatar
              :name="form.display_name || m.display_name"
              :color="form.color"
              :src="preview ?? m.avatar_url"
              size="4rem"
            />
            <label>
              Picture (PNG, JPEG or WebP, up to 512 KB)
              <input
                type="file"
                name="avatar_file"
                accept="image/png,image/jpeg,image/webp"
                @change="pickFile"
              />
            </label>
            <button v-if="m.avatar_url && !preview" type="button" @click="removeAvatar(m)">
              Remove picture
            </button>
          </div>
          <label
            >Initials or emoji <input v-model="form.avatar" name="avatar" maxlength="4"
          /></label>
          <label>Grade <input v-model="form.grade" name="grade" /></label>
          <label>
            Interests (comma separated)
            <input v-model="form.interests" name="interests" />
          </label>
          <label>
            Hobbies (comma separated)
            <input v-model="form.hobbies" name="hobbies" />
          </label>
          <label>
            Responsibilities (comma separated)
            <input v-model="form.responsibilities" name="responsibilities" />
          </label>
          <ErrorBanner v-if="members.conflictId === m.id" conflict @reload="reload" />
          <ErrorBanner v-if="error" :message="error" />
          <button type="submit">Save</button>
          <button type="button" @click="editing = null">Cancel</button>
        </form>
      </li>
    </ul>

    <ErrorBanner v-if="error && editing === null" :message="error" />
    <form v-if="session.isAdult" class="create" @submit.prevent="create">
      <h2>Add member</h2>
      <label>Name <input v-model="creating.display_name" name="new_name" required /></label>
      <label>
        Role
        <select v-model="creating.role" name="new_role">
          <option value="child">child</option>
          <option value="adult">adult</option>
        </select>
      </label>
      <button type="submit">Add</button>
    </form>
  </section>
</template>

<style scoped>
.members {
  list-style: none;
  padding: 0;
  display: grid;
  gap: 0.5rem;
}
.members li {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  padding: 0.5rem;
  border: 1px solid var(--color-border);
  border-radius: 0.5rem;
}
.avatar-edit {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}
form {
  display: grid;
  gap: 0.5rem;
  width: 100%;
}
label {
  display: grid;
  gap: 0.15rem;
}
.create {
  margin-top: 2rem;
  max-width: 22rem;
}
</style>

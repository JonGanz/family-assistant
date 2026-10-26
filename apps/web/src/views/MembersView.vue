<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import type { Member, PatchMemberReq } from '../api/client'
import type { Gender } from '../api/mailSettings'
import ChipInput from './parts/settings-mail/ChipInput.vue'
import Avatar from '../components/Avatar.vue'
import ErrorBanner from '../components/ErrorBanner.vue'
import PageHeader from '../components/PageHeader.vue'
import StateBlock from '../components/StateBlock.vue'
import { describeError } from './shared'
import { useMembersStore } from '../stores/members'
import { useSessionStore } from '../stores/session'
import './parts/settings.css'

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
  aliases: [] as string[],
  gender: '' as Gender | '',
  tags: [] as string[],
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
  form.aliases = [...(m.aliases ?? [])]
  form.gender = m.gender ?? ''
  form.tags = [...(m.relevance_tags ?? [])]
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
      // M3 fields; the generated patch type does not list them yet.
      aliases: form.aliases,
      ...(m.role === 'child' ? { gender: form.gender || null, relevance_tags: form.tags } : {}),
    } as PatchMemberReq)
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
    <PageHeader eyebrow="Household" title="Members" />
    <StateBlock
      :error="members.error"
      :loading="members.loading && !members.items.length"
      :empty="!members.loading && !members.items.length"
      empty-text="No members yet."
    />
    <ul class="members card-list">
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
          <button
            v-if="session.isAdult && m.version !== undefined"
            class="btn-sm"
            @click="startEdit(m)"
          >
            Edit
          </button>
        </template>
        <form v-else class="form form-wide" @submit.prevent="save(m)">
          <label class="field"
            >Name <input v-model="form.display_name" name="display_name" required
          /></label>
          <label class="field"
            >Colour <input v-model="form.color" name="color" type="color"
          /></label>
          <div class="avatar-edit">
            <Avatar
              :name="form.display_name || m.display_name"
              :color="form.color"
              :src="preview ?? m.avatar_url"
              size="4rem"
            />
            <label class="field">
              Picture (PNG, JPEG or WebP, up to 512 KB)
              <input
                type="file"
                name="avatar_file"
                accept="image/png,image/jpeg,image/webp"
                @change="pickFile"
              />
            </label>
            <button
              v-if="m.avatar_url && !preview"
              type="button"
              class="btn-sm"
              @click="removeAvatar(m)"
            >
              Remove picture
            </button>
          </div>
          <label class="field"
            >Initials or emoji <input v-model="form.avatar" name="avatar" maxlength="4"
          /></label>
          <label class="field">Grade <input v-model="form.grade" name="grade" /></label>
          <label class="field">
            Interests (comma separated)
            <input v-model="form.interests" name="interests" />
          </label>
          <label class="field">
            Hobbies (comma separated)
            <input v-model="form.hobbies" name="hobbies" />
          </label>
          <label class="field">
            Responsibilities (comma separated)
            <input v-model="form.responsibilities" name="responsibilities" />
          </label>
          <ChipInput v-model="form.aliases" label="Also known as (names used in mail)" />
          <template v-if="m.role === 'child'">
            <label class="field">
              Gender (helps match girls' or boys' groups; optional)
              <select v-model="form.gender" name="gender">
                <option value="">Not set</option>
                <option value="female">Female</option>
                <option value="male">Male</option>
                <option value="other">Other</option>
              </select>
            </label>
            <ChipInput
              v-model="form.tags"
              label="Relevance tags (for example a sport or a class)"
            />
          </template>
          <ErrorBanner v-if="members.conflictId === m.id" conflict @reload="reload" />
          <ErrorBanner v-if="error" :message="error" />
          <div class="row">
            <button type="submit" class="btn-primary">Save</button>
            <button type="button" class="btn-ghost" @click="editing = null">Cancel</button>
          </div>
        </form>
      </li>
    </ul>

    <ErrorBanner v-if="error && editing === null" :message="error" />
    <form v-if="session.isAdult" class="form create" @submit.prevent="create">
      <h2>Add member</h2>
      <label class="field"
        >Name <input v-model="creating.display_name" name="new_name" required
      /></label>
      <label class="field">
        Role
        <select v-model="creating.role" name="new_role">
          <option value="child">child</option>
          <option value="adult">adult</option>
        </select>
      </label>
      <button type="submit" class="btn-primary">Add</button>
    </form>
  </section>
</template>

<style scoped>
.members > li {
  display: flex;
  gap: var(--space-3);
  align-items: center;
}
.avatar-edit {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}
form {
  width: 100%;
}
.create {
  margin-top: 2rem;
  max-width: 22rem;
}
</style>

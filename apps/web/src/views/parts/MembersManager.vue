<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import MemberAccess from './MemberAccess.vue'
import { validateNewMember } from './memberForm'
import { api, type Member, type PatchMemberReq } from '../../api/client'
import type { Gender } from '../../api/mailSettings'
import HuePicker from '../../components/HuePicker.vue'
import Avatar from '../../components/Avatar.vue'
import ResponsiveItems from '../../app/ResponsiveItems.vue'
import { hueVar, type Hue } from '../../lib/hues'
import { describeError } from '../shared'
import { useMembersStore } from '../../stores/members'
import { useSessionStore } from '../../stores/session'
import {
  UiButton,
  UiCard,
  UiChip,
  UiEmptyState,
  UiErrorState,
  UiFab,
  UiSaveBar,
  UiSelect,
  UiSheet,
  UiSkeleton,
  UiTextField,
  useFormDraft,
  useSnackbar,
} from '../../ui'

const props = withDefaults(defineProps<{ fab?: boolean }>(), { fab: true })

const members = useMembersStore()
const session = useSessionStore()
const snackbar = useSnackbar()

interface EditForm {
  display_name: string
  color: Hue
  avatar: string
  grade: string
  interests: string
  hobbies: string
  responsibilities: string
  aliases: string[]
  gender: Gender | ''
  tags: string[]
  /** Name of the picture waiting to be uploaded; the file itself is in `pending`. */
  file: string
}

const editing = ref<string | null>(null)
const member = computed(() => members.items.find((x) => x.id === editing.value) ?? null)
const error = ref('')

const toForm = (m: Member | null): EditForm => ({
  display_name: m?.display_name ?? '',
  color: m?.color ?? 'blue',
  avatar: m?.avatar ?? '',
  grade: m?.grade ?? '',
  interests: (m?.interests ?? []).join(', '),
  hobbies: (m?.hobbies ?? []).join(', '),
  responsibilities: (m?.responsibilities ?? []).join(', '),
  aliases: [...(m?.aliases ?? [])],
  gender: m?.gender ?? '',
  tags: [...(m?.relevance_tags ?? [])],
  file: '',
})

const preview = ref<string | null>(null)
const pending = ref<File | null>(null)
const chipText = reactive({ aliases: '', tags: '' })

const csv = (s: string) =>
  s
    .split(',')
    .map((x) => x.trim())
    .filter(Boolean)

/** A claimed adult other than the signed-in one cannot be edited (D-006). */
const claimedByOther = (m: Member) =>
  m.role === 'adult' && m.id !== session.member?.id && m.has_account && !m.initial_password

const form = useFormDraft(
  () => toForm(member.value),
  async (d) => {
    const m = member.value
    if (!m) return
    error.value = ''
    const ok = await members.update(m, {
      display_name: d.display_name,
      color: d.color,
      avatar: d.avatar || null,
      grade: m.role === 'child' ? d.grade || null : null,
      interests: csv(d.interests),
      hobbies: csv(d.hobbies),
      responsibilities: csv(d.responsibilities),
      // M3 fields; the generated patch type does not list them yet.
      aliases: d.aliases,
      ...(m.role === 'child' ? { gender: d.gender || null, relevance_tags: d.tags } : {}),
    } as PatchMemberReq)
    if (!ok) throw new Error('Changed elsewhere. Reload to see the latest, then try again.')
    if (pending.value) await members.uploadAvatar(m.id, pending.value)
    clearPending()
  },
)
const d = form.draft

onMounted(() => members.load())

function clearPending() {
  if (preview.value) URL.revokeObjectURL(preview.value)
  preview.value = null
  pending.value = null
}

function startEdit(m: Member) {
  members.conflictId = null
  error.value = ''
  clearPending()
  editing.value = m.id
  form.discard()
}

async function saveEdit() {
  if (await form.save()) editing.value = null
}

function closeEditor() {
  if (form.dirty.value && !window.confirm('Discard unsaved changes?')) return
  form.discard()
  clearPending()
  editing.value = null
}

function pickFile(ev: Event) {
  const file = (ev.target as HTMLInputElement).files?.[0]
  clearPending()
  d.value.file = ''
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
  d.value.file = file.name
}

async function removeAvatar(m: Member) {
  error.value = ''
  try {
    await members.removeAvatar(m.id)
    snackbar.show({ text: 'Picture removed' })
  } catch (e) {
    error.value = describeError(e, 'Could not remove the picture.')
  }
}

async function reload() {
  form.discard()
  await members.load()
  form.discard()
}

function addChip(key: 'aliases' | 'tags') {
  const v = chipText[key].trim()
  chipText[key] = ''
  if (v && !d.value[key].includes(v)) d.value[key].push(v)
}
function removeChip(key: 'aliases' | 'tags', v: string) {
  d.value[key] = d.value[key].filter((x) => x !== v)
}

const genderOptions = [
  { value: '', label: 'Not set' },
  { value: 'female', label: 'Female' },
  { value: 'male', label: 'Male' },
  { value: 'other', label: 'Other' },
]

// Adding a member: a form in a sheet.
const adding = ref(false)
const blankCreate = () => ({
  display_name: '',
  role: 'child',
  color: 'blue' as Hue,
  grade: '',
  username: '',
  password: '',
  pin: '',
})
const creating = reactive(blankCreate())
const createAliases = ref<string[]>([])
const createAliasText = ref('')
const createErrors = ref<Record<string, string>>({})
const createError = ref('')
const createdNote = ref('')
const roleOptions = [
  { value: 'child', label: 'child' },
  { value: 'adult', label: 'adult' },
]

function openAdd() {
  createdNote.value = ''
  createError.value = ''
  createErrors.value = {}
  adding.value = true
}

function addCreateAlias() {
  const v = createAliasText.value.trim()
  createAliasText.value = ''
  if (v && !createAliases.value.includes(v)) createAliases.value.push(v)
}

async function create() {
  createError.value = ''
  createdNote.value = ''
  createErrors.value = validateNewMember({
    ...creating,
    pin: creating.role === 'adult' ? '' : creating.pin,
  })
  if (Object.keys(createErrors.value).length) return
  try {
    const made = await members.create({
      display_name: creating.display_name.trim(),
      role: creating.role,
      color: creating.color,
      grade: creating.role === 'child' ? creating.grade.trim() || null : null,
      aliases: createAliases.value,
    })
    const user = creating.username.trim()
    if (user) await api.createAccount(made.id, user, creating.password)
    if (creating.pin && creating.role === 'child') await api.setPin(made.id, creating.pin)
    await members.load()
    createdNote.value =
      user && creating.role === 'adult' ? 'They must change this password at first sign-in.' : ''
    if (!createdNote.value) {
      Object.assign(creating, { ...blankCreate(), role: creating.role, color: creating.color })
      createAliases.value = []
      snackbar.show({ text: 'Member added' })
    }
  } catch (e) {
    createError.value = describeError(e, 'Create failed.')
    await members.load()
  }
}
</script>

<template>
  <div class="members-manager">
    <UiErrorState v-if="members.error" :message="members.error" @retry="members.load()" />
    <UiSkeleton v-else-if="members.loading && !members.items.length" :lines="4" />
    <UiEmptyState
      v-else-if="!members.loading && !members.items.length"
      icon="group"
      title="No members yet"
    />
    <ResponsiveItems v-else :items="members.items" class="members">
      <template #item="{ item: m }">
        <div class="member" :data-testid="`member-${m.display_name}`">
          <Avatar
            :name="m.display_name"
            :color="hueVar(m.color)"
            :src="m.avatar_url"
            size="2.5rem"
          />
          <div class="member__text">
            <strong>{{ m.display_name }}</strong>
            <UiChip>{{ m.role }}</UiChip>
          </div>
          <UiButton
            v-if="m.version !== undefined && !claimedByOther(m)"
            variant="tonal"
            icon="edit"
            @click="startEdit(m)"
          >
            Edit
          </UiButton>
          <span v-else-if="claimedByOther(m)" class="muted">Manages their own profile</span>
        </div>
      </template>
    </ResponsiveItems>

    <p v-if="error && !member" class="err" role="alert">{{ error }}</p>

    <div v-if="props.fab" class="fab">
      <UiFab icon="add" label="Add member" extended @click="openAdd" />
    </div>
    <UiButton v-else variant="tonal" icon="add" @click="openAdd">Add member</UiButton>

    <UiSheet
      :model-value="!!member"
      :title="member ? `Edit ${member.display_name}` : 'Edit member'"
      @update:model-value="!$event && closeEditor()"
    >
      <div v-if="member" class="editor" data-testid="member-editor">
        <UiCard v-if="members.conflictId === member.id" variant="outlined" role="alert">
          This member changed elsewhere. Reload to see the latest.
          <template #actions><UiButton variant="tonal" @click="reload">Reload</UiButton></template>
        </UiCard>

        <UiCard variant="outlined">
          <template #header>Profile</template>
          <div class="stack">
            <UiTextField v-model="d.display_name" label="Name" name="display_name" required />
            <HuePicker v-model="d.color" name="color" />
            <div class="avatar-edit">
              <Avatar
                :name="d.display_name || member.display_name"
                :color="hueVar(d.color)"
                :src="preview ?? member.avatar_url"
                size="4rem"
              />
              <label class="file">
                <span>Picture (PNG, JPEG or WebP, up to 512 KB)</span>
                <input
                  type="file"
                  name="avatar_file"
                  accept="image/png,image/jpeg,image/webp"
                  @change="pickFile"
                />
              </label>
              <UiButton
                v-if="member.avatar_url && !preview"
                variant="text"
                @click="removeAvatar(member)"
              >
                Remove picture
              </UiButton>
            </div>
            <UiTextField v-model="d.avatar" label="Initials or emoji" name="avatar" maxlength="4" />
            <UiTextField
              v-if="member.role === 'child'"
              v-model="d.grade"
              label="Grade"
              name="grade"
            />
          </div>
        </UiCard>

        <UiCard variant="outlined">
          <template #header>Aliases and tags</template>
          <div class="stack">
            <div class="chips">
              <span class="chips__label">Also known as (names used in mail)</span>
              <div class="chips__row">
                <UiChip
                  v-for="v in d.aliases"
                  :key="v"
                  variant="input"
                  @remove="removeChip('aliases', v)"
                >
                  {{ v }}
                </UiChip>
              </div>
              <UiTextField
                v-model="chipText.aliases"
                label="Add an alias"
                supporting="Press Enter to add"
                @keydown.enter.prevent="addChip('aliases')"
                @blur="addChip('aliases')"
              />
            </div>
            <template v-if="member.role === 'child'">
              <UiSelect
                v-model="d.gender"
                label="Gender (helps match girls' or boys' groups; optional)"
                :options="genderOptions"
              />
              <div class="chips">
                <span class="chips__label">Relevance tags (for example a sport or a class)</span>
                <div class="chips__row">
                  <UiChip
                    v-for="v in d.tags"
                    :key="v"
                    variant="input"
                    @remove="removeChip('tags', v)"
                  >
                    {{ v }}
                  </UiChip>
                </div>
                <UiTextField
                  v-model="chipText.tags"
                  label="Add a tag"
                  supporting="Press Enter to add"
                  @keydown.enter.prevent="addChip('tags')"
                  @blur="addChip('tags')"
                />
              </div>
            </template>
          </div>
        </UiCard>

        <UiCard variant="outlined">
          <template #header>Interests</template>
          <div class="stack">
            <UiTextField
              v-model="d.interests"
              label="Interests (comma separated)"
              name="interests"
            />
            <UiTextField v-model="d.hobbies" label="Hobbies (comma separated)" name="hobbies" />
            <UiTextField
              v-model="d.responsibilities"
              label="Responsibilities (comma separated)"
              name="responsibilities"
            />
          </div>
        </UiCard>

        <MemberAccess v-if="member.has_account !== undefined" :member="member" />

        <p v-if="error" class="err" role="alert">{{ error }}</p>
        <UiSaveBar
          :dirty="form.dirty.value"
          :saving="form.saving.value"
          :error="form.error.value"
          @save="saveEdit"
          @discard="form.discard"
        />
      </div>
    </UiSheet>

    <UiSheet v-model="adding" title="Add member">
      <form class="stack" data-testid="add-member" @submit.prevent="create">
        <UiTextField
          v-model="creating.display_name"
          label="Name"
          name="new_name"
          :error="createErrors.display_name"
        />
        <UiSelect v-model="creating.role" label="Role" :options="roleOptions" />
        <HuePicker v-model="creating.color" name="new_color" />
        <UiTextField
          v-if="creating.role === 'child'"
          v-model="creating.grade"
          label="Grade (optional)"
          name="new_grade"
        />
        <div class="chips">
          <span class="chips__label">Also known as (names used in mail)</span>
          <div class="chips__row">
            <UiChip
              v-for="v in createAliases"
              :key="v"
              variant="input"
              @remove="createAliases = createAliases.filter((x) => x !== v)"
            >
              {{ v }}
            </UiChip>
          </div>
          <UiTextField
            v-model="createAliasText"
            label="Add an alias"
            supporting="Press Enter to add"
            @keydown.enter.prevent="addCreateAlias"
            @blur="addCreateAlias"
          />
        </div>
        <fieldset class="login">
          <legend>
            {{ creating.role === 'adult' ? 'Login (optional)' : 'Login and PIN (optional)' }}
          </legend>
          <UiTextField
            v-model="creating.username"
            label="Username"
            name="new_username"
            autocomplete="off"
            :error="createErrors.username"
          />
          <UiTextField
            v-model="creating.password"
            label="Password"
            name="new_password"
            type="password"
            autocomplete="new-password"
            :error="createErrors.password"
          />
          <p v-if="creating.role === 'adult'" class="muted">
            The adult sets their own PIN after signing in, and must change this password at first
            sign-in.
          </p>
          <UiTextField
            v-if="creating.role === 'child'"
            v-model="creating.pin"
            label="PIN (4 to 6 digits)"
            name="new_pin"
            inputmode="numeric"
            autocomplete="off"
            maxlength="6"
            :error="createErrors.pin"
          />
        </fieldset>
        <p v-if="createError" class="err" role="alert">{{ createError }}</p>
        <p v-if="createdNote" class="muted" role="status" data-testid="created-note">
          {{ createdNote }}
        </p>
        <div class="actions">
          <UiButton variant="text" @click="adding = false">Close</UiButton>
          <UiButton type="submit">Add</UiButton>
        </div>
      </form>
    </UiSheet>
  </div>
</template>

<style scoped>
.members-manager {
  display: grid;
  gap: var(--ui-space-4);
}
.member {
  display: flex;
  align-items: center;
  gap: var(--ui-space-3);
}
.member__text {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
.fab {
  position: fixed;
  right: calc(var(--ui-space-4) + env(safe-area-inset-right));
  bottom: calc(var(--ui-bottom-offset, 0px) + var(--ui-space-4) + env(safe-area-inset-bottom));
  z-index: 15;
}
.editor,
.stack {
  display: grid;
  gap: var(--ui-space-3);
}
.avatar-edit {
  display: flex;
  align-items: center;
  gap: var(--ui-space-3);
  flex-wrap: wrap;
}
.file {
  display: grid;
  gap: var(--ui-space-1);
  font: var(--ui-type-body-md);
  color: var(--ui-on-surface-variant);
  min-width: 0;
}
.chips {
  display: grid;
  gap: var(--ui-space-2);
}
.chips__label {
  font: var(--ui-type-label-lg);
  color: var(--ui-on-surface-variant);
}
.chips__row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
.login {
  display: grid;
  gap: var(--ui-space-3);
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-md);
  padding: var(--ui-space-3);
  margin: 0;
}
.login legend {
  font: var(--ui-type-label-lg);
  color: var(--ui-on-surface-variant);
  padding: 0 var(--ui-space-1);
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--ui-space-2);
}
.muted {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.err {
  margin: 0;
  color: var(--ui-error);
}
</style>

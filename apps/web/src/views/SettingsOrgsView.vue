<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import type { ContactMode, ContactRole, OrgKind, OrgView } from '../api/mailSettings'
import ResponsiveItems from '../app/ResponsiveItems.vue'
import SettingsPage from '../app/SettingsPage.vue'
import { useMembersStore } from '../stores/members'
import { useOrgsStore } from '../stores/orgs'
import { useSourcesStore } from '../stores/sources'
import {
  UiButton,
  UiChip,
  UiEmptyState,
  UiIconButton,
  UiSection,
  UiSelect,
  UiSkeleton,
  UiTextField,
} from '../ui'
import ChipInput from './parts/settings-mail/ChipInput.vue'
import ConfirmDialog from './parts/settings-mail/ConfirmDialog.vue'
import EnrollmentsPanel from './parts/settings-mail/EnrollmentsPanel.vue'
import PageFab from './parts/settings-mail/PageFab.vue'
import PageNotices from './parts/settings-mail/PageNotices.vue'
import SheetForm from './parts/settings-mail/SheetForm.vue'
import { useConfirm } from './parts/settings-mail/useConfirm'
import { describeError } from './shared'

const store = useOrgsStore()
const members = useMembersStore()
const sources = useSourcesStore()
const confirm = useConfirm()
const error = ref('')

const KINDS: OrgKind[] = ['school', 'daycare', 'club', 'biller', 'other']
const ROLES: ContactRole[] = ['teacher', 'coach', 'admin', 'office', 'other']
const MODES: { key: ContactMode; label: string }[] = [
  { key: 'suggest', label: 'Suggest new contacts (you approve)' },
  { key: 'auto', label: 'Add new contacts automatically' },
  { key: 'manual', label: 'Never suggest' },
]
const plain = (xs: string[]) => xs.map((x) => ({ value: x, label: x }))
const kindOptions = plain(KINDS)
const roleOptions = plain(ROLES)
const modeOptions = MODES.map((m) => ({ value: m.key, label: m.label }))
const icsOptions = computed(() => [
  { value: '', label: 'None' },
  ...sources.calendars.map((c) => ({ value: c.id, label: c.name })),
])
const defaultCalOptions = computed(() => [
  { value: '', label: 'Choose each time' },
  ...sources.calendars
    .filter((x) => x.is_default_target)
    .map((c) => ({ value: c.id, label: c.name })),
])

const editing = ref<string | 'new' | null>(null)
const fresh = () => ({
  kind: 'school' as OrgKind,
  name: '',
  domains: [] as string[],
  senders: [] as string[],
  ics: '',
  defaultCal: '',
  mode: 'suggest' as ContactMode,
  amount: '',
  due: '',
  account: '',
})
const form = reactive(fresh())
const contact = reactive({ name: '', email: '', role: 'other' as ContactRole })
const contactFor = ref<OrgView | null>(null)

const children = computed(() => members.items.filter((m) => m.role === 'child'))
const calName = (id?: string | null) => sources.calendars.find((c) => c.id === id)?.name

onMounted(() => {
  void store.load()
  void members.load()
  void sources.load()
})

async function run(fn: () => Promise<unknown>, fallback: string) {
  error.value = ''
  try {
    await fn()
  } catch (e) {
    error.value = describeError(e, fallback)
  }
}

function open(o: OrgView | 'new') {
  editing.value = o === 'new' ? 'new' : o.id
  Object.assign(
    form,
    o === 'new'
      ? fresh()
      : {
          kind: o.kind,
          name: o.name,
          domains: [...o.domains],
          senders: [...o.senders],
          ics: o.ics_calendar ?? '',
          defaultCal: o.default_calendar ?? '',
          mode: o.contact_mode,
          amount: o.bill_template?.amount ?? '',
          due: o.bill_template?.due ?? '',
          account: o.bill_template?.account ?? '',
        },
  )
}

function template() {
  const t = {
    amount: form.amount.trim() || null,
    due: form.due.trim() || null,
    account: form.account.trim() || null,
  }
  return t.amount || t.due || t.account ? t : null
}

async function save() {
  await run(async () => {
    const body = {
      kind: form.kind,
      name: form.name.trim(),
      domains: form.domains,
      senders: form.senders,
      ics_calendar: form.ics || null,
      default_calendar: form.defaultCal || null,
      contact_mode: form.mode,
      bill_template: template(),
    }
    const cur = store.orgs.find((o) => o.id === editing.value)
    const ok = cur ? await store.patchOrg(cur, body) : await store.createOrg(body)
    if (ok) editing.value = null
  }, 'Could not save the organization.')
}

const remove = (o: OrgView) =>
  confirm.ask(`Delete “${o.name}”?`, 'Its contacts and enrollments go with it.', () =>
    run(() => store.removeOrg(o.id), 'Could not delete.'),
  )

async function addContact() {
  const o = contactFor.value
  if (!o) return
  await run(async () => {
    const ok = await store.addContact(o.id, {
      name: contact.name.trim(),
      email: contact.email.trim(),
      role: contact.role,
    })
    if (ok) {
      Object.assign(contact, { name: '', email: '', role: 'other' })
      contactFor.value = null
    }
  }, 'Could not add the contact.')
}
</script>

<template>
  <SettingsPage title="Organizations">
    <p class="intro">
      Schools, clubs and billers. Mail from their domains or senders is linked to them, and their
      contacts to your children.
    </p>
    <PageNotices
      :load-error="store.error"
      :conflict="store.conflict"
      :error="error"
      @reload="store.load()"
    />
    <UiSkeleton v-if="!store.loaded && !store.error" :lines="4" />
    <UiEmptyState
      v-else-if="store.loaded && !store.orgs.length"
      icon="group"
      title="No organizations yet."
    >
      <UiButton icon="add" @click="open('new')">Add organization</UiButton>
    </UiEmptyState>

    <ResponsiveItems :items="store.orgs">
      <template #item="{ item: o }">
        <div class="org" :data-testid="`org-${o.name}`">
          <div class="org__head">
            <strong class="org__name">{{ o.name }}</strong>
            <UiChip>{{ o.kind }}</UiChip>
          </div>
          <p class="meta">
            {{ [...o.domains, ...o.senders].join(', ') }}
            <template v-if="calName(o.default_calendar)">
              · events go to {{ calName(o.default_calendar) }}</template
            >
          </p>
          <ul class="contacts">
            <li v-for="c in o.contacts" :key="c.id">
              <span>{{ c.name }} ({{ c.email }}, {{ c.role }})</span>
              <UiIconButton
                icon="close"
                :label="`Remove ${c.name}`"
                @click="run(() => store.removeContact(c.id), 'Could not remove.')"
              />
            </li>
          </ul>
          <div class="row">
            <UiButton variant="tonal" @click="contactFor = o">Add contact</UiButton>
            <UiButton variant="text" @click="open(o)">Edit</UiButton>
            <UiButton variant="text" @click="remove(o)">Delete</UiButton>
          </div>
        </div>
      </template>
    </ResponsiveItems>

    <UiSection
      heading="Enrollments"
      supporting="Which school, class or club each child belongs to."
    >
      <EnrollmentsPanel v-for="c in children" :key="c.id" :child="c" :orgs="store.orgs" />
      <p v-if="!children.length" class="meta">No children yet.</p>
    </UiSection>

    <PageFab label="Add organization" @click="open('new')" />

    <SheetForm
      :model-value="editing !== null"
      :title="editing === 'new' ? 'New organization' : 'Edit organization'"
      @update:model-value="editing = null"
      @save="save"
    >
      <div data-testid="org-form" class="fields">
        <UiTextField v-model="form.name" label="Name" name="name" required />
        <UiSelect
          :model-value="form.kind"
          label="Kind"
          :options="kindOptions"
          @update:model-value="form.kind = $event as OrgKind"
        />
        <ChipInput v-model="form.domains" label="Mail domains (example.edu)" />
        <ChipInput v-model="form.senders" label="Specific sender addresses" />
        <UiSelect
          v-model="form.ics"
          label="Calendar feed from this organization"
          :options="icsOptions"
        />
        <UiSelect
          v-model="form.defaultCal"
          label="Default calendar for events from its mail"
          :options="defaultCalOptions"
        />
        <UiSelect
          :model-value="form.mode"
          label="New contacts"
          :options="modeOptions"
          @update:model-value="form.mode = $event as ContactMode"
        />
        <fieldset v-if="form.kind === 'biller' || form.amount || form.due || form.account">
          <legend>Bill patterns (regular expressions; group 1 is the value)</legend>
          <UiTextField v-model="form.amount" label="Amount" name="bill_amount" />
          <UiTextField v-model="form.due" label="Due date" name="bill_due" />
          <UiTextField v-model="form.account" label="Account" name="bill_account" />
        </fieldset>
      </div>
    </SheetForm>

    <SheetForm
      :model-value="!!contactFor"
      :title="`Add a contact for ${contactFor?.name ?? ''}`"
      save-label="Add contact"
      @update:model-value="contactFor = null"
      @save="addContact"
    >
      <UiTextField v-model="contact.name" label="Name" name="contact_name" required />
      <UiTextField
        v-model="contact.email"
        label="Email"
        type="email"
        name="contact_email"
        required
      />
      <UiSelect
        :model-value="contact.role"
        label="Role"
        :options="roleOptions"
        @update:model-value="contact.role = $event as ContactRole"
      />
    </SheetForm>
    <ConfirmDialog
      :pending="confirm.pending.value"
      @accept="confirm.accept"
      @cancel="confirm.cancel"
    />
  </SettingsPage>
</template>

<style scoped>
.intro,
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
.org,
.fields {
  display: grid;
  gap: var(--ui-space-2);
}
.fields {
  gap: var(--ui-space-3);
}
.org__head,
.row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--ui-space-2);
}
.org__name {
  flex: 1;
  font: var(--ui-type-title-md);
}
.contacts {
  display: grid;
  gap: var(--ui-space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}
.contacts li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ui-space-2);
}
fieldset {
  display: grid;
  gap: var(--ui-space-3);
  border: 0;
  padding: 0;
  margin: 0;
}
</style>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import type { ContactMode, ContactRole, OrgKind, OrgView } from '../api/mailSettings'
import ErrorBanner from '../components/ErrorBanner.vue'
import PageHeader from '../components/PageHeader.vue'
import StateBlock from '../components/StateBlock.vue'
import { useMembersStore } from '../stores/members'
import { useOrgsStore } from '../stores/orgs'
import { useSourcesStore } from '../stores/sources'
import ChipInput from './parts/settings-mail/ChipInput.vue'
import EnrollmentsPanel from './parts/settings-mail/EnrollmentsPanel.vue'
import { describeError } from './shared'
import './parts/settings.css'

const store = useOrgsStore()
const members = useMembersStore()
const sources = useSourcesStore()
const error = ref('')

const KINDS: OrgKind[] = ['school', 'daycare', 'club', 'biller', 'other']
const ROLES: ContactRole[] = ['teacher', 'coach', 'admin', 'office', 'other']
const MODES: { key: ContactMode; label: string }[] = [
  { key: 'suggest', label: 'Suggest new contacts (you approve)' },
  { key: 'auto', label: 'Add new contacts automatically' },
  { key: 'manual', label: 'Never suggest' },
]

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

async function remove(o: OrgView) {
  if (!confirm(`Delete "${o.name}"?`)) return
  await run(() => store.removeOrg(o.id), 'Could not delete.')
}

async function addContact(o: OrgView) {
  await run(async () => {
    const ok = await store.addContact(o.id, {
      name: contact.name.trim(),
      email: contact.email.trim(),
      role: contact.role,
    })
    if (ok) Object.assign(contact, { name: '', email: '', role: 'other' })
  }, 'Could not add the contact.')
}
</script>

<template>
  <section>
    <PageHeader eyebrow="Settings" title="Organizations">
      <template #actions>
        <button v-if="editing === null" type="button" class="btn-primary" @click="open('new')">
          Add organization
        </button>
      </template>
    </PageHeader>
    <p class="muted">
      Schools, clubs and billers. Mail from their domains or senders is linked to them, and their
      contacts to your children.
    </p>
    <ErrorBanner v-if="store.error" :message="store.error" />
    <ErrorBanner v-if="store.conflict" conflict @reload="store.load()" />
    <ErrorBanner v-if="error" :message="error" />

    <ul class="card-list">
      <li v-for="o in store.orgs" :key="o.id" :data-testid="`org-${o.name}`">
        <div class="row">
          <strong>{{ o.name }}</strong>
          <span class="badge">{{ o.kind }}</span>
          <span class="muted">
            {{ [...o.domains, ...o.senders].join(', ') }}
            <template v-if="calName(o.default_calendar)">
              · events go to {{ calName(o.default_calendar) }}</template
            >
          </span>
          <button type="button" class="btn-sm" @click="open(o)">Edit</button>
          <button type="button" class="btn-danger btn-sm" @click="remove(o)">Delete</button>
        </div>
        <ul class="contacts">
          <li v-for="c in o.contacts" :key="c.id">
            {{ c.name }} ({{ c.email }}, {{ c.role }})
            <button
              type="button"
              class="btn-ghost btn-sm"
              :aria-label="`Remove ${c.name}`"
              @click="run(() => store.removeContact(c.id), 'Could not remove.')"
            >
              ✕
            </button>
          </li>
        </ul>
        <form class="contact-form" @submit.prevent="addContact(o)">
          <input
            v-model="contact.name"
            :aria-label="`Contact name for ${o.name}`"
            placeholder="Name"
            required
          />
          <input
            v-model="contact.email"
            :aria-label="`Contact email for ${o.name}`"
            type="email"
            placeholder="Email"
            required
          />
          <select v-model="contact.role" :aria-label="`Role for ${o.name}`">
            <option v-for="r in ROLES" :key="r" :value="r">{{ r }}</option>
          </select>
          <button type="submit" class="btn-sm">Add contact</button>
        </form>
      </li>
    </ul>
    <StateBlock
      :loading="!store.loaded && !store.error"
      :empty="store.loaded && !store.orgs.length"
      empty-text="No organizations yet."
    />

    <form v-if="editing !== null" class="form" data-testid="org-form" @submit.prevent="save">
      <h2>{{ editing === 'new' ? 'New organization' : 'Edit organization' }}</h2>
      <label class="field">Name <input v-model="form.name" name="name" required /></label>
      <label class="field">
        Kind
        <select v-model="form.kind" name="kind">
          <option v-for="k in KINDS" :key="k" :value="k">{{ k }}</option>
        </select>
      </label>
      <ChipInput v-model="form.domains" label="Mail domains (example.edu)" />
      <ChipInput v-model="form.senders" label="Specific sender addresses" />
      <label class="field">
        Calendar feed from this organization
        <select v-model="form.ics" name="ics_calendar">
          <option value="">None</option>
          <option v-for="c in sources.calendars" :key="c.id" :value="c.id">{{ c.name }}</option>
        </select>
      </label>
      <label class="field">
        Default calendar for events from its mail
        <select v-model="form.defaultCal" name="default_calendar">
          <option value="">Choose each time</option>
          <option
            v-for="c in sources.calendars.filter((x) => x.is_default_target)"
            :key="c.id"
            :value="c.id"
          >
            {{ c.name }}
          </option>
        </select>
      </label>
      <label class="field">
        New contacts
        <select v-model="form.mode" name="contact_mode">
          <option v-for="m in MODES" :key="m.key" :value="m.key">{{ m.label }}</option>
        </select>
      </label>
      <fieldset v-if="form.kind === 'biller' || form.amount || form.due || form.account">
        <legend>Bill patterns (regular expressions; group 1 is the value)</legend>
        <label class="field">Amount <input v-model="form.amount" name="bill_amount" /></label>
        <label class="field">Due date <input v-model="form.due" name="bill_due" /></label>
        <label class="field">Account <input v-model="form.account" name="bill_account" /></label>
      </fieldset>
      <div class="row">
        <button type="submit" class="btn-primary">Save</button>
        <button type="button" class="btn-ghost" @click="editing = null">Cancel</button>
      </div>
    </form>

    <h2>Enrollments</h2>
    <p class="muted">Which school, class or club each child belongs to.</p>
    <EnrollmentsPanel v-for="c in children" :key="c.id" :child="c" :orgs="store.orgs" />
    <p v-if="!children.length" class="muted">No children yet.</p>
  </section>
</template>

<style scoped>
.contacts {
  list-style: none;
  margin: var(--space-2) 0;
  padding: 0;
}
.contact-form {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}
</style>

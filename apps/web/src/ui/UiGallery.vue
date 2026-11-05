<script setup lang="ts">
// Dev-only gallery (route /ui-gallery): every component in every state, with fictional sample data.
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { applyDensity, getDensity, type Density } from '../app/density'
import { FAMILIES, applyTheme, resolveVariant, type ThemeChoice } from '../themes'
import {
  UiBadge,
  UiBottomSheet,
  UiButton,
  UiCard,
  UiCheckbox,
  UiChip,
  UiDialog,
  UiDivider,
  UiEmptyState,
  UiErrorState,
  UiFab,
  UiIcon,
  UiIconButton,
  UiList,
  UiListItem,
  UiMenu,
  UiNavBar,
  UiNavDrawer,
  UiNavRail,
  UiPage,
  UiProgress,
  UiRadioGroup,
  UiSaveBar,
  UiSection,
  UiSegmented,
  UiSelect,
  UiSheet,
  UiSkeleton,
  UiSnackbarHost,
  UiSwitch,
  UiTextField,
  UiTopAppBar,
  icons,
  useBreakpoint,
  useFormDraft,
  useInstantSave,
  useSnackbar,
  type IconName,
  type NavDestination,
} from './index'

const route = useRoute()
const bp = useBreakpoint()
const snackbar = useSnackbar()

// ---- Theme (query: ?theme=<family>&variant=&mode=light|dark) ----
const q = (k: string) =>
  typeof route.query[k] === 'string' ? (route.query[k] as string) : undefined
const family = ref(q('theme') ?? 'catppuccin')
const mode = ref<'light' | 'dark'>(q('mode') === 'dark' ? 'dark' : 'light')
const variantId = ref(q('variant') ?? '')
const familyOptions = FAMILIES.map((f) => ({ value: f.id, label: f.label }))
function currentChoice(): ThemeChoice {
  return {
    family: family.value,
    mode: mode.value,
    ...(variantId.value ? { variant: variantId.value } : {}),
  }
}
const variants = computed(() => FAMILIES.find((f) => f.id === family.value)?.variants ?? [])
function apply() {
  applyTheme(resolveVariant(currentChoice(), { prefersDark: mode.value === 'dark' }))
}
watch([family, mode, variantId], apply)
onMounted(apply)
const showFocus = ref(false)
const density = ref<Density>(getDensity())
watch(density, (d) => applyDensity(d))
onMounted(() => applyDensity(density.value))

// ---- Sample data ----
const iconNames = Object.keys(icons) as IconName[]
const destinations: NavDestination[] = [
  { id: 'today', label: 'Today', icon: 'today', to: '/gallery/today' },
  { id: 'calendar', label: 'Calendar', icon: 'calendar', to: '/gallery/calendar' },
  { id: 'tasks', label: 'Tasks', icon: 'tasks', to: '/gallery/tasks', badge: 3 },
  { id: 'inbox', label: 'Inbox', icon: 'inbox', to: '/gallery/inbox', badge: 120 },
  { id: 'more', label: 'More', icon: 'more', to: '/gallery/more' },
]
const drawerDestinations: NavDestination[] = [
  ...destinations.slice(0, 4).map((d) => ({ ...d, group: 'Family' })),
  { id: 'routines', label: 'Routines', icon: 'routines', to: '/gallery/routines', group: 'Family' },
  { id: 'mail', label: 'Mail', icon: 'mail', to: '/gallery/mail', group: 'Household' },
  { id: 'bills', label: 'Bills', icon: 'bills', to: '/gallery/bills', group: 'Household' },
  {
    id: 'settings',
    label: 'Settings',
    icon: 'settings',
    to: '/gallery/settings',
    group: 'Household',
  },
]
const navDemo = ref<'bar' | 'rail' | 'drawer'>('bar')
const navActive = ref('/gallery/tasks/42')
const longText =
  'Parent-teacher conference for the whole upper elementary group, rescheduled from Thursday because of the school assembly'

// ---- Inputs ----
const longValue = ref(longText)
const text = ref('Pottery night')
const amount = ref('42.50')
const notes = ref('')
const choice = ref('weekly')
const radio = ref('adult')
const seg = ref('week')
const check = ref(true)
const sw = ref(false)
const chipSel = ref(true)
const chips = ref(['School', 'Sports'])
const selectOptions = [
  { value: 'weekly', label: 'Every week' },
  { value: 'monthly', label: 'Every month' },
  { value: 'never', label: 'Never', disabled: true },
]

// ---- Overlays ----
const dialog = ref(false)
const dialogLocked = ref(false)
const sheet = ref(false)
const bottomSheet = ref(false)
const menuItems = [
  { id: 'edit', label: 'Edit' },
  { id: 'move', label: 'Move to another list' },
  { id: 'delete', label: 'Delete', danger: true },
]
const picked = ref('')

// ---- useInstantSave demo ----
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms))
const instantOk = ref(false)
const okSave = useInstantSave(
  instantOk,
  async () => {
    await wait(250)
  },
  { label: 'Reminders saved' },
)
const instantFail = ref(true)
const failSave = useInstantSave(instantFail, async () => {
  await wait(250)
  throw new Error('The server said no. Nothing changed.')
})

// ---- useFormDraft demo ----
const stored = reactive({ name: 'Weekly review', day: 'sunday', remind: true })
const form = useFormDraft(
  () => ({ ...stored }),
  async (d) => {
    await wait(400)
    Object.assign(stored, d)
  },
  { confirmLeave: () => true },
)
const failing = ref(false)
const failForm = useFormDraft(
  () => ({ ...stored }),
  async () => {
    await wait(200)
    failing.value = true
    throw new Error('Could not reach the server.')
  },
  { confirmLeave: () => true },
)
const dayOptions = [
  { value: 'saturday', label: 'Saturday' },
  { value: 'sunday', label: 'Sunday' },
]
</script>

<template>
  <UiPage :class="{ 'ui-show-focus': showFocus }" wide>
    <template #top>
      <UiTopAppBar title="UI gallery">
        <template #actions>
          <UiIconButton icon="palette" label="Theme" disabled />
        </template>
      </UiTopAppBar>
    </template>

    <div class="g-wrap">
      <!-- Theme -->
      <UiSection heading="Theme" supporting="Query: ?theme=nord&mode=dark&variant=...">
        <div class="g-row">
          <UiSelect v-model="family" label="Family" :options="familyOptions" />
          <UiSelect
            v-model="variantId"
            label="Variant"
            :options="[
              { value: '', label: 'Default for mode' },
              ...variants.map((v) => ({ value: v.id, label: v.label })),
            ]"
          />
          <UiSegmented
            v-model="mode"
            label="Mode"
            :options="[
              { value: 'light', label: 'Light' },
              { value: 'dark', label: 'Dark' },
            ]"
          />
          <UiSegmented
            v-model="density"
            label="Density"
            :options="[
              { value: 'comfortable', label: 'Comfortable' },
              { value: 'compact', label: 'Compact' },
            ]"
          />
          <UiSwitch v-model="showFocus" label="Show focus rings" />
        </div>
        <p class="g-note">Window class: {{ bp }}</p>
      </UiSection>

      <UiSection heading="Colour tones" supporting="Surface containers and roles">
        <div class="g-tones">
          <div
            v-for="t in ['surface', 'lowest', 'low', 'mid', 'high', 'highest']"
            :key="t"
            class="g-tone"
            :class="`t-${t}`"
          >
            {{ t }}
          </div>
          <div class="g-tone t-primary">primary</div>
          <div class="g-tone t-primary-c">primary container</div>
          <div class="g-tone t-error">error</div>
          <div class="g-tone t-error-c">error container</div>
        </div>
      </UiSection>

      <UiSection heading="Type scale">
        <div class="g-type">
          <span style="font: var(--ui-type-display-sm)">Display small</span>
          <span style="font: var(--ui-type-headline-md)">Headline medium</span>
          <span style="font: var(--ui-type-title-lg)">Title large</span>
          <span style="font: var(--ui-type-title-md)">Title medium</span>
          <span style="font: var(--ui-type-body-lg)">Body large: the quick brown fox</span>
          <span style="font: var(--ui-type-body-md)">Body medium: the quick brown fox</span>
          <span style="font: var(--ui-type-label-lg)">Label large</span>
          <span style="font: var(--ui-type-label-sm)">Label small</span>
        </div>
        <div class="g-row g-shapes">
          <div
            v-for="r in ['none', 'xs', 'sm', 'md', 'lg', 'xl', 'full']"
            :key="r"
            class="g-shape"
            :style="{ borderRadius: `var(--ui-radius-${r})` }"
          >
            {{ r }}
          </div>
          <div
            v-for="e in [0, 1, 2, 3]"
            :key="e"
            class="g-shape g-elev"
            :style="{ boxShadow: `var(--ui-elev-${e})` }"
          >
            elev {{ e }}
          </div>
        </div>
      </UiSection>

      <!-- Icons -->
      <UiSection
        heading="Icons"
        supporting="Material Symbols Rounded via ui/icons.ts; 24 px and 20 px"
      >
        <div class="g-icons">
          <div v-for="n in iconNames" :key="n" class="g-icon">
            <UiIcon :name="n" />
            <span>{{ n }}</span>
          </div>
        </div>
        <div class="g-row">
          <UiIcon name="ai" :size="20" />
          <UiIcon name="ai" />
          <UiIcon name="warning" label="Warning" />
        </div>
      </UiSection>

      <!-- Actions -->
      <UiSection heading="Buttons, icon buttons, FAB">
        <div class="g-demo ui-demo">
          <div v-for="v in ['filled', 'tonal', 'outlined', 'text'] as const" :key="v" class="g-row">
            <UiButton :variant="v">{{ v }}</UiButton>
            <UiButton :variant="v" icon="add">With icon</UiButton>
            <UiButton :variant="v" disabled>Disabled</UiButton>
            <UiButton :variant="v" loading>Loading</UiButton>
          </div>
          <div class="g-row">
            <UiIconButton icon="edit" label="Edit" />
            <UiIconButton icon="delete" label="Delete" variant="tonal" />
            <UiIconButton icon="check" label="Done" variant="filled" />
            <UiIconButton icon="filter" label="Filter" :pressed="true" />
            <UiIconButton icon="more" label="More" disabled />
          </div>
          <div class="g-row">
            <UiFab icon="add" label="Add task" />
            <UiFab icon="add" label="New event" extended />
          </div>
        </div>
      </UiSection>

      <!-- Inputs -->
      <UiSection heading="Inputs">
        <div class="g-demo ui-demo g-grid">
          <UiTextField v-model="text" label="Title" supporting="Shown on the calendar" />
          <UiTextField
            v-model="amount"
            label="Amount"
            prefix="$"
            suffix="USD"
            type="text"
            inputmode="decimal"
          />
          <UiTextField v-model="notes" label="Name" error="A name is required" required />
          <UiTextField v-model="text" label="Disabled" disabled />
          <UiTextField v-model="notes" label="Notes" multiline supporting="Optional" />
          <UiTextField v-model="longValue" label="Long value" />
          <UiSelect
            v-model="choice"
            label="Repeat"
            :options="selectOptions"
            supporting="Never is unavailable"
          />
          <UiSelect
            v-model="choice"
            label="Repeat (error)"
            :options="selectOptions"
            error="Pick another option"
          />
          <UiRadioGroup
            v-model="radio"
            legend="Role"
            :options="[
              { value: 'adult', label: 'Adult', supporting: 'Can change settings' },
              { value: 'child', label: 'Child' },
              { value: 'guest', label: 'Guest', disabled: true },
            ]"
          />
          <div>
            <UiCheckbox v-model="check" label="Include completed tasks" />
            <UiCheckbox :model-value="false" label="Disabled checkbox" disabled />
            <UiCheckbox :model-value="false" label="Indeterminate" indeterminate />
          </div>
          <div>
            <UiSwitch v-model="sw" label="Notify me" />
            <UiSwitch :model-value="true" label="Disabled, on" disabled />
            <UiSwitch
              :model-value="false"
              label="A very long switch label that wraps onto a second line to show the layout"
            />
          </div>
          <UiSegmented
            v-model="seg"
            label="View"
            :options="[
              { value: 'day', label: 'Day' },
              { value: 'week', label: 'Week' },
              { value: 'month', label: 'Month' },
            ]"
          />
        </div>
        <div class="g-row ui-demo">
          <UiChip icon="label">Assist</UiChip>
          <UiChip variant="filter" :selected="chipSel" @click="chipSel = !chipSel"
            >Filter chip</UiChip
          >
          <UiChip variant="filter">Unselected</UiChip>
          <UiChip
            v-for="c in chips"
            :key="c"
            variant="input"
            @remove="chips = chips.filter((x) => x !== c)"
            >{{ c }}</UiChip
          >
          <UiChip disabled>Disabled</UiChip>
        </div>
      </UiSection>

      <!-- Surfaces and lists -->
      <UiSection heading="Cards">
        <div class="g-grid">
          <UiCard v-for="v in ['elevated', 'filled', 'outlined'] as const" :key="v" :variant="v">
            <template #header>{{ v }}</template>
            Pick up the library books before Friday.
            <template #actions
              ><UiButton variant="text">Later</UiButton
              ><UiButton variant="tonal">Done</UiButton></template
            >
          </UiCard>
          <UiCard variant="outlined" to="/ui-gallery">
            <template #header>Link card</template>The whole card is a link.
          </UiCard>
          <UiCard variant="filled">
            <template #header>Long text</template>
            {{ longText }}
          </UiCard>
          <UiCard variant="elevated"><UiSkeleton :lines="3" /></UiCard>
        </div>
      </UiSection>

      <UiSection heading="Lists">
        <div class="g-grid ui-demo">
          <UiCard variant="outlined">
            <UiList>
              <UiListItem headline="One line" />
              <UiDivider />
              <UiListItem
                headline="Two lines"
                supporting="With supporting text"
                @click="picked = 'two'"
              >
                <template #leading><UiIcon name="person" /></template>
                <template #trailing><UiIcon name="chevron_right" /></template>
              </UiListItem>
              <UiDivider />
              <UiListItem
                overline="Overdue"
                headline="Three lines"
                supporting="Overline, headline and supporting text"
                to="/ui-gallery"
              >
                <template #leading><UiIcon name="tasks" /></template>
                <template #trailing>12 May <UiBadge :count="2" /></template>
              </UiListItem>
              <UiDivider />
              <UiListItem :headline="longText" :supporting="longText" />
            </UiList>
          </UiCard>
          <UiCard variant="outlined">
            <UiEmptyState
              title="Nothing here yet"
              text="Tasks you add will show up in this list."
              icon="tasks"
            >
              <UiButton icon="add">Add a task</UiButton>
            </UiEmptyState>
          </UiCard>
        </div>
      </UiSection>

      <UiSection heading="Feedback">
        <div class="g-grid">
          <div class="g-stack">
            <UiProgress label="Syncing" />
            <UiProgress :value="60" label="Importing" />
            <div class="g-row">
              <UiProgress kind="circular" /><UiProgress kind="circular" :value="70" />
            </div>
          </div>
          <div class="g-stack">
            <UiSkeleton :lines="3" />
            <UiSkeleton shape="block" />
            <UiSkeleton shape="circle" />
          </div>
          <UiErrorState
            message="Could not load the calendar."
            @retry="snackbar.show({ text: 'Retrying' })"
          />
          <div class="g-row">
            Badges: <UiBadge :count="3" /> <UiBadge :count="150" /> <UiBadge dot />
            <UiBadge :count="0" />
          </div>
        </div>
      </UiSection>

      <!-- Overlays -->
      <UiSection heading="Overlays and snackbar">
        <div class="g-row ui-demo">
          <UiButton variant="outlined" @click="dialog = true">Dialog</UiButton>
          <UiButton variant="outlined" @click="dialogLocked = true"
            >Dialog (not dismissible)</UiButton
          >
          <UiButton variant="outlined" @click="bottomSheet = true">Bottom sheet</UiButton>
          <UiButton variant="outlined" @click="sheet = true">Sheet (auto)</UiButton>
          <UiMenu :items="menuItems" label="Task actions" @select="picked = $event">
            <template #activator="{ toggle, attrs }">
              <UiButton variant="tonal" icon="more" v-bind="attrs" @click="toggle">Menu</UiButton>
            </template>
          </UiMenu>
          <UiButton
            variant="text"
            @click="
              snackbar.show({
                text: 'Task moved to Tomorrow',
                action: {
                  label: 'Undo',
                  run: () => {
                    picked = 'undone'
                  },
                },
              })
            "
            >Snackbar</UiButton
          >
          <UiButton
            variant="text"
            @click="
              ['One', 'Two', 'Three'].forEach((t) =>
                snackbar.show({ text: `Queued message ${t}`, timeoutMs: 2000 }),
              )
            "
            >Queue three</UiButton
          >
        </div>
        <p class="g-note">Last action: {{ picked || 'none' }}</p>
        <UiDialog v-model="dialog" title="Delete this task?">
          Pickup groceries will be removed for everyone. This cannot be undone.
          <template #actions>
            <UiButton variant="text" @click="dialog = false">Cancel</UiButton>
            <UiButton @click="dialog = false">Delete</UiButton>
          </template>
        </UiDialog>
        <UiDialog v-model="dialogLocked" title="Finish setup" :dismissible="false">
          This dialog ignores Escape and the scrim.
          <template #actions><UiButton @click="dialogLocked = false">Close</UiButton></template>
        </UiDialog>
        <UiBottomSheet v-model="bottomSheet" title="New task">
          <UiTextField v-model="text" label="Title" />
          <template #actions><UiButton @click="bottomSheet = false">Add</UiButton></template>
        </UiBottomSheet>
        <UiSheet v-model="sheet" title="Add an event">
          <UiTextField v-model="text" label="Title" />
          <template #actions>
            <UiButton variant="text" @click="sheet = false">Cancel</UiButton>
            <UiButton @click="sheet = false">Add</UiButton>
          </template>
        </UiSheet>
      </UiSection>

      <!-- Apply-changes model -->
      <UiSection
        heading="Instant save (D-096)"
        supporting="Optimistic change, then a snackbar with Undo. A failure reverts."
      >
        <div class="g-grid ui-demo">
          <UiCard variant="outlined">
            <UiSwitch
              :model-value="instantOk"
              label="Daily digest"
              :loading="okSave.saving.value"
              @update:model-value="okSave.set"
            />
          </UiCard>
          <UiCard variant="outlined">
            <UiSwitch
              :model-value="instantFail"
              label="Save fails (reverts)"
              @update:model-value="failSave.set"
            />
          </UiCard>
        </div>
      </UiSection>

      <UiSection
        heading="Form draft and save bar (D-096)"
        supporting="Edit a field to show the bar; Discard resets; Save awaits the server."
      >
        <div class="g-grid ui-demo">
          <UiCard variant="outlined">
            <div class="g-stack g-pad">
              <UiTextField v-model="form.draft.value.name" label="Name" />
              <UiSelect v-model="form.draft.value.day" label="Day" :options="dayOptions" />
              <UiSwitch v-model="form.draft.value.remind" label="Remind me" />
              <UiSaveBar
                :dirty="form.dirty.value"
                :saving="form.saving.value"
                :error="form.error.value"
                @save="form.save"
                @discard="form.discard"
              />
            </div>
          </UiCard>
          <UiCard variant="outlined">
            <div class="g-stack g-pad">
              <UiTextField v-model="failForm.draft.value.name" label="Name (save fails)" />
              <UiSaveBar
                :dirty="failForm.dirty.value"
                :saving="failForm.saving.value"
                :error="failForm.error.value"
                @save="failForm.save"
                @discard="failForm.discard"
              />
            </div>
          </UiCard>
        </div>
      </UiSection>

      <!-- Navigation -->
      <UiSection
        heading="Navigation"
        supporting="One destinations list, three forms. The app picks by window class."
      >
        <UiSegmented
          v-model="navDemo"
          label="Form"
          :options="[
            { value: 'bar', label: 'Bar' },
            { value: 'rail', label: 'Rail' },
            { value: 'drawer', label: 'Drawer' },
          ]"
        />
        <div class="g-nav ui-demo">
          <UiNavBar
            v-if="navDemo === 'bar'"
            :destinations="destinations"
            :active-path="navActive"
          />
          <UiNavRail
            v-else-if="navDemo === 'rail'"
            :destinations="destinations"
            :active-path="navActive"
          />
          <UiNavDrawer v-else :destinations="drawerDestinations" :active-path="navActive" />
        </div>
        <UiSegmented
          v-model="navActive"
          label="Active path"
          :options="[
            { value: '/gallery/today', label: 'Today' },
            { value: '/gallery/tasks/42', label: 'Tasks/42' },
            { value: '/gallery/inbox', label: 'Inbox' },
          ]"
        />
      </UiSection>
    </div>
    <UiSnackbarHost />
  </UiPage>
</template>

<style scoped>
.g-wrap {
  display: grid;
  gap: var(--ui-space-6);
  max-width: 1200px;
  margin: 0 auto;
  color: var(--ui-on-surface);
}
.g-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--ui-space-3) var(--ui-space-4);
}
.g-grid {
  display: grid;
  gap: var(--ui-space-4);
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 280px), 1fr));
  align-items: start;
}
.g-stack {
  display: grid;
  gap: var(--ui-space-3);
}
.g-pad {
  padding: var(--ui-space-4);
}
.g-demo {
  display: grid;
  gap: var(--ui-space-3);
}
.g-note {
  margin: 0;
  font: var(--ui-type-body-sm);
  color: var(--ui-on-surface-variant);
}
.g-tones {
  display: grid;
  gap: var(--ui-space-2);
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
}
.g-tone {
  display: grid;
  place-items: center;
  min-height: 64px;
  padding: var(--ui-space-2);
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-sm);
  color: var(--ui-on-surface);
  font: var(--ui-type-label-md);
  text-align: center;
}
.t-surface {
  background: var(--ui-surface);
}
.t-lowest {
  background: var(--ui-surface-container-lowest);
}
.t-low {
  background: var(--ui-surface-container-low);
}
.t-mid {
  background: var(--ui-surface-container-mid);
}
.t-high {
  background: var(--ui-surface-container-high);
}
.t-highest {
  background: var(--ui-surface-container-highest);
}
.t-primary {
  background: var(--ui-primary);
  color: var(--ui-on-primary);
}
.t-primary-c {
  background: var(--ui-primary-container);
  color: var(--ui-on-primary-container);
}
.t-error {
  background: var(--ui-error);
  color: var(--ui-on-error);
}
.t-error-c {
  background: var(--ui-error-container);
  color: var(--ui-on-error-container);
}
.g-type {
  display: grid;
  gap: var(--ui-space-1);
}
.g-shapes {
  margin-top: var(--ui-space-3);
}
.g-shape {
  display: grid;
  place-items: center;
  width: 72px;
  height: 56px;
  border: 1px solid var(--ui-outline);
  font: var(--ui-type-label-md);
}
.g-elev {
  border: 0;
  background: var(--ui-surface-container-low);
}
.g-icons {
  display: grid;
  gap: var(--ui-space-2);
  grid-template-columns: repeat(auto-fill, minmax(88px, 1fr));
}
.g-icon {
  display: grid;
  justify-items: center;
  gap: var(--ui-space-1);
  padding: var(--ui-space-2) 0;
  font: var(--ui-type-label-sm);
  color: var(--ui-on-surface-variant);
  text-align: center;
  overflow-wrap: anywhere;
}
.g-icon :deep(svg) {
  color: var(--ui-on-surface);
}
.g-nav {
  max-width: 100%;
  overflow: hidden;
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-md);
}
.g-nav :deep(.ui-rail),
.g-nav :deep(.ui-drawer) {
  min-height: 420px;
}
</style>

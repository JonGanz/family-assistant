<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  addPanel,
  applyPreset,
  clampDays,
  FAMILY_OPTIONS,
  isValid,
  movePanel,
  panelHasId,
  removePanel,
  setPanelOption,
  themeControls,
  themeOf,
  toggleMember,
  togglePanelId,
  withFamily,
  withMode,
  withVariant,
} from './editor'
import { prefersDark } from './theme'
import type { ThemeChoice } from '../themes'
import { KNOWN_PANELS, PRESETS } from './layout'
import { S } from './strings'
import type { DashboardSettings, MemberPublic, TaskListView } from './types'

const props = defineProps<{
  modelValue: DashboardSettings
  members: MemberPublic[]
  lists?: TaskListView[]
  saving?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [s: DashboardSettings]; save: [] }>()

const s = computed(() => props.modelValue)
const memberIds = computed(() => props.members.map((m) => m.id))
const listIds = computed(() => (props.lists ?? []).map((l) => l.id))
const toAdd = ref('')
const theme = computed(() => themeOf(s.value))
const controls = computed(() => themeControls(theme.value, { prefersDark: prefersDark() }))
const setTheme = (t: ThemeChoice) => set({ ...s.value, theme: t })
const valid = computed(() => isValid(s.value))
const known = (k: string) => KNOWN_PANELS.includes(k)

function set(next: DashboardSettings) {
  emit('update:modelValue', next)
}
function isOn(id: string): boolean {
  return s.value.members === 'all' || s.value.members.includes(id)
}
function add() {
  if (toAdd.value) set(addPanel(s.value, toAdd.value))
  toAdd.value = ''
}
function preset(e: Event) {
  const v = (e.target as HTMLSelectElement).value
  if (v) set(applyPreset(s.value, v))
  ;(e.target as HTMLSelectElement).value = ''
}
</script>

<template>
  <form class="editor" data-testid="layout-editor" @submit.prevent="valid && emit('save')">
    <div class="row">
      <label
        >{{ S.preset }}
        <select data-testid="preset" @change="preset">
          <option value="">{{ S.choosePreset }}</option>
          <option v-for="name in Object.keys(PRESETS)" :key="name" :value="name">{{ name }}</option>
        </select>
      </label>
      <label
        >{{ S.orientation }}
        <select
          data-testid="orientation"
          :value="s.orientation"
          @change="
            set({
              ...s,
              orientation: ($event.target as HTMLSelectElement)
                .value as DashboardSettings['orientation'],
            })
          "
        >
          <option value="landscape">{{ S.landscape }}</option>
          <option value="portrait">{{ S.portrait }}</option>
        </select>
      </label>
      <label
        >{{ S.themeFamily }}
        <select
          data-testid="theme-family"
          :value="theme.family"
          @change="setTheme(withFamily(theme, ($event.target as HTMLSelectElement).value))"
        >
          <option v-for="f in FAMILY_OPTIONS" :key="f.id" :value="f.id">{{ f.label }}</option>
          <option v-if="!FAMILY_OPTIONS.some((f) => f.id === theme.family)" :value="theme.family">
            {{ theme.family }}
          </option>
        </select>
      </label>
      <label v-if="controls.showVariant"
        >{{ S.themeVariant }}
        <select
          data-testid="theme-variant"
          :value="controls.current.id"
          @change="setTheme(withVariant(theme, ($event.target as HTMLSelectElement).value))"
        >
          <option v-for="v in controls.variants" :key="v.id" :value="v.id">{{ v.label }}</option>
        </select>
      </label>
      <label v-if="controls.showMode"
        >{{ S.themeMode }}
        <select
          data-testid="theme"
          :value="theme.mode"
          @change="
            setTheme(
              withMode(theme, ($event.target as HTMLSelectElement).value as ThemeChoice['mode']),
            )
          "
        >
          <option value="auto">{{ S.themeAuto }}</option>
          <option value="light">{{ S.themeLight }}</option>
          <option value="dark">{{ S.themeDark }}</option>
        </select>
      </label>
      <label
        >{{ S.agendaDays }}
        <input
          data-testid="agenda-days"
          type="number"
          min="1"
          max="14"
          :value="s.agenda_days"
          @change="
            set({ ...s, agenda_days: clampDays(Number(($event.target as HTMLInputElement).value)) })
          "
        />
      </label>
    </div>

    <fieldset v-if="members.length">
      <legend>{{ S.memberFilter }}</legend>
      <label v-for="m in members" :key="m.id" class="check">
        <input
          type="checkbox"
          :data-member="m.id"
          :checked="isOn(m.id)"
          @change="set(toggleMember(s, m.id, memberIds))"
        />
        {{ m.display_name }}
      </label>
    </fieldset>

    <fieldset>
      <legend>{{ S.panels }}</legend>
      <ol>
        <li v-for="(p, i) in s.panels" :key="i" :data-panel="p.kind">
          <span class="name">{{ S.panelNames[p.kind] ?? p.kind }}</span>
          <span v-if="!known(p.kind)" class="unknown">?</span>
          <label v-if="p.kind === 'weather'" class="check">
            <input
              type="checkbox"
              :checked="p['hourly'] === true"
              @change="
                set(setPanelOption(s, i, 'hourly', ($event.target as HTMLInputElement).checked))
              "
            />
            {{ S.hourly }}
          </label>
          <label v-if="p.kind === 'routine'" class="check">
            <input
              type="checkbox"
              data-testid="pinned"
              :checked="p['pinned'] === true"
              @change="
                set(setPanelOption(s, i, 'pinned', ($event.target as HTMLInputElement).checked))
              "
            />
            {{ S.pinned }}
          </label>
          <span
            v-if="['now', 'member_tasks', 'routine'].includes(p.kind) && members.length"
            class="opts"
            :aria-label="S.panelMembers"
          >
            <label v-for="m in members" :key="m.id" class="check">
              <input
                type="checkbox"
                :data-opt="`members:${m.id}`"
                :checked="panelHasId(p, 'members', m.id)"
                @change="set(togglePanelId(s, i, 'members', m.id, memberIds))"
              />
              {{ m.display_name }}
            </label>
          </span>
          <span
            v-if="['now', 'member_tasks'].includes(p.kind) && listIds.length"
            class="opts"
            :aria-label="S.panelLists"
          >
            <label v-for="l in lists" :key="l.id" class="check">
              <input
                type="checkbox"
                :data-opt="`lists:${l.id}`"
                :checked="panelHasId(p, 'lists', l.id)"
                @change="set(togglePanelId(s, i, 'lists', l.id, listIds))"
              />
              {{ l.name }}
            </label>
          </span>
          <button
            type="button"
            :disabled="i === 0"
            :aria-label="S.moveUp"
            @click="set(movePanel(s, i, -1))"
          >
            ↑
          </button>
          <button
            type="button"
            :disabled="i === s.panels.length - 1"
            :aria-label="S.moveDown"
            @click="set(movePanel(s, i, 1))"
          >
            ↓
          </button>
          <button
            type="button"
            :aria-label="`${S.remove} ${p.kind}`"
            @click="set(removePanel(s, i))"
          >
            ✕
          </button>
        </li>
      </ol>
      <div class="row">
        <select v-model="toAdd" data-testid="add-kind" :aria-label="S.addPanel">
          <option value="">{{ S.addPanel }}…</option>
          <option v-for="k in KNOWN_PANELS" :key="k" :value="k">{{ S.panelNames[k] }}</option>
        </select>
        <button
          type="button"
          data-testid="add-panel"
          :disabled="!toAdd || s.panels.length >= 12"
          @click="add"
        >
          {{ S.addPanel }}
        </button>
      </div>
    </fieldset>

    <button type="submit" :disabled="!valid || saving" data-testid="save-layout">
      {{ S.save }}
    </button>
  </form>
</template>

<style scoped>
.editor {
  display: grid;
  gap: var(--space-3);
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
  align-items: end;
}
label {
  display: grid;
  gap: 0.15rem;
}
.check {
  display: inline-flex;
  grid-auto-flow: column;
  align-items: center;
  gap: var(--space-2);
}
fieldset {
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
}
ol {
  list-style: none;
  padding: 0;
  margin: 0 0 var(--space-2);
  display: grid;
  gap: var(--space-1);
}
li {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}
.opts {
  display: inline-flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.name {
  min-width: 7rem;
}
.unknown {
  color: var(--color-warning);
}
</style>

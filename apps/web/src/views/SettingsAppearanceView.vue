<script setup lang="ts">
// Appearance is per device. Every choice applies at once (D-096) with the shared Undo snackbar.
import { computed, ref } from 'vue'
import SettingsPage from '../app/SettingsPage.vue'
import { getDensity, setDensity, type Density } from '../app/density'
import { prefersDark } from '../dashboard/theme'
import { themeControls, withFamily, withMode, withVariant } from '../dashboard/editor'
import { S } from '../dashboard/strings'
import { FAMILIES, HUES, resolveVariant, useDeviceTheme } from '../themes'
import type { ThemeChoice, ThemeFamily } from '../themes'
import { UiCard, UiSegmented, UiSelect, useInstantSave } from '../ui'

const { choice, setChoice } = useDeviceTheme()
const controls = computed(() => themeControls(choice.value, { prefersDark: prefersDark() }))

// The theme choice is a writable view over the device store, so Undo can put the old one back.
const target = computed<ThemeChoice>({
  get: () => choice.value,
  set: (c) => setChoice(c),
})
const themeSave = useInstantSave(target, async () => {}, { label: 'Theme saved' })
const pick = (c: ThemeChoice) => void themeSave.set(c)

const density = ref<Density>(getDensity())
const densitySave = useInstantSave(density, async (v) => setDensity(v), { label: 'Density saved' })

const preview = (f: ThemeFamily) =>
  resolveVariant({ family: f.id, mode: choice.value.mode }, { prefersDark: prefersDark() })
const isCurrent = (f: ThemeFamily) => f.id === choice.value.family
const onlyNote = computed(() =>
  controls.value.onlyMode === 'dark'
    ? S.onlyDark
    : controls.value.onlyMode === 'light'
      ? S.onlyLight
      : '',
)
const modeOptions = [
  { value: 'auto', label: S.themeAuto },
  { value: 'light', label: S.themeLight },
  { value: 'dark', label: S.themeDark },
]
const variantOptions = computed(() =>
  controls.value.variants.map((v) => ({ value: v.id, label: v.label })),
)
const densityOptions = [
  { value: 'comfortable', label: 'Comfortable' },
  { value: 'compact', label: 'Compact' },
]
</script>

<template>
  <SettingsPage :title="S.appearanceTitle">
    <p class="note" data-testid="device-only">{{ S.appearanceDeviceOnly }}</p>

    <UiCard>
      <template #header>Theme</template>
      <ul class="families">
        <li
          v-for="f in FAMILIES"
          :key="f.id"
          class="family"
          :class="{ current: isCurrent(f) }"
          :data-family="f.id"
        >
          <button
            type="button"
            class="pick"
            :aria-pressed="isCurrent(f)"
            @click="pick(withFamily(choice, f.id))"
          >
            <strong>{{ f.label }}</strong>
            <span v-if="isCurrent(f)" class="note">{{ S.themeInUse }}</span>
          </button>
          <div class="strip" aria-hidden="true">
            <span
              v-for="c in [
                preview(f).neutrals.bg,
                preview(f).neutrals.surface,
                preview(f).neutrals.text,
              ]"
              :key="c"
              class="sw"
              :style="{ background: c }"
            ></span>
            <span
              v-for="h in HUES"
              :key="h"
              class="sw"
              :style="{ background: preview(f).hues[h] }"
            ></span>
          </div>
        </li>
      </ul>
    </UiCard>

    <UiCard>
      <template #header>Mode</template>
      <UiSegmented
        v-if="controls.showMode"
        data-testid="mode"
        :label="S.themeMode"
        :model-value="choice.mode"
        :options="modeOptions"
        @update:model-value="pick(withMode(choice, $event as ThemeChoice['mode']))"
      />
      <p v-else class="note" data-testid="mode-note">{{ onlyNote }}</p>
      <UiSelect
        v-if="controls.showVariant"
        data-testid="variant"
        :label="S.themeVariant"
        :model-value="controls.current.id"
        :options="variantOptions"
        @update:model-value="pick(withVariant(choice, $event))"
      />
    </UiCard>

    <UiCard>
      <template #header>Density</template>
      <UiSegmented
        data-testid="density"
        label="Density"
        :model-value="density"
        :options="densityOptions"
        @update:model-value="densitySave.set($event as Density)"
      />
      <p class="note">Compact fits more on the screen. Stored on this device.</p>
    </UiCard>
  </SettingsPage>
</template>

<style scoped>
.note {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-md);
}
.families {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(13rem, 1fr));
  gap: var(--ui-space-3);
}
.family {
  padding: var(--ui-space-3);
  border: 1px solid var(--ui-outline-variant);
  border-radius: var(--ui-radius-md);
  background: var(--ui-surface-container-low);
}
.family.current {
  border: 2px solid var(--ui-primary);
  padding: calc(var(--ui-space-3) - 1px);
}
.pick {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  min-height: var(--ui-touch);
  background: none;
  border: 0;
  padding: 0;
  color: var(--ui-on-surface);
  font: var(--ui-type-title-sm);
  cursor: pointer;
}
.pick:focus-visible {
  outline: var(--ui-focus-ring);
  outline-offset: 2px;
}
.strip {
  display: flex;
  border-radius: var(--ui-radius-sm);
  overflow: hidden;
  border: 1px solid var(--ui-outline-variant);
}
.sw {
  flex: 1;
  height: 1.5rem;
}
</style>

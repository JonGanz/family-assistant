<script setup lang="ts">
import { computed } from 'vue'
import PageHeader from '../components/PageHeader.vue'
import { prefersDark } from '../dashboard/theme'
import { themeControls, withFamily, withMode, withVariant } from '../dashboard/editor'
import { S } from '../dashboard/strings'
import { FAMILIES, HUES, resolveVariant, useDeviceTheme } from '../themes'
import type { ThemeChoice, ThemeFamily } from '../themes'

const { choice, setChoice } = useDeviceTheme()
const controls = computed(() => themeControls(choice.value, { prefersDark: prefersDark() }))

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
const val = (e: Event) => (e.target as HTMLSelectElement).value
</script>

<template>
  <section aria-labelledby="page-title">
    <PageHeader :title="S.appearanceTitle" eyebrow="Settings" />
    <p class="muted" data-testid="device-only">{{ S.appearanceDeviceOnly }}</p>

    <ul class="families">
      <li
        v-for="f in FAMILIES"
        :key="f.id"
        class="card"
        :class="{ current: isCurrent(f) }"
        :data-family="f.id"
      >
        <button
          type="button"
          class="pick"
          :aria-pressed="isCurrent(f)"
          @click="setChoice(withFamily(choice, f.id))"
        >
          <strong>{{ f.label }}</strong>
          <span v-if="isCurrent(f)" class="muted">{{ S.themeInUse }}</span>
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

    <div class="controls">
      <label v-if="controls.showMode"
        >{{ S.themeMode }}
        <select
          data-testid="mode"
          :value="choice.mode"
          @change="setChoice(withMode(choice, val($event) as ThemeChoice['mode']))"
        >
          <option value="auto">{{ S.themeAuto }}</option>
          <option value="light">{{ S.themeLight }}</option>
          <option value="dark">{{ S.themeDark }}</option>
        </select>
      </label>
      <p v-else class="muted" data-testid="mode-note">{{ onlyNote }}</p>
      <label v-if="controls.showVariant"
        >{{ S.themeVariant }}
        <select
          data-testid="variant"
          :value="controls.current.id"
          @change="setChoice(withVariant(choice, val($event)))"
        >
          <option v-for="v in controls.variants" :key="v.id" :value="v.id">{{ v.label }}</option>
        </select>
      </label>
    </div>
  </section>
</template>

<style scoped>
.families {
  list-style: none;
  margin: var(--space-3) 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
  gap: var(--space-3);
}
.card.current {
  border-color: var(--color-accent);
}
.pick {
  display: flex;
  justify-content: space-between;
  width: 100%;
  background: none;
  border: 0;
  padding: 0 0 var(--space-2);
  color: var(--color-text);
  font: inherit;
  cursor: pointer;
}
.strip {
  display: flex;
  border-radius: var(--radius-md);
  overflow: hidden;
  border: 1px solid var(--color-border);
}
.sw {
  flex: 1;
  height: 1.5rem;
}
.controls {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
  align-items: end;
}
</style>

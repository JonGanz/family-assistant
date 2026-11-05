<script setup lang="ts">
import ResponsiveItems from '../../../app/ResponsiveItems.vue'
import { useMailSettingsStore } from '../../../stores/mailSettings'
import InstantCheck from './InstantCheck.vue'

const store = useMailSettingsStore()
// A signal row is keyed by `key`; ResponsiveItems wants an `id`.
const items = () => store.signals.map((s) => ({ ...s, id: s.key }))
</script>

<template>
  <div class="panel">
    <ResponsiveItems :items="items()">
      <template #item="{ item: s }">
        <InstantCheck
          as="switch"
          :model-value="s.enabled"
          :label="s.description"
          :save="(v: boolean) => store.setSignal(s.key, v)"
        />
        <p class="meta">{{ s.key }} · {{ s.decision_point }}, confidence {{ s.confidence }}</p>
      </template>
    </ResponsiveItems>
  </div>
</template>

<style scoped>
.meta {
  margin: 0;
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-body-sm);
}
</style>

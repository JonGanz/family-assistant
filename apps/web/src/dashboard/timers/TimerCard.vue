<script setup lang="ts">
import { computed } from 'vue'
import UiButton from '../../ui/UiButton.vue'
import UiIconButton from '../../ui/UiIconButton.vue'
import { S } from '../strings'
import { formatRemaining, fraction, remaining, type Timer } from './timersModel'

const props = defineProps<{ timer: Timer; now: number }>()
const emit = defineEmits<{
  addTime: []
  pause: []
  resume: []
  cancel: []
  dismiss: []
  remove: []
}>()

const R = 54
const C = 2 * Math.PI * R
const left = computed(() => remaining(props.timer, props.now))
const text = computed(() => formatRemaining(left.value))
const offset = computed(() => C * (1 - fraction(props.timer, props.now)))
const ringing = computed(() => props.timer.state === 'ringing')
const title = computed(() => props.timer.name || S.timerDefaultName)
</script>

<template>
  <article
    class="card"
    :class="[`st-${timer.state}`, { ringing }]"
    :data-testid="`timer-card`"
    :data-state="timer.state"
    @click="ringing && emit('dismiss')"
  >
    <h3>{{ title }}</h3>
    <div class="ring">
      <svg viewBox="0 0 120 120" aria-hidden="true">
        <circle class="track" cx="60" cy="60" :r="R" />
        <circle
          class="bar"
          cx="60"
          cy="60"
          :r="R"
          :stroke-dasharray="C"
          :stroke-dashoffset="offset"
          transform="rotate(-90 60 60)"
        />
      </svg>
      <output class="time" data-testid="timer-time">{{ text }}</output>
    </div>
    <UiIconButton
      class="remove"
      icon="delete"
      :label="S.timerRemove"
      data-testid="timer-remove"
      @click.stop="emit('remove')"
    />
    <div class="acts" @click.stop>
      <UiButton
        v-if="ringing"
        class="wide"
        icon="check"
        data-testid="timer-dismiss"
        @click="emit('dismiss')"
      >
        {{ S.timerDismiss }}
      </UiButton>
      <template v-else>
        <UiButton
          v-if="timer.state === 'running'"
          variant="tonal"
          data-testid="timer-pause"
          @click="emit('pause')"
        >
          {{ S.timerPause }}
        </UiButton>
        <UiButton v-else data-testid="timer-resume" @click="emit('resume')">
          {{ timer.state === 'idle' ? S.timerStart : S.timerResume }}
        </UiButton>
        <UiButton variant="tonal" icon="add" data-testid="timer-add-time" @click="emit('addTime')">
          {{ S.timerPlusMinute }}
        </UiButton>
        <UiButton
          v-if="timer.state !== 'idle'"
          class="wide"
          variant="tonal"
          data-testid="timer-cancel-run"
          @click="emit('cancel')"
        >
          {{ S.timerReset }}
        </UiButton>
      </template>
    </div>
  </article>
</template>

<style scoped>
.card {
  position: relative;
  container-type: inline-size;
  display: grid;
  /* title, ring, actions: the ring sits under the title and the actions at the bottom, so
     rings and buttons line up across cards of one grid row whatever the state */
  grid-template-rows: auto auto 1fr;
  justify-items: center;
  gap: var(--space-3);
  padding: var(--space-4);
  font-size: var(--kiosk-font-small);
  border: 1px solid var(--color-border);
  border-radius: var(--ui-radius-md);
  background: var(--color-layer-1, var(--color-surface));
  box-shadow: 0 1px 3px color-mix(in srgb, var(--color-text) 14%, transparent);
}
h3 {
  margin: 0;
  max-width: 100%;
  padding-inline: var(--ui-touch);
  box-sizing: border-box;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--kiosk-font-base);
}
.ring {
  position: relative;
  width: min(100%, 13rem);
  margin-inline: auto;
  aspect-ratio: 1;
}
svg {
  width: 100%;
  height: 100%;
}
.track,
.bar {
  fill: none;
  stroke-width: 8;
}
.track {
  stroke: var(--color-layer-3);
}
.bar {
  stroke: var(--color-accent);
  stroke-linecap: round;
}
.st-paused .bar,
.st-idle .bar {
  opacity: 0.55;
}
.time {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  font-size: clamp(1.6rem, 14cqi, 2.2rem);
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}
.acts {
  --btn-h: max(var(--ui-touch, 48px), 2.4em);
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  grid-auto-rows: var(--btn-h);
  align-self: end;
  gap: var(--space-2);
  width: 100%;
}
/* The shared button sized from the kiosk text (its own label and height are app-sized) */
.acts :deep(.ui-btn) {
  box-sizing: border-box;
  min-width: 0;
  width: 100%;
  height: var(--btn-h);
  min-height: 0;
  padding-inline: 0.5em;
  gap: 0.3em;
  font: inherit;
  font-weight: 600;
  line-height: 1;
  white-space: nowrap;
}
.acts :deep(.ui-btn svg) {
  flex: none;
  width: 1.1em;
  height: 1.1em;
}
.acts .wide {
  grid-column: 1 / -1;
}
.remove {
  position: absolute;
  top: var(--space-1);
  right: var(--space-1);
}
.remove :deep(svg) {
  width: 1.3em;
  height: 1.3em;
}
.ringing {
  border-color: var(--color-accent);
  animation: ring-pulse 1s ease-in-out infinite;
  cursor: pointer;
}
.ringing .time {
  color: var(--color-accent);
}
@keyframes ring-pulse {
  0%,
  100% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--color-accent) 60%, transparent);
    background: var(--color-layer-1, var(--color-surface));
  }
  50% {
    box-shadow: 0 0 0 10px color-mix(in srgb, var(--color-accent) 0%, transparent);
    background: color-mix(
      in srgb,
      var(--color-accent) 22%,
      var(--color-layer-1, var(--color-surface))
    );
  }
}
@media (prefers-reduced-motion: reduce) {
  .ringing {
    animation: none;
    background: color-mix(
      in srgb,
      var(--color-accent) 22%,
      var(--color-layer-1, var(--color-surface))
    );
  }
}
</style>

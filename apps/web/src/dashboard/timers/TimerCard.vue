<script setup lang="ts">
import { computed } from 'vue'
import UiIcon from '../../ui/UiIcon.vue'
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
    <div class="acts" @click.stop>
      <div class="row main">
        <button
          v-if="ringing"
          type="button"
          class="a primary"
          data-testid="timer-dismiss"
          @click="emit('dismiss')"
        >
          <UiIcon name="check" :size="20" /> {{ S.timerDismiss }}
        </button>
        <template v-else>
          <button type="button" class="a" data-testid="timer-add-time" @click="emit('addTime')">
            <UiIcon name="add" :size="20" /> {{ S.timerPlusMinute }}
          </button>
          <button
            v-if="timer.state === 'running'"
            type="button"
            class="a"
            data-testid="timer-pause"
            @click="emit('pause')"
          >
            {{ S.timerPause }}
          </button>
          <button v-else type="button" class="a" data-testid="timer-resume" @click="emit('resume')">
            {{ timer.state === 'idle' ? S.timerStart : S.timerResume }}
          </button>
        </template>
      </div>
      <div class="row sub">
        <button
          v-if="!ringing && timer.state !== 'idle'"
          type="button"
          class="a"
          data-testid="timer-cancel-run"
          @click="emit('cancel')"
        >
          {{ S.timerReset }}
        </button>
        <button
          type="button"
          class="a icon"
          :aria-label="S.timerRemove"
          :title="S.timerRemove"
          data-testid="timer-remove"
          @click="emit('remove')"
        >
          <UiIcon name="delete" :size="20" />
        </button>
      </div>
    </div>
  </article>
</template>

<style scoped>
.card {
  display: grid;
  justify-items: center;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-5);
  border: 1px solid var(--color-border);
  border-radius: var(--ui-radius-md);
  background: var(--color-layer-1, var(--color-surface));
  box-shadow: 0 1px 3px color-mix(in srgb, var(--color-text) 14%, transparent);
}
h3 {
  margin: 0;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--font-size-lg);
}
.ring {
  position: relative;
  width: min(100%, 13rem);
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
  font-size: 2.2rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}
.acts {
  display: grid;
  gap: var(--space-2);
  width: 100%;
  font-size: clamp(0.9rem, 0.8rem + 0.4vw, 1.15rem);
}
.row {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: var(--space-2);
}
.a {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-1);
  flex: 1 1 auto;
  box-sizing: border-box;
  min-width: var(--min-touch);
  min-height: var(--min-touch);
  padding: var(--space-1) var(--space-4);
  border: 0;
  border-radius: var(--ui-radius-lg, var(--ui-radius-sm));
  background: var(--color-layer-3);
  color: inherit;
  font: inherit;
  line-height: 1.2;
  white-space: nowrap;
  cursor: pointer;
}
.main .a {
  min-height: calc(var(--min-touch) * 1.15);
  font-weight: 600;
}
.a.icon {
  flex: 0 0 auto;
  padding-inline: var(--space-3);
}
.a.primary {
  background: var(--color-accent);
  color: var(--color-on-accent);
  font-weight: 700;
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

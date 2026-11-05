<script setup lang="ts">
import { computed } from 'vue'
import type { CategoryView, Member, MessageSummary } from '../../api/client'
import { UiCard, UiIcon, UiList, UiListItem } from '../../ui'
import { fmt } from './format'

const props = defineProps<{
  msg: MessageSummary
  tz: string
  categories: CategoryView[]
  members: Member[]
}>()

const cat = computed(() => props.categories.find((c) => c.key === props.msg.category))
const people = computed(() =>
  props.members.filter((m) => props.msg.members.includes(m.id)).map((m) => m.display_name),
)
// The third line: category and members, then the snippet (the chips of the old row).
const supporting = computed(() => {
  const parts = [
    cat.value?.label ?? props.msg.category ?? '',
    ...people.value,
    props.msg.is_bill ? 'Bill' : '',
    props.msg.has_attachments ? 'Attachment' : '',
  ].filter(Boolean)
  const meta = parts.join(' · ')
  const snippet = props.msg.snippet?.trim() ?? ''
  return [meta, snippet].filter(Boolean).join(' — ')
})
</script>

<template>
  <li class="mail-row">
    <UiCard variant="filled">
      <UiList>
        <UiListItem
          :to="`/mail/${msg.id}`"
          :overline="msg.from_name || msg.from_addr"
          :headline="msg.subject"
          :supporting="supporting || undefined"
        >
          <template #trailing>
            <span class="mail-row__trail">
              <UiIcon v-if="msg.time_sensitive" name="warning" :size="20" label="Time-sensitive" />
              <time :datetime="msg.received_at">{{ fmt(msg.received_at, tz) }}</time>
            </span>
          </template>
        </UiListItem>
      </UiList>
    </UiCard>
  </li>
</template>

<style scoped>
.mail-row {
  list-style: none;
}
.mail-row__trail {
  display: inline-flex;
  align-items: center;
  gap: var(--ui-space-2);
  color: var(--ui-on-surface-variant);
  font: var(--ui-type-label-sm);
  text-align: right;
}
</style>

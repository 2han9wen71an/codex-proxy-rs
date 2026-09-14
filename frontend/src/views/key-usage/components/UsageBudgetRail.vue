<script setup lang="ts">
import type { ClientBudgetWindow } from '@/api'

import { computed } from 'vue'

import { formatDateTime } from '@/utils/date'
import { formatUsd } from '@/views/usage/utils/format'

const props = defineProps<{
  label: string
  budget: ClientBudgetWindow
}>()

const limit = computed(() => numeric(props.budget.limitUsd))
const used = computed(() => numeric(props.budget.usedUsd))
const limited = computed(() => limit.value > 0)
const percentage = computed(() => limited.value ? Math.max(0, used.value / limit.value * 100) : 0)
const progressStyle = computed(() => ({ width: `${Math.min(100, percentage.value)}%` }))
const tone = computed(() => {
  if (!limited.value)
    return { progress: '', text: 'text-cp-text-secondary' }
  if (percentage.value >= 95)
    return { progress: 'bg-cp-error', text: 'text-cp-error-text' }
  if (percentage.value >= 80)
    return { progress: 'bg-cp-warning', text: 'text-cp-warning-text' }
  return { progress: 'bg-cp-success', text: 'text-cp-success-text' }
})
const remainingLabel = computed(() => {
  if (!limited.value)
    return '未设置额度'
  if (used.value >= limit.value)
    return '额度已耗尽，仍可查看用量'
  return `${formatUsd(props.budget.remainingUsd ?? '0')} 可用`
})
const resetLabel = computed(() => props.budget.resetsAt
  ? `${formatDateTime(props.budget.resetsAt, '—', 'Asia/Shanghai')} 重置`
  : '下次使用时确定')

function numeric(value: string) {
  const parsed = Number.parseFloat(value)
  return Number.isFinite(parsed) ? parsed : 0
}
</script>

<template>
  <div class="min-w-0 rounded-cp-lg bg-cp-fill-alter/70 px-4 py-3 2xl:py-2.5">
    <div class="flex items-start justify-between gap-4">
      <div class="min-w-0">
        <h3 class="m-0 text-cp-base leading-none font-heavy text-cp-text">
          {{ label }}
        </h3>
        <span class="mt-2 block text-cp-sm font-emphasis text-cp-text-secondary">
          {{ limited ? `额度 ${formatUsd(budget.limitUsd)}` : '当前窗口累计' }}
        </span>
      </div>
      <strong class="shrink-0 font-mono text-xl leading-none font-heavy tabular-nums" :class="limited ? tone.text : 'text-cp-text'">
        {{ formatUsd(budget.usedUsd) }}
      </strong>
    </div>

    <div
      class="mt-4 h-2 overflow-hidden rounded-full bg-cp-fill-secondary 2xl:mt-2.5"
      role="progressbar"
      :aria-label="`${label}使用进度`"
      :aria-valuenow="limited ? Math.min(100, Math.round(percentage)) : undefined"
      aria-valuemin="0"
      :aria-valuemax="limited ? 100 : undefined"
      :aria-valuetext="limited ? `${percentage.toFixed(1)}%，${remainingLabel}` : `已使用 ${formatUsd(budget.usedUsd)}`"
    >
      <span
        v-if="limited"
        class="block h-full min-w-0.75 rounded-full transition-[width,background-color] duration-200 motion-reduce:transition-none"
        :class="tone.progress"
        :style="progressStyle"
      />
      <span
        v-else
        class="block h-full w-full bg-[repeating-linear-gradient(115deg,color-mix(in_srgb,var(--cp-color-primary)_50%,transparent)_0_8px,color-mix(in_srgb,var(--cp-color-primary)_12%,transparent)_8px_15px)]"
      />
    </div>

    <div class="mt-3 flex flex-wrap items-center justify-between gap-x-4 gap-y-1 text-cp-sm font-emphasis 2xl:mt-2">
      <span :class="tone.text">{{ remainingLabel }}</span>
      <span class="text-cp-text-quaternary">{{ resetLabel }}</span>
    </div>
  </div>
</template>

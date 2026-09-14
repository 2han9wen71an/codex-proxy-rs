<script setup lang="ts">
import type { ClientUsageResponse } from '@/api'

import { computed } from 'vue'

import BaseCard from '@/components/base/BaseCard.vue'
import { formatCompactNumber, formatInteger } from '@/utils/number'

type Totals = ClientUsageResponse['activity']['totals']

const props = defineProps<{
  totals: Totals
}>()

const segments = computed(() => {
  const total = props.totals.requestCount
  const success = ratio(props.totals.successCount, total)
  const failure = ratio(props.totals.failureCount, total)
  return {
    success: { width: `${success * 100}%` },
    failure: { width: `${failure * 100}%` },
    pending: { width: `${Math.max(0, 1 - success - failure) * 100}%` },
  }
})
const outcomes = computed(() => [
  { label: '成功', value: props.totals.successCount, tone: 'text-cp-success-text' },
  { label: '失败', value: props.totals.failureCount, tone: 'text-cp-error-text' },
  { label: '取消', value: props.totals.cancelledCount, tone: 'text-cp-text' },
  { label: '未完成', value: props.totals.incompleteCount, tone: 'text-cp-warning-text' },
])
const tokens = computed(() => [
  { label: '输入 Token', value: props.totals.inputTokens },
  { label: '输出 Token', value: props.totals.outputTokens },
  { label: '缓存 Token', value: props.totals.cachedTokens },
])

function ratio(value: number, total: number) {
  return total > 0 ? value / total : 0
}
</script>

<template>
  <BaseCard as="article" title="请求结果" description="所选时间范围内的完成状态" class="h-full">
    <div class="flex h-2 overflow-hidden rounded-full bg-cp-fill-secondary" aria-hidden="true">
      <span class="bg-cp-success transition-[width] duration-200 motion-reduce:transition-none" :style="segments.success" />
      <span class="bg-cp-error transition-[width] duration-200 motion-reduce:transition-none" :style="segments.failure" />
      <span class="bg-cp-warning transition-[width] duration-200 motion-reduce:transition-none" :style="segments.pending" />
    </div>

    <dl class="mt-5 grid grid-cols-2 gap-2 sm:grid-cols-4">
      <div v-for="item in outcomes" :key="item.label" class="rounded-cp-lg bg-cp-fill-alter/70 p-3">
        <dt class="text-cp-sm font-bold text-cp-text-quaternary">
          {{ item.label }}
        </dt>
        <dd class="mt-2 mb-0 font-mono text-xl font-heavy tabular-nums" :class="item.tone">
          {{ formatInteger(item.value) }}
        </dd>
      </div>
    </dl>

    <dl class="mt-3 grid flex-1 grid-cols-3 items-center gap-2">
      <div v-for="item in tokens" :key="item.label" class="min-w-0 rounded-cp-lg bg-cp-fill-alter/70 p-3">
        <dt class="text-cp-sm font-emphasis text-cp-text-secondary">
          {{ item.label }}
        </dt>
        <dd class="mt-1.5 mb-0 font-mono text-lg font-heavy tabular-nums text-cp-text" :title="formatInteger(item.value)">
          {{ formatCompactNumber(item.value) }}
        </dd>
      </div>
    </dl>
  </BaseCard>
</template>

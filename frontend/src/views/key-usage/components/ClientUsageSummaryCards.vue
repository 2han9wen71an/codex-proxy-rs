<script setup lang="ts">
import type { ClientUsageResponse } from '@/api'

import { Activity, FileText, Gauge, WalletCards } from '@lucide/vue'
import { computed } from 'vue'

import BaseCard from '@/components/base/BaseCard.vue'
import BaseMotionIcon from '@/components/base/BaseMotionIcon.vue'
import { formatCompactNumber, formatInteger } from '@/utils/number'
import { formatPercent, formatUsd } from '@/views/usage/utils/format'

type Totals = ClientUsageResponse['activity']['totals']
type Limits = ClientUsageResponse['limits']

const props = defineProps<{
  totals: Totals
  limits: Limits
}>()

const items = computed(() => {
  const knownCostCount = props.totals.costCoverage.providerReportedCount
    + props.totals.costCoverage.calculatedCount
    + props.totals.costCoverage.partialCount
  const billableCount = knownCostCount + props.totals.costCoverage.unavailableCount

  return [
    {
      key: 'requests',
      label: '请求总量',
      icon: Activity,
      value: formatCompactNumber(props.totals.requestCount),
      detail: `成功率 ${formatPercent(ratio(props.totals.successCount, props.totals.requestCount))}`,
      tone: 'bg-cp-blue-container text-cp-blue-on-container',
    },
    {
      key: 'tokens',
      label: 'Token 总量',
      icon: FileText,
      value: formatCompactNumber(props.totals.totalTokens),
      detail: `缓存占输入 ${formatPercent(ratio(props.totals.cachedTokens, props.totals.inputTokens))}`,
      tone: 'bg-cp-green-container text-cp-green-on-container',
    },
    {
      key: 'cost',
      label: '可见成本',
      icon: WalletCards,
      value: formatUsd(props.totals.billedUsd),
      detail: `成本覆盖 ${formatPercent(ratio(knownCostCount, billableCount))}`,
      tone: 'bg-cp-orange-container text-cp-orange-on-container',
    },
    {
      key: 'rate-limit',
      label: '限流边界',
      icon: Gauge,
      value: props.limits.requestsPerMinute > 0 ? `${formatInteger(props.limits.requestsPerMinute)}RPM` : '不限',
      detail: props.limits.maxConcurrency > 0 ? `${formatInteger(props.limits.maxConcurrency)} 并发` : '不限',
      tone: 'bg-cp-cyan-container text-cp-cyan-on-container',
    },
  ]
})

function ratio(value: number, total: number) {
  return total > 0 ? value / total : null
}
</script>

<template>
  <section class="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2 2xl:grid-cols-4 2xl:gap-6" aria-label="用量概览">
    <BaseCard
      v-for="item in items"
      :key="item.key"
      as="article"
      padding="compact"
      class="grid min-h-23 grid-cols-[36px_minmax(0,1fr)] items-stretch gap-3"
    >
      <BaseMotionIcon class="inline-flex size-9 shrink-0 items-center justify-center rounded-cp" :class="item.tone">
        <component :is="item.icon" class="size-4.5" />
      </BaseMotionIcon>
      <div class="flex min-w-0 flex-col justify-between py-0.5">
        <span class="text-cp-sm leading-none font-bold text-cp-text-quaternary">{{ item.label }}</span>
        <strong class="truncate font-mono text-[22px] leading-none font-extrabold tabular-nums text-cp-text">
          {{ item.value }}
        </strong>
        <span class="truncate text-cp-sm leading-none font-emphasis text-cp-text-secondary">{{ item.detail }}</span>
      </div>
    </BaseCard>
  </section>
</template>

<script setup lang="ts">
import type { ClientUsageRecord } from '@/api'

import { useTemplateRef, watch } from 'vue'

import BaseButton from '@/components/base/BaseButton.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import BaseEmpty from '@/components/base/BaseEmpty.vue'
import { defineTableColumns } from '@/components/base/BaseTable/columns'
import BaseTable from '@/components/base/BaseTable/index.vue'
import { formatDateTime } from '@/utils/date'
import { formatCompactNumber } from '@/utils/number'
import { formatDuration, formatUsd } from '@/views/usage/utils/format'

const props = defineProps<{
  items: ClientUsageRecord[]
  hasMore: boolean
  loading: boolean
  initialLoading: boolean
  error: string
  revision: number
}>()
const emit = defineEmits<{
  loadMore: []
  retry: []
  topChange: [atTop: boolean]
}>()
const table = useTemplateRef('table')
const columns = defineTableColumns<ClientUsageRecord>([
  { key: 'startedAt', label: '时间', kind: 'datetime', size: 'lg', format: (_, row) => formatDateTime(row.startedAt, '—', 'Asia/Shanghai').slice(5) },
  { key: 'model', label: '模型', kind: 'mono', size: 'lg' },
  { key: 'outcome', label: '结果', kind: 'status', size: 'sm' },
  { key: 'totalTokens', label: 'Token', kind: 'numeric', size: 'sm', format: (_, row) => row.totalTokens === null ? '—' : formatCompactNumber(row.totalTokens) },
  { key: 'cost', label: '成本', kind: 'numeric', size: 'md', format: (_, row) => costLabel(row.cost) },
  { key: 'latencyMs', label: '耗时', kind: 'numeric', size: 'sm', format: (_, row) => formatDuration(row.latencyMs) },
])
const outcomes: Record<string, { label: string, tone: string }> = {
  succeeded: { label: '成功', tone: 'text-cp-success-text' },
  failed: { label: '失败', tone: 'text-cp-error-text' },
  cancelled: { label: '取消', tone: 'text-cp-text-quaternary' },
  incomplete: { label: '未完成', tone: 'text-cp-warning-text' },
  running: { label: '进行中', tone: 'text-cp-primary-text' },
}

function outcome(value: string) {
  return outcomes[value] ?? { label: '未知', tone: 'text-cp-text-quaternary' }
}

function costLabel(cost: ClientUsageRecord['cost']) {
  if (!cost)
    return '—'
  return cost.currency === 'USD' ? formatUsd(cost.amount, true) : `${cost.amount} ${cost.currency}`
}

function handleScroll(payload: { scrollTop: number, scrollHeight: number, clientHeight: number }) {
  emit('topChange', payload.scrollTop === 0)
  if (!props.error && payload.scrollHeight - payload.scrollTop - payload.clientHeight < 80)
    emit('loadMore')
}

watch(() => props.revision, () => {
  table.value?.scrollToTop()
}, { flush: 'post' })
</script>

<template>
  <BaseCard as="article" title="调用明细" description="请求用量、成本与耗时" class="h-82.5 2xl:h-full">
    <BaseEmpty v-if="error && !items.length" title="明细加载失败" surface="none" size="sm" class="flex-1 place-content-center">
      <template #action>
        <BaseButton size="sm" @click="emit('retry')">
          重试
        </BaseButton>
      </template>
    </BaseEmpty>
    <BaseTable
      v-else
      ref="table"
      :columns="columns"
      :rows="items"
      :loading="initialLoading"
      empty-text="暂无调用记录"
      density="compact"
      class="min-h-0 flex-1"
      @scroll="handleScroll"
    >
      <template #outcome="{ row }">
        <span class="font-emphasis" :class="outcome(row.outcome).tone">{{ outcome(row.outcome).label }}</span>
      </template>
      <template #totalTokens="{ row, displayValue }">
        <span :title="row.totalTokens === null ? 'Token 暂不可用' : String(row.totalTokens)">{{ displayValue }}</span>
      </template>
      <template #cost="{ row, displayValue }">
        <span :title="row.cost ? `${row.cost.amount} ${row.cost.currency}` : '成本暂不可用'">{{ displayValue }}</span>
      </template>
      <template v-if="items.length && (hasMore || loading || error)" #append>
        <div class="flex h-10 items-center justify-center gap-2 text-cp-sm text-cp-text-secondary" role="status">
          <span v-if="error">明细加载失败</span>
          <BaseButton size="sm" variant="ghost" :loading="loading" @click="error ? emit('retry') : emit('loadMore')">
            {{ error ? '重试' : loading ? '正在加载' : '加载更多' }}
          </BaseButton>
        </div>
      </template>
    </BaseTable>
  </BaseCard>
</template>

<script setup lang="ts">
import type { EChartsOption, LineSeriesOption } from 'echarts'
import type { ClientUsageResponse } from '@/api'

import { computed } from 'vue'

import BaseEmpty from '@/components/base/BaseEmpty.vue'
import BaseChart from '@/components/charts/BaseChart.vue'
import { useChartPalette } from '@/composables/useChartPalette'
import { formatDateTime } from '@/utils/date'
import { formatCompactNumber } from '@/utils/number'
import {
  tooltipIndex,
  tooltipRows,
  usageCategoryAxis,
  usageGapAwareLineSeries,
  usageTooltip,
  usageTooltipContent,
  usageTooltipItem,
  usageValueAxis,
} from '@/views/usage/utils/chart'
import { formatUsd, formatUsdAxis } from '@/views/usage/utils/format'

type TrendPoint = ClientUsageResponse['activity']['trend'][number]
export type ClientTrendMetric = 'requests' | 'tokens' | 'cost'

const props = defineProps<{
  points: TrendPoint[]
  metric: ClientTrendMetric
}>()

const { palette } = useChartPalette()
const values = computed(() => props.points.map(valueFor))
const hasData = computed(() => props.points.some(point => point.requestCount > 0))
const metricView = computed(() => {
  if (props.metric === 'tokens') {
    return {
      label: 'Token',
      color: palette.value.success,
      formatter: formatCompactNumber,
    }
  }
  if (props.metric === 'cost') {
    return {
      label: '成本',
      color: palette.value.warning,
      formatter: formatUsdAxis,
    }
  }
  return {
    label: '请求',
    color: palette.value.info,
    formatter: formatCompactNumber,
  }
})

const chartOption = computed<EChartsOption>(() => {
  const theme = palette.value
  const failureSeries: LineSeriesOption = {
    name: '失败请求',
    type: 'line',
    data: props.points.map((point, index) => point.failureCount > 0 ? values.value[index] : null),
    connectNulls: false,
    showSymbol: true,
    showAllSymbol: true,
    symbol: 'circle',
    symbolSize: 7,
    lineStyle: { width: 0, opacity: 0 },
    itemStyle: {
      color: theme.danger,
      borderColor: theme.surface,
      borderWidth: 2,
    },
    z: 5,
  }

  return {
    animationDuration: 240,
    grid: {
      left: 0,
      right: 0,
      top: 16,
      bottom: 0,
      outerBoundsMode: 'same',
      outerBoundsContain: 'axisLabel',
    },
    tooltip: usageTooltip(theme, formatTooltip),
    xAxis: usageCategoryAxis(props.points.map(point => bucketLabel(point.bucketStart)), theme),
    yAxis: usageValueAxis(theme, metricView.value.formatter),
    series: [
      ...usageGapAwareLineSeries(
        metricView.value.label,
        values.value,
        metricView.value.color,
        { area: 'strong' },
      ),
      failureSeries,
    ],
  }
})

function valueFor(point: TrendPoint) {
  if (props.metric === 'tokens')
    return point.totalTokens
  if (props.metric === 'cost') {
    const value = Number.parseFloat(point.billedUsd)
    return Number.isFinite(value) ? value : 0
  }
  return point.requestCount
}

function bucketLabel(value: string) {
  return formatDateTime(value, '—', 'Asia/Shanghai').slice(5, 16)
}

function metricValue(value: number) {
  return props.metric === 'cost' ? formatUsd(value, true) : formatCompactNumber(value)
}

function formatTooltip(params: unknown) {
  const rows = tooltipRows(params)
  const point = props.points[tooltipIndex(rows[0])]
  if (!point)
    return ''

  const theme = palette.value
  const lines = [
    usageTooltipItem(metricView.value.label, metricValue(valueFor(point)), metricView.value.color),
  ]
  if (point.failureCount > 0)
    lines.push(usageTooltipItem('失败请求', formatCompactNumber(point.failureCount), theme.danger))

  return usageTooltipContent(
    theme,
    formatDateTime(point.bucketStart, '—', 'Asia/Shanghai'),
    lines,
    { divider: false },
  )
}
</script>

<template>
  <BaseChart v-if="hasData" :option="chartOption" height="auto" class="min-h-52 flex-1" />
  <BaseEmpty
    v-else
    surface="none"
    title="当前范围暂无请求"
    class="min-h-52 flex-1 place-content-center"
  />
</template>

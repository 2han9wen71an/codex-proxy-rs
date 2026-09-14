<script setup lang="ts">
import type { ClientTrendMetric } from './components/UsageActivityChart.vue'

import { KeyRound, LogOut, Moon, RefreshCw, Sun, Tag } from '@lucide/vue'
import { computed, onMounted, shallowRef } from 'vue'
import { useRouter } from 'vue-router'

import { getClientSystemVersion } from '@/api'
import AppBrandMark from '@/components/AppBrandMark.vue'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseCard from '@/components/base/BaseCard.vue'
import BaseEmpty from '@/components/base/BaseEmpty.vue'
import BaseIconButton from '@/components/base/BaseIconButton.vue'
import BaseMotionIcon from '@/components/base/BaseMotionIcon.vue'
import BaseScrollbar from '@/components/base/BaseScrollbar.vue'
import BaseSegmented from '@/components/base/BaseSegmented.vue'
import BaseSkeleton from '@/components/base/BaseSkeleton.vue'
import { useAuthStore } from '@/stores/modules/auth'
import { useThemeStore } from '@/stores/modules/theme'

import ClientUsageOutcomeCard from './components/ClientUsageOutcomeCard.vue'
import ClientUsageRecordsCard from './components/ClientUsageRecordsCard.vue'
import ClientUsageSummaryCards from './components/ClientUsageSummaryCards.vue'
import UsageActivityChart from './components/UsageActivityChart.vue'
import UsageBudgetRail from './components/UsageBudgetRail.vue'
import { useClientUsage } from './composables/useClientUsage'

const router = useRouter()
const authStore = useAuthStore()
const themeStore = useThemeStore()
const themeToggleLabel = computed(() => themeStore.effectiveTheme === 'dark' ? '切换浅色模式' : '切换暗黑模式')
const { error, load, loading, range, refreshing, snapshot, records, recordsCursor, recordsLoading, recordsInitialLoading, recordsError, recordsAtTop, recordsRevision, loadMoreRecords, retryRecords } = useClientUsage()
const trendMetric = shallowRef<ClientTrendMetric>('requests')
const versionText = shallowRef('')

const rangeOptions = [
  { label: '24 小时', value: '24h' },
  { label: '7 天', value: '7d' },
  { label: '30 天', value: '30d' },
]
const trendMetricOptions = [
  { label: '请求', value: 'requests' },
  { label: 'Token', value: 'tokens' },
  { label: '成本', value: 'cost' },
]

async function handleLogout() {
  if (!await authStore.logout())
    return
  await router.replace({ name: 'login', state: { loginType: 'key' } })
}

onMounted(() => {
  void getClientSystemVersion({ silent: true })
    .then(({ version }) => versionText.value = version.trim())
    .catch(() => undefined)
})
</script>

<template>
  <div class="flex h-dvh flex-col overflow-hidden bg-cp-bg-layout text-cp-text">
    <header class="z-10 shrink-0 bg-cp-bg-layout/90 shadow-cp-tertiary backdrop-blur-lg">
      <div class="mx-auto flex min-h-18 max-w-400 items-center justify-between gap-5 px-4 min-[961px]:px-6">
        <div class="flex min-w-0 items-center gap-3">
          <BaseMotionIcon variant="brand" class="inline-flex size-10 shrink-0 items-center justify-center rounded-cp">
            <AppBrandMark class="size-10" />
          </BaseMotionIcon>
          <span class="grid min-w-0 gap-1.5">
            <strong class="truncate text-cp-base leading-none font-heavy text-cp-text">Codex Proxy RS</strong>
            <span class="flex min-w-0 items-center gap-2">
              <span class="shrink-0 text-cp-sm leading-none font-emphasis text-cp-text-secondary">Rust build</span>
              <span
                v-if="versionText"
                class="rounded-cp-sm bg-cp-fill-quaternary px-1.5 font-mono text-[10px] leading-4.5 font-bold text-cp-text-quaternary"
              >
                v{{ versionText }}
              </span>
            </span>
          </span>
        </div>

        <div class="flex shrink-0 items-center gap-2">
          <BaseIconButton :label="themeToggleLabel" @click="themeStore.toggleTheme($event)">
            <Sun v-if="themeStore.effectiveTheme === 'dark'" :size="18" />
            <Moon v-else :size="18" />
          </BaseIconButton>
          <BaseIconButton
            class="text-cp-primary-text"
            :loading="refreshing"
            :disabled="refreshing"
            label="刷新用量"
            @click="load(false, true)"
          >
            <template #loading>
              <RefreshCw class="animate-spin motion-reduce:animate-none" :size="18" />
            </template>
            <RefreshCw :size="18" />
          </BaseIconButton>
          <BaseIconButton variant="destructive" label="退出 Key 会话" @click="handleLogout">
            <LogOut :size="18" />
          </BaseIconButton>
        </div>
      </div>
    </header>

    <BaseScrollbar class="min-h-0 flex-1">
      <main class="mx-auto w-full max-w-400 p-4 min-[961px]:px-6">
        <div v-if="loading && !snapshot" class="grid gap-4" role="status" aria-live="polite" aria-busy="true">
          <span class="sr-only">正在读取 Key 用量</span>
          <div class="flex justify-end">
            <BaseSkeleton class="h-cp-control w-52 max-w-full rounded-cp" />
          </div>
          <div class="grid grid-cols-1 gap-4 md:grid-cols-2 2xl:grid-cols-4">
            <BaseSkeleton v-for="item in 4" :key="item" class="h-23 rounded-cp-card" />
          </div>
          <div class="grid gap-6 2xl:grid-cols-2">
            <BaseSkeleton class="h-92 rounded-cp-card" />
            <BaseSkeleton class="h-92 rounded-cp-card" />
          </div>
        </div>

        <BaseEmpty
          v-else-if="!snapshot"
          class="mx-auto mt-[12vh] max-w-xl"
          title="暂时无法打开用量页"
          :description="error || '请稍后重新尝试。'"
        >
          <template #action>
            <BaseButton variant="primary" @click="load(false, true)">
              重新加载
            </BaseButton>
          </template>
        </BaseEmpty>

        <template v-else>
          <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div class="grid min-w-0 gap-1">
              <span
                v-if="snapshot.key.label"
                class="flex min-w-0 items-center gap-2 text-cp-sm leading-4 font-emphasis text-cp-text-secondary"
              >
                <Tag class="shrink-0" :size="14" aria-hidden="true" />
                <span class="truncate" :title="snapshot.key.label">{{ snapshot.key.label }}</span>
              </span>
              <span class="flex min-w-0 items-center gap-2 text-cp-text-quaternary">
                <KeyRound class="shrink-0" :size="14" aria-hidden="true" />
                <code class="truncate font-mono text-cp-sm leading-4 font-emphasis">{{ snapshot.key.prefix }}••••</code>
              </span>
            </div>
            <BaseSegmented
              v-model="range"
              class="shrink-0 self-end bg-(--cp-input-bg)! sm:self-auto"
              label="用量时间范围"
              :options="rangeOptions"
              size="md"
              :disabled="refreshing"
            />
          </div>

          <div
            v-if="error"
            class="mt-4 flex items-center justify-between gap-4 rounded-cp-lg bg-cp-warning-container px-4 py-3 text-cp-sm font-semibold text-cp-warning-on-container"
            role="status"
          >
            <span>{{ error }}，当前显示上次读取的数据。</span>
            <button class="shrink-0 border-0 bg-transparent font-bold text-current underline underline-offset-3" type="button" @click="load(false, true)">
              重试
            </button>
          </div>

          <ClientUsageSummaryCards :totals="snapshot.activity.totals" :limits="snapshot.limits" />

          <section class="mt-4 grid grid-cols-1 gap-6 2xl:h-82.5 2xl:grid-cols-[minmax(0,0.88fr)_minmax(0,1.12fr)]" aria-label="额度与趋势">
            <BaseCard title="额度使用" description="每日与七日预算">
              <div class="grid gap-3">
                <UsageBudgetRail label="当日额度" :budget="snapshot.budget.daily" />
                <UsageBudgetRail label="七日额度" :budget="snapshot.budget.weekly" />
              </div>
            </BaseCard>

            <BaseCard title="使用趋势" description="请求、Token 与成本走势">
              <template #actions>
                <BaseSegmented
                  v-model="trendMetric"
                  class="w-full max-w-56 sm:w-56"
                  label="趋势指标"
                  :options="trendMetricOptions"
                  :disabled="refreshing"
                />
              </template>
              <UsageActivityChart :points="snapshot.activity.trend" :metric="trendMetric" />
            </BaseCard>
          </section>

          <section class="mt-4 grid grid-cols-1 gap-6 2xl:h-82.5 2xl:grid-cols-[minmax(0,0.88fr)_minmax(0,1.12fr)]" aria-label="请求结果与调用明细">
            <ClientUsageOutcomeCard :totals="snapshot.activity.totals" />
            <ClientUsageRecordsCard :items="records" :has-more="recordsCursor !== null" :loading="recordsLoading" :initial-loading="recordsInitialLoading" :error="recordsError" :revision="recordsRevision" @load-more="loadMoreRecords" @retry="retryRecords" @top-change="recordsAtTop = $event" />
          </section>
        </template>
      </main>
    </BaseScrollbar>
  </div>
</template>

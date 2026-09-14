import type { ClientUsageCursor, ClientUsageRange, ClientUsageRecord, ClientUsageResponse } from '@/api'

import { useDocumentVisibility, useIntervalFn } from '@vueuse/core'
import { computed, onMounted, onScopeDispose, shallowRef, watch } from 'vue'

import { getClientUsage, getClientUsageRecords } from '@/api'
import { errorMessage } from '@/utils/async'

const refreshIntervalMs = 30_000

export function useClientUsage() {
  const range = shallowRef<ClientUsageRange>('7d')
  const snapshot = shallowRef<ClientUsageResponse>()
  const loading = shallowRef(true)
  const refreshing = shallowRef(false)
  const error = shallowRef('')
  const records = shallowRef<ClientUsageRecord[]>([])
  const recordsCursor = shallowRef<ClientUsageCursor | null>(null)
  const recordsLoading = shallowRef(false)
  const recordsError = shallowRef('')
  const recordsAtTop = shallowRef(true)
  const recordsRevision = shallowRef(0)
  const recordsRange = shallowRef<ClientUsageResponse['activity']['range']>()
  const recordsInitialLoading = computed(() => recordsLoading.value && !recordsRange.value)
  let retryRecordsFromStart = true
  let recordsController: AbortController | undefined
  const visibility = useDocumentVisibility()
  let mounted = false
  let disposed = false
  let requestId = 0
  let controller: AbortController | undefined

  const hasSnapshot = computed(() => snapshot.value !== undefined)

  async function load(silent = hasSnapshot.value, resetRecords = false) {
    const currentRequestId = ++requestId
    controller?.abort()
    const currentController = new AbortController()
    controller = currentController
    error.value = ''
    if (hasSnapshot.value)
      refreshing.value = true
    else
      loading.value = true

    try {
      const result = await getClientUsage({ range: range.value }, {
        silent,
        signal: currentController.signal,
      })
      if (!disposed && currentRequestId === requestId) {
        snapshot.value = result
        // 自动刷新不打断历史浏览；主动刷新或切换范围才重置到最新一页。
        const rangeChanged = recordsRange.value?.kind !== result.activity.range.kind
        if (resetRecords || recordsAtTop.value || rangeChanged) {
          recordsController?.abort()
          // 同范围刷新保留旧明细与游标，成功后一起替换；切换范围不能混用旧数据。
          if (rangeChanged) {
            records.value = []
            recordsCursor.value = null
            recordsRange.value = undefined
          }
          await loadRecords(true, result.activity.range)
        }
      }
    }
    catch (cause: unknown) {
      if (!currentController.signal.aborted && !disposed && currentRequestId === requestId)
        error.value = errorMessage(cause, '暂时无法读取 Key 用量')
    }
    finally {
      if (currentRequestId === requestId) {
        loading.value = false
        refreshing.value = false
      }
    }
  }

  async function loadRecords(reset = false, queryRange = recordsRange.value) {
    if (!queryRange || (!reset && (!recordsCursor.value || recordsLoading.value || refreshing.value)))
      return
    const cursor = reset ? null : recordsCursor.value
    const currentController = new AbortController()
    recordsController = currentController
    recordsLoading.value = true
    recordsError.value = ''
    retryRecordsFromStart = reset
    try {
      const result = await getClientUsageRecords({
        range: queryRange.kind,
        endsAt: queryRange.endsAt,
        beforeAt: cursor?.startedAt,
        beforeId: cursor?.id,
      }, { silent: true, signal: currentController.signal })
      if (currentController.signal.aborted || disposed)
        return
      records.value = reset ? result.items : [...records.value, ...result.items]
      recordsCursor.value = result.nextCursor
      recordsRange.value = queryRange
      if (reset) {
        recordsAtTop.value = true
        recordsRevision.value++
      }
    }
    catch (cause: unknown) {
      if (!currentController.signal.aborted && !disposed)
        recordsError.value = errorMessage(cause, '暂时无法读取调用明细')
    }
    finally {
      if (recordsController === currentController)
        recordsLoading.value = false
    }
  }

  function loadMoreRecords() {
    return loadRecords()
  }

  function retryRecords() {
    if (!recordsLoading.value)
      return loadRecords(retryRecordsFromStart, retryRecordsFromStart ? snapshot.value?.activity.range : recordsRange.value)
  }

  const { pause } = useIntervalFn(() => {
    if (visibility.value === 'visible' && !refreshing.value)
      void load(true)
  }, refreshIntervalMs)

  watch(range, () => {
    if (mounted) {
      recordsController?.abort()
      void load()
    }
  })

  watch(visibility, (current, previous) => {
    if (mounted && current === 'visible' && previous === 'hidden')
      void load(true)
  })

  onMounted(() => {
    mounted = true
    void load(false)
  })

  onScopeDispose(() => {
    disposed = true
    controller?.abort()
    recordsController?.abort()
    pause()
  })

  return {
    error,
    hasSnapshot,
    load,
    loading,
    range,
    refreshing,
    snapshot,
    records,
    recordsCursor,
    recordsLoading,
    recordsInitialLoading,
    recordsError,
    recordsAtTop,
    recordsRevision,
    loadMoreRecords,
    retryRecords,
  }
}

import type { RequestOptions } from '../request'
import request from '../request'

export type ClientUsageRange = '24h' | '7d' | '30d'

export interface ClientUsageCursor {
  startedAt: string
  id: string
}

export interface ClientUsageRecord {
  id: string
  startedAt: string
  model: string | null
  outcome: string
  totalTokens: number | null
  latencyMs: number | null
  cost: { amount: string, currency: string } | null
}

export interface ClientUsageRecordsResponse {
  items: ClientUsageRecord[]
  nextCursor: ClientUsageCursor | null
}

export interface ClientUsageRecordsQuery {
  range: ClientUsageRange
  endsAt: string
  beforeAt?: string
  beforeId?: string
}

export interface ClientKeySummary {
  name: string
  label: string | null
  prefix: string
  lastUsedAt?: string
}

export interface ClientBudgetWindow {
  limitUsd: string
  usedUsd: string
  remainingUsd: string | null
  startsAt: string | null
  resetsAt: string | null
}

export interface ClientUsageResponse {
  asOf: string
  timezone: 'Asia/Shanghai'
  currency: 'USD'
  key: ClientKeySummary
  budget: {
    daily: ClientBudgetWindow
    weekly: ClientBudgetWindow
  }
  limits: {
    maxConcurrency: number
    requestsPerMinute: number
  }
  activity: {
    range: {
      kind: ClientUsageRange
      startsAt: string
      endsAt: string
      granularity: '15m' | 'hour' | 'day'
    }
    totals: {
      requestCount: number
      successCount: number
      failureCount: number
      cancelledCount: number
      incompleteCount: number
      inputTokens: number
      outputTokens: number
      cachedTokens: number
      totalTokens: number
      billedUsd: string
      costCoverage: {
        providerReportedCount: number
        calculatedCount: number
        partialCount: number
        unavailableCount: number
        notBillableCount: number
      }
    }
    trend: Array<{
      bucketStart: string
      requestCount: number
      successCount: number
      failureCount: number
      totalTokens: number
      billedUsd: string
    }>
  }
}

export function getClientUsage(data: { range: ClientUsageRange }, options: RequestOptions = {}) {
  return request<ClientUsageResponse>({
    url: '/api/client/usage',
    method: 'GET',
    params: data,
    ...options,
  })
}

export function getClientSystemVersion(options: RequestOptions = {}) {
  return request<{ version: string }>({
    url: '/api/client/system/version',
    method: 'GET',
    ...options,
  })
}

export function getClientUsageRecords(data: ClientUsageRecordsQuery, options: RequestOptions = {}) {
  return request<ClientUsageRecordsResponse>({
    url: '/api/client/usage/records',
    method: 'GET',
    params: data,
    ...options,
  })
}

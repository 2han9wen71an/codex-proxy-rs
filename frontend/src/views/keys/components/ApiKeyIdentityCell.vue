<script setup lang="ts">
import type { getApiKeys } from '@/api'

type ApiKeyRow = Awaited<ReturnType<typeof getApiKeys>>['items'][number]

defineProps<{
  apiKey: ApiKeyRow
}>()
</script>

<template>
  <div class="flex min-w-0 flex-col gap-0.5">
    <span class="truncate text-cp font-bold text-cp-text" :title="apiKey.name">
      {{ apiKey.name }}
    </span>
    <span v-if="apiKey.label" class="truncate text-cp-sm font-emphasis text-cp-text-tertiary" :title="apiKey.label">
      {{ apiKey.label }}
    </span>
  </div>
</template>

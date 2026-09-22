<script setup lang="ts">
import BaseCard from '@/components/base/BaseCard.vue'
import BaseSwitch from '@/components/base/BaseSwitch.vue'

const enabled = defineModel<boolean>('enabled', { required: true })
</script>

<template>
  <BaseCard
    title="Codex 会话亲和"
    description="开启后同一会话固定复用首次选中的账号；关闭后每个请求都按调度策略分配账号，用量更均匀"
  >
    <BaseSwitch
      v-model="enabled"
      class="justify-self-start"
      label="启用 Codex 会话亲和"
      show-label
    />
  </BaseCard>
</template>

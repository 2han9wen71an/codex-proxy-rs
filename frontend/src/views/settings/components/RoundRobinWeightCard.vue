<script setup lang="ts">
import BaseCard from '@/components/base/BaseCard.vue'
import BaseSwitch from '@/components/base/BaseSwitch.vue'

const enabled = defineModel<boolean>('enabled', { required: true })
</script>

<template>
  <BaseCard
    title="轮询跨权重"
    description="仅轮询调度生效；开启后所有可用账号都参与，并按账号权重比例分配请求。额度耗尽、冷却或不满足模型权限的账号仍会暂时排除。"
  >
    <BaseSwitch
      v-model="enabled"
      class="justify-self-start"
      label="启用跨权重轮询"
      show-label
    />
  </BaseCard>
</template>

<script setup lang="ts">
import type { PluginCapabilityBinding, PluginContribution } from '@/api'

import { BaseSwitch } from '@codex-proxy/ui'
import PluginHelpPopover from './PluginHelpPopover.vue'

const props = withDefaults(defineProps<{
  contribution: PluginContribution
  disabled?: boolean
}>(), {
  disabled: false,
})

const binding = defineModel<PluginCapabilityBinding | null>({ required: true })

function setEnabled(enabled: boolean) {
  binding.value = enabled
    ? {
        contribution: props.contribution.id,
        stage: 'maintenance',
        order: 0,
        failurePolicy: 'reject',
        providerIds: [],
        models: [],
        clientKeyIds: [],
        accountGroupIds: [],
        identityBindings: [],
      }
    : null
}
</script>

<template>
  <div class="flex items-center gap-2 rounded-cp-lg bg-cp-fill-alter p-4">
    <BaseSwitch
      :model-value="Boolean(binding)"
      label="启用维护任务"
      show-label
      :disabled="disabled"
      @update:model-value="setEnabled"
    />
    <PluginHelpPopover label="维护任务说明">
      <p class="m-0">
        由宿主调度插件提供的刷新、额度、模型或请求画像维护操作
      </p>
      <p class="m-0">
        关闭后不执行定时维护，其他已配置能力不受影响
      </p>
      <p class="m-0 break-all font-mono text-cp-text-quaternary">
        {{ contribution.id }}
      </p>
    </PluginHelpPopover>
  </div>
</template>

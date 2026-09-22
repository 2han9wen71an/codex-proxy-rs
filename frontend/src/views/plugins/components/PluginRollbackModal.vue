<script setup lang="ts">
import type { PluginInstance, PluginRollbackPlan } from '@/api'

import { BaseButton, BaseForm, BaseFormItem, BaseModal, BaseSelect } from '@codex-proxy/ui'

import { computed, shallowRef, watch } from 'vue'
import { shortDigest } from '../utils/model'
import PluginHelpPopover from './PluginHelpPopover.vue'

const props = defineProps<{
  instance: PluginInstance | null
  plan: PluginRollbackPlan | null
  loading: boolean
  saving: boolean
}>()

defineEmits<{
  confirm: [artifactSha256: string]
  reload: [instance: PluginInstance]
}>()

const open = defineModel<boolean>({ required: true })
const selected = shallowRef('')
const options = computed(() => (props.plan?.targets ?? []).map(target => ({
  value: target.artifactSha256,
  label: `${target.version} · ${target.platforms.join(', ')}`,
  description: shortDigest(target.artifactSha256),
})))
const canConfirm = computed(() => !props.loading && options.value.some(option => option.value === selected.value))

watch(() => props.plan, () => {
  selected.value = ''
})
</script>

<template>
  <BaseModal v-model="open" title="回滚插件版本" description="仅切换当前配置，保留数据与启停状态" size="md" :dismissible="!saving">
    <BaseForm class="grid gap-4">
      <BaseFormItem label="当前配置">
        <p class="m-0 break-all text-cp-sm text-cp-text">
          {{ instance?.name }} <span v-if="plan" class="font-mono">· {{ plan.currentVersion }}</span>
        </p>
      </BaseFormItem>
      <BaseFormItem v-if="loading || options.length" label="目标版本" required>
        <template #label-extra>
          <PluginHelpPopover label="回滚说明">
            <p class="m-0">
              只能回滚到已安装的旧版，不删除任何版本，配置、密钥、权限和启停状态保持不变，目标版本须通过兼容检查
            </p>
            <p v-if="selected" class="m-0 break-all font-mono">
              SHA-256 {{ selected }}
            </p>
          </PluginHelpPopover>
        </template>
        <BaseSelect
          v-model="selected"
          class="w-full"
          :options="options"
          :disabled="loading || saving"
          :placeholder="loading ? '正在加载旧版' : '请选择回滚目标'"
          aria-label="回滚目标版本"
        />
      </BaseFormItem>
      <p v-else-if="plan" role="status" class="m-0 text-cp-sm text-cp-text-secondary">
        没有可回滚的旧版，切换新版请前往“版本”
      </p>
    </BaseForm>
    <template #footer>
      <BaseButton v-if="instance" variant="secondary" :disabled="saving" :loading="loading" @click="$emit('reload', instance)">
        重新加载
      </BaseButton>
      <BaseButton variant="secondary" :disabled="saving" @click="open = false">
        取消
      </BaseButton>
      <BaseButton variant="primary" :disabled="!canConfirm" :loading="saving" @click="$emit('confirm', selected)">
        确认回滚
      </BaseButton>
    </template>
  </BaseModal>
</template>

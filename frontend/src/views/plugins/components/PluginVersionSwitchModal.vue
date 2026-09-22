<script setup lang="ts">
import type { PluginArtifact, PluginInstance } from '@/api'
import { BaseButton, BaseFormItem, BaseModal, BaseSelect, BaseTag } from '@codex-proxy/ui'
import { ArrowRight, Plus } from '@lucide/vue'
import { computed, shallowRef, watch } from 'vue'
import PluginHelpPopover from './PluginHelpPopover.vue'

const props = defineProps<{
  artifact?: PluginArtifact
  configurations: PluginInstance[]
  artifacts: PluginArtifact[]
  saving: boolean
  rejectedInstanceId?: string
}>()
defineEmits<{ confirm: [instance: PluginInstance], edit: [instance: PluginInstance], create: [] }>()
const open = defineModel<boolean>({ required: true })
const selectedId = shallowRef('')
const selected = computed(() => props.configurations.find(instance => instance.id === selectedId.value))
const options = computed(() => props.configurations.map(instance => ({ label: instance.name, value: instance.id })))
const currentVersion = computed(() => props.artifacts.find(artifact => artifact.metadata.sha256 === selected.value?.artifactSha256)?.metadata.version)
watch(open, (value) => {
  if (value)
    selectedId.value = props.configurations.length === 1 ? props.configurations[0]!.id : ''
})
</script>

<template>
  <BaseModal v-model="open" title="切换插件版本" description="沿用现有配置，兼容后切换" size="md" :dismissible="!saving">
    <div class="grid gap-4">
      <BaseFormItem label="配置">
        <BaseSelect v-if="configurations.length > 1" v-model="selectedId" :options="options" placeholder="选择要切换的配置" aria-label="切换版本的配置" class="w-full" :disabled="saving" />
        <span v-else class="text-cp-sm">{{ selected?.name }}</span>
      </BaseFormItem>
      <div class="flex flex-wrap items-center gap-3 rounded-cp bg-cp-fill-alter p-4 text-cp-sm">
        <span>{{ currentVersion ?? '当前版本' }}</span>
        <ArrowRight class="size-4 text-cp-text-secondary" aria-hidden="true" />
        <BaseTag type="primary">
          {{ artifact?.metadata.version }}
        </BaseTag>
        <PluginHelpPopover label="版本切换说明">
          配置不兼容时不会切换，可选择调整配置，不会自动清空参数或密钥，私有数据不兼容时需要插件支持迁移，迁移失败且无法安全恢复时配置会保持停用
        </PluginHelpPopover>
      </div>
    </div>
    <template #footer>
      <BaseButton v-if="rejectedInstanceId !== selectedId" variant="secondary" class="mr-auto" :disabled="saving || !artifact" @click="$emit('create')">
        <template #icon>
          <Plus class="size-4" />
        </template>
        新增配置
      </BaseButton>
      <BaseButton variant="secondary" :disabled="saving" @click="open = false">
        取消
      </BaseButton>
      <BaseButton v-if="rejectedInstanceId === selectedId" variant="primary" :disabled="saving || !selected" @click="selected && $emit('edit', selected)">
        调整配置
      </BaseButton>
      <BaseButton v-if="rejectedInstanceId !== selectedId" variant="primary" :disabled="!selected" :loading="saving" @click="selected && $emit('confirm', selected)">
        确认切换
      </BaseButton>
    </template>
  </BaseModal>
</template>

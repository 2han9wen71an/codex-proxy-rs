<script setup lang="ts">
import type { JsonSchema } from '../utils/model'
import type {
  ConfigurePluginInstanceRequest,
  PluginArtifact,
  PluginInstance,
} from '@/api'

import { BaseButton, BaseForm, BaseFormItem, BaseIconButton, BaseInput, BaseModal, BaseSegmented, BaseSelect, toast } from '@codex-proxy/ui'

import { Boxes, RefreshCw, Save, Settings2, ShieldCheck, Trash2 } from '@lucide/vue'
import { computed, nextTick, ref, shallowRef, useTemplateRef, watch } from 'vue'
import {
  artifactForInstance,
  cloneJsonValue,
  configurationForSchema,
  pluginCapabilityForContribution,
  pluginContributionForCapability,
  pluginRuntimeStatusLabel,
  shortDigest,
  uniquePluginArtifacts,
} from '../utils/model'
import PluginBindingEditor from './PluginBindingEditor.vue'
import PluginConfigurationFields from './PluginConfigurationFields.vue'
import PluginFrontendAuthenticationEditor from './PluginFrontendAuthenticationEditor.vue'
import PluginHelpPopover from './PluginHelpPopover.vue'
import PluginMaintenanceBindingEditor from './PluginMaintenanceBindingEditor.vue'

const props = defineProps<{
  artifacts: PluginArtifact[]
  instance: PluginInstance | null
  artifact: PluginArtifact | null
  defaultName: string
  saving: boolean
}>()

const emit = defineEmits<{
  save: [value: ConfigurePluginInstanceRequest]
}>()

const open = defineModel<boolean>({ required: true })
const section = shallowRef<'general' | 'bindings' | 'authentication' | 'maintenance'>('general')
const configurationValid = shallowRef(true)
const authenticationValid = shallowRef(true)
const bindingsValid = shallowRef(true)
const secretMode = shallowRef<'preserve' | 'replace' | 'clear'>('replace')
const secretValues = ref<Record<string, string>>({})
const form = ref<ConfigurePluginInstanceRequest>(emptyForm())
const configurationFields = useTemplateRef<{ focusInvalid: () => Promise<'configuration' | 'secret' | 'secretMode' | null>, validationMessage: string }>('configurationFields')
const authenticationEditor = useTemplateRef<{ validationError: string }>('authenticationEditor')
const secretModeControl = useTemplateRef<HTMLElement>('secretModeControl')

const currentArtifact = computed(() => props.instance
  ? artifactForInstance(props.instance, props.artifacts)
  : undefined)
const artifactChoices = computed(() => {
  const pluginId = currentArtifact.value?.metadata.pluginId ?? props.artifact?.metadata.pluginId
  return uniquePluginArtifacts(props.artifacts).filter(artifact =>
    artifact.acceptedAt && (!pluginId || artifact.metadata.pluginId === pluginId),
  )
})
const artifactOptions = computed(() => artifactChoices.value.map(artifact => ({
  label: `${artifact.metadata.displayName} ${artifact.metadata.version} · ${shortDigest(artifact.metadata.sha256)}`,
  value: artifact.metadata.sha256,
  description: artifact.metadata.pluginId,
})))
const selectedArtifact = computed(() => props.artifacts.find(artifact =>
  artifact.metadata.sha256 === form.value.artifactSha256,
))
const unsupportedBindings = computed(() => form.value.bindings.filter(binding => !Object.values(selectedArtifact.value?.metadata.contributes ?? {})
  .some(contribution => contribution.id === binding.contribution && contribution.stages.includes(binding.stage))))
const frontendAuthenticationContribution = computed(() => {
  const metadata = selectedArtifact.value?.metadata
  const contribution = metadata
    ? pluginContributionForCapability(metadata, 'frontend_authentication')
    : undefined
  return contribution?.stages.includes('authentication') ? contribution : undefined
})
const maintenanceContribution = computed(() => {
  const metadata = selectedArtifact.value?.metadata
  const contribution = metadata
    ? pluginContributionForCapability(metadata, 'maintenance')
    : undefined
  return contribution?.stages.includes('maintenance') ? contribution : undefined
})
const frontendAuthenticationSupported = computed(() => Boolean(frontendAuthenticationContribution.value))
const maintenanceSupported = computed(() => Boolean(maintenanceContribution.value))
const frontendAuthenticationBinding = computed({
  get: () => {
    const metadata = selectedArtifact.value?.metadata
    return metadata
      ? form.value.bindings.find(binding =>
        pluginCapabilityForContribution(metadata, binding.contribution) === 'frontend_authentication',
      ) ?? null
      : null
  },
  set: (binding) => {
    const metadata = selectedArtifact.value?.metadata
    const otherBindings = metadata
      ? form.value.bindings.filter(value =>
          pluginCapabilityForContribution(metadata, value.contribution) !== 'frontend_authentication',
        )
      : form.value.bindings
    form.value.bindings = binding ? [...otherBindings, cloneJsonValue(binding)] : otherBindings
  },
})
const maintenanceBinding = computed({
  get: () => {
    const metadata = selectedArtifact.value?.metadata
    return metadata
      ? form.value.bindings.find(binding =>
        pluginCapabilityForContribution(metadata, binding.contribution) === 'maintenance',
      ) ?? null
      : null
  },
  set: (binding) => {
    const metadata = selectedArtifact.value?.metadata
    const otherBindings = metadata
      ? form.value.bindings.filter(value =>
          pluginCapabilityForContribution(metadata, value.contribution) !== 'maintenance',
        )
      : form.value.bindings
    form.value.bindings = binding ? [...otherBindings, cloneJsonValue(binding)] : otherBindings
  },
})
const title = computed(() => props.instance ? '编辑插件配置' : '配置并启用插件')
const versionChanged = computed(() => Boolean(props.instance && props.instance.artifactSha256 !== form.value.artifactSha256))
const primaryLabel = computed(() => versionChanged.value ? '保存并切换' : props.instance?.enabled ? '保存' : '保存并启用')
const statusSummary = computed(() => props.instance
  ? props.instance.configurationRequired ? '待配置' : pluginRuntimeStatusLabel(props.instance.runtime.status)
  : '')
const statusLabel = computed(() => {
  if (!props.instance)
    return ''
  if (props.instance.configurationRequired)
    return '待配置 · 补充必填配置后才能启用'
  const runtime = props.instance.runtime
  const label = pluginRuntimeStatusLabel(runtime.status)
  if (runtime.failure)
    return `${label} · ${runtime.failure.message}`
  if (runtime.actualRevision != null)
    return `${label} · 生效修订 r${runtime.actualRevision}`
  if (props.instance.publishedRevision != null)
    return `${label} · 发布 r${props.instance.publishedRevision}`
  return label
})
const canSubmit = computed(() => Boolean(
  form.value.name.trim()
  && selectedArtifact.value,
))

const requestBindingStages = new Set(['request', 'attempt', 'routing', 'scheduling', 'observation'])
function supportsRequestBindings(artifact?: PluginArtifact) {
  return Boolean(artifact && Object.values(artifact.metadata.contributes)
    .some(contribution => contribution.stages.some(stage => requestBindingStages.has(stage))))
}
const requestBindingsSupported = computed(() => supportsRequestBindings(selectedArtifact.value))
const sectionOptions = computed(() => [
  { label: '基本设置', value: 'general', icon: Settings2 },
  ...(requestBindingsSupported.value
    ? [{ label: '请求处理', value: 'bindings', icon: Boxes }]
    : []),
  ...(frontendAuthenticationSupported.value
    ? [{ label: '客户端认证', value: 'authentication', icon: ShieldCheck }]
    : []),
  ...(maintenanceSupported.value
    ? [{ label: '维护任务', value: 'maintenance', icon: RefreshCw }]
    : []),
])
const secretModeOptions = [
  { label: '保留已有值', value: 'preserve' },
  { label: '替换全部值', value: 'replace' },
  { label: '清除全部值', value: 'clear' },
]

function emptyForm(): ConfigurePluginInstanceRequest {
  return {
    name: '',
    artifactSha256: '',
    enabled: false,
    configuration: {},
    bindings: [],
  }
}

function resetForm() {
  const instance = props.instance
  const artifact = instance
    ? artifactForInstance(instance, props.artifacts)
    : props.artifact ?? undefined
  form.value = instance
    ? {
        name: instance.name,
        artifactSha256: instance.artifactSha256,
        enabled: instance.enabled,
        configuration: cloneJsonValue(instance.configuration),
        bindings: cloneJsonValue(instance.bindings),
      }
    : {
        ...emptyForm(),
        name: props.defaultName,
        artifactSha256: artifact?.metadata.sha256 ?? '',
        configuration: artifact
          ? configurationForSchema(artifact.metadata.configurationSchema, {}, artifact.metadata.secretFields)
          : {},
      }
  const required = (artifact?.metadata.configurationSchema as JsonSchema | undefined)?.required ?? []
  const missingSecret = artifact?.metadata.secretFields.some(field => required.includes(field) && !instance?.secretFields.includes(field))
  secretMode.value = instance && !missingSecret ? 'preserve' : 'replace'
  secretValues.value = {}
  configurationValid.value = true
  authenticationValid.value = true
  section.value = 'general'
  if (instance && props.artifact && instance.artifactSha256 !== props.artifact.metadata.sha256)
    selectArtifact(props.artifact.metadata.sha256)
}

function selectArtifact(sha256: string) {
  const previous = selectedArtifact.value
  const next = props.artifacts.find(artifact => artifact.metadata.sha256 === sha256)
  form.value.artifactSha256 = sha256
  if (!next || previous?.metadata.sha256 === next.metadata.sha256)
    return
  secretValues.value = {}
  bindingsValid.value = true
  if (!supportsRequestBindings(next) && section.value === 'bindings')
    section.value = 'general'
  if (!pluginContributionForCapability(next.metadata, 'frontend_authentication')?.stages.includes('authentication')) {
    authenticationValid.value = true
    if (section.value === 'authentication')
      section.value = 'general'
  }
  if (!pluginContributionForCapability(next.metadata, 'maintenance')?.stages.includes('maintenance')) {
    if (section.value === 'maintenance')
      section.value = 'general'
  }
  if (props.instance)
    return
  if (previous?.metadata.pluginId !== next.metadata.pluginId) {
    form.value.configuration = configurationForSchema(
      next.metadata.configurationSchema,
      {},
      next.metadata.secretFields,
    )
  }
}

async function focusConfigurationInvalid() {
  section.value = 'general'
  await nextTick()
  const target = await configurationFields.value?.focusInvalid()
  if (target !== 'secretMode')
    return
  secretModeControl.value
    ?.querySelector<HTMLElement>('button, input:not([type="hidden"]), [tabindex]:not([tabindex="-1"])')
    ?.focus()
}

async function submit(enabled: boolean) {
  if (!form.value.name.trim()) {
    section.value = 'general'
    toast.warning('请输入配置名称')
    return
  }
  if (!selectedArtifact.value) {
    section.value = 'general'
    toast.warning('请选择已安装的版本')
    return
  }
  if (!configurationValid.value) {
    toast.warning(configurationFields.value?.validationMessage || '请检查基本设置')
    await focusConfigurationInvalid()
    return
  }
  if (unsupportedBindings.value.length) {
    section.value = 'general'
    toast.warning('请先移除目标版本不支持的功能绑定')
    return
  }
  if (!bindingsValid.value) {
    section.value = 'bindings'
    toast.warning('请设置请求范围，或勾选“应用于所有请求”')
    return
  }
  if (frontendAuthenticationSupported.value && !authenticationValid.value) {
    section.value = 'authentication'
    toast.warning(authenticationEditor.value?.validationError || '请检查客户端认证身份映射')
    return
  }

  const value: ConfigurePluginInstanceRequest = {
    ...cloneJsonValue(form.value),
    enabled,
    name: form.value.name.trim(),
  }
  if (!props.instance || secretMode.value === 'replace')
    value.secrets = cloneJsonValue(secretValues.value)
  else if (secretMode.value === 'clear')
    value.secrets = {}
  else delete value.secrets
  emit('save', value)
}

watch(open, async (isOpen) => {
  if (isOpen) {
    resetForm()
    if (props.instance?.configurationRequired) {
      await nextTick()
      if (open.value)
        await focusConfigurationInvalid()
    }
    return
  }
  form.value = emptyForm()
  secretValues.value = {}
  configurationValid.value = true
  authenticationValid.value = true
}, { immediate: true })
</script>

<template>
  <BaseModal
    v-model="open"
    :title="title"
    description="设置插件参数与生效范围"
    size="lg"
    :dismissible="!saving"
  >
    <div class="grid gap-5">
      <BaseSegmented
        v-if="sectionOptions.length > 1"
        v-model="section"
        :options="sectionOptions"
        label="插件配置分区"
        :disabled="saving"
        class="w-full sm:w-auto"
      />

      <BaseForm v-show="section === 'general'" class="grid gap-5">
        <BaseFormItem v-if="artifactOptions.length > 1" label="使用版本" required>
          <BaseSelect
            :model-value="form.artifactSha256"
            :options="artifactOptions"
            :disabled="saving || artifactOptions.length === 0"
            placeholder="选择已安装版本"
            class="w-full"
            @update:model-value="selectArtifact"
          />
        </BaseFormItem>

        <BaseFormItem label="配置名称" required>
          <BaseInput
            v-model="form.name"
            maxlength="128"
            :disabled="saving"
            aria-label="配置名称"
            placeholder="例如：默认配置、测试环境"
          />
        </BaseFormItem>

        <div v-if="statusSummary" class="flex items-center gap-1.5 text-cp-xs text-cp-text-secondary">
          <span>{{ statusSummary }}</span>
          <PluginHelpPopover v-if="statusLabel !== statusSummary" label="配置状态说明">
            {{ statusLabel }}
          </PluginHelpPopover>
        </div>
      </BaseForm>

      <div v-show="section === 'general'" class="grid gap-5">
        <div v-if="unsupportedBindings.length" class="flex items-center gap-2 text-cp-xs text-cp-warning-text">
          <span>{{ unsupportedBindings.length }} 项功能不受此版本支持</span>
          <PluginHelpPopover label="不兼容功能详情">
            <p v-for="binding in unsupportedBindings" :key="`${binding.contribution}:${binding.stage}`" class="m-0">
              {{ binding.contribution }} · {{ binding.stage }}
            </p>
            <p class="m-0">
              移除仅修改当前草稿，保存后生效
            </p>
          </PluginHelpPopover>
          <BaseIconButton label="移除不兼容功能" variant="destructive" size="sm" :disabled="saving" @click="form.bindings = form.bindings.filter(binding => !unsupportedBindings.includes(binding))">
            <Trash2 class="size-4" />
          </BaseIconButton>
        </div>
        <div v-if="instance && (selectedArtifact?.metadata.secretFields.length || instance.secretFields.length)" ref="secretModeControl">
          <BaseSegmented
            v-model="secretMode"
            :options="secretModeOptions"
            label="敏感配置保存方式"
            :disabled="saving"
            class="w-full sm:w-auto"
          />
        </div>
        <PluginConfigurationFields
          v-if="selectedArtifact"
          ref="configurationFields"
          v-model:configuration="form.configuration"
          v-model:secrets="secretValues"
          :schema="selectedArtifact.metadata.configurationSchema"
          :secret-fields="selectedArtifact.metadata.secretFields"
          :existing-secret-fields="instance?.secretFields ?? []"
          :secret-mode="secretMode"
          :disabled="saving"
          @validity-change="configurationValid = $event"
        />
      </div>

      <PluginBindingEditor
        v-if="open && selectedArtifact && requestBindingsSupported"
        v-show="section === 'bindings'"
        :key="selectedArtifact.metadata.sha256"
        v-model="form.bindings"
        :metadata="selectedArtifact.metadata"
        :disabled="saving"
        @validity-change="bindingsValid = $event"
      />

      <div v-show="section === 'authentication'" class="grid gap-3">
        <PluginFrontendAuthenticationEditor
          v-if="frontendAuthenticationContribution"
          ref="authenticationEditor"
          v-model="frontendAuthenticationBinding"
          :contribution="frontendAuthenticationContribution"
          :active="open"
          :disabled="saving"
          @validity-change="authenticationValid = $event"
        />
      </div>

      <div v-show="section === 'maintenance'" class="grid gap-3">
        <PluginMaintenanceBindingEditor
          v-if="maintenanceContribution"
          v-model="maintenanceBinding"
          :contribution="maintenanceContribution"
          :disabled="saving"
        />
      </div>
    </div>

    <template #footer>
      <BaseButton variant="secondary" :disabled="saving" @click="open = false">
        取消
      </BaseButton>
      <BaseButton v-if="!versionChanged || instance?.enabled" variant="secondary" :disabled="saving || !canSubmit" @click="submit(false)">
        保存为停用
      </BaseButton>
      <BaseButton variant="primary" :loading="saving" :disabled="!canSubmit" @click="submit(versionChanged ? Boolean(instance?.enabled) : true)">
        <template #icon>
          <Save class="size-4" />
        </template>
        {{ primaryLabel }}
      </BaseButton>
    </template>
  </BaseModal>
</template>

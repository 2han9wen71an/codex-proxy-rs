<script setup lang="ts">
import type { InstalledPlugin } from '../utils/catalog'
import type { PluginArtifact } from '@/api'
import { BaseIconButton, BasePopover, BaseTag } from '@codex-proxy/ui'
import { ArrowDownToLine, ArrowLeftRight, Info, Trash2 } from '@lucide/vue'
import { shortDigest, sourceDetail, sourceLabel } from '../utils/model'

const props = defineProps<{ plugin: InstalledPlugin, busy: boolean }>()
defineEmits<{
  switchVersion: [artifact: PluginArtifact]
  deleteVersion: [artifact: PluginArtifact]
  accept: [artifact: PluginArtifact]
}>()
function uses(artifact: PluginArtifact) {
  return props.plugin.configurations.filter(instance => instance.artifactSha256 === artifact.metadata.sha256)
}
</script>

<template>
  <div class="grid gap-4">
    <ul class="m-0 grid list-none gap-2 p-0" aria-label="已安装版本">
      <li v-for="artifact in plugin.artifacts" :key="artifact.metadata.sha256" class="flex min-w-0 items-center gap-2 rounded-cp bg-cp-fill-alter px-3 py-2 sm:gap-3">
        <div class="grid min-w-0 flex-1 gap-1 sm:flex sm:flex-wrap sm:items-center sm:gap-x-3">
          <strong class="truncate text-cp-sm">{{ artifact.metadata.version }}</strong>
          <span class="text-cp-xs text-cp-text-secondary">{{ sourceLabel(artifact.source) }}</span>
          <span class="truncate font-mono text-cp-xs text-cp-text-quaternary">{{ shortDigest(artifact.metadata.sha256) }}</span>
        </div>
        <BaseTag v-if="!artifact.acceptedAt" type="warning" size="sm">
          待安装
        </BaseTag>
        <BaseTag v-else-if="uses(artifact).length" size="sm">
          使用中
        </BaseTag>
        <div class="flex shrink-0 items-center gap-1">
          <BasePopover trigger="hover-click" placement="top-end">
            <template #trigger>
              <BaseIconButton :label="`${artifact.metadata.version} 来源与摘要`" size="sm" :title="undefined">
                <Info class="size-4" />
              </BaseIconButton>
            </template>
            <dl class="m-0 grid max-w-80 gap-2 p-3 text-cp-xs">
              <dt class="text-cp-text-secondary">
                {{ sourceLabel(artifact.source) }}
              </dt>
              <dd class="m-0 break-all leading-relaxed">
                {{ sourceDetail(artifact.source) }}
              </dd>
              <dt class="text-cp-text-secondary">
                SHA-256
              </dt>
              <dd class="m-0 break-all font-mono leading-relaxed">
                {{ artifact.metadata.sha256 }}
              </dd>
              <template v-if="uses(artifact).length">
                <dt class="text-cp-text-secondary">
                  使用配置
                </dt>
                <dd class="m-0 wrap-anywhere">
                  {{ uses(artifact).map(instance => instance.name).join('、') }}
                </dd>
              </template>
            </dl>
          </BasePopover>
          <BaseIconButton v-if="!artifact.acceptedAt" :label="`安装 ${artifact.metadata.version}`" size="sm" variant="primary" :disabled="busy" @click="$emit('accept', artifact)">
            <ArrowDownToLine class="size-4" />
          </BaseIconButton>
          <BaseIconButton v-else :label="`切换至 ${artifact.metadata.version}`" size="sm" variant="secondary" :disabled="busy || !plugin.configurations.some(instance => instance.artifactSha256 !== artifact.metadata.sha256)" @click="$emit('switchVersion', artifact)">
            <ArrowLeftRight class="size-4" />
          </BaseIconButton>
          <BaseIconButton v-if="artifact.acceptedAt" :label="`删除版本 ${artifact.metadata.version}`" size="sm" variant="destructive" :disabled="busy || uses(artifact).length > 0 || artifact.source.kind === 'builtin'" @click="$emit('deleteVersion', artifact)">
            <Trash2 class="size-4" />
          </BaseIconButton>
        </div>
      </li>
    </ul>
  </div>
</template>

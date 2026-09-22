import type { PluginArtifact, PluginInstance, PluginUpdateSourceBinding } from '@/api'

export interface InstalledPlugin {
  id: string
  artifact: PluginArtifact
  artifacts: PluginArtifact[]
  configurations: PluginInstance[]
  source?: PluginUpdateSourceBinding
}

export type PluginCatalogStatus = 'unaccepted' | 'unconfigured' | 'enabled' | 'disabled' | 'pending' | 'failed'

export function groupInstalledPlugins(artifacts: PluginArtifact[], instances: PluginInstance[], sources: PluginUpdateSourceBinding[]): InstalledPlugin[] {
  const groups = new Map<string, InstalledPlugin>()
  // 最近安装的包用于展示元信息，不把字符串排序误当作语义版本比较。
  for (const artifact of [...artifacts].sort((a, b) => b.installedAt.localeCompare(a.installedAt))) {
    const id = artifact.metadata.pluginId
    const group = groups.get(id)
    if (group)
      group.artifacts.push(artifact)
    else groups.set(id, { id, artifact, artifacts: [artifact], configurations: [], source: sources.find(source => source.pluginId === id) })
  }
  const artifactPlugins = new Map(artifacts.map(artifact => [artifact.metadata.sha256, artifact.metadata.pluginId]))
  for (const instance of instances) {
    const id = artifactPlugins.get(instance.artifactSha256)
    if (id)
      groups.get(id)?.configurations.push(instance)
  }
  return [...groups.values()]
}

export function configurationStatus(instance: PluginInstance): PluginCatalogStatus {
  if (instance.configurationRequired)
    return 'unconfigured'
  if (!instance.enabled)
    return 'disabled'
  if (['blocked', 'preparation_failed', 'faulted'].includes(instance.runtime.status))
    return 'failed'
  return instance.runtime.status === 'running' ? 'enabled' : 'pending'
}

export function pluginStatus(plugin: InstalledPlugin): PluginCatalogStatus {
  if (!plugin.artifacts.some(artifact => artifact.acceptedAt))
    return 'unaccepted'
  const states = plugin.configurations.map(configurationStatus)
  if (!states.length)
    return 'unconfigured'
  for (const state of ['failed', 'unconfigured', 'pending', 'enabled'] as const) {
    if (states.includes(state))
      return state
  }
  return 'disabled'
}

export const PLUGIN_STATUS_LABELS: Record<PluginCatalogStatus, string> = {
  unaccepted: '待安装',
  unconfigured: '待配置',
  enabled: '已启用',
  disabled: '已停用',
  pending: '等待生效',
  failed: '需要处理',
}

export function pluginStatusType(status: PluginCatalogStatus) {
  if (status === 'enabled')
    return 'success' as const
  if (status === 'failed')
    return 'danger' as const
  if (status === 'pending' || status === 'unconfigured' || status === 'unaccepted')
    return 'warning' as const
  return 'neutral' as const
}

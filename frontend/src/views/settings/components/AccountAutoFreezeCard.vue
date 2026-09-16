<script setup lang="ts">
import { Activity, Gauge, Snowflake } from '@lucide/vue'

import BaseCard from '@/components/base/BaseCard.vue'
import BaseFormItem from '@/components/base/BaseForm/FormItem.vue'
import BaseForm from '@/components/base/BaseForm/index.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseSwitch from '@/components/base/BaseSwitch.vue'

const enabled = defineModel<boolean>('enabled', { required: true })
const threshold = defineModel<string>('threshold', { required: true })
const windowSeconds = defineModel<string>('windowSeconds', { required: true })
const durationSeconds = defineModel<string>('durationSeconds', { required: true })
const probeEnabled = defineModel<boolean>('probeEnabled', { required: true })
const probeModel = defineModel<string>('probeModel', { required: true })
const adaptiveConcurrency = defineModel<boolean>('adaptiveConcurrency', { required: true })
</script>

<template>
  <BaseCard
    title="账号自动冻结"
    description="容量类错误高频出现时冻结账号，到期探测后自动恢复"
  >
    <BaseForm class="max-w-6xl sm:grid-cols-2">
      <div class="col-span-full flex min-h-6 items-center justify-between gap-3">
        <span class="text-cp leading-none font-medium text-cp-text-secondary">启用自动冻结</span>
        <BaseSwitch
          v-model="enabled"
          label="切换账号自动冻结"
        />
      </div>

      <BaseFormItem
        label="触发阈值"
        description="统计窗口内容量类失败的次数（按尝试计数），达到后冻结该账号"
      >
        <BaseInput
          v-model="threshold"
          aria-label="触发阈值"
          type="number"
          min="2"
          max="1000"
          step="1"
        >
          <template #prefix>
            <Activity class="size-4" />
          </template>
        </BaseInput>
      </BaseFormItem>

      <BaseFormItem
        label="统计窗口（秒）"
        description="失败计数随每次失败滑动顺延的窗口长度"
      >
        <BaseInput
          v-model="windowSeconds"
          aria-label="统计窗口秒数"
          type="number"
          min="60"
          max="3600"
          step="1"
        >
          <template #prefix>
            <Activity class="size-4" />
          </template>
        </BaseInput>
      </BaseFormItem>

      <BaseFormItem
        label="冻结时长（秒）"
        description="冻结持续时间；恢复探测失败时按该时长顺延"
      >
        <BaseInput
          v-model="durationSeconds"
          aria-label="冻结时长秒数"
          type="number"
          min="300"
          max="604800"
          step="1"
        >
          <template #prefix>
            <Snowflake class="size-4" />
          </template>
        </BaseInput>
      </BaseFormItem>

      <BaseFormItem
        label="恢复探测模型"
        description="留空时自动选择账号可用的第一个模型；探测内容为极短的确认回复"
      >
        <BaseInput
          v-model="probeModel"
          aria-label="恢复探测模型"
          placeholder="留空自动选择"
        >
          <template #prefix>
            <Gauge class="size-4" />
          </template>
        </BaseInput>
      </BaseFormItem>

      <div class="col-span-full grid gap-4 pt-4 sm:grid-cols-2">
        <div class="flex min-h-6 items-center justify-between gap-3">
          <span class="text-cp leading-none font-medium text-cp-text-secondary">恢复前探测</span>
          <BaseSwitch
            v-model="probeEnabled"
            label="切换恢复前探测"
          />
        </div>
        <div class="flex min-h-6 items-center justify-between gap-3">
          <span class="text-cp leading-none font-medium text-cp-text-secondary">自适应并发下调</span>
          <BaseSwitch
            v-model="adaptiveConcurrency"
            label="切换自适应并发下调"
          />
        </div>
      </div>
    </BaseForm>
  </BaseCard>
</template>

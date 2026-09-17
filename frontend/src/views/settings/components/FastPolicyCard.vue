<script setup lang="ts">
import BaseCard from '@/components/base/BaseCard.vue'
import BaseFormItem from '@/components/base/BaseForm/FormItem.vue'
import BaseForm from '@/components/base/BaseForm/index.vue'
import BaseSwitch from '@/components/base/BaseSwitch.vue'

defineProps<{ disabled: boolean }>()

const model = defineModel<boolean>({ required: true })
</script>

<template>
  <BaseCard title="服务档位" description="Fast 模式下的请求将按普通档处理，关闭后仍遵循分组限制">
    <BaseForm class="max-w-6xl">
      <BaseFormItem>
        <BaseSwitch v-model="model" label="关闭" show-label :disabled="disabled" />
      </BaseFormItem>
    </BaseForm>
  </BaseCard>
</template>

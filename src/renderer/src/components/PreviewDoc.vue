<script setup>
/**
 * 预览/打印：渲染工单表格版式（v-html）。
 * 传入 order，computed 生成 HTML，数据变化自动刷新。
 */
import { computed } from 'vue';
import { buildWorkOrderDoc } from '../lib/workOrderDoc';

const props = defineProps({
  order: { type: Object, required: true }
});

const docHtml = computed(() => buildWorkOrderDoc(props.order));
</script>

<template>
  <div class="preview-root">
    <!-- eslint-disable-next-line vue/no-v-html -->
    <div v-html="docHtml"></div>
  </div>
</template>

<style scoped>
.preview-root { display: flex; justify-content: center; min-height: 200px; width: 100%; }
</style>
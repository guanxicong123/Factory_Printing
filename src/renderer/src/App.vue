<script setup>
/**
 * 主应用：顶部工具栏 + 左侧工单列表 + 中部编辑/预览。
 * 所有业务动作（新建/打开/保存/复制/打印）在此集中。
 */
import { ref, reactive, onMounted } from 'vue';
import { ElMessage } from 'element-plus';
import { api } from './api';
import {
  defaultOrder, normalizeOrder, ensureNewOrderDates,
  displayOrderNo
} from './lib/order';
import EditForm from './components/EditForm.vue';
import PreviewDoc from './components/PreviewDoc.vue';

// currentOrder 用 reactive：整棵树响应式，子组件 v-model 直接改字段能触发刷新
const currentOrder = reactive(normalizeOrder(defaultOrder()));
const currentFileName = ref('');
const ordersCache = ref([]);
const currentView = ref('edit'); // 'edit' | 'preview'
const statusMsg = ref('就绪');
let dirty = false;

/* ================= 视图切换 ================= */
function switchView(view) {
  currentView.value = view;
  setStatus(view === 'preview' ? '预览模式（只读）' : '编辑模式', 'ok');
}

/* ================= 原地载入新工单（保持引用稳定，避免编辑态绑定失效） ================= */
function loadOrderInto(obj) {
  const n = normalizeOrder(obj);
  // 顶层标量字段（orderNo、meta 等）整体替换
  ['orderNo', 'meta'].forEach((k) => { currentOrder[k] = n[k]; });
  // fields / cks：保留原对象引用（EditForm 的 reactive 绑定依赖它），原地清空重填
  if (!(currentOrder.fields && typeof currentOrder.fields === 'object')) currentOrder.fields = {};
  if (!(currentOrder.cks && typeof currentOrder.cks === 'object')) currentOrder.cks = {};
  Object.keys(currentOrder.fields).forEach((k) => { delete currentOrder.fields[k]; });
  Object.keys(currentOrder.cks).forEach((k) => { delete currentOrder.cks[k]; });
  Object.assign(currentOrder.fields, n.fields || {});
  Object.assign(currentOrder.cks, n.cks || {});
}

/* ================= 状态栏 ================= */
function setStatus(msg, type) {
  statusMsg.value = msg;
}

/* ================= 工具条动作 ================= */
async function doNewOrder() {
  if (!confirmDiscardDirty()) return;
  loadOrderInto(await api.newOrder());
  ensureNewOrderDates(currentOrder);
  currentFileName.value = currentOrder.meta?.fileName || '';
  switchView('edit');
  dirty = false;
  setStatus('已新建工单', 'ok');
}

async function doOpenOrder() {
  const order = await api.openOrder();
  if (!order) return;
  loadOrderInto(order);
  currentFileName.value = order.meta?.fileName || '';
  switchView('edit');
  dirty = false;
  setStatus(`已打开：${currentFileName.value || '外部文件'}`, 'ok');
}

async function doSaveOrder() {
  if (!currentOrder.meta) { ElMessage.warning('当前无工单'); return; }
  prepareOrderForSave();
  try {
    const res = await api.saveOrder(currentOrder);
    currentFileName.value = res.fileName;
    loadOrderInto(res.order);
    await refreshOrderList();
    dirty = false;
    setStatus(`已保存：${currentFileName.value}`, 'ok');
  } catch (e) {
    ElMessage.error('保存失败：' + e);
  }
}

function prepareOrderForSave() {
  const raw = currentOrder.fields?.mJob_No || currentOrder.fields?.orderNoDisplay || currentOrder.orderNo || '';
  currentOrder.orderNo = raw.replace(/^No\.\s?/i, '');
  if (!currentOrder.meta.fileName) {
    currentOrder.meta.fileName = `${currentOrder.orderNo || 'order'}.json`;
  }
}

async function doDuplicateOrder() {
  if (!currentOrder.meta) { ElMessage.warning('当前无工单'); return; }
  prepareOrderForSave();
  try {
    const newOrder = await api.duplicateOrder(currentOrder);
    loadOrderInto(newOrder);
    currentFileName.value = currentOrder.meta?.fileName || '';
    switchView('edit');
    dirty = false;
    setStatus(`已复制为新工单：${displayOrderNo(currentOrder.orderNo)}`, 'ok');
  } catch (e) {
    ElMessage.error('复制失败：' + e);
  }
}

async function doPrint() {
  if (currentView.value !== 'preview') {
    switchView('preview');
    // 等待 DOM 渲染
    await new Promise((r) => setTimeout(r, 50));
  }
  try {
    await api.printOrder();
    window.print();
    setStatus('已发送打印任务', 'ok');
  } catch (e) {
    ElMessage.error('打印出错：' + e);
  }
}

/* ================= 列表 ================= */
async function refreshOrderList() {
  try {
    ordersCache.value = await api.listOrders();
  } catch (e) {
    ordersCache.value = [];
  }
}
async function doLoadSelected(fileName) {
  if (!fileName) return;
  try {
    const order = await api.loadOrder(fileName);
    loadOrderInto(order);
    currentFileName.value = fileName;
    switchView('edit');
    dirty = false;
    setStatus(`已打开：${fileName}`, 'ok');
  } catch (e) {
    ElMessage.error('打开失败：' + e);
  }
}

/* ================= 未保存提醒 ================= */
function confirmDiscardDirty() {
  if (!dirty) return true;
  return window.confirm('当前工单有未保存的修改，确定放弃吗？');
}

/* ================= 菜单事件 ================= */
onMounted(async () => {
  await refreshOrderList();
  try {
    loadOrderInto(await api.newOrder());
  } catch (e) {
    loadOrderInto(defaultOrder());
  }
  ensureNewOrderDates(currentOrder);
  currentFileName.value = currentOrder.meta?.fileName || '';
  dirty = false;
  setStatus('新工单已就绪', 'ok');

  // 开发调试：URL 带 ?view=preview 自动切到预览（便于 headless 打印验证）
  if (new URLSearchParams(location.search).get('view') === 'preview') {
    await new Promise((r) => setTimeout(r, 100));
    switchView('preview');
  }
});
</script>

<template>
  <!-- 工具栏 -->
  <header class="app-toolbar">
    <span class="app-title">印刷印件工单</span>
    <div class="toolbar-actions">
      <el-button size="small" @click="doNewOrder">新建</el-button>
      <el-button size="small" @click="doOpenOrder">打开…</el-button>
      <el-button size="small" type="primary" @click="doSaveOrder">保存</el-button>
      <el-button size="small" @click="doDuplicateOrder">复制</el-button>
      <el-divider direction="vertical" />
      <el-button size="small" @click="switchView('edit')" type="primary" v-if="currentView!=='edit'">✏️ 编辑</el-button>
      <el-button size="small" @click="switchView('edit')" plain v-else>✏️ 编辑</el-button>
      <el-button size="small" @click="switchView('preview')" plain v-if="currentView!=='preview'">👁 预览</el-button>
      <el-button size="small" type="warning" plain v-else>👁 预览</el-button>
      <el-button size="small" type="danger" @click="doPrint">打印</el-button>
    </div>
  </header>

  <div class="app-body">
    <!-- 左侧列表 -->
    <aside class="app-sidebar">
      <div class="sidebar-head">
        <span>工单列表</span>
        <el-button size="small" text @click="refreshOrderList; setStatus('已刷新列表','ok')">↻</el-button>
      </div>
      <ul class="order-list" v-if="ordersCache.length">
        <li
          v-for="o in ordersCache"
          :key="o.fileName"
          class="order-item"
          :data-fname="o.fileName"
          :class="{ active: o.fileName === currentFileName }"
          @click="doLoadSelected(o.fileName)"
        >
          <span class="order-no">{{ displayOrderNo(o.orderNo || o.id || '未编号') }}</span>
          <span class="order-name">{{ o.productName || '（未填产品名）' }}</span>
          <span class="order-time">{{ o.updatedAt ? o.updatedAt.slice(0,16).replace('T',' ') : '' }}</span>
        </li>
      </ul>
      <ul v-else class="order-list"><li class="order-empty">（暂无工单）</li></ul>
    </aside>

    <!-- 中部内容 -->
    <section class="app-content">
      <!-- 编辑态 -->
      <div id="edit-root" ref="editRoot" v-show="currentView==='edit'" class="edit-scroll">
        <EditForm :order="currentOrder" />
      </div>
      <!-- 预览态 -->
      <div v-show="currentView==='preview'" class="preview-scroll">
        <PreviewDoc :order="currentOrder" :key="currentFileName + 'p'" />
      </div>
    </section>
  </div>

  <div class="app-statusbar">{{ statusMsg }}</div>
</template>

<style scoped>
.app-toolbar {
  display: flex; align-items: center; justify-content: space-between;
  padding: 8px 14px; background: #fff; border-bottom: 1px solid #cfd6dc;
  box-shadow: 0 1px 3px rgba(0,0,0,.08); z-index: 10; flex-shrink: 0;
}
.app-title { font-size: 16px; font-weight: 700; color: #1a237e; letter-spacing: 1px; }
.toolbar-actions { display: flex; align-items: center; gap: 6px; }

.app-body { display: flex; flex: 1; min-height: 0; overflow: hidden; }

.app-sidebar {
  width: 240px; flex-shrink: 0; background: #fafbfc;
  border-right: 1px solid #d5dbdf;
  display: flex; flex-direction: column; min-height: 0;
}
.sidebar-head {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 12px; font-weight: 700; color: #37474f;
  border-bottom: 1px solid #e0e0e0; flex-shrink: 0;
}
.order-list { list-style: none; flex: 1; overflow-y: auto; padding: 6px 8px; margin: 0; }
.order-item {
  padding: 9px 10px; margin-bottom: 4px; border-radius: 6px; cursor: pointer;
  border: 1px solid transparent; transition: background .12s, border-color .12s;
}
.order-item:hover { background: #e8f0fe; }
.order-item.active { background: #e3f2fd; border-color: #90caf9; }
.order-no { display: block; font-weight: 700; color: #1a237e; font-size: 13.5px; }
.order-name { display: block; color: #455a64; font-size: 12px; margin-top: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.order-time { display: block; color: #90a4ae; font-size: 11px; margin-top: 2px; }
.order-empty { color: #999; text-align: center; padding: 20px 0; list-style: none; }

.app-content { flex: 1; min-width: 0; overflow: hidden; display: flex; flex-direction: column; }
.edit-scroll { flex: 1; overflow: auto; background: #eceff1; padding: 16px 20px; }
.preview-scroll { flex: 1; overflow: auto; background: #eceff1; padding: 16px 20px; }

.app-statusbar {
  flex-shrink: 0; padding: 5px 14px; font-size: 12px; color: #555;
  background: #fafafa; border-top: 1px solid #e0e0e0;
}
</style>
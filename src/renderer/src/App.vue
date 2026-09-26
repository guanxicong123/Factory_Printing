<script setup>
/**
 * 主应用：顶部工具栏 + 左侧工单列表 + 中部编辑/预览。
 * 所有业务动作（新建/打开/保存/复制/打印）在此集中。
 *
 * 左侧列表：后端分页拉取 + DOM 虚拟滚动，避免几万条一次性渲染崩溃。
 */
import { ref, reactive, computed, onMounted, nextTick } from 'vue';
import { ElMessage } from 'element-plus';
import { api } from './api';
import {
  defaultOrder, normalizeOrder, ensureNewOrderDates,
  displayOrderNo
} from './lib/order';
import EditForm from './components/EditForm.vue';
import PreviewDoc from './components/PreviewDoc.vue';
import ListOrders from './components/ListOrders.vue';

/** 侧栏每页拉取条数 */
const SIDEBAR_PAGE_SIZE = 60;
/** 侧栏行高（与 CSS .order-item 视觉高度对齐，用于虚拟滚动） */
const SIDEBAR_ITEM_H = 68;
const SIDEBAR_OVERSCAN = 8;

// currentOrder 用 reactive：整棵树响应式，子组件 v-model 直接改字段能触发刷新
const currentOrder = reactive(normalizeOrder(defaultOrder()));
const currentFileName = ref('');
const ordersCache = ref([]);
const ordersTotal = ref(0);
const listLoading = ref(false);
const currentView = ref('edit'); // 'edit' | 'preview' | 'list'
const statusMsg = ref('就绪');
let dirty = false;

/* ---------- 虚拟滚动状态 ---------- */
const orderListEl = ref(null);
const scrollTop = ref(0);
const viewportH = ref(400);

const listHasMore = computed(() => ordersCache.value.length < ordersTotal.value);

const virtualStart = computed(() =>
  Math.max(0, Math.floor(scrollTop.value / SIDEBAR_ITEM_H) - SIDEBAR_OVERSCAN)
);
const virtualEnd = computed(() =>
  Math.min(
    ordersCache.value.length,
    Math.ceil((scrollTop.value + viewportH.value) / SIDEBAR_ITEM_H) + SIDEBAR_OVERSCAN
  )
);
const visibleOrders = computed(() =>
  ordersCache.value.slice(virtualStart.value, virtualEnd.value)
);
const padTop = computed(() => virtualStart.value * SIDEBAR_ITEM_H);
const padBottom = computed(() =>
  Math.max(0, (ordersCache.value.length - virtualEnd.value) * SIDEBAR_ITEM_H)
);

function onSidebarScroll(e) {
  const el = e.target;
  scrollTop.value = el.scrollTop;
  viewportH.value = el.clientHeight;
  // 距底部不足约 12 行时预取下一页
  if (el.scrollHeight - el.scrollTop - el.clientHeight < SIDEBAR_ITEM_H * 12) {
    loadMoreOrders();
  }
}

/* ================= 视图切换 ================= */
function switchView(view) {
  currentView.value = view;
  if (view === 'preview') {
    setStatus('预览模式（只读）', 'ok');
  } else if (view === 'list') {
    setStatus('工单列表', 'ok');
  } else {
    setStatus('编辑模式', 'ok');
  }
}

/* 进入列表页：先刷新数据源再切换 */
async function goList() {
  if (!confirmDiscardDirty()) return;
  await refreshOrderList();
  switchView('list');
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
  try {
    loadOrderInto(await api.newOrder());
    ensureNewOrderDates(currentOrder);
    prepareOrderForSave();
    const res = await api.saveOrder(currentOrder);
    currentFileName.value = res.fileName;
    loadOrderInto(res.order);
    await refreshOrderList();
    switchView('edit');
    dirty = false;
    setStatus(`已新建工单：${displayOrderNo(currentOrder.orderNo)}`, 'ok');
  } catch (e) {
    ElMessage.error('新建失败：' + e);
  }
}

/**
 * 导入 SQL：选择 MySQL dump（.sql），后端解析 t_gd（优先）/ t_job 并落盘为工单。
 * count = 成功导入条数（0 表示取消或无可导入数据）；skipped = 跳过的数据行数。
 */
async function doImportSql() {
  try {
    const res = await api.importSql();
    if (!res || !res.count) {
      ElMessage.info(res?.message || '未导入任何工单');
      setStatus(res?.message || '未导入任何工单');
      return;
    }
    await refreshOrderList();
    const extra = res.skipped ? `，跳过 ${res.skipped} 条` : '';
    ElMessage.success(res.message || `已导入 ${res.count} 条工单`);
    setStatus(`导入完成：${res.count} 条${extra}`, 'ok');
  } catch (e) {
    ElMessage.error('导入失败：' + e);
  }
}

/* 从列表页刷新数据源（复制/删除后同步 sidebar 与列表页） */
async function requestRefreshList() {
  await refreshOrderList();
  setStatus('已刷新列表', 'ok');
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
  if (!confirmDiscardDirty()) return;
  prepareOrderForSave();
  try {
    const newOrder = await api.duplicateOrder(currentOrder);
    loadOrderInto(newOrder);
    prepareOrderForSave();
    const res = await api.saveOrder(currentOrder);
    currentFileName.value = res.fileName;
    loadOrderInto(res.order);
    await refreshOrderList();
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

/* ================= 列表（分页 + 追加） ================= */
/** 内容不足以产生滚动条时，继续预取直到可滚或无更多 */
async function fillViewportIfNeeded() {
  await nextTick();
  const el = orderListEl.value;
  if (!el || listLoading.value || !listHasMore.value) return;
  viewportH.value = el.clientHeight;
  if (el.scrollHeight <= el.clientHeight + SIDEBAR_ITEM_H) {
    await loadMoreOrders();
  }
}

async function loadMoreOrders(reset = false) {
  if (listLoading.value) return;
  if (!reset && !listHasMore.value) return;
  listLoading.value = true;
  try {
    const offset = reset ? 0 : ordersCache.value.length;
    const page = await api.listOrders({ limit: SIDEBAR_PAGE_SIZE, offset });
    const items = page?.items || [];
    ordersTotal.value = page?.total ?? items.length;
    if (reset) {
      ordersCache.value = items;
      scrollTop.value = 0;
      await nextTick();
      if (orderListEl.value) orderListEl.value.scrollTop = 0;
    } else {
      const seen = new Set(ordersCache.value.map((o) => o.fileName));
      ordersCache.value.push(...items.filter((o) => o && !seen.has(o.fileName)));
    }
  } catch (e) {
    if (reset) {
      ordersCache.value = [];
      ordersTotal.value = 0;
    }
  } finally {
    listLoading.value = false;
  }
  await fillViewportIfNeeded();
}

async function refreshOrderList() {
  await loadMoreOrders(true);
}

async function onSidebarRefresh() {
  await refreshOrderList();
  setStatus('已刷新列表', 'ok');
}

async function doLoadSelected(fileName) {
  if (!fileName) return;
  try {
    const order = await api.loadOrder(fileName);
    loadOrderInto(order);
    currentFileName.value = fileName;
    // 侧栏切换：保持当前编辑/预览态；从列表页打开则进入编辑
    if (currentView.value === 'list') {
      switchView('edit');
    }
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

/* ================= 启动：不自动占号；有工单则打开最近一条，否则进列表 ================= */
onMounted(async () => {
  await refreshOrderList();
  dirty = false;

  const list = ordersCache.value || [];
  if (list.length) {
    await doLoadSelected(list[0].fileName);
  } else {
    switchView('list');
    setStatus('暂无工单，请点击新建', 'ok');
  }

  // 开发调试：URL 带 ?view=preview 自动切到预览（便于 headless 打印验证）
  if (new URLSearchParams(location.search).get('view') === 'preview' && currentFileName.value) {
    await new Promise((r) => setTimeout(r, 100));
    switchView('preview');
  }
});
</script>

<template>
  <!-- 工具栏 -->
  <header class="app-toolbar">
    <span class="app-title">印刷工单</span>
    <div class="toolbar-actions">
      <el-button size="small" @click="doNewOrder">新建</el-button>
      <el-button size="small" id="btn-open" @click="doImportSql">导入</el-button>
      <el-button size="small" type="primary" @click="doSaveOrder">保存</el-button>
      <el-button size="small" @click="doDuplicateOrder">复制</el-button>
      <el-divider direction="vertical" />
      <el-button size="small" @click="switchView('edit')" type="primary" v-if="currentView!=='edit'">✏️ 编辑</el-button>
      <el-button size="small" @click="switchView('edit')" plain v-else>✏️ 编辑</el-button>
      <el-button size="small" @click="switchView('preview')" plain v-if="currentView!=='preview'">👁 预览</el-button>
      <el-button size="small" type="warning" plain v-else>👁 预览</el-button>
      <el-button size="small" @click="goList" type="primary" v-if="currentView!=='list'">📋 列表</el-button>
      <el-button size="small" type="primary" plain v-else>📋 列表</el-button>
      <el-button size="small" type="danger" @click="doPrint">打印</el-button>
    </div>
  </header>

  <div class="app-body">
    <!-- 左侧列表：列表视图中隐藏，列表页全宽 -->
    <aside class="app-sidebar" v-if="currentView!=='list'">
      <div class="sidebar-head">
        <span>工单列表{{ ordersTotal ? `（${ordersTotal}）` : '' }}</span>
        <el-button size="small" text :loading="listLoading" @click="onSidebarRefresh">↻</el-button>
      </div>
      <ul
        v-if="ordersCache.length"
        ref="orderListEl"
        class="order-list"
        @scroll.passive="onSidebarScroll"
      >
        <li class="order-spacer" :style="{ height: padTop + 'px' }" aria-hidden="true" />
        <li
          v-for="o in visibleOrders"
          :key="o.fileName"
          class="order-item"
          :style="{ height: SIDEBAR_ITEM_H + 'px' }"
          :data-fname="o.fileName"
          :class="{ active: o.fileName === currentFileName }"
          @click="doLoadSelected(o.fileName)"
        >
          <span class="order-no">{{ displayOrderNo(o.orderNo || o.id || '未编号') }}</span>
          <span class="order-name">{{ o.productName || '（未填产品名）' }}</span>
          <span class="order-time">{{ o.updatedAt ? o.updatedAt.slice(0,16).replace('T',' ') : '' }}</span>
        </li>
        <li class="order-spacer" :style="{ height: padBottom + 'px' }" aria-hidden="true" />
        <li v-if="listLoading" class="order-empty order-loading">加载中…</li>
        <li v-else-if="!listHasMore && ordersCache.length" class="order-empty order-end">已加载全部</li>
      </ul>
      <ul v-else class="order-list">
        <li class="order-empty">{{ listLoading ? '加载中…' : '（暂无工单）' }}</li>
      </ul>
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
      <!-- 列表态：全宽工单列表（自行按页请求，不依赖侧栏缓存全量） -->
      <ListOrders
        v-show="currentView==='list'"
        class="list-scroll"
        :refresh-token="ordersTotal"
        @open="doLoadSelected"
        @refresh="requestRefreshList"
      />
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
.order-spacer { padding: 0; margin: 0; border: none; list-style: none; pointer-events: none; }
.order-item {
  box-sizing: border-box;
  padding: 8px 10px; margin-bottom: 0; border-radius: 6px; cursor: pointer;
  border: 1px solid transparent; transition: background .12s, border-color .12s;
  overflow: hidden;
}
.order-item:hover { background: #e8f0fe; }
.order-item.active { background: #e3f2fd; border-color: #90caf9; }
.order-no { display: block; font-weight: 700; color: #1a237e; font-size: 13.5px; line-height: 1.3; }
.order-name { display: block; color: #455a64; font-size: 12px; margin-top: 2px; line-height: 1.3; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.order-time { display: block; color: #90a4ae; font-size: 11px; margin-top: 2px; line-height: 1.3; }
.order-empty { color: #999; text-align: center; padding: 12px 0; list-style: none; font-size: 12px; }
.order-loading, .order-end { padding: 8px 0; }

.app-content { flex: 1; min-width: 0; overflow: hidden; display: flex; flex-direction: column; }
.edit-scroll { flex: 1; overflow: auto; background: #eceff1; padding: 16px 20px; }
.preview-scroll { flex: 1; overflow: auto; background: #eceff1; padding: 16px 20px; }
.list-scroll { flex: 1; overflow: auto; background: #fff; padding: 16px 20px; }

.app-statusbar {
  flex-shrink: 0; padding: 5px 14px; font-size: 12px; color: #555;
  background: #fafafa; border-top: 1px solid #e0e0e0;
}
</style>

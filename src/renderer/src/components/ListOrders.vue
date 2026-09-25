<script setup>
/**
 * 工单列表页：全宽 el-table 展示全部工单，支持搜索、分页、复制、删除、打开。
 *
 * 职责
 * - 使用 api.listOrders 刷新列表并传给父级共享的 ordersCache（新增/删除/打开后父级可同步刷新）。
 * - 搜索按 单号 / 客户 / 合同号 前端过滤；分页在前端数据上进行。
 * - 复制：api.duplicateOrder(order) 生成新工单号，随后 api.saveOrder 持久化；refresh 后提示。
 * - 删除：ElMessageBox.confirm 确认后 api.deleteOrder(fileName)。
 * - 打开：emit open(fileName) 由 App 载入并切到编辑视图。
 */
import { ref, reactive, computed, onMounted, watch } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Search } from '@element-plus/icons-vue';
import { api } from '../api';
import { displayOrderNo, cnDateToIso } from '../lib/order';

const props = defineProps({
  orders: { type: Array, default: () => [] }
});
const emit = defineEmits(['open', 'refresh']);

const loading = ref(false);
const keyword = ref('');
const page = ref(1);
const pageSize = ref(10);
const pageSizes = [10, 20, 50, 100];

/* listOrders 的 item 不含客户/合同号/开单日期，按 fileName 懒加载字段缓存 */
const extraMap = reactive(new Map());

async function loadExtra(item) {
  if (!item || extraMap.has(item.fileName)) return;
  try {
    const o = (await api.loadOrder(item.fileName)) || {};
    const f = o.fields || {};
    extraMap.set(item.fileName, {
      customer: f.customer || '',
      contractNo: f.contractNo || '',
      openDate: f.openDate || f.mJob_Date || '',
      productSpec: f.productSpec || f.mProduct_Name || ''
    });
  } catch (e) {
    extraMap.set(item.fileName, { customer: '', contractNo: '', openDate: '', productSpec: '' });
  }
}

function matches(row, kw) {
  const extra = extraMap.get(row.fileName) || {};
  const customer = (row.productName || extra.customer || '').toLowerCase();
  const no = String(row.orderNo || '').toLowerCase();
  const contract = (extra.contractNo || '').toLowerCase();
  const spec = (extra.productSpec || '').toLowerCase();
  return no.includes(kw) || customer.includes(kw) || contract.includes(kw) || spec.includes(kw);
}

const filtered = computed(() => {
  const raw = props.orders || [];
  const kw = keyword.value.trim().toLowerCase();
  return kw ? raw.filter((o) => matches(o, kw)) : raw;
});

const rows = computed(() => {
  const list = filtered.value;
  const start = (page.value - 1) * pageSize.value;
  return list.slice(start, start + pageSize.value);
});

const filteredTotal = computed(() => filtered.value.length);

/* 数据源变化时：预载全部行的扩展字段，并修正页码 */
watch(
  () => props.orders,
  (list) => {
    (list || []).forEach((o) => loadExtra(o));
    const max = Math.max(1, Math.ceil(filteredTotal.value / pageSize.value));
    if (page.value > max) page.value = max;
  },
  { immediate: true }
);

watch(keyword, () => { page.value = 1; });

function cellDate(row) {
  const extra = extraMap.get(row.fileName);
  if (!extra) return '';
  return extra.openDate && cnDateToIso(extra.openDate);
}

async function handleOpen(row) {
  emit('open', row.fileName);
}

async function handleCopy(row) {
  try {
    const order = await api.loadOrder(row.fileName);
    if (!order) { ElMessage.warning('工单加载失败'); return; }
    const newOrder = await api.duplicateOrder(order);
    const res = await api.saveOrder(newOrder);
    ElMessage.success(`已复制为新工单：No. ${res.order.orderNo || ''}`);
    refresh();
    // 复制成功后直接打开编辑态
    if (res.order.meta && res.order.meta.fileName) {
      emit('open', res.order.meta.fileName);
    }
  } catch (e) {
    ElMessage.error('复制失败：' + e);
  }
}

async function handleDelete(row) {
  try {
    await ElMessageBox.confirm(
      `确定删除工单「${displayOrderNo(row.orderNo)}」吗？此操作不可撤销。`,
      '删除工单',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' }
    );
  } catch (e) {
    return; // 用户取消
  }
  try {
    await api.deleteOrder(row.fileName);
    ElMessage.success(`已删除 ${row.fileName}`);
    refresh();
  } catch (e) {
    ElMessage.error('删除失败：' + e);
  }
}

function refresh() { emit('refresh'); }

onMounted(() => {
  (props.orders || []).forEach((o) => loadExtra(o));
});
</script>

<template>
  <div class="list-orders">
    <div class="list-head">
      <div class="list-title">工单列表（{{ filteredTotal }} 条）</div>
      <div class="list-filters">
        <el-input
          v-model="keyword"
          placeholder="搜索 单号 / 客户 / 合同号"
          clearable
          size="small"
          class="search-input"
        >
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-button size="small" :loading="loading" @click="refresh">刷新</el-button>
      </div>
    </div>

    <el-table
      v-loading="loading"
      :data="rows"
      border
      stripe
      highlight-current-row
      row-key="fileName"
      class="orders-table"
      size="default"
      @row-dblclick="handleOpen"
    >
      <el-table-column prop="orderNo" label="工号" width="140" sortable>
        <template #default="{ row }">
          <span class="order-no-cell" @click="handleOpen(row)">{{ displayOrderNo(row.orderNo) }}</span>
        </template>
      </el-table-column>
      <el-table-column label="开单日期" width="160" sortable>
        <template #default="{ row }">
          <span class="o-cell">{{ cellDate(row) }}</span>
        </template>
      </el-table-column>
      <el-table-column label="订印单位" min-width="160">
        <template #default="{ row }">
          <span class="o-cell">{{ extraMap.get(row.fileName)?.customer || row.productName || '（未填）' }}</span>
        </template>
      </el-table-column>
      <el-table-column label="产品名称" min-width="320">
        <template #default="{ row }">
          <span class="o-cell">{{ extraMap.get(row.fileName)?.productSpec || '(未填)' }}</span>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="200" fixed="right">
        <template #default="{ row }">
          <el-button size="small" type="primary" text @click="handleOpen(row)">打开</el-button>
          <el-button size="small" type="success" text @click="handleCopy(row)">复制</el-button>
          <el-button size="small" type="danger" text @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
      <template #empty>
        <el-empty :description="loading ? '加载中…' : '暂无工单'" :image-size="70" />
      </template>
    </el-table>

    <div class="list-foot">
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :page-sizes="pageSizes"
        :total="filteredTotal"
        layout="total, sizes, prev, pager, next, jumper"
        background
        small
      />
    </div>
  </div>
</template>

<style scoped>
.list-orders { display: flex; flex-direction: column; height: 100%; min-height: 0; }
.list-head {
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 0 12px; gap: 12px; flex-wrap: wrap;
}
.list-title { font-size: 16px; font-weight: 700; color: #1a237e; }
.list-filters { display: flex; align-items: center; gap: 8px; }
.search-input { width: 300px; }

.orders-table { flex: 1; min-height: 0; }
.orders-table :deep(.cell) { line-height: 1.5; }
.order-no-cell {
  font-weight: 700; color: #1a237e; cursor: pointer;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 160px;
}
.order-no-cell:hover { color: #0d47a1; text-decoration: underline; }
.o-cell { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; display: block; }
.fname { color: #90a4ae; }

.list-foot {
  display: flex; justify-content: flex-end; padding-top: 12px; flex-shrink: 0;
}
</style>
<script setup>
/**
 * 工单列表页：全宽 el-table，服务端分页（与侧栏同源 order_list）。
 *
 * - 按页请求 api.listOrders({ limit, offset })，避免几万条全量进内存
 * - 搜索：在当前已加载页的摘要 + 懒加载扩展字段上过滤（跨全库搜索需后续加后端 q）
 * - 复制 / 删除后 emit refresh，由父级刷新侧栏；本页自行再拉当前页
 */
import { ref, reactive, computed, watch } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Search } from '@element-plus/icons-vue';
import { api } from '../api';
import { displayOrderNo, cnDateToIso } from '../lib/order';

const props = defineProps({
  /** 父级刷新侧栏后递增/变化，触发本页重新拉取 */
  refreshToken: { type: [Number, String], default: 0 }
});
const emit = defineEmits(['open', 'refresh']);

const loading = ref(false);
const keyword = ref('');
const page = ref(1);
const pageSize = ref(10);
const pageSizes = [10, 20, 50, 100];
const pageItems = ref([]);
const total = ref(0);

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

const rows = computed(() => {
  const kw = keyword.value.trim().toLowerCase();
  if (!kw) return pageItems.value;
  return pageItems.value.filter((o) => matches(o, kw));
});

async function fetchPage() {
  loading.value = true;
  try {
    const offset = (page.value - 1) * pageSize.value;
    const res = await api.listOrders({ limit: pageSize.value, offset });
    pageItems.value = res?.items || [];
    total.value = res?.total ?? pageItems.value.length;
    pageItems.value.forEach((o) => loadExtra(o));
  } catch (e) {
    pageItems.value = [];
    total.value = 0;
    ElMessage.error('加载列表失败：' + e);
  } finally {
    loading.value = false;
  }
}

watch([page, pageSize], fetchPage, { immediate: true });
watch(() => props.refreshToken, () => { fetchPage(); });
watch(keyword, () => { /* 仅过滤当前页，不重置页码 */ });

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
    emit('refresh');
    await fetchPage();
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
    return;
  }
  try {
    await api.deleteOrder(row.fileName);
    ElMessage.success(`已删除 ${row.fileName}`);
    emit('refresh');
    await fetchPage();
  } catch (e) {
    ElMessage.error('删除失败：' + e);
  }
}

async function refresh() {
  emit('refresh');
  await fetchPage();
}
</script>

<template>
  <div class="list-orders">
    <div class="list-head">
      <div class="list-title">工单列表（{{ total }} 条）</div>
      <div class="list-filters">
        <el-input
          v-model="keyword"
          placeholder="搜索本页 单号 / 客户 / 合同号"
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
        :total="total"
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

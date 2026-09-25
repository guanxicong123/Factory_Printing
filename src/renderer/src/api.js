/**
 * Tauri 桥接：window.api 包装。
 * 真实 Tauri（window.__TAURI__.core.invoke 存在）用 invoke 调用 Rust。
 * 纯浏览器开发环境回退到本地 mock，便于 vite 浏览器调试。
 *
 * 说明：Tauri v2 配置了 withGlobalTauri: true，前端可直接用全局 window.__TAURI__，
 * 无需再 import @tauri-apps/api。
 */

/* ---------- invoke 选择器 ---------- */
function getInvoke() {
  try {
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
      return window.__TAURI__.core.invoke;
    }
  } catch (e) { /* ignore */ }
  return null;
}
const invoke = getInvoke();

/* ---------- 浏览器开发用 mock ---------- */
function mockApi() {
  let seq = 22382;
  const ts = () => new Date().toISOString().replace('T', ' ').slice(0, 19);
  const today = ((d) => d.getFullYear() + ' 年 ' + (d.getMonth() + 1) + ' 月 ' + d.getDate() + ' 日')(new Date());
  function makeOrder(no, extra = {}) {
    return {
      meta: { id: no, createdAt: ts(), updatedAt: ts(), fileName: no + '.json' },
      orderNo: no,
      fields: Object.assign({
        openDate: today, mJob_Date: today,
        deliverDate: '', mFinished_Date: '',
        customer: '', contractNo: '', productSpec: '',
        orderQty: '', unit: '个'
      }, extra),
      cks: {}
    };
  }
  // 内存存储：seed 几条示例，便于浏览器调试列表页
  const store = new Map([
    ['22380.json', makeOrder('22380', { customer: '安徽可新材料', contractNo: 'HT-2026-088', productSpec: '保暖毯(小)' })],
    ['22381.json', makeOrder('22381', { customer: '广州蓝盒子包装', contractNo: 'HT-2026-091', productSpec: '手提袋' })],
    ['22382.json', makeOrder('22382', { customer: '深圳凯旋印刷', contractNo: 'HT-2026-095', productSpec: '礼品盒' })]
  ]);
  function snapshot(o) { return JSON.parse(JSON.stringify(o)); }
  function freshNo() { seq += 1; return String(seq); }
  function newOrder() {
    const no = freshNo();
    return Promise.resolve(makeOrder(no, {
      customer: '示例客户', contractNo: 'HT-2026-0xx', productSpec: '待填产品规格',
      orderQty: '1200', unit: '个'
    }));
  }
  function listOrders() {
    const now = ts();
    const items = [];
    store.forEach((o, fileName) => {
      if (!store.has(fileName)) return; // 实际为遍历，防写入半途
      items.push({
        id: o.meta.id,
        fileName,
        orderNo: o.orderNo,
        productName: o.fields.customer || o.fields.productSpec || '',
        updatedAt: now
      });
    });
    items.sort((a, b) => (b.updatedAt || '').localeCompare(a.updatedAt || ''));
    return Promise.resolve(items);
  }
  function loadOrder(fileName) {
    const o = store.get(fileName);
    return o ? Promise.resolve(snapshot(o)) : Promise.resolve(null);
  }
  function saveOrder(order) {
    const fn = order.meta?.fileName || (order.orderNo + '.json');
    order.meta = order.meta || {};
    order.meta.fileName = fn;
    order.meta.updatedAt = ts();
    store.set(fn, snapshot(order));
    return Promise.resolve({ fileName: fn, order: snapshot(order) });
  }
  function duplicateOrder(order) {
    const no = freshNo();
    const copy = makeOrder(no, JSON.parse(JSON.stringify(order.fields || {})));
    copy.cks = JSON.parse(JSON.stringify(order.cks || {}));
    store.set(copy.meta.fileName, copy);
    return Promise.resolve(snapshot(copy));
  }
  function deleteOrder(fileName) {
    store.delete(fileName);
    return Promise.resolve(true);
  }
  return {
    newOrder,
    openOrder: () => Promise.resolve(null),
    saveOrder,
    saveOrderAs: () => Promise.resolve(null),
    printOrder: () => Promise.resolve({ ok: true }),
    listOrders,
    loadOrder,
    duplicateOrder,
    deleteOrder,
    // 浏览器环境无法弹系统文件对话框，返回空结果
    importSql: () => Promise.resolve({ count: 0, skipped: 0, message: '浏览器环境不支持导入' }),
    onMenu: () => () => {}
  };
}

export const api = invoke
  ? {
      newOrder: () => invoke('order_new'),
      openOrder: () => invoke('order_open'),
      saveOrder: (order) => invoke('order_save', { order }),
      saveOrderAs: (order) => invoke('order_save_as', { order }),
      printOrder: () => invoke('order_print'),
      listOrders: () => invoke('order_list'),
      loadOrder: (fileName) => invoke('order_load', { fileName }),
      duplicateOrder: (order) => invoke('order_duplicate', { order }),
      deleteOrder: (fileName) => invoke('order_delete', { fileName }),
      importSql: () => invoke('order_import_sql'),
      onMenu: (cb) => { window.__menuCb = cb; return () => { window.__menuCb = null; }; }
    }
  : mockApi();
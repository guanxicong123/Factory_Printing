/**
 * 工单数据模型与工具函数。
 * 从旧版 renderer.js 迁移：字段读取、日期转换、默认工单、工单号处理。
 */

/* ================= 工单号 ================= */
export function displayOrderNo(raw) {
  if (!raw) return '';
  if (/^No\.\s?/i.test(raw)) return raw;
  return 'No. ' + raw;
}
export function stripOrderNo(disp) {
  return String(disp || '').replace(/^No\.\s?/i, '');
}

/* ================= 空工单 ================= */
export function defaultOrder() {
  return {
    meta: { id: '', createdAt: '', updatedAt: '', fileName: '' },
    orderNo: '',        // 纯数字
    fields: {},         // 任意 data-field 键值（SQL 对齐名 + 工艺细节）
    cks: {}             // data-ck 勾选框状态 true/false
  };
}

/* ================= 数据读取：带别名回退 ================= */
export function getField(f, key) {
  if (key instanceof Array) {
    for (const k of key) {
      if (f[k] !== undefined && f[k] !== '') return f[k];
    }
    return '';
  }
  return f[key] !== undefined ? f[key] : '';
}

export function normalizeOrder(order) {
  if (!order) return defaultOrder();
  return {
    meta: order.meta || {},
    orderNo: order.orderNo || '',
    fields: order.fields || {},
    cks: order.cks || {}
  };
}

/* ================= 日期转换 ================= */
function pad2(n) { return String(n).padStart(2, '0'); }
export function pad2Date(n) { return pad2(n); }

/* 工单中文日期 -> ISO */
export function cnDateToIso(s) {
  s = String(s || '').trim();
  if (!s) return '';
  const isoMatch = s.match(/^(\d{4})[-/](\d{1,2})[-/](\d{1,2})$/);
  if (isoMatch) {
    const [, y, m, d] = isoMatch;
    return y + '-' + pad2(m) + '-' + pad2(d);
  }
  const cnMatch = s.match(/(\d{4})\s*年\s*(\d{1,2})\s*月\s*(\d{1,2})\s*日/);
  if (cnMatch) {
    const [, y, m, d] = cnMatch;
    return y + '-' + pad2(m) + '-' + pad2(d);
  }
  return '';
}

/* ISO -> 工单中文日期 */
export function isoToCnDate(iso) {
  iso = String(iso || '').trim();
  if (!iso) return '';
  const m = iso.match(/^(\d{4})-(\d{1,2})-(\d{1,2})$/);
  if (!m) return '';
  const y = m[1], mo = parseInt(m[2], 10), d = parseInt(m[3], 10);
  if (mo < 1 || mo > 12 || d < 1 || d > 31) return '';
  return y + ' 年 ' + mo + ' 月 ' + d + ' 日';
}

/* 今天中文日期 */
export function todayCn() {
  const d = new Date();
  return d.getFullYear() + ' 年 ' + (d.getMonth() + 1) + ' 月 ' + d.getDate() + ' 日';
}

/* ================= 新建工单默认日期 ================= */
export function ensureNewOrderDates(order) {
  const f = order.fields || {};
  const hasOpen = f.openDate || f.mJob_Date;
  const hasDeliver = f.deliverDate || f.mFinished_Date;
  if (!hasOpen) {
    const today = todayCn();
    f.openDate = today;
    f.mJob_Date = today;
  }
  if (!hasDeliver) {
    f.deliverDate = '';
    f.mFinished_Date = '';
  }
}

/* ================= 字段收集（从 DOM data-field/data-ck） ================= */
/**
 * 收集编辑区 DOM -> fields/cks。
 * @param {HTMLElement} scope 编辑区根节点
 * @param {object} order 目标 order（就地修改）
 */
export function collectFields(scope, order) {
  if (!order || !scope) return;
  const f = order.fields;
  const seen = new Set();
  scope.querySelectorAll('[data-field]').forEach((el) => {
    const field = el.dataset.field;
    if (!field || seen.has(field)) return;
    seen.add(field);
    const elType = el.tagName;
    let value;
    if (elType === 'INPUT' || elType === 'TEXTAREA' || elType === 'SELECT') {
      value = el.value;
    } else {
      value = el.innerText || '';
    }
    if (el.dataset.date) value = isoToCnDate(value);
    f[field] = value;
    if (el.dataset.sync) {
      el.dataset.sync.split(',').forEach((alias) => {
        f[alias.trim()] = value;
      });
    }
  });
  scope.querySelectorAll('[data-ck]').forEach((el) => {
    const ck = el.dataset.ck;
    if (ck) order.cks[ck] = el.classList.contains('on');
  });
  // orderNo
  const raw = f.mJob_No || f.orderNoDisplay || order.orderNo || '';
  order.orderNo = stripOrderNo(raw);
}
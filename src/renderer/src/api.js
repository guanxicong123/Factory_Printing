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
  let seq = 22380;
  const today = (() => {
    const d = new Date();
    return d.getFullYear() + ' 年 ' + (d.getMonth() + 1) + ' 月 ' + d.getDate() + ' 日';
  })();
  function newOrder() {
    seq += 1;
    const no = String(seq);
    // 开发样例数据：便于预览/打印看到效果
    return Promise.resolve({
      meta: { id: no, createdAt: today, updatedAt: today, fileName: no + '.json' },
      orderNo: no,
      fields: {
        openDate: '2026 年 9 月 18 日', mJob_Date: '2026 年 9 月 18 日',
        deliverDate: '2026 年 9 月 28 日', mFinished_Date: '2026 年 9 月 28 日',
        customer: '安徽可新材料', contractNo: 'HT-2026-088',
        productSpec: '保暖毯(小)（面：4+0；底：1+0对裱纸；内贴1个小袋）展开62X23；（300克白卡+300克白卡+内小袋300克白卡（成品11X13展开14X14.5））',
        orderQty: '1200', unit: '个', numFrom: '0001', numLian: '2', numYeBen: '50',
        xinJian: '6', jiuJian: '2', gongJian: '3', yiZhaoJian: '4',
        dkXin: '5', dkJiu: '6', dkGong: '7', dkZhao: '8',
        pinbanH: '1', pinbanS: '2', remark: '封面发城东出CTP1套',
        paperType: '金蝶兰300克白卡', paperCount: '720',
        sz47_5: '47.5', sz64_5: '64.5', sz47_3: '47.3', sz64_3: '64.3',
        paperNote: '不标准尺寸开纸图样\\n内小袋贴260克白卡',
        print_paper1: '面：300克白卡纸', print_color1: '4+0', print_qty1: '600+60', print_extra1: '80',
        print_paper2: '底：300克白卡纸', print_color2: '黑1+0', print_qty2: '600+60', print_extra2: '50',
        print_note: '附彩打样1张\\n注意：印底时要用粗糙那边印刷',
        houGongxuNote: '啤毛边+旧版优先',
        zTeshushuoming: '面300克白卡底300克白卡对裱纸；内贴1个小袋', zZhangCount: '1200',
        fkHeng: '25', fkShu: '18', sbTou: '1', sbJiao: '1', sbZuo: '1', sbYou: '1',
        fkSpecial: '包装要求：不用折，100个/大包',
        signedBy: '张工', business: '李销售', proofread: '王校对'
      },
      cks: {
        duikai: true, gPi: true, gTie: true, gBiaoZhi: true, gZhanKeng: true,
        zJinSong: true, bxYouChangMing: true, bxZhiBao: true,
        sz109a: true, sz089a: true, sz079a: true, sz109b: true, sz089b: true, sz079b: true
      }
    });
  }
  function saveOrder(order) {
    const fn = order.meta?.fileName || (order.orderNo + '.json');
    order.meta = order.meta || {};
    order.meta.fileName = fn;
    order.meta.updatedAt = today;
    return Promise.resolve({ fileName: fn, order: JSON.parse(JSON.stringify(order)) });
  }
  return {
    newOrder,
    openOrder: () => Promise.resolve(null),
    saveOrder,
    saveOrderAs: () => Promise.resolve(null),
    printOrder: () => Promise.resolve({ ok: true }),
    listOrders: () => Promise.resolve([]),
    loadOrder: () => Promise.resolve(null),
    duplicateOrder: (o) => newOrder().then((fresh) => ({
      ...fresh,
      fields: JSON.parse(JSON.stringify(o.fields || {})),
      cks: JSON.parse(JSON.stringify(o.cks || {}))
    })),
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
      onMenu: (cb) => { window.__menuCb = cb; return () => { window.__menuCb = null; }; }
    }
  : mockApi();
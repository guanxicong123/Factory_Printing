/**
 * 工单预览模板：生成 工单0022379.html 版式的只读 HTML。
 * 从旧版迁移，字段名/勾选名与编辑表单完全对应。
 */

import { getField, displayOrderNo } from './order.js';

function escapeHtml(s) {
  return String(s == null ? '' : s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

export function buildWorkOrderDoc(order) {
  const f = order.fields || {};
  const cks = order.cks || {};

  const t = (...keys) => getField(f, keys.length === 1 ? keys[0] : keys);
  const ck = (name) => (cks[name] ? 'on' : '');

  // 后工序工艺勾选展示：勾选 -> 大对勾；未勾选 -> 空
  function procMark(name) {
    return cks[name]
      ? '<span class="proc-mark">✓</span>'
      : '<span class="proc-mark"></span>';
  }

  function resolveKey(key) {
    if (typeof key === 'string') return { read: [key] };
    return { read: key };
  }
  function inp(key) {
    const r = resolveKey(key);
    const value = getField(f, r.read);
    return '<span class="pv-txt">' + escapeHtml(value) + '</span>';
  }
  function txtarea(key) {
    const r = resolveKey(key);
    const value = getField(f, r.read);
    return '<span class="pv-txt multiline">' + escapeHtml(value) + '</span>';
  }

  const procRows = [1, 2, 3, 4, 5].map((n) => `
    <tr>
      <td>${inp('print_paper' + n)}</td>
      <td>${inp('print_color' + n)}</td>
      <td>${inp('print_qty' + n)}</td>
      <td>${inp('print_extra' + n)}</td>
      ${n === 1 ? `<td class="left" style="vertical-align:top; padding:4px 8px;" rowspan="5">${txtarea('print_note')}</td>` : ''}
    </tr>`).join('');

  return `<div class="form-doc">
    <div class="form-body">

      <div class="title-bar"><span class="title">印 刷 印 件 工 单</span></div>

      <div class="top-info">
        <div style="width:40%;"><b>开单日期： ${inp(['openDate', 'mJob_Date'])}</b></div>
        <div style="width:42%;"><b>交货日期： ${inp(['deliverDate', 'mFinished_Date'])}</b></div>
        <div class="right" style="width:18%;"><b style="font-size:17px;">${displayOrderNo(order.orderNo)}</b></div>
      </div>

      <table class="bordered gap-top">
        <tr>
          <td class="label" style="width:12%;">订印单位</td>
          <td class="left" style="width:60%;">${inp(['customer', 'mCustomer_FullName'])}</td>
          <td class="label" style="width:8%;">合同号</td>
          <td style="width:16%;">${inp(['contractNo', 'mSales_Confirmation_No'])}</td>
        </tr>
      </table>

      <table class="bordered gap-top">
        <colgroup>
          <col style="width:12%;"><col style="width:15%;"><col style="width:15%;"><col style="width:15%;"><col style="width:15%;"><col style="width:8%;"><col style="width:5.33%;"><col style="width:5.33%;"><col style="width:5.34%;">
        </colgroup>
        <tr>
          <td class="label">产品名称</td>
          <td class="left" colspan="4" style="padding:60px 6px;">${txtarea(['productSpec', 'mProduct_Name'])}</td>
          <td rowspan="2">订印<br>数量</td>
          <td colspan="3" style="text-align:right; padding-right:8px;">${inp(['orderQty', 'mQty_Job'])} ${inp(['unit', 'mUnit'])}个</td>
        </tr>
        <tr>
          <td>号码</td>
          <td colspan="3" style="text-align:left; padding-left:8px;">由 ${inp('numFrom')}</td>
          <td style="text-align:right; padding-right:8px;">${inp('numLian')} 联</td>
          <td colspan="3" style="text-align:right; padding-right:8px;">${inp('numYeBen')} 页/本</td>
        </tr>
      </table>

      <table class="bordered gap-top">
        <tr style="height:auto;">
          <td style="width:54%; padding:6px; white-space:nowrap; overflow:visible;">
            <span class="ck ${ck('liukai')}" data-ck="liukai"></span>&nbsp;六开&nbsp;新&nbsp;${inp('xinJian')}&nbsp;件&nbsp;旧&nbsp;${inp('jiuJian')}&nbsp;件&nbsp;共&nbsp;${inp('gongJian')}&nbsp;件&nbsp;已找&nbsp;${inp('yiZhaoJian')}&nbsp;件
          </td>
          <td class="label" style="width:5%;" rowspan="2">拼版<br>数量</td>
          <td style="width:10%;">横&nbsp;${inp('pinbanH')}&nbsp;个</td>
          <td class="left" style="width:31%; padding:6px 8px; vertical-align:top;" rowspan="2">备注：${txtarea(['remark', 'mRemarks'])}</td>
        </tr>
        <tr style="height:auto;">
          <td style="width:54%; padding:6px; white-space:nowrap; overflow:visible;">
            <span class="ck ${ck('duikai')}" data-ck="duikai"></span>&nbsp;对开&nbsp;新&nbsp;${inp('dkXin')}&nbsp;件&nbsp;旧&nbsp;${inp('dkJiu')}&nbsp;件&nbsp;共&nbsp;${inp('dkGong')}&nbsp;件&nbsp;已找&nbsp;${inp('dkZhao')}&nbsp;件
          </td>
          <td style="width:10%;">竖&nbsp;${inp('pinbanS')}&nbsp;个</td>
        </tr>
      </table>

      <table class="bordered gap-top paper-tbl">
        <tr>
          <td class="label" style="width:30%;">纸类</td>
          <td class="label" style="width:10%;">发纸数(张)</td>
          <td rowspan="12" class="label" style="width:7%; writing-mode:vertical-rl; text-orientation:upright; padding:8px 2px;">开 纸</td>
          <td class="size-merge" rowspan="6" style="width:9%;">
            1.19 <span class="ck ${ck('sz119a')}" data-ck="sz119a"></span><br>
            1.09 <span class="ck ${ck('sz109a')}" data-ck="sz109a"></span>
          </td>
          <td class="size-merge" rowspan="6" style="width:9%;">
            0.89 <span class="ck ${ck('sz089a')}" data-ck="sz089a"></span><br>
            0.79 <span class="ck ${ck('sz079a')}" data-ck="sz079a"></span>
          </td>
          <td class="left" style="width:35%; padding:4px 8px; vertical-align:top;" rowspan="12">${txtarea('paperNote')}</td>
        </tr>
        <tr>
          <td>${inp('paperType')}</td><td>${inp('paperCount')}</td>
        </tr>
        <tr>
          <td>${inp('paperType2')}</td><td>${inp('paperCount2')}</td>
        </tr>
        <tr>
          <td>${inp('paperType3')}</td><td>${inp('paperCount3')}</td>
        </tr>
        <tr>
          <td>${inp('paperType4')}</td><td>${inp('paperCount4')}</td>
        </tr>
        <tr>
          <td>${inp('paperType5')}</td><td>${inp('paperCount5')}</td>
        </tr>
        <tr>
          <td rowspan="2" class="size-cell-left">${t('paperMian')}</td><td rowspan="2"></td>
          <td class="size-cell">${inp('sz47_5')}</td><td class="size-cell">${inp('sz64_5')}</td>
        </tr>
        <tr>
          <td class="size-cell">开数</td><td class="size-cell">${inp('kaifangMian')}</td>
        </tr>
        <tr>
          <td></td><td></td>
          <td class="size-merge" rowspan="2">
            1.19 <span class="ck ${ck('sz119b')}" data-ck="sz119b"></span><br>
            1.09 <span class="ck ${ck('sz109b')}" data-ck="sz109b"></span>
          </td>
          <td class="size-merge" rowspan="2">
            0.89 <span class="ck ${ck('sz089b')}" data-ck="sz089b"></span><br>
            0.79 <span class="ck ${ck('sz079b')}" data-ck="sz079b"></span>
          </td>
        </tr>
        <tr>
          <td rowspan="2" class="size-cell-left">${t('paperDi')}</td><td rowspan="2"></td>
        </tr>
        <tr>
          <td class="size-cell">${inp('sz47_3')}</td><td class="size-cell">${inp('sz64_3')}</td>
        </tr>
        <tr>
          <td></td><td></td>
          <td class="size-cell">开数</td><td class="size-cell">${inp('kaifangDi')}</td>
        </tr>
      </table>

      <table class="bordered gap-top">
        <tr>
          <td class="label" style="width:4%;" rowspan="6">机<br>印<br>说<br>明</td>
          <td class="label" style="width:27%;">纸 别</td>
          <td class="label" style="width:10%;">印 色</td>
          <td class="label" style="width:10%;">实印数(张)</td>
          <td class="label" style="width:10%;">放数(张)</td>
          <td class="label" style="width:35%;">备 注</td>
        </tr>
        ${procRows}
      </table>

      <table class="bordered gap-top">
        <tr>
          <td class="label" style="width:4%;" rowspan="3">后<br>工<br>序</td>
          <td style="padding:0;">
            <table class="inner" style="width:100%; table-layout:auto; min-width:600px;">
              <tr style="font-size:11px;">
                <td style="min-width:36px;">光胶</td><td style="min-width:36px;">哑胶</td><td>磨光</td><td>吸塑油</td><td>烫金</td><td>烫银</td><td>过油</td><td>UV</td><td>凹凸</td><td>压纹</td><td>啤</td><td>贴</td><td>裱纸</td><td>粘坑</td><td>打孔</td><td>鸡眼</td><td>压线</td><td style="min-width:36px;">啤皮筋</td><td>贴PVC片</td><td>其它</td>
              </tr>
              <tr style="font-size:11px;">
                <td style="min-width:36px;">${procMark('gGuangJiaoDan')}单面<br>${procMark('gGuangJiaoShuang')}双面</td>
                <td style="min-width:36px;">${procMark('gYaJiaoDan')}单面<br>${procMark('gYaJiaoShuang')}双面</td>
                <td>${procMark('gMoGuang')}</td>
                <td>${procMark('gXiSuYou')}</td>
                <td>${procMark('gTangJin')}</td>
                <td>${procMark('gTangYin')}</td>
                <td>${procMark('gGuoYou')}</td>
                <td>${procMark('gUV')}</td>
                <td>${procMark('gAoTu')}</td>
                <td>${procMark('gYaWen')}</td>
                <td>${procMark('gPi')}</td>
                <td>${procMark('gTie')}</td>
                <td>${procMark('gBiaoZhi')}</td>
                <td>${procMark('gZhanKeng')}</td>
                <td>${procMark('gDaKong')}</td>
                <td>${procMark('gJiYan')}</td>
                <td>${procMark('gYaXian')}</td>
                <td style="min-width:36px;">${procMark('gPiPiJin')}</td>
                <td>${procMark('gTiePVC')}</td>
                <td>${procMark('gQiTa')}</td>
              </tr>
              ${(cks.gQiTa && t('gQiTaText')) ? `<tr>
                <td colspan="19"></td>
                <td class="left" style="padding:2px 4px; font-size:11px;">${escapeHtml(t('gQiTaText'))}</td>
              </tr>` : ''}
            </table>
          </td>
        </tr>
        <tr>
          <td style="padding:0;">
            <table class="inner" style="width:100%;">
              <tr><td style="width:8%;">啤</td><td style="width:10%;">${t('piVersion') === 'old' ? '旧版' : (t('piVersion') === 'new' ? '新版' : '')}</td><td style="width:12%;">特别说明：</td><td style="width:70%;">${inp('houGongxuNote')}</td></tr>
            </table>
          </td>
        </tr>
        <tr>
          <td style="padding:0;">
            <table class="inner" style="width:100%;">
              <tr>
                <td class="label" style="width:6%;" rowspan="2"><div style="writing-mode:vertical-rl; text-orientation:upright;">装 订</div></td>
                <td class="left zhuangding" style="width:22%; padding:6px;" rowspan="2">
                  <span class="ck ${ck('zSanZhang')}" data-ck="zSanZhang"></span>散张&nbsp;
                  <span class="ck ${ck('zQiDing')}" data-ck="zQiDing"></span>骑钉<br>
                  <span class="ck ${ck('zSuoXian')}" data-ck="zSuoXian"></span>锁线&nbsp;
                  <span class="ck ${ck('zJiaoZhuang')}" data-ck="zJiaoZhuang"></span>胶装<br>
                  其它 <span class="pv-txt underline">${t('zQiTaText')}</span>
                </td>
                <td style="width:12%; padding:0px 6px; text-align:right; padding-right:12px;">张</td>
                <td style="width:12%; padding:0px 6px; text-align:center; vertical-align:top;">本</td>
                <td style="width:14%; padding:0px 6px; text-align:center; vertical-align:top;" rowspan="2">每本<br>
                  <span style="display:block; text-align:right; padding-top:4px;">${inp('zMeiBenFen')} 份</span>
                </td>
                <td class="left" style="width:23%; padding:6px; vertical-align:top;" rowspan="2">特殊说明：<br>${txtarea('zTeshushuoming')}</td>
                <td style="width:13%; padding:6px;" rowspan="2">检查点数<br>
                  <span class="ck ${ck('zJinSong')}" data-ck="zJinSong"></span>尽 送<br>
                  <span class="ck ${ck('zShiSong')}" data-ck="zShiSong"></span>实 送
                </td>
              </tr>
              <tr>
                <td style="text-align:right; padding-right:12px;">${inp('zZhangCount')}个</td>
                <td style="text-align:right; padding-right:12px;">${inp('zBenCount')}本</td>
              </tr>
            </table>
          </td>
        </tr>
      </table>

      <table class="bordered gap-top">
        <tr>
          <td class="label" style="width:4%;" rowspan="4"><div style="writing-mode:vertical-rl; text-orientation:upright;">成 品 规 格</div></td>
          <td style="width:24%;" colspan="5" class="spec-cell">
              <div class="spec-wrap">
                <span class="spec-group"><span class="spec-lbl">横</span><span class="pv-txt spec-val">${escapeHtml(t('fkHeng'))}</span><span class="spec-unit">cm</span></span>
                <span class="spec-group"><span class="spec-lbl">竖</span><span class="pv-txt spec-val">${escapeHtml(t('fkShu'))}</span><span class="spec-unit">cm</span></span>
              </div>
            </td>
          <td class="left" style="width:28%; padding:6px 8px; vertical-align:top;" rowspan="4">特殊说明：<br>${txtarea('fkSpecial')}</td>
          <td class="label" style="width:8%;" rowspan="4"><div style="writing-mode:vertical-rl; text-orientation:upright;">包 装</div></td>
          <td style="width:16%;" rowspan="2">合格证</td>
          <td class="left" style="width:16%; padding:4px 6px;"><span class="ck ${ck('bxYouChangMing')}" data-ck="bxYouChangMing"></span>有厂名</td>
        </tr>
        <tr>
          <td class="label" rowspan="3">四边留位</td>
          <td>头</td><td>脚</td><td>左</td><td>右</td>
          <td class="left" style="padding:4px 6px;"><span class="ck ${ck('bxWuChangMing')}" data-ck="bxWuChangMing"></span>无厂名</td>
        </tr>
        <tr>
          <td>${inp('sbTou')}</td><td>${inp('sbJiao')}</td><td>${inp('sbZuo')}</td><td>${inp('sbYou')}</td>
          <td class="left" style="padding:4px 6px;"><span class="ck ${ck('bxZhiBao')}" data-ck="bxZhiBao"></span>纸包</td>
          <td></td>
        </tr>
        <tr>
          <td></td><td></td><td></td><td></td>
          <td class="left" style="padding:4px 6px;"><span class="ck ${ck('bxZhiXiang')}" data-ck="bxZhiXiang"></span>纸箱</td>
          <td></td>
        </tr>
      </table>

      <div class="bottom-sign">
        <div style="width:35%;">开单人：${inp(['signedBy', 'mOperator'])}</div>
        <div style="width:32%;">业务：${inp('business')}</div>
        <div style="width:33%;">印刷前对稿：${inp('proofread')}</div>
      </div>

    </div>
    <div class="side-tag">
      <div>一 版房/存根 白</div>
      <div>二 印刷 红</div>
      <div>三 切纸 黄</div>
    </div>
  </div>`;
}
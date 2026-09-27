<script setup>
/**
 * 编辑表单：一屏高密度网格（对齐旧版区块排布，Element Plus 控件）。
 * 数据直接挂在 props.order 上（fields 中文日期 / cks 勾选），编辑即生效。
 */

import { reactive } from 'vue';
import { getField, cnDateToIso, isoToCnDate } from '../lib/order.js';

const props = defineProps({
  order: { type: Object, required: true }
});

if (!props.order.fields) props.order.fields = {};
if (!props.order.cks) props.order.cks = {};
const f = reactive(props.order.fields);
const cks = reactive(props.order.cks);

if (!props.order.fields.piVersion) {
  props.order.fields.piVersion = 'old';
}

function t(keys) {
  return getField(props.order.fields, Array.isArray(keys) ? keys : [keys]);
}

function dateIso(keys) {
  const cn = t(keys);
  return cn ? cnDateToIso(cn) : '';
}
function dateChanged(keys, iso) {
  const cn = iso ? isoToCnDate(iso) : '';
  if (!props.order.fields) props.order.fields = {};
  const arr = Array.isArray(keys) ? keys : [keys];
  arr.forEach((k) => { props.order.fields[k] = cn; });
}

const procRows = [1, 2, 3, 4, 5].map((n) => ({
  paper: `print_paper${n}`,
  color: `print_color${n}`,
  qty: `print_qty${n}`,
  extra: `print_extra${n}`
}));

const postCks = [
  ['gGuangJiaoDan', '光胶单'], ['gGuangJiaoShuang', '光胶双'],
  ['gYaJiaoDan', '哑胶单'], ['gYaJiaoShuang', '哑胶双'],
  ['gMoGuang', '磨光'], ['gXiSuYou', '吸塑油'],
  ['gTangJin', '烫金'], ['gTangYin', '烫银'], ['gGuoYou', '过油'],
  ['gUV', 'UV'], ['gAoTu', '凹凸'], ['gYaWen', '压纹'],
  ['gPi', '啤'], ['gTie', '贴'], ['gBiaoZhi', '裱纸'], ['gZhanKeng', '粘坑'],
  ['gDaKong', '打孔'], ['gJiYan', '鸡眼'], ['gYaXian', '压线'],
  ['gPiPiJin', '啤皮筋'], ['gTiePVC', '贴PVC片'], ['gQiTa', '其它']
];

const sizeCksA = [
  ['sz119a', '1.19'], ['sz109a', '1.09'], ['sz089a', '0.89'], ['sz079a', '0.79']
];
const sizeCksB = [
  ['sz119b', '1.19'], ['sz109b', '1.09'], ['sz089b', '0.89'], ['sz079b', '0.79']
];

const bindCks = [
  ['zSanZhang', '散张'], ['zQiDing', '骑钉'], ['zSuoXian', '锁线'],
  ['zJiaoZhuang', '胶装'], ['zQiTa', '其它']
];
function radioPair(ckA, ckB) {
  return {
    get: () => {
      const c = props.order.cks || {};
      if (c[ckA]) return ckA;
      if (c[ckB]) return ckB;
      return '';
    },
    set: (v) => {
      if (!props.order.cks) props.order.cks = {};
      props.order.cks[ckA] = v === ckA;
      props.order.cks[ckB] = v === ckB;
    }
  };
}
const songMode = radioPair('zJinSong', 'zShiSong');
const certMode = radioPair('bxYouChangMing', 'bxWuChangMing');
const packMode = radioPair('bxZhiBao', 'bxZhiXiang');

const postCksRest = postCks.filter(
  ([k]) => !['gGuangJiaoDan', 'gGuangJiaoShuang', 'gYaJiaoDan', 'gYaJiaoShuang'].includes(k)
);
</script>

<template>
  <div class="edit-form">
    <!-- 顶栏 -->
    <section class="panel panel-top">
      <div class="top-row1">
        <div class="fld-inline">
          <span class="lbl">No.</span>
          <span class="no-val">{{ order.orderNo }}</span>
        </div>
        <div class="fld-inline">
          <span class="lbl">开单日期</span>
          <el-date-picker
            size="small"
            class="date-inp"
            :model-value="dateIso(['openDate','mJob_Date'])"
            type="date" placeholder="日期" value-format="YYYY-MM-DD"
            @update:modelValue="(v) => dateChanged(['openDate','mJob_Date'], v)"
          />
        </div>
        <div class="fld-inline">
          <span class="lbl">交货日期</span>
          <el-date-picker
            size="small"
            class="date-inp"
            :model-value="dateIso(['deliverDate','mFinished_Date'])"
            type="date" placeholder="日期" value-format="YYYY-MM-DD"
            @update:modelValue="(v) => dateChanged(['deliverDate','mFinished_Date'], v)"
          />
        </div>
      </div>
      <div class="top-row2">
        <div class="fld-inline grow">
          <span class="lbl">订印单位</span>
          <el-input size="small" v-model="f.customer" placeholder="订印单位" @update:modelValue="(v)=>{f.mCustomer_FullName=v;}" />
        </div>
        <div class="fld-inline w-cost">
          <span class="lbl">成本单价</span>
          <el-input size="small" v-model="f.costUnitPrice" />
        </div>
        <div class="fld-inline w-contract">
          <span class="lbl">合同号</span>
          <el-input size="small" v-model="f.contractNo" />
        </div>
        <div class="fld-inline w-qty">
          <span class="lbl">订印数量</span>
          <el-input size="small" v-model="f.orderQty" />
        </div>
        <div class="fld-inline w-unit">
          <span class="lbl">单位</span>
          <el-input size="small" v-model="f.unit" />
        </div>
        <div class="fld-inline w-num">
          <span class="lbl">号码由</span>
          <el-input size="small" v-model="f.numFrom" />
        </div>
        <div class="fld-inline w-num-sm">
          <span class="lbl">联</span>
          <el-input size="small" v-model="f.numLian" />
        </div>
        <div class="fld-inline w-num-sm">
          <span class="lbl">页/本</span>
          <el-input size="small" v-model="f.numYeBen" />
        </div>
      </div>
      <div class="top-product">
        <span class="lbl">产品名称/规格</span>
        <el-input
          type="textarea"
          :rows="3"
          resize="none"
          v-model="f.productSpec"
          placeholder="产品名称、规格、工艺说明"
          @update:modelValue="(v)=>{f.mProduct_Name=v;}"
        />
      </div>
    </section>

    <!-- 中上：拼版 -->
    <section class="panel panel-sub">
      <div class="sub-left">
        <div class="sub-line">
          <el-checkbox size="small" v-model="cks.liukai" label="六开" />
        </div>
        <div class="sub-line">
          <el-checkbox size="small" v-model="cks.sikai" label="四开" />
        </div>
        <div class="sub-line">
          <el-checkbox size="small" v-model="cks.duikai" label="对开" />
        </div>
      </div>
      <div class="sub-pinban">
        <div class="fld-inline">
          <span class="lbl">拼版横</span>
          <el-input size="small" v-model="f.pinbanH" />
        </div>
        <div class="fld-inline">
          <span class="lbl">拼版竖</span>
          <el-input size="small" v-model="f.pinbanS" />
        </div>
      </div>
      <div class="sub-remark">
        <span class="lbl">备注</span>
        <el-input
          type="textarea"
          :rows="2"
          resize="none"
          v-model="f.remark"
          placeholder="开纸/拼版备注"
          @update:modelValue="(v)=>{f.mRemarks=v;}"
        />
      </div>
    </section>

    <!-- 中部：纸类 ‖ 机印；备注各占整行 -->
    <section class="panel panel-mid">
      <div class="mid-tables">
        <div class="mid-paper">
          <div class="sec-head">纸类</div>
          <table class="dense-table">
            <thead>
              <tr><th>纸类</th><th class="col-qty">发纸数</th></tr>
            </thead>
            <tbody>
              <tr v-for="n in 5" :key="n">
                <td>
                  <el-input
                    size="small"
                    :model-value="n===1 ? f.paperType : f['paperType'+n]"
                    @update:model-value="v => (n===1 ? (f.paperType=v) : (f['paperType'+n]=v))"
                  />
                </td>
                <td>
                  <el-input
                    size="small"
                    :model-value="n===1 ? f.paperCount : f['paperCount'+n]"
                    @update:model-value="v => (n===1 ? (f.paperCount=v) : (f['paperCount'+n]=v))"
                  />
                </td>
              </tr>
            </tbody>
          </table>
          <div class="size-block">
            <div class="fld-inline wrap">
              <span class="lbl">开纸面</span>
              <el-checkbox v-for="([k, lbl]) in sizeCksA" :key="k" size="small" :label="lbl" v-model="cks[k]" />
            </div>
            <div class="fld-inline wrap">
              <span class="lbl">开纸底</span>
              <el-checkbox v-for="([k, lbl]) in sizeCksB" :key="k" size="small" :label="lbl" v-model="cks[k]" />
            </div>
            <div class="size-dims">
              <div class="fld-inline">
                <span class="lbl">面</span>
                <el-input size="small" class="dim" v-model="f.sz47_5" />
                <span>×</span>
                <el-input size="small" class="dim" v-model="f.sz64_5" />
              </div>
              <div class="fld-inline">
                <span class="lbl">底</span>
                <el-input size="small" class="dim" v-model="f.sz47_3" />
                <span>×</span>
                <el-input size="small" class="dim" v-model="f.sz64_3" />
              </div>
              <div class="fld-inline">
                <span class="lbl">开数面</span>
                <el-input size="small" class="dim-w" v-model="f.kaifangMian" />
              </div>
              <div class="fld-inline">
                <span class="lbl">开数底</span>
                <el-input size="small" class="dim-w" v-model="f.kaifangDi" />
              </div>
            </div>
          </div>
        </div>

        <div class="mid-print">
          <div class="sec-head">机印说明</div>
          <table class="dense-table proc-table">
            <thead>
              <tr>
                <th class="col-paper">纸别</th>
                <th class="col-color">印色</th>
                <th class="col-pqty">实印数</th>
                <th class="col-extra">放数</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(r, i) in procRows" :key="i">
                <td><el-input size="small" v-model="f[r.paper]" /></td>
                <td><el-input size="small" v-model="f[r.color]" /></td>
                <td><el-input size="small" v-model="f[r.qty]" /></td>
                <td><el-input size="small" v-model="f[r.extra]" /></td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div class="note-row">
        <span class="lbl">纸类备注</span>
        <el-input type="textarea" :rows="2" resize="none" v-model="f.paperNote" placeholder="纸类/发纸备注" />
      </div>
      <div class="note-row">
        <span class="lbl">机印备注</span>
        <el-input type="textarea" :rows="2" resize="none" v-model="f.print_note" placeholder="机印备注" />
      </div>
    </section>

    <!-- 中下：后工序 / 装订 -->
    <section class="panel panel-post">
      <div class="post-row1">
        <div class="proc-item">
          <span class="proc-name">光胶</span>
          <el-checkbox size="small" v-model="cks.gGuangJiaoDan" label="单面" />
          <el-checkbox size="small" v-model="cks.gGuangJiaoShuang" label="双面" />
        </div>
        <div class="proc-item">
          <span class="proc-name">哑胶</span>
          <el-checkbox size="small" v-model="cks.gYaJiaoDan" label="单面" />
          <el-checkbox size="small" v-model="cks.gYaJiaoShuang" label="双面" />
        </div>
        <div class="ck-cluster">
          <el-checkbox
            v-for="([k, lbl]) in postCksRest"
            :key="k" size="small" :label="lbl" v-model="cks[k]"
          />
        </div>
      </div>
      <div class="post-row2">
        <div class="fld-inline">
          <span class="lbl">啤</span>
          <el-select size="small" v-model="f.piVersion" placeholder="旧版" style="width:90px">
            <el-option label="新版" value="new" />
            <el-option label="旧版" value="old" />
          </el-select>
        </div>
        <div class="fld-inline grow">
          <span class="lbl">后工序说明</span>
          <el-input size="small" v-model="f.houGongxuNote" />
        </div>
        <div v-if="cks.gQiTa" class="fld-inline grow">
          <span class="lbl">其它内容</span>
          <el-input size="small" v-model="f.gQiTaText" />
        </div>
      </div>
      <div class="post-row3">
        <div class="fld-inline wrap">
          <span class="lbl">装订</span>
          <el-checkbox
            v-for="([k, lbl]) in bindCks.filter(([k]) => k !== 'zQiTa')"
            :key="k" size="small" :label="lbl" v-model="cks[k]"
          />
          <el-input size="small" v-model="f.zQiTaText" placeholder="其它" class="bind-other" />
        </div>
        <div class="bind-qty">
          <el-input size="small" v-model="f.zBenCount" class="qty-inp">
            <template #prepend>本</template>
          </el-input>
          <span class="op">×</span>
          <el-input size="small" v-model="f.zMeiBenFen" class="qty-inp">
            <template #prepend>每本</template>
          </el-input>
          <span class="op">=</span>
          <el-input size="small" v-model="f.zZhangCount" class="qty-inp">
            <template #prepend>数</template>
          </el-input>
        </div>
        <el-radio-group size="small" :model-value="songMode.get()" @update:model-value="songMode.set">
          <el-radio value="zJinSong">尽送</el-radio>
          <el-radio value="zShiSong">实送</el-radio>
        </el-radio-group>
        <div class="fld-inline grow">
          <span class="lbl">装订说明</span>
          <el-input size="small" v-model="f.zTeshushuoming" />
        </div>
      </div>
    </section>

    <!-- 底栏 -->
    <section class="panel panel-bottom">
      <div class="bot-left">
        <div class="sec-head">成品规格</div>
        <div class="spec-row">
          <div class="fld-inline">
            <span class="lbl">横(cm)</span>
            <el-input size="small" class="dim-w" v-model="f.fkHeng" />
          </div>
          <div class="fld-inline">
            <span class="lbl">竖(cm)</span>
            <el-input size="small" class="dim-w" v-model="f.fkShu" />
          </div>
        </div>
        <table class="dense-table sb-table">
          <thead>
            <tr><th>头</th><th>脚</th><th>左</th><th>右</th></tr>
          </thead>
          <tbody>
            <tr>
              <td><el-input size="small" v-model="f.sbTou" /></td>
              <td><el-input size="small" v-model="f.sbJiao" /></td>
              <td><el-input size="small" v-model="f.sbZuo" /></td>
              <td><el-input size="small" v-model="f.sbYou" /></td>
            </tr>
            <tr>
              <td><el-input size="small" v-model="f.sbTou2" /></td>
              <td><el-input size="small" v-model="f.sbJiao2" /></td>
              <td><el-input size="small" v-model="f.sbZuo2" /></td>
              <td><el-input size="small" v-model="f.sbYou2" /></td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="bot-note">
        <span class="lbl">特殊说明</span>
        <el-input type="textarea" resize="none" v-model="f.fkSpecial" />
      </div>
      <div class="bot-pack">
        <div class="fld-stack">
          <span class="lbl">合格证</span>
          <el-radio-group size="small" :model-value="certMode.get()" @update:model-value="certMode.set">
            <el-radio value="bxYouChangMing">有厂名</el-radio>
            <el-radio value="bxWuChangMing">无厂名</el-radio>
          </el-radio-group>
        </div>
        <div class="fld-stack">
          <span class="lbl">包装</span>
          <el-radio-group size="small" :model-value="packMode.get()" @update:model-value="packMode.set">
            <el-radio value="bxZhiBao">纸包</el-radio>
            <el-radio value="bxZhiXiang">纸箱</el-radio>
          </el-radio-group>
        </div>
      </div>
      <div class="bot-sign">
        <div class="fld-inline">
          <span class="lbl">开单人</span>
          <el-input size="small" v-model="f.signedBy" />
        </div>
        <div class="fld-inline">
          <span class="lbl">业务</span>
          <el-input size="small" v-model="f.business" />
        </div>
        <div class="fld-inline grow">
          <span class="lbl">印刷前对稿</span>
          <el-input size="small" v-model="f.proofread" />
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.edit-form {
  flex: 1;
  min-height: 0;
  width: 100%;
  display: grid;
  /* 中部按内容增高，禁止压扁导致表格/开纸区重叠 */
  grid-template-rows: auto auto minmax(min-content, 1fr) auto auto;
  gap: 6px;
  font-size: 12px;
  color: #37474f;
}

.panel {
  background: #fff;
  border: 1px solid #c5ced6;
  border-radius: 4px;
  padding: 6px 8px;
  min-width: 0;
}
.sec-head {
  font-size: 12px;
  font-weight: 700;
  color: #1a237e;
  margin-bottom: 4px;
  letter-spacing: 0.5px;
}

.lbl {
  flex: none;
  font-size: 12px;
  font-weight: 600;
  color: #546e7a;
  white-space: nowrap;
}
.fld-inline {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
}
.fld-inline.grow { flex: 1; min-width: 80px; }
.fld-inline.wrap { flex-wrap: wrap; }
.fld-inline :deep(.el-input) { flex: 1; min-width: 0; }
.fld-stack {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* 顶栏 */
.panel-top { display: flex; flex-direction: column; gap: 6px; }
.top-row1, .top-row2 {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.top-product {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  min-width: 0;
}
.top-product .lbl { padding-top: 4px; }
.top-product :deep(.el-textarea) { flex: 1; min-width: 0; }
.top-product :deep(.el-textarea__inner) {
  min-height: 64px;
  padding: 4px 8px;
  font-size: 12px;
  line-height: 1.4;
}
.no-val {
  font-size: 16px;
  font-weight: 700;
  color: #1565c0;
  letter-spacing: 0.5px;
  min-width: 72px;
}
.date-inp { width: 140px; }
.w-cost { width: 110px; }
.w-contract { width: 130px; }
.w-qty { width: 100px; }
.w-unit { width: 72px; }
.w-num { width: 110px; }
.w-num-sm { width: 72px; }

/* 拼版 */
.panel-sub {
  display: grid;
  grid-template-columns: 100px 160px minmax(0, 1fr);
  gap: 8px;
  align-items: stretch;
}
.sub-left { display: flex; flex-direction: column; gap: 2px; }
.sub-line {
  display: flex;
  align-items: center;
  padding: 2px 6px;
  border: 1px solid #e3e8ee;
  border-radius: 3px;
}
.sub-pinban {
  display: flex;
  flex-direction: column;
  gap: 4px;
  justify-content: center;
  border-left: 1px solid #e3e8ee;
  padding-left: 8px;
}
.sub-remark {
  display: flex;
  flex-direction: column;
  gap: 2px;
  border-left: 1px solid #e3e8ee;
  padding-left: 8px;
  min-width: 0;
}
.sub-remark :deep(.el-textarea) { flex: 1; }
.sub-remark :deep(.el-textarea__inner) {
  height: 100% !important;
  min-height: 52px;
  padding: 4px 8px;
  font-size: 12px;
}

/* 中部：双表 + 备注整行 */
.panel-mid {
  display: flex;
  flex-direction: column;
  gap: 6px;
  overflow: visible;
}
.mid-tables {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 8px;
  align-items: start;
  flex: none;
}
.mid-paper, .mid-print {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}
.mid-paper .dense-table,
.mid-print .dense-table {
  flex: none;
}
.note-row {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  flex: none;
}
.note-row .lbl { padding-top: 4px; min-width: 56px; }
.note-row :deep(.el-textarea) { flex: 1; min-width: 0; }
.note-row :deep(.el-textarea__inner) {
  min-height: 44px;
  padding: 4px 8px;
  font-size: 12px;
  line-height: 1.4;
}

.dense-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}
.dense-table th,
.dense-table td {
  border: 1px solid #cfd8dc;
  padding: 0 2px;
  text-align: center;
  vertical-align: middle;
}
.dense-table td { height: 28px; }
.dense-table th {
  background: #eef2f7;
  font-size: 11px;
  font-weight: 700;
  color: #455a64;
  padding: 3px 4px;
}
.dense-table .col-qty { width: 28%; }
.dense-table :deep(.el-input__wrapper) {
  box-shadow: none !important;
  background: transparent !important;
  padding: 0 4px !important;
  min-height: 26px !important;
}
.dense-table :deep(.el-input__inner) {
  font-size: 12px;
  height: 24px !important;
  line-height: 24px !important;
  text-align: left;
}
.proc-table .col-paper,
.proc-table td:nth-child(1) { width: 42%; }
.proc-table .col-color,
.proc-table td:nth-child(2) { width: 20%; }
.proc-table .col-pqty,
.proc-table td:nth-child(3) { width: 19%; }
.proc-table .col-extra,
.proc-table td:nth-child(4) { width: 19%; }

.size-block {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin-top: 2px;
}
.size-dims {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 10px;
  align-items: center;
}
.dim { width: 56px !important; flex: none !important; }
.dim-w { width: 72px !important; flex: none !important; }

/* 后工序 */
.panel-post {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.post-row1, .post-row2, .post-row3 {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.proc-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 6px;
  background: #f5f7fa;
  border: 1px solid #e3e8ee;
  border-radius: 3px;
}
.proc-name {
  font-weight: 700;
  font-size: 12px;
  color: #1a237e;
}
.ck-cluster {
  display: flex;
  flex-wrap: wrap;
  gap: 2px 8px;
  align-items: center;
  flex: 1;
}
.bind-other { width: 90px; flex: none !important; }
.bind-qty {
  display: flex;
  align-items: center;
  gap: 4px;
}
.qty-inp { width: 100px; }
.op { color: #78909c; font-weight: 600; }

/* 底栏 */
.panel-bottom {
  display: grid;
  grid-template-columns: 220px minmax(0, 1fr) 120px;
  grid-template-rows: auto auto;
  gap: 6px 10px;
}
.bot-left { grid-row: 1; grid-column: 1; }
.bot-note {
  grid-row: 1;
  grid-column: 2;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.bot-note :deep(.el-textarea) { flex: 1; }
.bot-note :deep(.el-textarea__inner) {
  height: 100% !important;
  min-height: 72px;
  padding: 4px 8px;
  font-size: 12px;
}
.bot-pack {
  grid-row: 1;
  grid-column: 3;
  display: flex;
  flex-direction: column;
  gap: 6px;
  border-left: 1px solid #e3e8ee;
  padding-left: 8px;
}
.bot-sign {
  grid-row: 2;
  grid-column: 1 / -1;
  display: flex;
  gap: 10px;
  align-items: center;
  border-top: 1px dashed #d0d7de;
  padding-top: 4px;
}
.spec-row {
  display: flex;
  gap: 8px;
  margin-bottom: 4px;
}
.sb-table { max-width: 100%; }
.sb-table th, .sb-table td { width: 25%; }

:deep(.el-checkbox) { height: 22px; margin-right: 0; }
:deep(.el-checkbox__label) { font-size: 12px; padding-left: 4px; }
:deep(.el-radio) { height: 22px; margin-right: 8px; }
:deep(.el-radio__label) { font-size: 12px; padding-left: 4px; }
:deep(.el-input-group__prepend) {
  padding: 0 6px;
  font-size: 11px;
}
</style>

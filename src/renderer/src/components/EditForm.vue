<script setup>
/**
 * 编辑表单：Element Plus 组件，按工单图纸分区。
 * 数据直接挂在 props.order 上（fields 中文日期 / cks 勾选），编辑即生效，
 * 无需 emit（App 持有同一引用；预览组件同样引用 order 自动刷新）。
 */

import { reactive } from 'vue';
import { getField, cnDateToIso, isoToCnDate } from '../lib/order.js';

const props = defineProps({
  order: { type: Object, required: true }
});

// 确保可响应：无 fields/cks 则补默认，reactive 深层代理保证新增 key 可响应
if (!props.order.fields) props.order.fields = {};
if (!props.order.cks) props.order.cks = {};
const f = reactive(props.order.fields);
const cks = reactive(props.order.cks);

// 啤版本默认「旧版」
if (!props.order.fields.piVersion) {
  props.order.fields.piVersion = 'old';
}

/* ---------- 读取字段（别名回退） ---------- */
function t(keys) {
  return getField(props.order.fields, Array.isArray(keys) ? keys : [keys]);
}

/* ---------- 写入字段（含别名同步） ---------- */
function setF(key, val, sync) {
  if (!props.order.fields) props.order.fields = {};
  if (Array.isArray(key)) {
    key.forEach((k) => { props.order.fields[k] = val; });
  } else {
    props.order.fields[key] = val;
  }
  if (sync) sync.forEach((k) => { props.order.fields[k] = val; });
}

/* ---------- 日期处理：存中文，ElDatePicker 显示 ISO ---------- */
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

/* ---------- 机印说明 5 行字段名 ---------- */
const procRows = [1, 2, 3, 4, 5].map((n) => ({
  paper: `print_paper${n}`,
  color: `print_color${n}`,
  qty: `print_qty${n}`,
  extra: `print_extra${n}`
}));

/* ---------- 后工序勾选列表 ---------- */
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

/* ---------- 开纸尺寸勾选列表 ---------- */
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
const packCks = [
  ['bxYouChangMing', '有厂名'], ['bxWuChangMing', '无厂名'],
  ['bxZhiBao', '纸包'], ['bxZhiXiang', '纸箱']
];

/* ===== 单选组（二选一，映射到 cks 两个布尔键） ===== */
// 传入 [选A时ck键, 选B时ck键]，读写互斥单选
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
// 各单选组
const songMode = radioPair('zJinSong', 'zShiSong');          // 尽送/实送
const glassMode = radioPair('gGuangJiaoDan', 'gGuangJiaoShuang'); // 光胶单面/双面
const yaMode = radioPair('gYaJiaoDan', 'gYaJiaoShuang');     // 哑胶单面/双面
const certMode = radioPair('bxYouChangMing', 'bxWuChangMing'); // 合格证 有厂名/无厂名
const packMode = radioPair('bxZhiBao', 'bxZhiXiang');        // 纸包/纸箱
</script>

<template>
  <div class="edit-form">

    <!-- 区块1：信息条（开单/交货/No） -->
    <el-card shadow="never" class="ep-card">
      <template #header><span class="block-title">工单信息</span></template>
      <div class="info-row">
        <div class="info-item">
          <span class="info-label">开单日期</span>
          <el-date-picker
            class="info-date"
            :model-value="dateIso(['openDate','mJob_Date'])"
            type="date" placeholder="选择日期" value-format="YYYY-MM-DD"
            @update:modelValue="(v) => dateChanged(['openDate','mJob_Date'], v)"
          />
        </div>
        <div class="info-item">
          <span class="info-label">交货日期</span>
          <el-date-picker
            class="info-date"
            :model-value="dateIso(['deliverDate','mFinished_Date'])"
            type="date" placeholder="选择日期" value-format="YYYY-MM-DD"
            @update:modelValue="(v) => dateChanged(['deliverDate','mFinished_Date'], v)"
          />
        </div>
        <div class="info-no">
          <span class="info-label">No.</span>
          <span class="no-val">{{ order.orderNo }}</span>
        </div>
      </div>
    </el-card>

    <!-- 区块2：订印单位 / 合同号 -->
    <el-card shadow="never" class="ep-card">
      <template #header><span class="block-title">订印单位</span></template>
      <div class="row2">
        <div class="fld grow">
          <span class="fld-label">订印单位</span>
          <el-input v-model="f.customer" placeholder="订印单位" @update:modelValue="(v)=>{f.mCustomer_FullName=v;}" />
        </div>
        <div class="fld">
          <span class="fld-label">合同号</span>
          <el-input v-model="f.contractNo" placeholder="合同号" />
        </div>
      </div>
    </el-card>

    <!-- 区块3：产品 / 数量 / 号码 -->
    <el-card shadow="never" class="ep-card">
      <template #header><span class="block-title">产品 / 订印</span></template>
      <div class="fld grow">
        <span class="fld-label">产品名称 / 规格</span>
        <el-input type="textarea" :rows="3" v-model="f.productSpec" placeholder="产品名称、规格、工艺说明" @update:modelValue="(v)=>{f.mProduct_Name=v;}" />
      </div>
      <div class="row3">
        <div class="fld">
          <span class="fld-label">订印数量</span>
          <el-input v-model="f.orderQty" placeholder="数量" />
        </div>
        <div class="fld">
          <span class="fld-label">单位</span>
          <el-input v-model="f.unit" placeholder="单位" />
        </div>
      </div>
      <div class="row3">
        <div class="fld">
          <span class="fld-label">号码 由</span>
          <el-input v-model="f.numFrom" placeholder="起始号" />
        </div>
        <div class="fld">
          <span class="fld-label">联</span>
          <el-input v-model="f.numLian" placeholder="联数" />
        </div>
        <div class="fld">
          <span class="fld-label">页/本</span>
          <el-input v-model="f.numYeBen" placeholder="页/本" />
        </div>
      </div>
    </el-card>

    <!-- 区块4：开纸 / 拼版 -->
    <el-card shadow="never" class="ep-card">
      <template #header><span class="block-title">拼版 / 开纸</span></template>
      <div class="sub-table">
        <!-- 左：六开/对开 两行，件输入框在 新/件 之间 -->
        <div class="sub-left">
          <div class="sub-line">
            <el-checkbox v-model="cks.liukai" label="六开" />
            <span class="sub-grp"><span class="sub-p">新</span><el-input class="sub-inp" v-model="f.xinJian" /><span class="sub-p">件</span></span>
            <span class="sub-grp"><span class="sub-p">旧</span><el-input class="sub-inp" v-model="f.jiuJian" /><span class="sub-p">件</span></span>
            <span class="sub-grp"><span class="sub-p">共</span><el-input class="sub-inp" v-model="f.gongJian" /><span class="sub-p">件</span></span>
            <span class="sub-grp"><span class="sub-p">已找</span><el-input class="sub-inp" v-model="f.yiZhaoJian" /><span class="sub-p">件</span></span>
          </div>
          <div class="sub-line">
            <el-checkbox v-model="cks.duikai" label="对开" />
            <span class="sub-grp"><span class="sub-p">新</span><el-input class="sub-inp" v-model="f.dkXin" /><span class="sub-p">件</span></span>
            <span class="sub-grp"><span class="sub-p">旧</span><el-input class="sub-inp" v-model="f.dkJiu" /><span class="sub-p">件</span></span>
            <span class="sub-grp"><span class="sub-p">共</span><el-input class="sub-inp" v-model="f.dkGong" /><span class="sub-p">件</span></span>
            <span class="sub-grp"><span class="sub-p">已找</span><el-input class="sub-inp" v-model="f.dkZhao" /><span class="sub-p">件</span></span>
          </div>
        </div>
        <!-- 右：拼版数量 -->
        <div class="sub-right">
          <div class="fld">
            <span class="fld-label">拼版数量 横</span>
            <el-input v-model="f.pinbanH" placeholder="横" />
          </div>
          <div class="fld">
            <span class="fld-label">拼版数量 竖</span>
            <el-input v-model="f.pinbanS" placeholder="竖" />
          </div>
        </div>
        <!-- 最右：备注 -->
        <div class="sub-remark">
          <span class="fld-label">备注</span>
          <el-input type="textarea" :rows="2" v-model="f.remark" placeholder="开纸/拼版备注" @update:modelValue="(v)=>{f.mRemarks=v;}" />
        </div>
      </div>
    </el-card>

    <!-- 区块5：纸类 / 开纸尺寸 -->
    <el-card shadow="never" class="ep-card">
      <template #header><span class="block-title">纸类</span></template>
      <!-- 纸类 / 发纸数：表格形式，5 行一一对应 -->
      <table class="paper-table">
        <thead>
          <tr><th>纸类</th><th>发纸数(张)</th></tr>
        </thead>
        <tbody>
          <tr v-for="n in 5" :key="n">
            <td>
              <el-input
                :model-value="n===1 ? f.paperType : f['paperType'+n]"
                @update:model-value="v => (n===1 ? (f.paperType=v) : (f['paperType'+n]=v))"
                placeholder="纸类规格"
              />
            </td>
            <td>
              <el-input
                :model-value="n===1 ? f.paperCount : f['paperCount'+n]"
                @update:model-value="v => (n===1 ? (f.paperCount=v) : (f['paperCount'+n]=v))"
                placeholder="张数"
              />
            </td>
          </tr>
        </tbody>
      </table>
      <div class="fld">
        <span class="fld-label">开纸尺寸 面</span>
        <div class="ck-cluster">
          <el-checkbox v-for="([k, lbl]) in sizeCksA" :key="k" :label="lbl" v-model="cks[k]" />
        </div>
      </div>
      <div class="fld">
        <span class="fld-label">开纸尺寸 底</span>
        <div class="ck-cluster">
          <el-checkbox v-for="([k, lbl]) in sizeCksB" :key="k" :label="lbl" v-model="cks[k]" />
        </div>
      </div>
      <div class="row3">
        <div class="fld">
          <span class="fld-label">面 尺寸</span>
          <div class="dim-inline">
            <el-input class="dim-inp" v-model="f.sz47_5" /> × <el-input class="dim-inp" v-model="f.sz64_5" />
          </div>
        </div>
        <div class="fld">
          <span class="fld-label">底 尺寸</span>
          <div class="dim-inline">
            <el-input class="dim-inp" v-model="f.sz47_3" /> × <el-input class="dim-inp" v-model="f.sz64_3" />
          </div>
        </div>
      </div>
      <div class="row3">
        <div class="fld">
          <span class="fld-label">开数 面</span>
          <el-input v-model="f.kaifangMian" placeholder="2开面" />
        </div>
        <div class="fld">
          <span class="fld-label">开数 底</span>
          <el-input v-model="f.kaifangDi" placeholder="2开底" />
        </div>
      </div>
      <div class="fld">
        <span class="fld-label">纸类 / 发纸备注</span>
        <el-input type="textarea" :rows="2" v-model="f.paperNote" placeholder="纸类规格、发纸数量、特殊说明" />
      </div>
    </el-card>

    <!-- 区块6：机印说明 -->
    <el-card shadow="never" class="ep-card">
      <template #header><span class="block-title">机印说明</span></template>
      <table class="paper-table proc-table">
        <thead>
          <tr><th>纸别</th><th>印色</th><th>实印数(张)</th><th>放数(张)</th></tr>
        </thead>
        <tbody>
          <tr v-for="(r, i) in procRows" :key="i">
            <td><el-input v-model="f[r.paper]" placeholder="纸别" /></td>
            <td><el-input v-model="f[r.color]" placeholder="印色" /></td>
            <td><el-input v-model="f[r.qty]" placeholder="实印数" /></td>
            <td><el-input v-model="f[r.extra]" placeholder="放数" /></td>
          </tr>
        </tbody>
      </table>
      <div class="fld">
        <span class="fld-label">机印备注</span>
        <el-input type="textarea" :rows="2" v-model="f.print_note" />
      </div>
    </el-card>

    <!-- 区块7：后工序 + 装订（合并卡片） -->
    <el-card shadow="never" class="ep-card">
      <template #header><span class="block-title">后工序 / 装订</span></template>
      <!-- 带三级：光胶/哑胶 各自一行，子向单选 单面/双面 -->
      <div class="proc-group">
        <div class="proc-item">
          <span class="proc-item-name">光胶</span>
          <el-radio-group :model-value="glassMode.get()" @update:model-value="glassMode.set">
            <el-radio value="gGuangJiaoDan">单面</el-radio>
            <el-radio value="gGuangJiaoShuang">双面</el-radio>
          </el-radio-group>
        </div>
        <div class="proc-item">
          <span class="proc-item-name">哑胶</span>
          <el-radio-group :model-value="yaMode.get()" @update:model-value="yaMode.set">
            <el-radio value="gYaJiaoDan">单面</el-radio>
            <el-radio value="gYaJiaoShuang">双面</el-radio>
          </el-radio-group>
        </div>
      </div>
      <!-- 其余工艺勾选（平铺） -->
      <div class="ck-cluster wrap">
        <el-checkbox
          v-for="([k, lbl]) in postCks.filter(([k]) => !['gGuangJiaoDan','gGuangJiaoShuang','gYaJiaoDan','gYaJiaoShuang'].includes(k))"
          :key="k" :label="lbl" v-model="cks[k]"
        />
      </div>
      <!-- 「其它」勾选后：输入其它内容 -->
      <div v-if="props.order.cks && props.order.cks.gQiTa" class="fld">
        <span class="fld-label">其它内容</span>
        <el-input v-model="f.gQiTaText" placeholder="其它工序内容" />
      </div>
      <!-- 啤 / 版本 / 特别说明（对齐预览） -->
      <div class="beer-row">
        <div class="fld">
          <span class="fld-label">啤</span>
          <el-select v-model="f.piVersion" placeholder="选择啤类型" clearable style="width:120px">
            <el-option label="新版" value="new" />
            <el-option label="旧版" value="old" />
          </el-select>
        </div>
        <div class="fld grow">
          <span class="fld-label">后工序特别说明</span>
          <el-input v-model="f.houGongxuNote" placeholder="后工序特别说明" />
        </div>
      </div>

      <!-- 装订（并入后工序卡片） -->
      <div class="fld">
        <span class="fld-label">装订方式</span>
        <div class="ck-cluster bind-row">
          <el-checkbox
            v-for="([k, lbl]) in bindCks.filter(([k]) => k !== 'zQiTa')"
            :key="k" :label="lbl" v-model="cks[k]"
          />
          <el-input v-model="f.zQiTaText" placeholder="其它装订方式" class="bind-other-input" />
        </div>
      </div>
      <div class="row4">
        <div class="fld">
          <span class="fld-label">张数</span>
          <el-input v-model="f.zZhangCount" />
        </div>
        <div class="fld">
          <span class="fld-label">本数</span>
          <el-input v-model="f.zBenCount" />
        </div>
        <div class="fld">
          <span class="fld-label">每本份数</span>
          <el-input v-model="f.zMeiBenFen" />
        </div>
        <div class="fld">
          <span class="fld-label">检查点数</span>
          <el-radio-group :model-value="songMode.get()" @update:model-value="songMode.set">
            <el-radio value="zJinSong">尽送</el-radio>
            <el-radio value="zShiSong">实送</el-radio>
          </el-radio-group>
        </div>
      </div>
      <div class="fld">
        <span class="fld-label">装订特殊说明</span>
        <el-input type="textarea" :rows="2" v-model="f.zTeshushuoming" />
      </div>
    </el-card>

    <!-- 区块9：成品规格 / 包装 -->
    <el-card shadow="never" class="ep-card">
      <template #header><span class="block-title">成品规格 / 包装</span></template>
      <div class="row3">
        <div class="fld">
          <span class="fld-label">成品规格 横(cm)</span>
          <el-input v-model="f.fkHeng" />
        </div>
        <div class="fld">
          <span class="fld-label">成品规格 竖(cm)</span>
          <el-input v-model="f.fkShu" />
        </div>
      </div>
      <!-- 四边留位：独占一行 -->
      <div class="fld">
        <span class="fld-label">四边留位</span>
        <table class="paper-table sb-table">
          <thead>
            <tr><th>头</th><th>脚</th><th>左</th><th>右</th></tr>
          </thead>
          <tbody>
            <tr>
              <td><el-input v-model="f.sbTou" /></td>
              <td><el-input v-model="f.sbJiao" /></td>
              <td><el-input v-model="f.sbZuo" /></td>
              <td><el-input v-model="f.sbYou" /></td>
            </tr>
            <tr>
              <td><el-input v-model="f.sbTou2" /></td>
              <td><el-input v-model="f.sbJiao2" /></td>
              <td><el-input v-model="f.sbZuo2" /></td>
              <td><el-input v-model="f.sbYou2" /></td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="row3">
        <div class="fld">
          <span class="fld-label">合格证</span>
          <el-radio-group :model-value="certMode.get()" @update:model-value="certMode.set">
            <el-radio value="bxYouChangMing">有厂名</el-radio>
            <el-radio value="bxWuChangMing">无厂名</el-radio>
          </el-radio-group>
        </div>
        <div class="fld">
          <span class="fld-label">包装</span>
          <el-radio-group :model-value="packMode.get()" @update:model-value="packMode.set">
            <el-radio value="bxZhiBao">纸包</el-radio>
            <el-radio value="bxZhiXiang">纸箱</el-radio>
          </el-radio-group>
        </div>
      </div>
      <div class="fld">
        <span class="fld-label">成品规格特殊说明</span>
        <el-input type="textarea" :rows="2" v-model="f.fkSpecial" />
      </div>
    </el-card>

    <!-- 区块10：签名 -->
    <el-card shadow="never" class="ep-card">
      <template #header><span class="block-title">签名 / 确认</span></template>
      <div class="row4">
        <div class="fld">
          <span class="fld-label">开单人</span>
          <el-input v-model="f.signedBy" placeholder="开单人" />
        </div>
        <div class="fld">
          <span class="fld-label">业务</span>
          <el-input v-model="f.business" />
        </div>
        <div class="fld">
          <span class="fld-label">印刷前对稿</span>
          <el-input v-model="f.proofread" />
        </div>
      </div>
    </el-card>

  </div>
</template>

<style scoped>
.edit-form {
  max-width: 980px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-bottom: 30px;
}
.ep-card :deep(.el-card__body) {
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.card-title { font-weight: 700; color: #1a237e; }

/* 卡片标题（el-card header） */
.block-title {
  font-weight: 700;
  font-size: 14px;
  color: #fff;
  background: #1a237e;
  padding: 6px 12px;
  border-radius: 4px;
  letter-spacing: 1px;
}

/* 纸类/发纸数 表格输入 */
.paper-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}
.paper-table th, .paper-table td {
  border: 1px solid #cfd8dc;
  padding: 2px 6px;
  text-align: center;
  vertical-align: middle;
}
.paper-table th {
  background: #f2f5fb;
  color: #37474f;
  font-weight: 700;
  font-size: 12px;
  padding: 6px;
}
.paper-table td {
  height: 34px;
}
.paper-table th:first-child, .paper-table td:first-child { width: 60%; }
.paper-table :deep(.el-input__wrapper) {
  box-shadow: none !important;
  background: transparent !important;
  padding: 0;
}
.paper-table :deep(.el-input__inner) {
  text-align: left;
  padding: 0 4px;
}

/* 机印说明表格：4 列等宽（覆盖 paper-table 首列 60% 规则） */
.proc-table th, .proc-table td { width: 25% !important; }
.proc-table th:first-child, .proc-table td:first-child { width: 25% !important; }

/* 四边留位表格：4 列等宽 */
.sb-table { max-width: 420px; }
.sb-table th, .sb-table td { width: 25% !important; }
.sb-table th:first-child, .sb-table td:first-child { width: 25% !important; }

/* 啤 / 版本 / 特别说明 行 */
.beer-row {
  display: flex;
  align-items: flex-end;
  gap: 16px;
}

/* 装订方式：勾选 + 其它输入框（仅输入框，无标签） */
.bind-row { gap: 8px 14px; }
.bind-other-input { width: 180px; }

/* 信息条 */
.info-row { display: flex; align-items: center; gap: 24px; flex-wrap: wrap; }
.info-item { display: flex; align-items: center; gap: 8px; }
.info-label { font-weight: 600; color: #37474f; white-space: nowrap; }
.info-date { width: 160px; }
.info-no { margin-left: auto; display: flex; align-items: center; gap: 6px; }
.no-val { font-size: 18px; font-weight: 700; color: #1a237e; letter-spacing: 1px; }

/* 行 */
.row2 { display: grid; grid-template-columns: 1fr 180px; gap: 12px; align-items: start; }
.row3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; align-items: start; }
.row4 { display: grid; grid-template-columns: 1fr 120px 1fr; gap: 12px; align-items: start; }

/* 字段 */
.fld { display: flex; flex-direction: column; gap: 4px; }
.fld.grow { grid-column: 1 / -1; }
.fld-label { font-size: 12px; color: #546e7a; font-weight: 600; }

/* 勾选簇 */
.ck-cluster { display: flex; gap: 6px 4px; flex-wrap: wrap; align-items: center; }
.ck-cluster.wrap { gap: 8px 12px; padding: 4px 0; }

/* 后工序：光胶/哑胶 带子向，层级感 */
.proc-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 10px;
}
.proc-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: #f7f9fc;
  border: 1px solid #e3e8ee;
  border-radius: 6px;
  max-width: 480px;
}
.proc-item-name {
  font-weight: 700;
  font-size: 13px;
  color: #1a237e;
  min-width: 44px;
}

/* 子表：六开/对开 + 拼版 + 备注 */
.sub-table {
  display: grid;
  grid-template-columns: 1fr 150px 280px;
  gap: 12px;
  align-items: stretch;
}
.sub-left { display: flex; flex-direction: column; gap: 8px; }
.sub-line {
  display: flex; align-items: center; gap: 6px;
  padding: 8px 10px; border: 1px solid #e3e8ee; border-radius: 6px;
}
.sub-line + .sub-line { border-top: none; border-top-left-radius: 0; border-top-right-radius: 0; }
.sub-p { font-size: 12px; color: #546e7a; white-space: nowrap; }
/* 件数组：固定宽度，保证六开/对开两行的 新/旧/共/已找 纵向对齐 */
.sub-grp {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  width: 96px;
  flex-shrink: 0;
}
.sub-inp { width: 52px; }
.sub-right { display: flex; flex-direction: column; gap: 8px; border-left: 1px solid #e3e8ee; padding-left: 12px; }
.sub-remark {
  border-left: 1px solid #e3e8ee;
  padding-left: 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}
/* 备注 textarea 铺满 sub-remark 高度（对齐拼版数量列） */
.sub-remark :deep(.el-textarea) {
  flex: 1;
  display: flex;
  min-height: 0;
}
.sub-remark :deep(.el-textarea__inner) {
  height: 100%;
  flex: 1;
  min-height: 88px;
  resize: none;
}

/* 机印行 */
.proc-row { display: grid; grid-template-columns: 1fr 0.7fr 0.8fr 0.8fr; gap: 10px; align-items: start; }

/* 尺寸行内 */
.dim-inline { display: inline-flex; align-items: center; gap: 4px; flex-wrap: wrap; color: #455a64; }
.dim-inp { width: 72px; }
</style>
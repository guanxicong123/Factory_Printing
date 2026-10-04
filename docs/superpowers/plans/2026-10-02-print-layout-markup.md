# 打印工单版式标注修正 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 按 `2026-10-02-print-layout-markup-design.md` 收紧标签格、重分配列宽，并修正装订单位与成品规格换行。

**Architecture:** 仅改预览/打印：`workOrderDoc.js` 调整列宽与装订 HTML；`base.css` 统一标签 padding、备注上移、开纸字号、成品规格 nowrap。

**Tech Stack:** Vue 前端预览 HTML 模板 + CSS

## Global Constraints

- 只改预览/打印：`src/renderer/src/lib/workOrderDoc.js`、`src/renderer/src/styles/base.css`
- 不改 EditForm、字段模型、cks
- 不全局清零所有 td padding
- 用户未要求则不 git commit

---

### Task 1: 装订单位修正

**Files:**
- Modify: `src/renderer/src/lib/workOrderDoc.js`（装订数量格）

**Interfaces:**
- Consumes: `inp('zZhangCount')` / `inp('zBenCount')` / `inp('zMeiBenFen')`
- Produces: 数值 + 固定 `.zd-unit`（张/本/份），无多余「个」

- [x] **Step 1:** 将 `</span>个<span class="zd-unit">张</span>` 改为 `</span><span class="zd-unit">张</span>`
- [x] **Step 2:** 确认「本」「份」格已是固定单位格式，无需再插编辑态单位

---

### Task 2: 列宽重分配

**Files:**
- Modify: `src/renderer/src/lib/workOrderDoc.js`

**Interfaces:**
- 订印单位行、成品规格/包装表的 inline `width` / colspan 区域

- [x] **Step 1:** 订印单位行：标签 `12%` → `7%`；合同号标签保持约 `8%`；合同号值 `16%` → `22%`；客户列 `60%` → `63%`（或按视觉微调，保证合同号明显加宽）
- [x] **Step 2:** 成品规格表：成品规格内容区（含四边留位）加宽；`合格证` `11%` → `7%`；有厂名列 `18%` → `10%`；左侧成品+留位合计相应增加

建议成品规格表宽度分配（合计 100%）：
- 竖排「成品规格」标签：`3%`
- 横/竖 + 四边留位区：约 `52%`（原约 20%+ 四边列）
- 特殊说明：`35%`（略收）
- 竖排「包装」：`3%`
- 合格证/纸包/纸箱标签：`7%`
- 有厂名等选项列：`10%`

---

### Task 3: CSS 收紧与排版微调

**Files:**
- Modify: `src/renderer/src/styles/base.css`

- [x] **Step 1:** `.form-doc .label` 增加 `padding: 0 1px !important;`（与 `.narrow-label` 一致）；`.pack-label` 同步收紧
- [x] **Step 2:** `.remark-cell`：`padding: 2px 10px 6px !important;`（上边减小）
- [x] **Step 3:** `.size-merge` 字号由 `9px` 提到约 `12px`；勾选框可略增至 `12px`
- [x] **Step 4:** `.spec-cell .spec-wrap` / `.spec-group` 确保 `white-space: nowrap`；必要时减小 gap/`padding` 防折行
- [x] **Step 5:** `.zd-qty` 可略减 padding，单位字保持可见

---

### Task 4: 验收

- [x] **Step 1:** 在 `workOrderDoc.js` / `base.css` 中 grep 确认无多余「个<span class="zd-unit">张」
- [x] **Step 2:** 目视检查关键 CSS 规则与列宽百分比已写入
- [x] **Step 3:** 若本地可启动预览则打开一张工单核对；否则以代码审查对照 spec 清单 1–10

---

## Spec coverage

| Spec # | Task |
|--------|------|
| 1 标签去 padding / 收窄 | Task 2 + 3 |
| 2 合同号加宽 | Task 2 |
| 3 备注偏上 | Task 3 |
| 4 开纸字号 | Task 3 |
| 5 装订单位 | Task 1 |
| 6 横竖单行 | Task 3 |
| 7 成品规格加宽 | Task 2 |
| 8–10 合格证/有厂名缩短、包装左收 | Task 2 |

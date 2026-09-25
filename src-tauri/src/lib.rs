use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::Manager;

/* ---------------- 数据结构 ---------------- */

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrderMeta {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub file_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub meta: OrderMeta,
    pub order_no: String,                     // 纯数字单号，如 0022379
    pub fields: HashMap<String, String>,      // 任意前端字段（data-field）
    pub cks: HashMap<String, bool>,           // 勾选框状态（data-ck）
}

impl Default for Order {
    fn default() -> Self {
        Self {
            meta: OrderMeta::default(),
            order_no: String::new(),
            fields: HashMap::new(),
            cks: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrderListItem {
    pub id: String,
    pub file_name: String,
    pub order_no: String,
    pub product_name: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub file_name: String,
    pub order: Order,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PrintResult {
    pub ok: bool,
    pub msg: String,
}

/// SQL 导入结果：count = 成功写入的工单数，skipped = 跳过的数据行数
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub count: usize,
    pub skipped: usize,
    pub message: String,
}

/* ---------------- 存储路径 ---------------- */

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建应用数据目录: {e}"))?;
    Ok(dir)
}

fn orders_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("orders");
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建 orders 目录: {e}"))?;
    Ok(dir)
}

/* ---------------- 单号生成（纯 7 位序号，跨重启递增） ---------------- */

fn seq_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("sequence.json"))
}

fn read_seq(app: &tauri::AppHandle) -> serde_json::Value {
    match seq_file(app) {
        Ok(p) => {
            if let Ok(raw) = fs::read_to_string(p) {
                serde_json::from_str(&raw).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            }
        }
        Err(_) => serde_json::json!({}),
    }
}

fn next_order_no(app: &tauri::AppHandle) -> Result<String, String> {
    // 纯序号递增，不用年份逻辑；格式 0000001、0000002……
    const SEQ_KEY: &str = "seq";
    let mut seq = read_seq(app);
    let mut n: u64 = seq
        .get(SEQ_KEY)
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // 扫描已保存文件（仅统计 0000001 这种纯序号格式，避免旧 26xxxxx 单号干扰），防止计数器丢失后重号
    let dir = orders_dir(app)?;
    let mut max_from_files: u64 = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let stem = name.trim_end_matches(".json");
                // 纯 7 位数字且以 0 开头的序号文件，如 0000001.json
                if stem.len() == 7 && stem.starts_with('0') {
                    if let Ok(v) = stem.parse::<u64>() {
                        if v > max_from_files {
                            max_from_files = v;
                        }
                    }
                }
            }
        }
    }
    n = n.max(max_from_files) + 1;
    seq[SEQ_KEY] = serde_json::json!(n);
    fs::write(seq_file(app)?, seq.to_string())
        .map_err(|e| format!("写入序号失败: {e}"))?;
    Ok(format!("{:07}", n))
}

/// YYYYMMDD 格式的今天日期，如 20260925（基于 civil_from_days，无外部依赖）
fn today_compact() -> String {
    let now = std::time::SystemTime::now();
    let dur = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let days = dur.as_secs() / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}{m:02}{d:02}")
}

/// 合同号：YYYYMMDD + 当天流水 2 位（01、02、03…），如 2026092501
/// 流水计数按日期分组存于 sequence.json 的 "contract:<YYYYMMDD>" 键。
fn next_contract_no(app: &tauri::AppHandle) -> Result<String, String> {
    let today = today_compact();
    let count_key = format!("contract:{today}");
    let mut seq = read_seq(app);
    let mut n: u64 = seq
        .get(&count_key)
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    n += 1;
    seq[count_key.as_str()] = serde_json::json!(n);
    fs::write(seq_file(app)?, seq.to_string())
        .map_err(|e| format!("写入合同号序号失败: {e}"))?;
    Ok(format!("{today}{:02}", n))
}

/* ---------------- Commands ---------------- */

#[tauri::command]
fn order_new(app: tauri::AppHandle) -> Result<Order, String> {
    let no = next_order_no(&app)?;
    let contract_no = next_contract_no(&app)?;
    let now = timestamp_now();
    let mut order = Order::default();
    order.order_no = no.clone();
    order.meta.id = no.clone();
    order.meta.file_name = format!("{no}.json");
    order.meta.created_at = now.clone();
    order.meta.updated_at = now;
    // 自动生成合同号：YYYYMMDD + 当天流水 2 位
    order.fields.insert("contractNo".into(), contract_no.clone());
    // SQL 对齐名旧别名也一起更新，前端字体看不到也不影响
    order
        .fields
        .insert("mSales_Confirmation_No".into(), contract_no.clone());
    Ok(order)
}

#[tauri::command]
fn order_save(app: tauri::AppHandle, order: Order) -> Result<SaveResult, String> {
    let now = timestamp_now();
    let mut meta = order.meta.clone();
    let file_name = if meta.file_name.ends_with(".json") {
        meta.file_name.clone()
    } else {
        let base = if !meta.id.is_empty() {
            meta.id.clone()
        } else if !order.order_no.is_empty() {
            order.order_no.clone()
        } else {
            "order".to_string()
        };
        format!("{base}.json")
    };
    meta.file_name = file_name.clone();
    meta.updated_at = now;
    let mut full = order;
    full.meta = meta;

    let path = orders_dir(&app)?.join(&file_name);
    let json = serde_json::to_string_pretty(&full)
        .map_err(|e| format!("序列化失败: {e}"))?;
    fs::write(path, json).map_err(|e| format!("写入文件失败: {e}"))?;
    Ok(SaveResult {
        file_name,
        order: full,
    })
}

#[tauri::command]
fn order_open(app: tauri::AppHandle) -> Result<Option<Order>, String> {
    use tauri_plugin_dialog::DialogExt;
    let win = app
        .get_webview_window("main")
        .ok_or("找不到主窗口")?;
    let file = win.dialog().file().add_filter("工单文件", &["json"]).blocking_pick_file();
    let Some(file) = file else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|e| format!("获取路径失败: {e}"))?;
    if !path.exists() {
        return Err("文件不存在".into());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    let data: Order = serde_json::from_str(&raw)
        .map_err(|e| format!("文件格式错误: {e}"))?;
    // 自动归入 orders 目录，便于下次列出
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let _ = order_save(app, data.clone());

    Ok(Some(Order {
        meta: OrderMeta {
            file_name,
            ..data.meta.clone()
        },
        ..data
    }))
}

#[tauri::command]
fn order_save_as(app: tauri::AppHandle, order: Order) -> Result<Option<SaveResult>, String> {
    use tauri_plugin_dialog::DialogExt;
    let suggested = if !order.order_no.is_empty() {
        format!("{}.json", order.order_no)
    } else if !order.meta.id.is_empty() {
        format!("{}.json", order.meta.id)
    } else {
        "工单.json".to_string()
    };
    let win = app
        .get_webview_window("main")
        .ok_or("找不到主窗口")?;
    let file = win
        .dialog()
        .file()
        .add_filter("工单文件", &["json"])
        .set_file_name(suggested)
        .blocking_save_file();
    let Some(file) = file else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|e| format!("获取路径失败: {e}"))?;
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut full = order;
    full.meta.file_name = file_name.clone();
    fs::write(&path, serde_json::to_string_pretty(&full).map_err(|e| format!("序列化失败: {e}"))?)
        .map_err(|e| format!("写入文件失败: {e}"))?;
    Ok(Some(SaveResult {
        file_name,
        order: full,
    }))
}

#[tauri::command]
fn order_list(app: tauri::AppHandle) -> Result<Vec<OrderListItem>, String> {
    let dir = orders_dir(&app)?;
    let mut items = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            match fs::read_to_string(&path) {
                Ok(raw) => {
                    if let Ok(data) = serde_json::from_str::<Order>(&raw) {
                        // orderNo：优先用字段里的 orderNoDisplay / orderNo，回退到结构体 order_no
                        let order_no = data
                            .fields
                            .get("orderNoDisplay")
                            .cloned()
                            .map(|s| s.trim_start_matches("No. ").trim().to_string())
                            .filter(|s| !s.is_empty())
                            .or_else(|| data.fields.get("orderNo").cloned())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| data.order_no.clone());
                        // 列表显示名：订印单位（customer），回退产品规格
                        let product_name = data
                            .fields
                            .get("customer")
                            .cloned()
                            .filter(|s| !s.is_empty())
                            .or_else(|| data.fields.get("productSpec").cloned())
                            .unwrap_or_default();
                        items.push(OrderListItem {
                            id: data.meta.id.clone(),
                            file_name,
                            order_no,
                            product_name,
                            updated_at: data.meta.updated_at.clone(),
                        });
                    } else {
                        items.push(OrderListItem {
                            id: file_name.trim_end_matches(".json").to_string(),
                            file_name,
                            order_no: String::new(),
                            product_name: String::new(),
                            updated_at: String::new(),
                        });
                    }
                }
                Err(_) => {
                    items.push(OrderListItem {
                        id: file_name.trim_end_matches(".json").to_string(),
                        file_name,
                        order_no: String::new(),
                        product_name: String::new(),
                        updated_at: String::new(),
                    });
                }
            }
        }
    }
    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(items)
}

#[tauri::command]
fn order_load(app: tauri::AppHandle, file_name: String) -> Result<Order, String> {
    let path = orders_dir(&app)?.join(&file_name);
    if !path.exists() {
        return Err(format!("工单文件不存在: {file_name}"));
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("读取失败: {e}"))?;
    let data: Order = serde_json::from_str(&raw).map_err(|e| format!("文件格式错误: {e}"))?;
    Ok(data)
}

#[tauri::command]
fn order_delete(app: tauri::AppHandle, file_name: String) -> Result<bool, String> {
    let path = orders_dir(&app)?.join(&file_name);
    if !path.exists() {
        return Err(format!("工单文件不存在: {file_name}"));
    }
    fs::remove_file(&path).map_err(|e| format!("删除文件失败: {e}"))?;
    Ok(true)
}

#[tauri::command]
fn order_print(_app: tauri::AppHandle) -> Result<PrintResult, String> {
    // 打印由前端 window.print() 触发（WebView2 走系统打印对话框）
    Ok(PrintResult {
        ok: true,
        msg: "请在弹出的打印对话框中选择打印机".into(),
    })
}

/**
 * 复制工单：
 * - 深拷贝当前工单全部内容（fields/cks/meta 除 id/fileName）
 * - 生成全新工单号
 * - 开单日期 → 今天
 * - 交货日期 → 清空
 */
#[tauri::command]
fn order_duplicate(app: tauri::AppHandle, order: Order) -> Result<Order, String> {
    let no = next_order_no(&app)?;
    let contract_no = next_contract_no(&app)?;
    let now = timestamp_now();
    let today = today_cn();

    let mut new_order = Order {
        meta: OrderMeta {
            id: no.clone(),
            created_at: now.clone(),
            updated_at: now,
            file_name: format!("{no}.json"),
        },
        order_no: no.clone(),
        fields: order.fields.clone(),
        cks: order.cks.clone(),
    };

    // 工单号显示：更新为 No. 新单号（兼容两种存储键）
    new_order.fields.insert("orderNoDisplay".into(), format!("No. {no}"));
    new_order.fields.insert("mJob_No".into(), format!("No. {no}"));
    // 合同号：生成全新合同号（YYYYMMDD + 当天流水）
    new_order
        .fields
        .insert("contractNo".into(), contract_no.clone());
    new_order
        .fields
        .insert("mSales_Confirmation_No".into(), contract_no);
    // 开单日期 → 今天（SQL 对齐名 mJob_Date + 旧别名 openDate 都更新）
    new_order.fields.insert("mJob_Date".into(), today.clone());
    new_order.fields.insert("openDate".into(), today);
    // 交货日期 → 清空（mFinished_Date + deliverDate 都清）
    new_order.fields.insert("mFinished_Date".into(), String::new());
    new_order.fields.insert("deliverDate".into(), String::new());

    Ok(new_order)
}

/* ---------------- SQL 工单导入 ---------------- */

/// 工单主表名（MySQL dump 中的 `t_job`）
const JOB_TABLE: &str = "t_job";
/// 工单号列名
const JOB_NO_COL: &str = "mJob_No";

/// 选择 .sql 文件 → 解析 `t_job` 的 INSERT 数据 → 每条数据落盘为一个工单 JSON。
/// 已存在同名工单文件时直接覆盖。
#[tauri::command]
fn order_import_sql(app: tauri::AppHandle) -> Result<ImportResult, String> {
    use tauri_plugin_dialog::DialogExt;
    let win = app.get_webview_window("main").ok_or("找不到主窗口")?;
    let picked = win
        .dialog()
        .file()
        .add_filter("SQL 文件", &["sql"])
        .blocking_pick_file();
    let Some(file) = picked else {
        // 用户取消：不算错误
        return Ok(ImportResult {
            count: 0,
            skipped: 0,
            message: "已取消导入".into(),
        });
    };
    let path = file.into_path().map_err(|e| format!("获取路径失败: {e}"))?;
    if !path.exists() {
        return Err("文件不存在".into());
    }
    // 文件可能很大（几十 MB）：整体读入后按字节扫描；非 UTF-8 内容做有损转换，避免直接失败
    let bytes = fs::read(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    let sql = String::from_utf8_lossy(&bytes).into_owned();

    let now = timestamp_now();
    let dir = orders_dir(&app)?;
    let mut count = 0usize;
    let mut failed = 0usize;
    // 边解析边落盘：避免大 dump 一次性把所有工单留在内存里
    let skipped = for_each_job_from_sql(&sql, &now, |order| {
        match serde_json::to_string_pretty(&order) {
            Ok(json) => {
                if fs::write(dir.join(&order.meta.file_name), json).is_ok() {
                    count += 1;
                } else {
                    failed += 1;
                }
            }
            Err(_) => failed += 1,
        }
    });
    let skipped = skipped + failed;

    let message = if count == 0 {
        if skipped == 0 {
            format!("SQL 中未找到 {JOB_TABLE} 工单数据")
        } else {
            format!("未导入工单，跳过 {skipped} 条无效数据行")
        }
    } else if skipped == 0 {
        format!("已导入 {count} 条工单")
    } else {
        format!("已导入 {count} 条工单，跳过 {skipped} 条")
    };
    Ok(ImportResult {
        count,
        skipped,
        message,
    })
}

/// 扫描 SQL 文本，每解析出一条工单就回调一次；返回跳过的数据行数。
fn for_each_job_from_sql<F: FnMut(Order)>(sql: &str, now: &str, mut on_job: F) -> usize {
    let bytes = sql.as_bytes();
    // INSERT 不带列名时的回退：用 CREATE TABLE 的列定义顺序
    let fallback_cols = create_table_columns(sql, JOB_TABLE);

    let mut skipped = 0usize;
    let mut seen: HashSet<String> = HashSet::new();
    let mut pos = 0usize;

    while let Some(rel) = find_ci(&bytes[pos..], b"INSERT INTO") {
        let start = pos + rel;
        let Some((table, cols, values_at)) = parse_insert_head(bytes, start) else {
            pos = start + INS_KW_LEN;
            continue;
        };
        if !table.eq_ignore_ascii_case(JOB_TABLE) {
            // 非目标表：只跳到语句末尾，不解析值（大 dump 里可省大量开销）
            pos = skip_statement(bytes, values_at);
            continue;
        }
        let columns: Vec<String> = if cols.is_empty() {
            fallback_cols.clone()
        } else {
            cols
        };
        if columns.is_empty() {
            // 没有列名可映射，整条语句作废
            let (tuples, end) = split_value_tuples(bytes, values_at);
            skipped += tuples.len();
            pos = end.max(start + INS_KW_LEN);
            continue;
        }
        let (tuples, end) = split_value_tuples(bytes, values_at);
        pos = end.max(start + INS_KW_LEN);
        for tuple in tuples {
            match tuple_to_order(&columns, &tuple, now) {
                Some(order) if seen.insert(order.order_no.clone()) => on_job(order),
                // 同一份 dump 里重复单号：保留首条，其余跳过
                Some(_) => skipped += 1,
                None => skipped += 1,
            }
        }
    }
    skipped
}

const INS_KW_LEN: usize = 11; // `INSERT INTO`.len()

/// 解析 INSERT 头部：表名、可选列名列表、VALUES 之后值的起始位置
fn parse_insert_head(bytes: &[u8], i: usize) -> Option<(String, Vec<String>, usize)> {
    let kw = b"INSERT INTO";
    if bytes.len() < i + kw.len() || !bytes[i..i + kw.len()].eq_ignore_ascii_case(kw) {
        return None;
    }
    // 表名：`tbl` 或 tbl
    let mut p = skip_ws(bytes, i + kw.len());
    let table = if p < bytes.len() && bytes[p] == b'`' {
        let start = p + 1;
        let end = start + bytes[start..].iter().position(|&c| c == b'`')?;
        p = end + 1;
        String::from_utf8_lossy(&bytes[start..end]).to_string()
    } else {
        let start = p;
        while p < bytes.len()
            && (bytes[p].is_ascii_alphanumeric()
                || matches!(bytes[p], b'_' | b'$' | b'.'))
        {
            p += 1;
        }
        if p == start {
            return None;
        }
        String::from_utf8_lossy(&bytes[start..p]).to_string()
    };
    // VALUES 关键字（表名与 VALUES 之间只可能是列名列表，不含字符串字面量）
    let values_at = find_ci(&bytes[p..], b"VALUES")? + p + b"VALUES".len();
    // 列名列表：表名与 VALUES 之间的括号内容
    let mut cols = Vec::new();
    let seg = &bytes[p..values_at];
    if let (Some(open), Some(close)) = (
        seg.iter().position(|&c| c == b'('),
        seg.iter().rposition(|&c| c == b')'),
    ) {
        if close > open {
            for raw in split_top_level_commas(&seg[open + 1..close]) {
                let name = clean_ident(raw);
                if !name.is_empty() {
                    cols.push(name);
                }
            }
        }
    }
    Some((table, cols, values_at))
}

/// 跳过语句剩余部分（字符串感知），返回 `;` 之后的位置
fn skip_statement(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => {
                let (_s, next) = read_sql_string(bytes, i + 1);
                i = next.max(i + 1);
            }
            b';' => return i + 1,
            _ => i += 1,
        }
    }
    i
}

/// 解析 `(v1, v2, ...), (...)` 形式的值载荷。
/// 返回（每行的值列表, 停止位置）；顶层 `;` 或下一条 INSERT 处停止。
fn split_value_tuples(bytes: &[u8], mut i: usize) -> (Vec<Vec<Option<String>>>, usize) {
    let n = bytes.len();
    let mut tuples: Vec<Vec<Option<String>>> = Vec::new();
    let mut depth: i32 = 0;
    let mut body_start = usize::MAX;

    while i < n {
        match bytes[i] {
            b'\'' => {
                let (_s, next) = read_sql_string(bytes, i + 1);
                i = next.max(i + 1);
            }
            b'(' => {
                if depth == 0 {
                    body_start = i + 1;
                }
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                if depth <= 0 {
                    if depth == 0 && body_start != usize::MAX {
                        tuples.push(split_tuple_values(&bytes[body_start..i]));
                    }
                    depth = 0;
                    body_start = usize::MAX;
                }
                i += 1;
            }
            b';' if depth == 0 => {
                i += 1;
                break;
            }
            b'I' | b'i' if depth == 0 => {
                // 容错：语句未以 `;` 结尾时，遇到下一条 INSERT 即停止
                if bytes.len() >= i + INS_KW_LEN
                    && bytes[i..i + INS_KW_LEN].eq_ignore_ascii_case(b"INSERT INTO")
                {
                    break;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    (tuples, i)
}

/// 拆分一行元组正文里的值（按顶层逗号，跳过括号与字符串内部的逗号）
fn split_tuple_values(body: &[u8]) -> Vec<Option<String>> {
    split_top_level_commas(body)
        .into_iter()
        .map(parse_sql_value)
        .collect()
}

/// 按顶层逗号切分（字符串、括号内的逗号不算）
fn split_top_level_commas(body: &[u8]) -> Vec<&[u8]> {
    let n = body.len();
    let mut out: Vec<&[u8]> = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0usize;
    let mut i = 0usize;
    while i < n {
        match body[i] {
            b'\'' => {
                let (_s, next) = read_sql_string(body, i + 1);
                i = next.max(i + 1);
            }
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
            }
            b',' if depth <= 0 => {
                out.push(&body[start..i]);
                start = i + 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    out.push(&body[start..]);
    out
}

/// 解析单个 SQL 值：NULL / 空串 → None；'...' → 反转义后的内容；其余原样
fn parse_sql_value(raw: &[u8]) -> Option<String> {
    let t = trim_ascii(raw);
    if t.is_empty() || t.eq_ignore_ascii_case(b"NULL") {
        return None;
    }
    let s = if t[0] == b'\'' {
        read_sql_string(t, 1).0
    } else {
        String::from_utf8_lossy(t).to_string()
    };
    let s = s.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// 读取以单引号起始的 SQL 字符串，`i` 指向开引号之后。
/// 返回（反转义后的内容, 结束引号之后的位置）。
fn read_sql_string(bytes: &[u8], mut i: usize) -> (String, usize) {
    let n = bytes.len();
    let mut out: Vec<u8> = Vec::new();
    while i < n {
        let c = bytes[i];
        if c == b'\\' && i + 1 < n {
            let e = bytes[i + 1];
            match e {
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'0' => out.push(0),
                b'b' => out.push(8),
                b'Z' => out.push(26),
                // MySQL 中 \% \_ 保留反斜杠
                b'%' => out.extend_from_slice(b"\\%"),
                b'_' => out.extend_from_slice(b"\\_"),
                // \\ \' \" 及其它未知转义 → 字符本身
                other => out.push(other),
            }
            i += 2;
            continue;
        }
        if c == b'\'' {
            // '' 视为转义的单引号；否则字符串结束
            if i + 1 < n && bytes[i + 1] == b'\'' {
                out.push(b'\'');
                i += 2;
                continue;
            }
            return (String::from_utf8_lossy(&out).to_string(), i + 1);
        }
        out.push(c);
        i += 1;
    }
    (String::from_utf8_lossy(&out).to_string(), i)
}

/// 从 CREATE TABLE 定义中按顺序取出列名（跳过 PRIMARY KEY / KEY / UNIQUE 等约束行）
fn create_table_columns(sql: &str, table: &str) -> Vec<String> {
    let bytes = sql.as_bytes();
    let needle = format!("CREATE TABLE `{table}`");
    let Some(p) = find_ci(bytes, needle.as_bytes()) else {
        return Vec::new();
    };
    // 定位列定义开始的 '('
    let mut i = p + needle.len();
    while i < bytes.len() && bytes[i] != b'(' {
        if bytes[i] == b';' {
            return Vec::new();
        }
        i += 1;
    }
    if i >= bytes.len() {
        return Vec::new();
    }

    let mut cols = Vec::new();
    let mut depth: i32 = 1; // 已吃掉最外层 '('
    let mut at_item_start = true;
    i += 1;
    while i < bytes.len() {
        let c = bytes[i];
        // 空白不改变「行首」状态，便于跳过缩进
        if is_ws(c) {
            i += 1;
            continue;
        }
        if c == b'`' && at_item_start {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'`' {
                j += 1;
            }
            let name = String::from_utf8_lossy(&bytes[start..j]).trim().to_string();
            if !name.is_empty() {
                cols.push(name);
            }
            at_item_start = false;
            i = if j < bytes.len() { j + 1 } else { j };
            continue;
        }
        if c == b'\'' {
            let (_s, next) = read_sql_string(bytes, i + 1);
            i = next.max(i + 1);
            at_item_start = false;
            continue;
        }
        if c == b'(' {
            depth += 1;
            at_item_start = false;
            i += 1;
            continue;
        }
        if c == b')' {
            depth -= 1;
            if depth == 0 {
                break;
            }
            at_item_start = false;
            i += 1;
            continue;
        }
        // 顶层逗号 = 下一个列定义开始
        if c == b',' && depth == 1 {
            at_item_start = true;
            i += 1;
            continue;
        }
        at_item_start = false;
        i += 1;
    }
    cols
}

/// 一行 SQL 数据 → 工单；无工单号或全空则返回 None
fn tuple_to_order(columns: &[String], values: &[Option<String>], now: &str) -> Option<Order> {
    let mut fields: HashMap<String, String> = HashMap::new();
    for (idx, col) in columns.iter().enumerate() {
        if let Some(Some(v)) = values.get(idx) {
            if !v.is_empty() {
                fields.entry(col.clone()).or_insert_with(|| v.clone());
            }
        }
    }
    if fields.is_empty() {
        return None;
    }
    let order_no = clean_order_no(fields.get(JOB_NO_COL).map(|s| s.as_str()).unwrap_or(""));
    if order_no.is_empty() {
        return None;
    }
    Some(Order {
        meta: OrderMeta {
            id: order_no.clone(),
            created_at: now.to_string(),
            updated_at: now.to_string(),
            file_name: format!("{order_no}.json"),
        },
        order_no,
        fields,
        cks: HashMap::new(),
    })
}

/// 单号规范化：去首尾空白、去 `No.` 前缀、替换文件名非法字符
fn clean_order_no(raw: &str) -> String {
    let s = raw.trim();
    let s = if s.as_bytes().len() >= 3 && s.as_bytes()[..3].eq_ignore_ascii_case(b"no.") {
        s[3..].trim()
    } else {
        s
    };
    let cleaned: String = s
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    let cleaned = cleaned.trim().to_string();
    // 防止超长单号导致路径过长
    if cleaned.chars().count() > 120 {
        cleaned.chars().take(120).collect()
    } else {
        cleaned
    }
}

/// 去掉标识符的反引号/引号并 trim
fn clean_ident(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    let t = s.trim().trim_matches('`').trim_matches('\'').trim_matches('"');
    t.trim().to_string()
}

/// ASCII 空白字符
fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n')
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && is_ws(bytes[i]) {
        i += 1;
    }
    i
}

/// 去除首尾 ASCII 空白
fn trim_ascii(mut b: &[u8]) -> &[u8] {
    while let Some((f, rest)) = b.split_first() {
        if is_ws(*f) {
            b = rest;
        } else {
            break;
        }
    }
    while let Some((l, rest)) = b.split_last() {
        if is_ws(*l) {
            b = rest;
        } else {
            break;
        }
    }
    b
}

/// ASCII 忽略大小写查找子串，返回下标
fn find_ci(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    let first = needle[0].to_ascii_lowercase();
    for i in 0..=(haystack.len() - needle.len()) {
        if haystack[i].to_ascii_lowercase() != first {
            continue;
        }
        if haystack[i..i + needle.len()].eq_ignore_ascii_case(needle) {
            return Some(i);
        }
    }
    None
}

/* ---------------- 工具函数 ---------------- */

fn timestamp_now() -> String {
    // 标准 ISO8601 本地时间（无外部依赖）
    let now = std::time::SystemTime::now();
    let dur = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let utc_secs = dur.as_secs();
    let days = utc_secs / 86400;
    let secs_of_day = utc_secs % 86400;
    // 公历转日期（2038 安全）
    let (y, m, d) = civil_from_days(days as i64);
    let (h, mi, s) = (
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60,
    );
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        h,
        mi,
        s
    )
}

/// 今天的日期，中文格式：`2026 年 9 月 22 日`（用于开单日期/复制）
fn today_cn() -> String {
    let now = std::time::SystemTime::now();
    let dur = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let days = dur.as_secs() / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y} 年 {m} 月 {d} 日")
}

/// 自 1970-01-01 的天数转公历日期（Hinnant 算法）
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/* ---------------- 入口 ---------------- */

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 开发调试：打开 devtools 查看 JS 错误
            #[cfg(debug_assertions)]
            if let Some(win) = app.get_webview_window("main") {
                win.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            order_new,
            order_save,
            order_open,
            order_save_as,
            order_list,
            order_load,
            order_delete,
            order_print,
            order_duplicate,
            order_import_sql
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/* ---------------- 测试：SQL 解析 ---------------- */

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-09-25T00:00:00Z";

    /// 测试辅助：解析全部工单
    fn parse(sql: &str) -> (Vec<Order>, usize) {
        let mut orders = Vec::new();
        let skipped = for_each_job_from_sql(sql, NOW, |o| orders.push(o));
        (orders, skipped)
    }

    #[test]
    fn parse_with_column_list_and_escapes() {
        let sql = "\
INSERT INTO `t_job` (`t_Job_ID`, `mJob_No`, `mRemarks`, `mQty_Job`, `mUnit`) VALUES (1,'0022379','第一行\\n第二行',1000,'个'),(2,'0022380','含\\'引号\\' 与 ''双写''',NULL,'张');
";
        let (orders, skipped) = parse(sql);
        assert_eq!(skipped, 0);
        assert_eq!(orders.len(), 2);

        let a = &orders[0];
        assert_eq!(a.order_no, "0022379");
        assert_eq!(a.meta.file_name, "0022379.json");
        assert_eq!(a.meta.id, "0022379");
        assert_eq!(a.meta.created_at, NOW);
        assert_eq!(a.fields.get("mRemarks").unwrap(), "第一行\n第二行");
        assert_eq!(a.fields.get("mUnit").unwrap(), "个");
        assert_eq!(a.fields.get("t_Job_ID").unwrap(), "1");
        assert!(a.cks.is_empty());

        let b = &orders[1];
        assert_eq!(b.order_no, "0022380");
        assert_eq!(b.fields.get("mRemarks").unwrap(), "含'引号' 与 '双写'");
        // NULL 列不入 fields
        assert!(!b.fields.contains_key("mQty_Job"));
    }

    #[test]
    fn parse_without_column_list_uses_create_table() {
        let sql = "\
CREATE TABLE `t_job` (
  `t_Job_ID` int(11) NOT NULL auto_increment,
  `mJob_No` varchar(255) default NULL,
  `mFinished_Date` varchar(255) default NULL,
  `mQty` decimal(19,2) default NULL,
  PRIMARY KEY  (`t_Job_ID`),
  UNIQUE KEY `mJob_No` (`mJob_No`)
) ENGINE=MyISAM DEFAULT CHARSET=gb2312;
INSERT INTO `t_job` VALUES (7,'No. 0022379','2026-09-20','12.50'),(8,'','2026-09-21','9.00');
";
        let (orders, skipped) = parse(sql);
        assert_eq!(orders.len(), 1, "空单号行应被跳过");
        assert_eq!(skipped, 1);
        assert_eq!(orders[0].order_no, "0022379", "应去掉 No. 前缀");
        assert_eq!(orders[0].meta.file_name, "0022379.json");
        assert_eq!(orders[0].fields.get("mQty").unwrap(), "12.50");
        assert_eq!(
            create_table_columns(sql, JOB_TABLE),
            vec!["t_Job_ID", "mJob_No", "mFinished_Date", "mQty"]
        );
    }

    #[test]
    fn ignores_other_tables_and_stops_at_semicolon() {
        let sql = "\
INSERT INTO `t_job` (`mJob_No`, `mRemarks`) VALUES ('0001','a;b, c');
INSERT INTO `t_other` (`mJob_No`) VALUES ('xxxx');
LOCK TABLES `t_job` WRITE;
INSERT INTO `t_job` (`mJob_No`) VALUES ('0002');
";
        let (orders, _skipped) = parse(sql);
        assert_eq!(orders.len(), 2);
        assert_eq!(orders[0].order_no, "0001");
        assert_eq!(orders[0].fields.get("mRemarks").unwrap(), "a;b, c");
        assert_eq!(orders[1].order_no, "0002");
    }

    #[test]
    fn duplicate_order_no_keeps_first() {
        let sql = "INSERT INTO `t_job` (`mJob_No`,`mUnit`) VALUES ('0001','个'),('0001','张');";
        let (orders, skipped) = parse(sql);
        assert_eq!(orders.len(), 1);
        assert_eq!(skipped, 1);
        assert_eq!(orders[0].fields.get("mUnit").unwrap(), "个");
    }

    #[test]
    fn malformed_and_empty_input_are_safe() {
        let (orders, skipped) = parse("");
        assert!(orders.is_empty() && skipped == 0);

        // 语句缺少结尾分号：不得把后续内容吞进来
        let sql = "INSERT INTO `t_job` (`mJob_No`) VALUES ('0003')";
        let (orders, _) = parse(sql);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].order_no, "0003");

        // 无 t_job 数据
        let sql = "INSERT INTO `t_gd` VALUES (1,'x');";
        let (orders, skipped) = parse(sql);
        assert!(orders.is_empty() && skipped == 0);
    }

    #[test]
    fn order_no_sanitized_for_file_name() {
        assert_eq!(clean_order_no(" No. 0022379 "), "0022379");
        assert_eq!(clean_order_no("0022379"), "0022379");
        assert_eq!(clean_order_no("A/B:C*D?E\"F<G>H|I"), "A_B_C_D_E_F_G_H_I");
        assert_eq!(clean_order_no("   "), "");
    }

    #[test]
    fn string_unescape_handles_mysql_escapes() {
        let (s, end) = read_sql_string(b"a\\'b\\\\c\\nd\\te''f' tail", 0);
        assert_eq!(s, "a'b\\c\nd\te'f");
        assert_eq!(&"a\\'b\\\\c\\nd\\te''f' tail"[end..], " tail");
    }
}
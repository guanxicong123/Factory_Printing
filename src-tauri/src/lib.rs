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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderListItem {
    pub id: String,
    pub file_name: String,
    pub order_no: String,
    pub product_name: String,
    pub updated_at: String,
}

/// 分页列表结果：只返回当页摘要，避免几万条一次性序列化到前端
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrderListPage {
    pub items: Vec<OrderListItem>,
    pub total: usize,
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

/// 合同号：LBP + YYYYMMDD + 当天流水 2 位（01、02、03…），如 LBP2026091501
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
    Ok(format!("LBP{today}{:02}", n))
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
    // 自动生成合同号：LBP + YYYYMMDD + 当天流水 2 位
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

fn order_list_item_placeholder(file_name: &str) -> OrderListItem {
    OrderListItem {
        id: file_name.trim_end_matches(".json").to_string(),
        file_name: file_name.to_string(),
        order_no: String::new(),
        product_name: String::new(),
        updated_at: String::new(),
    }
}

fn order_to_list_item(data: &Order, file_name: &str) -> OrderListItem {
    let order_no = data
        .fields
        .get("orderNoDisplay")
        .cloned()
        .map(|s| s.trim_start_matches("No. ").trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| data.fields.get("orderNo").cloned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| data.order_no.clone());
    let product_name = data
        .fields
        .get("customer")
        .cloned()
        .filter(|s| !s.is_empty())
        .or_else(|| data.fields.get("productSpec").cloned())
        .unwrap_or_default();
    OrderListItem {
        id: data.meta.id.clone(),
        file_name: file_name.to_string(),
        order_no,
        product_name,
        updated_at: data.meta.updated_at.clone(),
    }
}

/// 从单个工单文件解析列表摘要（解析失败时仍返回占位项）
fn order_list_item_from_path(path: &std::path::Path, file_name: &str) -> OrderListItem {
    match fs::read_to_string(path) {
        Ok(raw) => {
            if let Ok(data) = serde_json::from_str::<Order>(&raw) {
                order_to_list_item(&data, file_name)
            } else {
                order_list_item_placeholder(file_name)
            }
        }
        Err(_) => order_list_item_placeholder(file_name),
    }
}

/// 关键字是否命中单号 / 客户 / 合同号 / 产品名
fn order_matches_keyword(data: &Order, item: &OrderListItem, kw: &str) -> bool {
    let mut hay: Vec<&str> = vec![
        item.order_no.as_str(),
        item.product_name.as_str(),
        data.order_no.as_str(),
        data.meta.id.as_str(),
        data.meta.file_name.as_str(),
    ];
    for key in [
        "customer",
        "contractNo",
        "productSpec",
        "mProduct_Name",
        "mCustomer_FullName",
        "orderNo",
        "orderNoDisplay",
    ] {
        if let Some(v) = data.fields.get(key) {
            hay.push(v.as_str());
        }
    }
    hay.iter().any(|s| s.to_lowercase().contains(kw))
}

/// 有搜索词时：解析文件，命中则返回摘要
fn order_list_item_if_match(
    path: &std::path::Path,
    file_name: &str,
    kw: &str,
) -> Option<OrderListItem> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            if let Ok(data) = serde_json::from_str::<Order>(&raw) {
                let item = order_to_list_item(&data, file_name);
                if order_matches_keyword(&data, &item, kw) {
                    Some(item)
                } else {
                    None
                }
            } else {
                let item = order_list_item_placeholder(file_name);
                if file_name.to_lowercase().contains(kw) {
                    Some(item)
                } else {
                    None
                }
            }
        }
        Err(_) => None,
    }
}

/// 分页列出工单：按文件 mtime 降序。
/// 无 q 时只解析当页；有 q 时全库过滤后再分页（匹配单号/客户/合同号/产品名）。
#[tauri::command]
fn order_list(
    app: tauri::AppHandle,
    limit: Option<usize>,
    offset: Option<usize>,
    q: Option<String>,
) -> Result<OrderListPage, String> {
    let dir = orders_dir(&app)?;
    let mut entries: Vec<(PathBuf, String, std::time::SystemTime)> = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            entries.push((path, file_name, mtime));
        }
    }
    entries.sort_by(|a, b| b.2.cmp(&a.2));

    let kw = q
        .as_ref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());

    let items_all: Vec<OrderListItem> = if let Some(ref keyword) = kw {
        entries
            .iter()
            .filter_map(|(path, file_name, _)| {
                order_list_item_if_match(path, file_name, keyword)
            })
            .collect()
    } else {
        // 无搜索：先切片再解析，避免几万条全量反序列化
        let total = entries.len();
        let offset = offset.unwrap_or(0).min(total);
        let limit = limit.unwrap_or(total.saturating_sub(offset));
        let end = (offset + limit).min(total);
        let items = entries[offset..end]
            .iter()
            .map(|(path, file_name, _)| order_list_item_from_path(path, file_name))
            .collect();
        return Ok(OrderListPage { items, total });
    };

    let total = items_all.len();
    let offset = offset.unwrap_or(0).min(total);
    // 未传 limit 时返回全部（兼容旧调用）；侧栏应显式传 limit
    let limit = limit.unwrap_or(total.saturating_sub(offset));
    let end = (offset + limit).min(total);
    Ok(OrderListPage {
        items: items_all[offset..end].to_vec(),
        total,
    })
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
    // 合同号：生成全新合同号（LBP + YYYYMMDD + 当天流水）
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

/// 真实工单表（进发旧库 `t_gd`）
const GD_TABLE: &str = "t_gd";
/// 兼容旧导入目标（结构不同，字段原样落入 fields）
const JOB_TABLE: &str = "t_job";
const GD_NO_COL: &str = "mGD_No";
const JOB_NO_COL: &str = "mJob_No";

/// 选择 .sql 文件 → 解析 `t_gd`（优先）/ `t_job` → 落盘为工单 JSON。
/// 已存在同名单号文件时直接覆盖。
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
    let bytes = fs::read(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    let sql = decode_sql_bytes(&bytes);

    let now = timestamp_now();
    let dir = orders_dir(&app)?;
    let mut count = 0usize;
    let mut failed = 0usize;
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
            format!("SQL 中未找到 {GD_TABLE} / {JOB_TABLE} 工单数据")
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

/// dump 可能是 UTF-8（mysqldump SET NAMES utf8）或 GBK/GB2312 原始库编码
fn decode_sql_bytes(bytes: &[u8]) -> String {
    if std::str::from_utf8(bytes).is_ok() {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let (cow, _, _) = encoding_rs::GBK.decode(bytes);
    cow.into_owned()
}

/// 扫描 SQL：优先吃 `t_gd`，兼容 `t_job`；返回跳过的数据行数。
fn for_each_job_from_sql<F: FnMut(Order)>(sql: &str, now: &str, mut on_job: F) -> usize {
    let bytes = sql.as_bytes();
    let fallback_gd = create_table_columns(sql, GD_TABLE);
    let fallback_job = create_table_columns(sql, JOB_TABLE);

    let mut skipped = 0usize;
    let mut seen: HashSet<String> = HashSet::new();
    let mut pos = 0usize;

    while let Some(rel) = find_ci(&bytes[pos..], b"INSERT INTO") {
        let start = pos + rel;
        let Some((table, cols, values_at)) = parse_insert_head(bytes, start) else {
            pos = start + INS_KW_LEN;
            continue;
        };
        let is_gd = table.eq_ignore_ascii_case(GD_TABLE);
        let is_job = table.eq_ignore_ascii_case(JOB_TABLE);
        if !is_gd && !is_job {
            pos = skip_statement(bytes, values_at);
            continue;
        }
        let columns: Vec<String> = if cols.is_empty() {
            if is_gd {
                fallback_gd.clone()
            } else {
                fallback_job.clone()
            }
        } else {
            cols
        };
        if columns.is_empty() {
            let (tuples, end) = split_value_tuples(bytes, values_at);
            skipped += tuples.len();
            pos = end.max(start + INS_KW_LEN);
            continue;
        }
        let (tuples, end) = split_value_tuples(bytes, values_at);
        pos = end.max(start + INS_KW_LEN);
        for tuple in tuples {
            let order = if is_gd {
                gd_tuple_to_order(&columns, &tuple, now)
            } else {
                job_tuple_to_order(&columns, &tuple, now)
            };
            match order {
                Some(order) if seen.insert(order.order_no.clone()) => on_job(order),
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

/// 一行 `t_gd` 数据 → 工单（列名映射到前端 fields/cks）
fn gd_tuple_to_order(columns: &[String], values: &[Option<String>], now: &str) -> Option<Order> {
    let mut raw: HashMap<String, String> = HashMap::new();
    for (idx, col) in columns.iter().enumerate() {
        if let Some(Some(v)) = values.get(idx) {
            if !v.is_empty() {
                raw.entry(col.clone()).or_insert_with(|| v.clone());
            }
        }
    }
    if raw.is_empty() {
        return None;
    }
    let order_no = clean_order_no(raw.get(GD_NO_COL).map(|s| s.as_str()).unwrap_or(""));
    if order_no.is_empty() {
        return None;
    }

    let mut fields: HashMap<String, String> = HashMap::new();
    let mut cks: HashMap<String, bool> = HashMap::new();

    // 文本/日期字段映射（一列可写多个前端别名）
    for (sql_col, aliases) in GD_TEXT_MAP {
        let Some(v) = raw.get(*sql_col) else { continue };
        if *sql_col == "mPrice" && is_zeroish_decimal(v) {
            continue;
        }
        let mapped = if *sql_col == "mGD_Date" || *sql_col == "mGD_D_Date" {
            sql_datetime_to_cn(v)
        } else {
            v.clone()
        };
        if mapped.is_empty() {
            continue;
        }
        for alias in *aliases {
            fields.entry((*alias).to_string()).or_insert_with(|| mapped.clone());
        }
    }

    // 勾选：smallint / varchar(√、V、1…)
    for (sql_col, ck_key) in GD_CK_MAP {
        if let Some(v) = raw.get(*sql_col) {
            if is_sql_truthy(v) {
                cks.insert((*ck_key).to_string(), true);
            }
        }
    }

    // 啤版：mBB → piVersion（有值视为旧版，与表单默认一致）
    if raw.get("mBB").map(|s| is_sql_truthy(s)).unwrap_or(false) {
        fields.entry("piVersion".into()).or_insert_with(|| "old".into());
    }

    // 其它工序：有文案则勾选
    if fields.get("gQiTaText").map(|s| !s.trim().is_empty()).unwrap_or(false) {
        cks.insert("gQiTa".into(), true);
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
        cks,
    })
}

/// `t_gd` 文本列 → 前端 fields 别名
const GD_TEXT_MAP: &[(&str, &[&str])] = &[
    ("mGD_Date", &["openDate", "mJob_Date"]),
    ("mGD_D_Date", &["deliverDate", "mFinished_Date"]),
    ("mDYDW", &["customer", "mCustomer_FullName"]),
    ("mHTH", &["contractNo", "mSales_Confirmation_No"]),
    ("mPrice", &["costUnitPrice"]),
    ("mCPMC", &["productSpec", "mProduct_Name"]),
    ("mHMY", &["numFrom"]),
    ("mHML", &["numLian"]),
    ("mDYSL", &["orderQty", "mQty_Job"]),
    ("mDYSLY", &["numYeBen"]),
    ("mLKX", &["xinJian"]),
    ("mLKJ", &["jiuJian"]),
    ("mLKG", &["gongJian"]),
    ("mLKZ", &["yiZhaoJian"]),
    ("mDKX", &["dkXin"]),
    ("mDKJ", &["dkJiu"]),
    ("mDKG", &["dkGong"]),
    ("mDKZ", &["dkZhao"]),
    ("mPBSLH", &["pinbanH"]),
    ("mPBSLS", &["pinbanS"]),
    ("mPBSLB", &["remark", "mRemarks"]),
    ("mZL1", &["paperType"]),
    ("mZL2", &["paperType2"]),
    ("mZL3", &["paperType3"]),
    ("mZL4", &["paperType4"]),
    ("mZL5", &["paperType5"]),
    ("mFZS1", &["paperCount"]),
    ("mFZS2", &["paperCount2"]),
    ("mFZS3", &["paperCount3"]),
    ("mFZS4", &["paperCount4"]),
    ("mFZS5", &["paperCount5"]),
    ("mKZ1", &["sz47_5"]),
    ("mKZ2", &["sz64_5"]),
    ("mKZ3", &["kaifangMian"]),
    ("mKZ4", &["sz47_3"]),
    ("mKZ5", &["sz64_3"]),
    ("mKZ6", &["kaifangDi"]),
    ("mZB1", &["print_paper1"]),
    ("mYS1", &["print_color1"]),
    ("mSYS1", &["print_qty1"]),
    ("mFS1", &["print_extra1"]),
    ("mZB2", &["print_paper2"]),
    ("mYS2", &["print_color2"]),
    ("mSYS2", &["print_qty2"]),
    ("mFS2", &["print_extra2"]),
    ("mZB3", &["print_paper3"]),
    ("mYS3", &["print_color3"]),
    ("mSYS3", &["print_qty3"]),
    ("mFS3", &["print_extra3"]),
    ("mZB4", &["print_paper4"]),
    ("mYS4", &["print_color4"]),
    ("mSYS4", &["print_qty4"]),
    ("mFS4", &["print_extra4"]),
    ("mZB5", &["print_paper5"]),
    ("mYS5", &["print_color5"]),
    ("mSYS5", &["print_qty5"]),
    ("mFS5", &["print_extra5"]),
    ("mJYSMB", &["print_note"]),
    ("mBTSSM", &["houGongxuNote"]),
    ("mQT", &["gQiTaText"]),
    ("mZDQT", &["zQiTaText"]),
    ("mZDZ", &["zZhangCount"]),
    ("mZDB", &["zBenCount"]),
    ("mZDMB", &["zMeiBenFen"]),
    ("mZDTSSM", &["zTeshushuoming"]),
    ("mCPGGH", &["fkHeng"]),
    ("mCPGGS", &["fkShu"]),
    ("mCPGGT1", &["sbTou"]),
    ("mCPGGJ1", &["sbJiao"]),
    ("mCPGGZ1", &["sbZuo"]),
    ("mCPGGY1", &["sbYou"]),
    ("mCPGGT2", &["sbTou2"]),
    ("mCPGGJ2", &["sbJiao2"]),
    ("mCPGGZ2", &["sbZuo2"]),
    ("mCPGGY2", &["sbYou2"]),
    ("mCPGGTSSM", &["fkSpecial"]),
    ("mKDR", &["signedBy", "mOperator"]),
    ("mYWY", &["business"]),
    ("mYSQDG", &["proofread"]),
    ("mKZTYB", &["paperNote"]),
];

/// `t_gd` 勾选列 → cks
const GD_CK_MAP: &[(&str, &str)] = &[
    ("mLK", "liukai"),
    ("mDK", "duikai"),
    ("mGJD", "gGuangJiaoDan"),
    ("mGJS", "gGuangJiaoShuang"),
    ("mYJD", "gYaJiaoDan"),
    ("mYJS", "gYaJiaoShuang"),
    ("mMG", "gMoGuang"),
    ("mXSY", "gXiSuYou"),
    ("mTJ", "gTangJin"),
    ("mTY", "gTangYin"),
    ("mGY", "gGuoYou"),
    ("mUV", "gUV"),
    ("mAT", "gAoTu"),
    ("mYW", "gYaWen"),
    ("mB", "gPi"),
    ("mT", "gTie"),
    ("mBZ", "gBiaoZhi"),
    ("mBEK", "gZhanKeng"),
    ("mDKO", "gDaKong"),
    ("mJY", "gJiYan"),
    ("mYX", "gYaXian"),
    ("mBCX", "gPiPiJin"),
    ("mTPVC", "gTiePVC"),
    ("mZDSZ", "zSanZhang"),
    ("mZDQD", "zQiDing"),
    ("mZDSX", "zSuoXian"),
    ("mZDJZ", "zJiaoZhuang"),
    ("mZDJS", "zJinSong"),
    ("mZDSS", "zShiSong"),
    ("mCPGGYCM", "bxYouChangMing"),
    ("mCPGGWCM", "bxWuChangMing"),
    ("mCPGGZB", "bxZhiBao"),
    ("mCPGGZX", "bxZhiXiang"),
    ("mSUZI1", "sz119a"),
    ("mSUZI2", "sz109a"),
    ("mSUZI3", "sz089a"),
    ("mSUZI4", "sz079a"),
    ("mSUZI5", "sz119b"),
    ("mSUZI6", "sz109b"),
    ("mSUZI7", "sz089b"),
    ("mSUZI8", "sz079b"),
];

fn is_sql_truthy(v: &str) -> bool {
    let t = v.trim();
    if t.is_empty() {
        return false;
    }
    if t == "0" || t.eq_ignore_ascii_case("false") || t.eq_ignore_ascii_case("null") {
        return false;
    }
    true
}

fn is_zeroish_decimal(v: &str) -> bool {
    let t = v.trim();
    if t.is_empty() {
        return true;
    }
    t.parse::<f64>().map(|n| n == 0.0).unwrap_or(false)
}

/// MySQL datetime / date → 表单中文日期
fn sql_datetime_to_cn(s: &str) -> String {
    let t = s.trim();
    let date_part = t.split_whitespace().next().unwrap_or(t);
    let parts: Vec<&str> = date_part.split(['-', '/']).collect();
    if parts.len() >= 3 {
        if let (Ok(y), Ok(m), Ok(d)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            if y >= 1000 && (1..=12).contains(&m) && (1..=31).contains(&d) {
                return format!("{y} 年 {m} 月 {d} 日");
            }
        }
    }
    t.to_string()
}

/// 一行 `t_job` 数据 → 工单（字段原样保留，兼容旧 dump）
fn job_tuple_to_order(columns: &[String], values: &[Option<String>], now: &str) -> Option<Order> {
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
    fn parse_gd_maps_fields_and_checks() {
        let sql = "\
INSERT INTO `t_gd` (`mGD_No`,`mGD_Date`,`mGD_D_Date`,`mDYDW`,`mHTH`,`mCPMC`,`mDYSL`,`mLK`,`mB`,`mPrice`,`mKDR`) \
VALUES ('0000004','2010-08-02 00:00:00','2010-08-06 00:00:00','世亚','PO-1','检测记录表','20本',1,'√','0.0000','萍');
";
        let (orders, skipped) = parse(sql);
        assert_eq!(skipped, 0);
        assert_eq!(orders.len(), 1);
        let o = &orders[0];
        assert_eq!(o.order_no, "0000004");
        assert_eq!(o.fields.get("customer").unwrap(), "世亚");
        assert_eq!(o.fields.get("mCustomer_FullName").unwrap(), "世亚");
        assert_eq!(o.fields.get("productSpec").unwrap(), "检测记录表");
        assert_eq!(o.fields.get("contractNo").unwrap(), "PO-1");
        assert_eq!(o.fields.get("orderQty").unwrap(), "20本");
        assert_eq!(o.fields.get("openDate").unwrap(), "2010 年 8 月 2 日");
        assert_eq!(o.fields.get("deliverDate").unwrap(), "2010 年 8 月 6 日");
        assert_eq!(o.fields.get("signedBy").unwrap(), "萍");
        assert!(!o.fields.contains_key("costUnitPrice"), "零价格应跳过");
        assert_eq!(o.cks.get("liukai"), Some(&true));
        assert_eq!(o.cks.get("gPi"), Some(&true));
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
    fn parse_gd_without_column_list() {
        let sql = "\
CREATE TABLE `t_gd` (
  `mGD_No` varchar(255) NOT NULL default '',
  `mDYDW` varchar(255) default NULL,
  `mCPMC` varchar(255) default NULL,
  PRIMARY KEY  (`mGD_No`)
) ENGINE=MyISAM DEFAULT CHARSET=gb2312;
INSERT INTO `t_gd` VALUES ('0001422','益汇通','标签');
";
        let (orders, skipped) = parse(sql);
        assert_eq!(skipped, 0);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].order_no, "0001422");
        assert_eq!(orders[0].fields.get("customer").unwrap(), "益汇通");
        assert_eq!(orders[0].fields.get("productSpec").unwrap(), "标签");
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

        // 无目标表数据
        let sql = "INSERT INTO `t_company` VALUES (1,'x');";
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
    fn sql_datetime_to_cn_formats() {
        assert_eq!(sql_datetime_to_cn("2010-08-02 00:00:00"), "2010 年 8 月 2 日");
        assert_eq!(sql_datetime_to_cn("2010-08-02"), "2010 年 8 月 2 日");
    }

    #[test]
    fn string_unescape_handles_mysql_escapes() {
        let (s, end) = read_sql_string(b"a\\'b\\\\c\\nd\\te''f' tail", 0);
        assert_eq!(s, "a'b\\c\nd\te'f");
        assert_eq!(&"a\\'b\\\\c\\nd\\te''f' tail"[end..], " tail");
    }
}
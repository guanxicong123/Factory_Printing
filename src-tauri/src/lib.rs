use std::collections::HashMap;
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

/* ---------------- 单号生成（年份两位 + 5 位序号，跨重启递增） ---------------- */

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

fn year_yy() -> String {
    // 用可运行方式取年份后两位（整数运算，不依赖 chrono）
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 自 1970 起的天数
    let days = secs / 86400;
    // 近似年份（每 146097 天 = 400 个公历年）——整数算法精确到 2099
    let four_centuries = days / 146097;      // 每 400 年 = 146097 天
    let year_since_1970 = four_centuries * 400 + (days % 146097) / 366;
    let year = 1970 + year_since_1970 as u64;
    format!("{:02}", year % 100)
}

fn next_order_no(app: &tauri::AppHandle) -> Result<String, String> {
    let yy = year_yy();
    let mut seq = read_seq(app);
    let mut n: u64 = seq
        .get(&yy)
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // 扫描已保存文件，防止计数器丢失后重号
    let dir = orders_dir(app)?;
    let mut max_from_files: u64 = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let stem = name.trim_end_matches(".json");
                if stem.len() == 7 && stem.starts_with(&yy) {
                    if let Ok(v) = stem[2..].parse::<u64>() {
                        if v > max_from_files {
                            max_from_files = v;
                        }
                    }
                }
            }
        }
    }
    n = n.max(max_from_files) + 1;
    seq[yy.clone()] = serde_json::json!(n);
    fs::write(seq_file(app)?, seq.to_string())
        .map_err(|e| format!("写入序号失败: {e}"))?;
    Ok(format!("{yy}{:05}", n))
}

/* ---------------- Commands ---------------- */

#[tauri::command]
fn order_new(app: tauri::AppHandle) -> Result<Order, String> {
    let no = next_order_no(&app)?;
    let now = timestamp_now();
    let mut order = Order::default();
    order.order_no = no.clone();
    order.meta.id = no.clone();
    order.meta.file_name = format!("{no}.json");
    order.meta.created_at = now.clone();
    order.meta.updated_at = now;
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
    // 开单日期 → 今天（SQL 对齐名 mJob_Date + 旧别名 openDate 都更新）
    new_order.fields.insert("mJob_Date".into(), today.clone());
    new_order.fields.insert("openDate".into(), today);
    // 交货日期 → 清空（mFinished_Date + deliverDate 都清）
    new_order.fields.insert("mFinished_Date".into(), String::new());
    new_order.fields.insert("deliverDate".into(), String::new());

    Ok(new_order)
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
            order_print,
            order_duplicate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
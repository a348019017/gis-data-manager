use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictItem {
    pub id: String,
    pub category: String,
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub sort_order: i32,
}

// ========== Tauri commands ==========

#[tauri::command]
pub async fn get_dict_items(
    category: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<DictItem>, String> {
    let db = state.db.lock().await;

    if let Some(cat) = category {
        let mut stmt = db
            .prepare("SELECT id, category, label, value, sort_order FROM dict_items WHERE category=?1 ORDER BY sort_order")
            .map_err(|e| format!("查询失败: {}", e))?;
        let items: Vec<DictItem> = stmt
            .query_map(params![cat], |row| {
                Ok(DictItem {
                    id: row.get(0)?,
                    category: row.get(1)?,
                    label: row.get(2)?,
                    value: row.get(3)?,
                    sort_order: row.get(4)?,
                })
            })
            .map_err(|e| format!("查询失败: {}", e))?
            .map(|r| r.expect("row"))
            .collect();
        return Ok(items);
    }

    let mut stmt = db
        .prepare("SELECT id, category, label, value, sort_order FROM dict_items ORDER BY category, sort_order")
        .map_err(|e| format!("查询失败: {}", e))?;
    let items: Vec<DictItem> = stmt
        .query_map([], |row| {
            Ok(DictItem {
                id: row.get(0)?,
                category: row.get(1)?,
                label: row.get(2)?,
                value: row.get(3)?,
                sort_order: row.get(4)?,
            })
        })
        .map_err(|e| format!("查询失败: {}", e))?
        .map(|r| r.expect("row"))
        .collect();

    Ok(items)
}

#[tauri::command]
pub async fn add_dict_item(
    item: DictItem,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<DictItem, String> {
    let db = state.db.lock().await;
    db.execute(
        "INSERT INTO dict_items (id, category, label, value, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![item.id, item.category, item.label, item.value, item.sort_order],
    )
    .map_err(|e| format!("添加字典项失败: {}", e))?;
    Ok(item)
}

#[tauri::command]
pub async fn update_dict_item(
    item: DictItem,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<DictItem, String> {
    let db = state.db.lock().await;
    db.execute(
        "UPDATE dict_items SET label=?2, value=?3, sort_order=?4 WHERE id=?1",
        params![item.id, item.label, item.value, item.sort_order],
    )
    .map_err(|e| format!("更新字典项失败: {}", e))?;
    Ok(item)
}

#[tauri::command]
pub async fn delete_dict_item(
    id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    let rows = db
        .execute("DELETE FROM dict_items WHERE id=?1", params![id])
        .map_err(|e| format!("删除失败: {}", e))?;
    if rows == 0 {
        return Err("字典项不存在".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn get_dict_categories(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let db = state.db.lock().await;
    let mut stmt = db
        .prepare("SELECT DISTINCT category FROM dict_items ORDER BY category")
        .map_err(|e| format!("查询失败: {}", e))?;
    let categories: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("查询失败: {}", e))?
        .map(|r| r.expect("row"))
        .collect();
    Ok(categories)
}

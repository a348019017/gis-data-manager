use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GISTool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: String,
    pub params: String,
    #[serde(default)]
    pub returns: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub example: String,
}

// ========== DB helper ==========

pub fn load_tools_from_db(conn: &Connection) -> Vec<GISTool> {
    let mut stmt = conn
        .prepare("SELECT id, name, description, category, tags, params, returns, example FROM gis_tools")
        .expect("prepare query");

    stmt.query_map([], |row| {
        Ok(GISTool {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            category: row.get(3)?,
            tags: row.get(4)?,
            params: row.get(5)?,
            returns: row.get(6)?,
            example: row.get(7)?,
        })
    })
    .expect("query")
    .map(|r| r.expect("row"))
    .collect()
}

// ========== Tauri commands ==========

#[tauri::command]
pub async fn get_gis_tools(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<GISTool>, String> {
    let db = state.db.lock().await;
    Ok(load_tools_from_db(&db))
}

#[tauri::command]
pub async fn add_gis_tool(
    tool: GISTool,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<GISTool, String> {
    let db = state.db.lock().await;
    db.execute(
        "INSERT INTO gis_tools (id, name, description, category, tags, params, returns, example)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![tool.id, tool.name, tool.description, tool.category, tool.tags, tool.params, tool.returns, tool.example],
    )
    .map_err(|e| format!("添加工具失败: {}", e))?;
    Ok(tool)
}

#[tauri::command]
pub async fn update_gis_tool(
    tool: GISTool,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<GISTool, String> {
    let db = state.db.lock().await;
    db.execute(
        "UPDATE gis_tools SET name=?2, description=?3, category=?4, tags=?5, params=?6, returns=?7, example=?8 WHERE id=?1",
        params![tool.id, tool.name, tool.description, tool.category, tool.tags, tool.params, tool.returns, tool.example],
    )
    .map_err(|e| format!("更新工具失败: {}", e))?;
    Ok(tool)
}

#[tauri::command]
pub async fn delete_gis_tool(
    id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    let rows = db
        .execute("DELETE FROM gis_tools WHERE id=?1", params![id])
        .map_err(|e| format!("删除失败: {}", e))?;
    if rows == 0 {
        return Err("工具不存在".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn execute_gis_tool(
    tool_id: String,
    params: serde_json::Value,
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "tool_id": tool_id,
        "status": "completed",
        "message": format!("工具 {} 已执行（模拟）", tool_id),
        "params": params
    }))
}

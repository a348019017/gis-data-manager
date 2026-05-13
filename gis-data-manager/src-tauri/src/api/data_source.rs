use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub ds_type: String,
    pub subtype: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub database: String,
    pub username: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub password: String,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub connected: bool,
}

fn default_port() -> u16 { 5432 }

// ========== DB helpers ==========

pub fn load_sources_from_db(conn: &Connection) -> Vec<DataSource> {
    let mut stmt = conn
        .prepare("SELECT id, name, ds_type, subtype, host, port, database, username, password, remark, connected FROM data_sources")
        .expect("prepare query");

    stmt.query_map([], |row| {
        Ok(DataSource {
            id: row.get(0)?,
            name: row.get(1)?,
            ds_type: row.get(2)?,
            subtype: row.get(3)?,
            host: row.get(4)?,
            port: row.get(5)?,
            database: row.get(6)?,
            username: row.get(7)?,
            password: row.get(8).unwrap_or_default(),
            remark: row.get(9).unwrap_or_default(),
            connected: row.get::<_, i32>(10)? != 0,
        })
    })
    .expect("query")
    .map(|r| r.expect("row"))
    .collect()
}

pub fn insert_source(conn: &Connection, source: &DataSource) -> Result<(), String> {
    conn.execute(
        "INSERT INTO data_sources (id, name, ds_type, subtype, host, port, database, username, password, remark, connected)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            source.id, source.name, source.ds_type, source.subtype,
            source.host, source.port, source.database, source.username,
            source.password, source.remark, source.connected as i32
        ],
    )
    .map_err(|e| format!("插入失败: {}", e))?;
    Ok(())
}

pub fn update_source(conn: &Connection, source: &DataSource) -> Result<(), String> {
    conn.execute(
        "UPDATE data_sources SET name=?2, ds_type=?3, subtype=?4, host=?5, port=?6,
         database=?7, username=?8, password=?9, remark=?10, connected=?11 WHERE id=?1",
        params![
            source.id, source.name, source.ds_type, source.subtype,
            source.host, source.port, source.database, source.username,
            source.password, source.remark, source.connected as i32
        ],
    )
    .map_err(|e| format!("更新失败: {}", e))?;
    Ok(())
}

pub fn delete_source(conn: &Connection, id: &str) -> Result<bool, String> {
    let rows = conn
        .execute("DELETE FROM data_sources WHERE id=?1", params![id])
        .map_err(|e| format!("删除失败: {}", e))?;
    Ok(rows > 0)
}

// ========== Tauri commands ==========

use std::sync::Arc;
use crate::AppState;
use crate::PaginatedResponse;

#[tauri::command]
pub async fn get_data_sources(
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PaginatedResponse<DataSource>, String> {
    let db = state.db.lock().await;

    let kw = keyword.unwrap_or_default();
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(50);

    let total: i64 = if !kw.is_empty() {
        let like = format!("%{}%", kw);
        db.query_row(
            "SELECT COUNT(*) FROM data_sources WHERE name LIKE ?1 OR host LIKE ?2 OR remark LIKE ?3",
            params![like, like, like],
            |row| row.get(0),
        ).map_err(|e| format!("查询总数失败: {}", e))?
    } else {
        db.query_row("SELECT COUNT(*) FROM data_sources", [], |row| row.get(0))
            .map_err(|e| format!("查询总数失败: {}", e))?
    };

    let (sql, params_arr): (String, Vec<Box<dyn rusqlite::ToSql>>) = if !kw.is_empty() {
        let like = format!("%{}%", kw);
        (
            "SELECT id, name, ds_type, subtype, host, port, database, username, password, remark, connected
             FROM data_sources
             WHERE name LIKE ?1 OR host LIKE ?2 OR remark LIKE ?3
             ORDER BY id LIMIT ?4 OFFSET ?5".to_string(),
            vec![
                Box::new(like.clone()),
                Box::new(like.clone()),
                Box::new(like),
                Box::new(limit),
                Box::new(offset),
            ],
        )
    } else {
        (
            "SELECT id, name, ds_type, subtype, host, port, database, username, password, remark, connected
             FROM data_sources ORDER BY id LIMIT ?1 OFFSET ?2".to_string(),
            vec![Box::new(limit), Box::new(offset)],
        )
    };

    let mut stmt = db.prepare(&sql).map_err(|e| format!("查询失败: {}", e))?;
    let items: Vec<DataSource> = stmt
        .query_map(rusqlite::params_from_iter(params_arr.iter().map(|p| p.as_ref())), |row| {
            Ok(DataSource {
                id: row.get(0)?,
                name: row.get(1)?,
                ds_type: row.get(2)?,
                subtype: row.get(3)?,
                host: row.get(4)?,
                port: row.get(5)?,
                database: row.get(6)?,
                username: row.get(7)?,
                password: row.get(8).unwrap_or_default(),
                remark: row.get(9).unwrap_or_default(),
                connected: row.get::<_, i32>(10)? != 0,
            })
        })
        .map_err(|e| format!("查询失败: {}", e))?
        .map(|r| r.expect("row"))
        .collect();

    Ok(PaginatedResponse { total, items })
}

#[tauri::command]
pub async fn add_data_source(
    source: DataSource,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<DataSource, String> {
    let mut sources = state.sources.lock().await;
    if sources.iter().any(|s| s.id == source.id) {
        return Err("数据源ID已存在".into());
    }

    let db = state.db.lock().await;
    insert_source(&db, &source)?;
    drop(db);

    sources.push(source.clone());
    Ok(source)
}

#[tauri::command]
pub async fn update_data_source(
    source: DataSource,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<DataSource, String> {
    let mut sources = state.sources.lock().await;
    if let Some(pos) = sources.iter().position(|s| s.id == source.id) {
        let db = state.db.lock().await;
        update_source(&db, &source)?;
        drop(db);

        sources[pos] = source.clone();
        Ok(source)
    } else {
        Err("数据源不存在".into())
    }
}

#[tauri::command]
pub async fn delete_data_source(
    id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    let deleted = delete_source(&db, &id)?;
    drop(db);

    if !deleted {
        return Err("数据源不存在".into());
    }

    let mut sources = state.sources.lock().await;
    sources.retain(|s| s.id != id);
    Ok(())
}

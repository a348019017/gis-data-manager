use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::PaginatedResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub service_type: String,
    pub endpoint: String,
    #[serde(default)]
    pub username: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub password: String,
    #[serde(default)]
    pub connected: bool,
    #[serde(default)]
    pub remark: String,
}

// ========== DB helpers ==========

pub fn load_services_from_db(conn: &Connection) -> Vec<Service> {
    let mut stmt = conn
        .prepare("SELECT id, name, service_type, endpoint, username, password, connected, remark FROM services")
        .expect("prepare query");

    stmt.query_map([], |row| {
        Ok(Service {
            id: row.get(0)?,
            name: row.get(1)?,
            service_type: row.get(2)?,
            endpoint: row.get(3)?,
            username: row.get(4).unwrap_or_default(),
            password: row.get(5).unwrap_or_default(),
            connected: row.get::<_, i32>(6)? != 0,
            remark: row.get(7).unwrap_or_default(),
        })
    })
    .expect("query")
    .map(|r| r.expect("row"))
    .collect()
}

// ========== Tauri commands ==========

#[tauri::command]
pub async fn get_services(
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PaginatedResponse<Service>, String> {
    let db = state.db.lock().await;

    let kw = keyword.unwrap_or_default();
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(50);

    let total: i64 = if !kw.is_empty() {
        let like = format!("%{}%", kw);
        db.query_row(
            "SELECT COUNT(*) FROM services WHERE name LIKE ?1 OR endpoint LIKE ?2",
            params![like, like],
            |row| row.get(0),
        ).map_err(|e| format!("查询总数失败: {}", e))?
    } else {
        db.query_row("SELECT COUNT(*) FROM services", [], |row| row.get(0))
            .map_err(|e| format!("查询总数失败: {}", e))?
    };

    let (sql, params_arr): (String, Vec<Box<dyn rusqlite::ToSql>>) = if !kw.is_empty() {
        let like = format!("%{}%", kw);
        (
            "SELECT id, name, service_type, endpoint, username, password, connected, remark
             FROM services
             WHERE name LIKE ?1 OR endpoint LIKE ?2
             ORDER BY id LIMIT ?3 OFFSET ?4".to_string(),
            vec![
                Box::new(like.clone()),
                Box::new(like),
                Box::new(limit),
                Box::new(offset),
            ],
        )
    } else {
        (
            "SELECT id, name, service_type, endpoint, username, password, connected, remark
             FROM services ORDER BY id LIMIT ?1 OFFSET ?2".to_string(),
            vec![Box::new(limit), Box::new(offset)],
        )
    };

    let mut stmt = db.prepare(&sql).map_err(|e| format!("查询失败: {}", e))?;
    let items: Vec<Service> = stmt
        .query_map(rusqlite::params_from_iter(params_arr.iter().map(|p| p.as_ref())), |row| {
            Ok(Service {
                id: row.get(0)?,
                name: row.get(1)?,
                service_type: row.get(2)?,
                endpoint: row.get(3)?,
                username: row.get(4).unwrap_or_default(),
                password: row.get(5).unwrap_or_default(),
                connected: row.get::<_, i32>(6)? != 0,
                remark: row.get(7).unwrap_or_default(),
            })
        })
        .map_err(|e| format!("查询失败: {}", e))?
        .map(|r| r.expect("row"))
        .collect();

    Ok(PaginatedResponse { total, items })
}

#[tauri::command]
pub async fn add_service(
    service: Service,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Service, String> {
    let mut services = state.services.lock().await;
    if services.iter().any(|s| s.id == service.id) {
        return Err("服务ID已存在".into());
    }

    let db = state.db.lock().await;
    db.execute(
        "INSERT INTO services (id, name, service_type, endpoint, username, password, connected, remark)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            service.id, service.name, service.service_type, service.endpoint,
            service.username, service.password, service.connected as i32, service.remark
        ],
    )
    .map_err(|e| format!("插入失败: {}", e))?;
    drop(db);

    services.push(service.clone());
    Ok(service)
}

#[tauri::command]
pub async fn update_service(
    service: Service,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Service, String> {
    let mut services = state.services.lock().await;
    if let Some(pos) = services.iter().position(|s| s.id == service.id) {
        let db = state.db.lock().await;
        db.execute(
            "UPDATE services SET name=?2, service_type=?3, endpoint=?4, username=?5,
             password=?6, connected=?7, remark=?8 WHERE id=?1",
            params![
                service.id, service.name, service.service_type, service.endpoint,
                service.username, service.password, service.connected as i32, service.remark
            ],
        )
        .map_err(|e| format!("更新失败: {}", e))?;
        drop(db);

        services[pos] = service.clone();
        Ok(service)
    } else {
        Err("服务不存在".into())
    }
}

#[tauri::command]
pub async fn delete_service(
    id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    let rows = db
        .execute("DELETE FROM services WHERE id=?1", params![id])
        .map_err(|e| format!("删除失败: {}", e))?;
    drop(db);

    if rows == 0 {
        return Err("服务不存在".into());
    }

    let mut services = state.services.lock().await;
    services.retain(|s| s.id != id);
    Ok(())
}

#[tauri::command]
pub async fn test_service_connection(
    service: Service,
) -> Result<bool, String> {
    let client = reqwest::Client::new();
    let endpoint = service.endpoint.trim_end_matches('/');

    let url = match service.service_type.as_str() {
        "wmts" => format!("{}?request=GetCapabilities&service=WMTS&version=1.0.0", endpoint),
        "tms" => format!("{}/1.0.0", endpoint),
        "wms" => format!("{}?request=GetCapabilities&service=WMS&version=1.3.0", endpoint),
        "wfs" => format!("{}?request=GetCapabilities&service=WFS&version=2.0.0", endpoint),
        "geoserver" => format!("{}/rest/about/version", endpoint),
        "arcgis" => format!("{}/f=json", endpoint),
        _ => return Err(format!("不支持的服务类型: {}", service.service_type)),
    };

    let mut req = client.get(&url);
    if !service.username.is_empty() {
        req = req.basic_auth(&service.username, Some(&service.password));
    }

    let resp = req.send().await
        .map_err(|e| format!("请求失败: {}", e))?;

    if resp.status().is_success() {
        Ok(true)
    } else {
        Err(format!("连接失败: HTTP {}", resp.status()))
    }
}

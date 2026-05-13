use serde::{Deserialize, Serialize};
use rusqlite::params;
use crate::AppState;
use crate::db_init::init_db;
use std::sync::Arc;

// ========== Structs ==========

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelSettings {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub api_url: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub api_key: String,
    #[serde(default)]
    pub model_name: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
}

fn default_max_tokens() -> u32 { 4096 }
fn default_temperature() -> f64 { 0.7 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub db_path: String,
    pub data_dir: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// ========== Tauri commands ==========

#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, Arc<AppState>>
) -> Result<Option<ModelSettings>, String> {
    let db = state.db.lock().await;
    let row: Option<String> = db
        .query_row("SELECT value FROM settings WHERE key='model_config'", [], |r| r.get(0))
        .ok();
    drop(db);

    match row {
        Some(json) => serde_json::from_str(&json).map_err(|e| format!("解析配置失败: {}", e)),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn save_settings(
    settings: ModelSettings,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    let json = serde_json::to_string(&settings).map_err(|e| format!("序列化失败: {}", e))?;

    db.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('model_config', ?1)",
        params![json],
    )
    .map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn test_model_connection(settings: ModelSettings) -> Result<bool, String> {
    if settings.api_key.is_empty() {
        return Err("API Key 不能为空".into());
    }
    if settings.model_name.is_empty() {
        return Err("模型名称不能为空".into());
    }

    let client = reqwest::Client::new();
    let api_url = settings.api_url.trim_end_matches('/');

    match settings.provider.as_str() {
        "anthropic" => {
            let url = if api_url.ends_with("/v1") {
                format!("{}/messages", api_url)
            } else {
                format!("{}/v1/messages", api_url)
            };

            let resp = client.post(&url)
                .header("x-api-key", &settings.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&serde_json::json!({
                    "model": &settings.model_name,
                    "max_tokens": settings.max_tokens.min(1024),
                    "messages": [{"role": "user", "content": "Hi"}]
                }))
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?;

            if resp.status().is_success() {
                Ok(true)
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(format!("连接失败 ({}): {}", status, body.chars().take(200).collect::<String>()))
            }
        }
        "ollama" => {
            let url = if api_url.ends_with("/api") {
                format!("{}/generate", api_url)
            } else {
                format!("{}/api/generate", api_url)
            };

            let resp = client.post(&url)
                .json(&serde_json::json!({
                    "model": &settings.model_name,
                    "prompt": "Hi",
                    "stream": false
                }))
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?;

            if resp.status().is_success() {
                Ok(true)
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(format!("连接失败 ({}): {}", status, body.chars().take(200).collect::<String>()))
            }
        }
        _ => {
            let url = if api_url.ends_with("/v1") {
                format!("{}/chat/completions", api_url)
            } else {
                format!("{}/v1/chat/completions", api_url)
            };

            let resp = client.post(&url)
                .bearer_auth(&settings.api_key)
                .header("content-type", "application/json")
                .json(&serde_json::json!({
                    "model": &settings.model_name,
                    "max_tokens": settings.max_tokens.min(1024),
                    "messages": [{"role": "user", "content": "Hi"}]
                }))
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?;

            if resp.status().is_success() {
                Ok(true)
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(format!("连接失败 ({}): {}", status, body.chars().take(200).collect::<String>()))
            }
        }
    }
}

#[tauri::command]
pub async fn get_app_info(
    state: tauri::State<'_, Arc<AppState>>
) -> Result<AppInfo, String> {
    let db = state.db.lock().await;
    let db_path = db.path().unwrap_or("未知").to_string();
    drop(db);

    let data_dir = std::path::Path::new(&db_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(AppInfo {
        db_path,
        data_dir,
        version: "0.1.0".to_string(),
    })
}

#[tauri::command]
pub async fn chat_message(
    settings: ModelSettings,
    message: String,
    history: Vec<ChatMessage>,
) -> Result<String, String> {
    if settings.api_key.is_empty() {
        return Err("API Key 未配置".into());
    }
    if settings.model_name.is_empty() {
        return Err("模型名称未配置".into());
    }

    let client = reqwest::Client::new();
    let api_url = settings.api_url.trim_end_matches('/');

    let system_prompt = "你是一个 GIS 数据管理助手。你可以帮助用户管理数据源、导入 GIS 数据文件、注册和预览地图服务。请用简洁、专业的中文回答。";

    let messages: Vec<serde_json::Value> = std::iter::once(serde_json::json!({
        "role": "system",
        "content": system_prompt
    }))
    .chain(history.iter().map(|m| serde_json::json!({ "role": m.role, "content": m.content })))
    .chain(std::iter::once(serde_json::json!({ "role": "user", "content": message })))
    .collect();

    let url = if settings.provider == "anthropic" {
        if api_url.ends_with("/v1") {
            format!("{}/messages", api_url)
        } else {
            format!("{}/v1/messages", api_url)
        }
    } else {
        if api_url.ends_with("/v1") {
            format!("{}/chat/completions", api_url)
        } else {
            format!("{}/v1/chat/completions", api_url)
        }
    };

    let resp = if settings.provider == "anthropic" {
        client.post(&url)
            .header("x-api-key", &settings.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": &settings.model_name,
                "max_tokens": settings.max_tokens.min(4096),
                "messages": messages
            }))
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?
    } else {
        client.post(&url)
            .bearer_auth(&settings.api_key)
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": &settings.model_name,
                "max_tokens": settings.max_tokens.min(4096),
                "temperature": settings.temperature,
                "messages": messages
            }))
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API 错误 ({}): {}", status, body.chars().take(200).collect::<String>()));
    }

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;

    if settings.provider == "anthropic" {
        data["content"][0]["text"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "未获取到回复".into())
    } else {
        data["choices"][0]["message"]["content"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "未获取到回复".into())
    }
}

// ========== Database management ==========

const REQUIRED_TABLES: &[&str] = &[
    "data_sources", "settings", "services", "import_records", "gis_tools", "dict_items",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbTableInfo {
    pub name: String,
    pub row_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchDbResult {
    pub app_info: AppInfo,
    pub created_tables: Vec<String>, // tables that were missing and auto-created
}

/// Preview current database tables and row counts
#[tauri::command]
pub async fn preview_db_tables(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<DbTableInfo>, String> {
    let db = state.db.lock().await;
    let mut stmt = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .map_err(|e| format!("查询表列表失败: {}", e))?;
    let tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("查询表列表失败: {}", e))?
        .map(|r| r.expect("row"))
        .collect();

    let mut result = Vec::new();
    for table in tables {
        let count: i64 = db
            .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0))
            .map_err(|e| format!("查询 {} 行数失败: {}", table, e))?;
        result.push(DbTableInfo { name: table, row_count: count });
    }

    Ok(result)
}

/// Switch to a new SQLite database file with validation
#[tauri::command]
pub async fn switch_database(
    db_path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<SwitchDbResult, String> {
    // Step 1: verify file is a valid SQLite database
    let check_conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("无法打开数据库文件: {}\n请确认文件为有效的 SQLite 数据库", e))?;

    // Step 2: check which required tables exist
    let existing_tables: Vec<String> = check_conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .map_err(|e| format!("查询表结构失败: {}", e))?
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("查询表结构失败: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let missing_tables: Vec<String> = REQUIRED_TABLES
        .iter()
        .filter(|t| !existing_tables.contains(&t.to_string()))
        .map(|t| t.to_string())
        .collect();

    drop(check_conn);

    // Step 3: if tables are missing, run init_db to create them
    let new_conn = if missing_tables.is_empty() {
        rusqlite::Connection::open(&db_path)
            .map_err(|e| format!("打开数据库失败: {}", e))?
    } else {
        // init_db uses CREATE TABLE IF NOT EXISTS, so it safely adds missing tables
        // but we need to re-open after init_db since it also handles parent dirs
        init_db(&db_path)?
    };

    // Step 4: save the path for persistence
    {
        let _ = new_conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('custom_db_path', ?1)",
            params![&db_path],
        );
    }

    // Step 5: reload sources and services from the new database
    let sources = crate::api::load_sources_from_db(&new_conn);
    let services = crate::api::load_services_from_db(&new_conn);

    // Step 6: atomically swap connection and caches
    {
        let mut db = state.db.lock().await;
        *db = new_conn;
    }
    {
        let mut src = state.sources.lock().await;
        *src = sources;
    }
    {
        let mut svc = state.services.lock().await;
        *svc = services;
    }

    let data_dir = std::path::Path::new(&db_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(SwitchDbResult {
        app_info: AppInfo {
            db_path,
            data_dir,
            version: "0.1.0".to_string(),
        },
        created_tables: missing_tables,
    })
}

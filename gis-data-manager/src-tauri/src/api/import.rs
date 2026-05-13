use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use crate::AppState;
use crate::PaginatedResponse;
use crate::api::data_source::DataSource;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRecord {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub file_type: String,
    pub format: String,
    pub target_source_id: String,
    pub target_source_name: String,
    pub target_type: String,
    pub status: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error_msg: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tags: String,
}

pub const VECTOR_EXTENSIONS: &[&str] = &["shp", "dbf", "prj", "geojson", "json", "gpkg", "kml", "kmz", "fgb", "fgb2"];
pub const DOCUMENT_EXTENSIONS: &[&str] = &["pdf", "doc", "docx", "xls", "xlsx", "txt", "csv", "zip", "rar", "7z"];

pub fn detect_file_info(path: &str) -> (String, String, String) {
    let path_buf = std::path::Path::new(path);
    let file_name = path_buf
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let ext = path_buf
        .extension()
        .map(|e| e.to_string_lossy().to_string().to_lowercase())
        .unwrap_or_default();

    let file_type = if VECTOR_EXTENSIONS.contains(&ext.as_str()) {
        "vector"
    } else if DOCUMENT_EXTENSIONS.contains(&ext.as_str()) {
        "document"
    } else {
        "unknown"
    };

    (file_type.to_string(), ext, file_name)
}

fn urlencoding_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn base64_encode(bytes: Vec<u8>) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).map_or(0, |&b| b as u32);
        let b2 = chunk.get(2).map_or(0, |&b| b as u32);
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn extract_xml_message(xml: &str) -> String {
    for line in xml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<Message>") && trimmed.ends_with("</Message>") {
            return trimmed.strip_prefix("<Message>")
                .and_then(|s| s.strip_suffix("</Message>"))
                .unwrap_or("").to_string();
        }
        if trimmed.starts_with("<Code>") && trimmed.ends_with("</Code>") {
            let code = trimmed.strip_prefix("<Code>")
                .and_then(|s| s.strip_suffix("</Code>"))
                .unwrap_or("");
            if !code.is_empty() {
                return code.to_string();
            }
        }
    }
    String::new()
}

// ========== Upload helpers ==========

async fn upload_to_s3(
    file_path: &str, source: &DataSource, file_name: &str, app: &tauri::AppHandle,
) -> Result<(), String> {
    use minio::s3::MinioClient;
    use minio::s3::builders::ObjectContent;
    use minio::s3::creds::StaticProvider;
    use minio::s3::http::BaseUrl;
    use minio::s3::types::{BucketName, ObjectKey, S3Api};

    let access_key = &source.username;
    let secret_key = &source.password;
    if access_key.is_empty() || secret_key.is_empty() {
        return Err("S3 存储需要提供 AccessKey 和 SecretKey".into());
    }

    let (scheme, host_only) = if source.host.starts_with("http://") {
        ("http://", source.host.trim_start_matches("http://").trim_end_matches('/').to_string())
    } else if source.host.starts_with("https://") {
        ("https://", source.host.trim_start_matches("https://").trim_end_matches('/').to_string())
    } else if source.subtype == "minio" {
        ("http://", source.host.trim_end_matches('/').to_string())
    } else {
        ("https://", source.host.trim_end_matches('/').to_string())
    };
    let port = if source.port != 0 { source.port } else {
        if source.subtype == "minio" { 9000 } else { 443 }
    };
    let base_url = if host_only.contains(':') {
        format!("{}{}", scheme, host_only)
    } else {
        format!("{}{}:{}", scheme, host_only, port)
    };

    let base_url = base_url.parse::<BaseUrl>()
        .map_err(|e| format!("URL 解析失败: {}", e))?;

    let provider = StaticProvider::new(access_key, secret_key, None);
    let client = MinioClient::new(base_url, Some(provider), None, None)
        .map_err(|e| format!("创建客户端失败: {}", e))?;

    let bucket_name = BucketName::try_from(&source.database)
        .map_err(|e| format!("Bucket 名称无效: {}", e))?;

    let exists_resp = client.bucket_exists(bucket_name.clone())
        .map_err(|e| format!("检查 Bucket 失败: {}", e))?
        .build()
        .send()
        .await
        .map_err(|e| format!("检查 Bucket 失败: {}", e))?;

    if !exists_resp.exists() {
        client.create_bucket(bucket_name.clone())
            .map_err(|e| format!("创建 Bucket 失败: {}", e))?
            .build()
            .send()
            .await
            .map_err(|e| format!("创建 Bucket 失败: {}", e))?;
    }

    let existing = client.stat_object(bucket_name.clone(), ObjectKey::new(file_name)
        .map_err(|e| format!("对象键无效: {}", e))?)
        .map_err(|e| format!("对象键无效: {}", e))?
        .build()
        .send()
        .await;
    if existing.is_ok() {
        return Err(format!("文件 '{}' 已存在于 Bucket 中，请勿重复上传", file_name));
    }

    let metadata = std::fs::metadata(file_path)
        .map_err(|e| format!("获取文件信息失败: {}", e))?;
    let file_size = metadata.len();

    let _ = app.emit("import-progress", serde_json::json!({
        "file_name": file_name,
        "status": "uploading",
        "bytes_sent": 0,
        "total_bytes": file_size,
    }));

    let content = ObjectContent::from(std::path::Path::new(file_path));

    client.put_object_content(bucket_name, ObjectKey::new(file_name)
        .map_err(|e| format!("对象键无效: {}", e))?, content)
        .map_err(|e| format!("构建请求失败: {}", e))?
        .build()
        .send()
        .await
        .map_err(|e| format!("上传失败: {}", e))?;

    let _ = app.emit("import-progress", serde_json::json!({
        "file_name": file_name,
        "status": "success",
        "bytes_sent": file_size,
        "total_bytes": file_size,
    }));

    Ok(())
}

async fn upload_to_aliyun_oss(
    file_path: &str, source: &DataSource, file_name: &str,
) -> Result<(), String> {
    use sha2::Sha256;
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<Sha256>;

    let access_key = &source.username;
    let secret_key = &source.password;
    if access_key.is_empty() || secret_key.is_empty() {
        return Err("阿里云 OSS 需要提供 AccessKey 和 SecretKey".into());
    }

    let file_bytes = std::fs::read(file_path)
        .map_err(|e| format!("读取文件失败: {}", e))?;

    let endpoint = if source.host.starts_with("http") {
        source.host.clone()
    } else {
        format!("https://{}", source.host)
    };

    let bucket = &source.database;
    let object_key = urlencoding_encode(file_name);

    let head_url = format!("{}/{}/{}", endpoint.trim_end_matches('/'), bucket, object_key);
    let head_client = reqwest::Client::new();
    let head_resp = head_client.head(&head_url).send().await;
    if let Ok(resp) = head_resp {
        if resp.status().is_success() {
            return Err(format!("文件 '{}' 已存在于 Bucket 中，请勿重复上传", file_name));
        }
    }

    let upload_url = format!("{}/{}/{}", endpoint.trim_end_matches('/'), bucket, object_key);

    let now = chrono::Utc::now();
    let date = now.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    let content_type = "application/octet-stream";

    let string_to_sign = format!("PUT\n\n{}\n{}\n/{}/{}", content_type, date, bucket, object_key);

    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes()).unwrap();
    mac.update(string_to_sign.as_bytes());
    let signature = base64_encode(mac.finalize().into_bytes().to_vec());

    let auth_header = format!("OSS {}:{}", access_key, signature);

    let client = reqwest::Client::new();
    let resp = client.put(&upload_url)
        .header("Authorization", &auth_header)
        .header("Date", &date)
        .header("Content-Type", content_type)
        .body(reqwest::Body::from(file_bytes))
        .send()
        .await
        .map_err(|e| format!("上传失败: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let error_msg = extract_xml_message(&body);
        Err(format!("上传失败 ({}): {}", status, if error_msg.is_empty() { body.chars().take(200).collect::<String>() } else { error_msg }))
    }
}

async fn upload_to_oss(
    file_path: &str, source: &DataSource, file_name: &str, app: &tauri::AppHandle,
) -> Result<(), String> {
    let bucket = &source.database;
    if bucket.is_empty() {
        return Err("OSS Bucket 不能为空".into());
    }

    match source.subtype.as_str() {
        "minio" | "aws" => upload_to_s3(file_path, source, file_name, app).await,
        "aliyun" => upload_to_aliyun_oss(file_path, source, file_name).await,
        _ => Err(format!("不支持的存储类型: {}", source.subtype)),
    }
}

// ========== Download helpers ==========

async fn download_from_s3(
    source: &DataSource, file_name: &str, save_path: &str, app: &tauri::AppHandle,
) -> Result<(), String> {
    use minio::s3::MinioClient;
    use minio::s3::creds::StaticProvider;
    use minio::s3::http::BaseUrl;
    use minio::s3::types::{BucketName, ObjectKey, S3Api};

    let access_key = &source.username;
    let secret_key = &source.password;
    if access_key.is_empty() || secret_key.is_empty() {
        return Err("S3 存储需要提供 AccessKey 和 SecretKey".into());
    }

    let (scheme, host_only) = if source.host.starts_with("http://") {
        ("http://", source.host.trim_start_matches("http://").trim_end_matches('/').to_string())
    } else if source.host.starts_with("https://") {
        ("https://", source.host.trim_start_matches("https://").trim_end_matches('/').to_string())
    } else if source.subtype == "minio" {
        ("http://", source.host.trim_end_matches('/').to_string())
    } else {
        ("https://", source.host.trim_end_matches('/').to_string())
    };
    let port = if source.port != 0 { source.port } else {
        if source.subtype == "minio" { 9000 } else { 443 }
    };
    let base_url = if host_only.contains(':') {
        format!("{}{}", scheme, host_only)
    } else {
        format!("{}{}:{}", scheme, host_only, port)
    };

    let base_url = base_url.parse::<BaseUrl>()
        .map_err(|e| format!("URL 解析失败: {}", e))?;

    let provider = StaticProvider::new(access_key, secret_key, None);
    let client = MinioClient::new(base_url, Some(provider), None, None)
        .map_err(|e| format!("创建客户端失败: {}", e))?;

    let bucket = BucketName::try_from(&source.database)
        .map_err(|e| format!("Bucket 名称无效: {}", e))?;

    let stat_resp = client.stat_object(bucket.clone(), ObjectKey::new(file_name)
        .map_err(|e| format!("对象键无效: {}", e))?)
        .map_err(|e| format!("获取文件信息失败: {}", e))?
        .build()
        .send()
        .await
        .map_err(|e| format!("获取文件信息失败: {}", e))?;

    let total_size = stat_resp.size()
        .map_err(|e| format!("获取文件大小失败: {}", e))?;

    let _ = app.emit("download-progress", serde_json::json!({
        "file_name": file_name,
        "status": "downloading",
        "bytes_received": 0,
        "total_bytes": total_size,
    }));

    let resp = client.get_object(bucket, ObjectKey::new(file_name)
        .map_err(|e| format!("对象键无效: {}", e))?)
        .map_err(|e| format!("构建请求失败: {}", e))?
        .build()
        .send()
        .await
        .map_err(|e| format!("下载失败: {}", e))?;

    if let Some(parent) = std::path::Path::new(save_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    resp.content()
        .map_err(|e| format!("获取内容失败: {}", e))?
        .to_file(std::path::Path::new(save_path))
        .await
        .map_err(|e| format!("保存文件失败: {}", e))?;

    let _ = app.emit("download-progress", serde_json::json!({
        "file_name": file_name,
        "status": "success",
        "bytes_received": total_size,
        "total_bytes": total_size,
    }));

    Ok(())
}

async fn download_from_aliyun_oss(
    source: &DataSource, file_name: &str, save_path: &str, app: &tauri::AppHandle,
) -> Result<(), String> {
    let access_key = &source.username;
    let secret_key = &source.password;
    if access_key.is_empty() || secret_key.is_empty() {
        return Err("阿里云 OSS 需要提供 AccessKey 和 SecretKey".into());
    }

    let endpoint = if source.host.starts_with("http") {
        source.host.clone()
    } else {
        format!("https://{}", source.host)
    };

    let bucket = &source.database;
    let object_key = urlencoding_encode(file_name);
    let download_url = format!("{}/{}/{}", endpoint.trim_end_matches('/'), bucket, object_key);

    let _ = app.emit("download-progress", serde_json::json!({
        "file_name": file_name,
        "status": "downloading",
        "bytes_received": 0,
        "total_bytes": 0,
    }));

    let client = reqwest::Client::new();
    let resp = client.get(&download_url).send().await
        .map_err(|e| format!("下载失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("下载失败: HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    if let Some(parent) = std::path::Path::new(save_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    std::fs::write(save_path, &bytes)
        .map_err(|e| format!("保存文件失败: {}", e))?;

    let _ = app.emit("download-progress", serde_json::json!({
        "file_name": file_name,
        "status": "success",
        "bytes_received": bytes.len(),
        "total_bytes": bytes.len(),
    }));

    Ok(())
}

#[allow(dead_code)]
pub async fn copy_to_data_dir(
    file_path: &str, file_name: &str, app: &tauri::AppHandle,
) -> Result<(), String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?
        .join("imports");

    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("创建导入目录失败: {}", e))?;

    let dest = data_dir.join(file_name);
    std::fs::copy(file_path, &dest)
        .map_err(|e| format!("复制文件失败: {}", e))?;

    Ok(())
}

// ========== Tauri commands ==========

#[tauri::command]
pub async fn get_import_records(
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PaginatedResponse<ImportRecord>, String> {
    let db = state.db.lock().await;

    let kw = keyword.unwrap_or_default();
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(50);

    let total: i64 = if !kw.is_empty() {
        let like = format!("%{}%", kw);
        db.query_row(
            "SELECT COUNT(*) FROM import_records WHERE file_name LIKE ?1 OR target_source_name LIKE ?2 OR format LIKE ?3",
            params![like, like, like],
            |row| row.get(0),
        ).map_err(|e| format!("查询总数失败: {}", e))?
    } else {
        db.query_row("SELECT COUNT(*) FROM import_records", [], |row| row.get(0))
            .map_err(|e| format!("查询总数失败: {}", e))?
    };

    let (sql, params_arr): (String, Vec<Box<dyn rusqlite::ToSql>>) = if !kw.is_empty() {
        let like = format!("%{}%", kw);
        (
            "SELECT id, file_name, file_path, file_size, file_type, format,
                    target_source_id, target_source_name, target_type, status,
                    created_at, error_msg, tags
             FROM import_records
             WHERE file_name LIKE ?1 OR target_source_name LIKE ?2 OR format LIKE ?3
             ORDER BY created_at DESC LIMIT ?4 OFFSET ?5".to_string(),
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
            "SELECT id, file_name, file_path, file_size, file_type, format,
                    target_source_id, target_source_name, target_type, status,
                    created_at, error_msg, tags
             FROM import_records ORDER BY created_at DESC LIMIT ?1 OFFSET ?2".to_string(),
            vec![Box::new(limit), Box::new(offset)],
        )
    };

    let mut stmt = db.prepare(&sql).map_err(|e| format!("查询失败: {}", e))?;
    let records: Vec<ImportRecord> = stmt
        .query_map(rusqlite::params_from_iter(params_arr.iter().map(|p| p.as_ref())), |row| {
            Ok(ImportRecord {
                id: row.get(0)?,
                file_name: row.get(1)?,
                file_path: row.get(2)?,
                file_size: row.get(3)?,
                file_type: row.get(4)?,
                format: row.get(5)?,
                target_source_id: row.get(6)?,
                target_source_name: row.get(7)?,
                target_type: row.get(8)?,
                status: row.get(9)?,
                created_at: row.get(10)?,
                error_msg: row.get(11).unwrap_or_default(),
                tags: row.get(12).unwrap_or_default(),
            })
        })
        .map_err(|e| format!("查询失败: {}", e))?
        .map(|r| r.expect("row"))
        .collect();

    Ok(PaginatedResponse { total, items: records })
}

#[tauri::command]
pub async fn delete_import_record(
    id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.execute("DELETE FROM import_records WHERE id=?1", params![id])
        .map_err(|e| format!("删除失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn import_file(
    file_path: String,
    target_source_id: String,
    tags: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<ImportRecord, String> {
    let (file_type, format, file_name) = detect_file_info(&file_path);
    if file_type == "unknown" {
        return Err(format!("不支持的文件格式: {}", format));
    }

    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| format!("读取文件信息失败: {}", e))?;
    let file_size = metadata.len();

    let sources = state.sources.lock().await;
    let source = sources
        .iter()
        .find(|s| s.id == target_source_id)
        .ok_or("目标数据源不存在")?
        .clone();
    drop(sources);

    if source.ds_type != "oss" {
        return Err("当前仅支持导入到 OSS 数据源，数据库导入暂不可用".into());
    }

    let record_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    let tags = tags.unwrap_or_default();

    let record = ImportRecord {
        id: record_id.clone(),
        file_name: file_name.clone(),
        file_path: file_path.clone(),
        file_size,
        file_type: file_type.clone(),
        format: format.clone(),
        target_source_id: target_source_id.clone(),
        target_source_name: source.name.clone(),
        target_type: source.ds_type.clone(),
        status: "pending".to_string(),
        created_at: created_at.clone(),
        error_msg: String::new(),
        tags: tags.clone(),
    };

    {
        let db = state.db.lock().await;
        db.execute(
            "INSERT INTO import_records (id, file_name, file_path, file_size, file_type, format, target_source_id, target_source_name, target_type, status, created_at, tags)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![record.id, record.file_name, record.file_path, record.file_size as i64, record.file_type, record.format, record.target_source_id, record.target_source_name, record.target_type, record.status, record.created_at, record.tags],
        ).map_err(|e| format!("保存导入记录失败: {}", e))?;
    }

    let result = match source.ds_type.as_str() {
        "oss" => upload_to_oss(&file_path, &source, &file_name, &app).await,
        "database" => Err("数据库数据源导入暂未实现".into()),
        _ => Err(format!("不支持的数据源类型: {}", source.ds_type)),
    };

    {
        let db = state.db.lock().await;
        match &result {
            Ok(_) => {
                db.execute(
                    "UPDATE import_records SET status='success' WHERE id=?1",
                    params![record_id],
                )
                .map_err(|e| format!("更新状态失败: {}", e))?;
            }
            Err(err) => {
                db.execute(
                    "UPDATE import_records SET status='failed', error_msg=?2 WHERE id=?1",
                    params![record_id, err],
                )
                .map_err(|e| format!("更新状态失败: {}", e))?;
            }
        }
    }

    let mut updated_record = record;
    match result {
        Ok(_) => updated_record.status = "success".to_string(),
        Err(err) => {
            updated_record.status = "failed".to_string();
            updated_record.error_msg = err.clone();
        }
    }

    Ok(updated_record)
}

#[tauri::command]
pub async fn download_file(
    record_id: String,
    target_source_id: String,
    save_path: String,
    state: tauri::State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let sources = state.sources.lock().await;
    let db = state.db.lock().await;

    let row = db.query_row(
        "SELECT id, file_name, file_size, target_type, status FROM import_records WHERE id=?1",
        params![record_id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        },
    ).map_err(|_| "导入记录不存在")?;

    if row.4 != "success" {
        return Err("仅支持下载成功导入的文件".into());
    }

    let source = sources
        .iter()
        .find(|s| s.id == target_source_id)
        .ok_or("目标数据源不存在")?
        .clone();
    drop(sources);
    drop(db);

    if source.ds_type != "oss" {
        return Err("当前仅支持从 OSS 数据源下载".into());
    }

    let local_path = match source.subtype.as_str() {
        "minio" | "aws" => {
            download_from_s3(&source, &row.1, &save_path, &app).await?;
            save_path.clone()
        }
        "aliyun" => {
            download_from_aliyun_oss(&source, &row.1, &save_path, &app).await?;
            save_path.clone()
        }
        _ => return Err(format!("不支持的存储类型: {}", source.subtype)),
    };

    Ok(local_path)
}

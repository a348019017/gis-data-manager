use crate::api::data_source::DataSource;

fn urlencoding_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

// ========== OSS connection test ==========

fn parse_oss_endpoint(source: &DataSource) -> (&'static str, String, u16) {
    if source.host.starts_with("http://") {
        let host = source.host.trim_start_matches("http://").trim_end_matches('/').to_string();
        ("http", host, 0)
    } else if source.host.starts_with("https://") {
        let host = source.host.trim_start_matches("https://").trim_end_matches('/').to_string();
        ("https", host, 0)
    } else if source.subtype == "minio" {
        ("http", source.host.trim_end_matches('/').to_string(), 9000)
    } else {
        ("https", source.host.trim_end_matches('/').to_string(), 0)
    }
}

async fn test_oss_reachable(base_url: &str) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let resp = client.get(base_url).send().await
        .map_err(|e| format!("OSS 连接失败: {}", e))?;

    if resp.status().is_success() || resp.status() == 403 || resp.status() == 404 {
        Ok(true)
    } else {
        Err(format!("OSS 连接失败: HTTP {}", resp.status()))
    }
}

async fn test_minio_sdk_connection(
    base_url: &str, access_key: &str, secret_key: &str, bucket: &str,
) -> Result<bool, String> {
    use minio::s3::MinioClient;
    use minio::s3::creds::StaticProvider;
    use minio::s3::http::BaseUrl;
    use minio::s3::types::S3Api;

    let base_url = base_url.parse::<BaseUrl>()
        .map_err(|e| format!("MinIO URL 解析失败: {}", e))?;

    let provider = StaticProvider::new(access_key, secret_key, None);
    let client = MinioClient::new(base_url, Some(provider), None, None)
        .map_err(|e| format!("创建 MinIO 客户端失败: {}", e))?;

    if !bucket.is_empty() {
        let resp = client.bucket_exists(bucket)
            .map_err(|e| format!("MinIO 请求构建失败: {}", e))?
            .build()
            .send()
            .await
            .map_err(|e| format!("MinIO 连接失败: {}", e))?;
        if resp.exists() {
            Ok(true)
        } else {
            Err(format!("MinIO 连接成功但 bucket '{}' 不存在", bucket))
        }
    } else {
        let resp = client.list_buckets()
            .build()
            .send()
            .await
            .map_err(|e| format!("MinIO 连接失败: {}", e))?;
        let _buckets = resp.buckets().map_err(|e| format!("解析响应失败: {}", e))?;
        Ok(true)
    }
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

async fn test_aliyun_oss_connection(
    base_url: &str, access_key: &str, secret_key: &str, _bucket: &str,
) -> Result<bool, String> {
    use sha2::Sha256;
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<Sha256>;

    let now = chrono::Utc::now();
    let date = now.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    let string_to_sign = format!("GET\n\n\n{}\n/", date);

    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes()).unwrap();
    mac.update(string_to_sign.as_bytes());
    let signature = base64_encode(mac.finalize().into_bytes().to_vec());

    let auth_header = format!("OSS {}:{}", access_key, signature);

    let url = format!("{}/", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let resp = client.get(&url)
        .header("Authorization", &auth_header)
        .header("Date", &date)
        .send()
        .await
        .map_err(|e| format!("阿里云 OSS 连接失败: {}", e))?;

    if resp.status().is_success() {
        Ok(true)
    } else if resp.status() == 403 {
        Err("阿里云 OSS 连接失败: 认证被拒绝，请检查 AccessKey 和 SecretKey".into())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let error_msg = extract_xml_message(&body);
        Err(format!("阿里云 OSS 连接失败 ({}): {}", status, if error_msg.is_empty() { body.chars().take(200).collect::<String>() } else { error_msg }))
    }
}

pub async fn test_oss_connection(source: &DataSource) -> Result<bool, String> {
    let (scheme, host_str, default_port) = parse_oss_endpoint(source);
    let port = if source.port != 0 { source.port } else { default_port };

    let base_url = if port == default_port || default_port == 0 {
        format!("{}://{}", scheme, host_str)
    } else {
        format!("{}://{}:{}", scheme, host_str, port)
    };

    let access_key = &source.username;
    let secret_key = &source.password;

    if access_key.is_empty() || secret_key.is_empty() {
        return test_oss_reachable(&base_url).await;
    }

    match source.subtype.as_str() {
        "minio" | "aws" => {
            test_minio_sdk_connection(&base_url, access_key, secret_key, &source.database).await
        }
        "aliyun" => {
            test_aliyun_oss_connection(&base_url, access_key, secret_key, &source.database).await
        }
        _ => test_oss_reachable(&base_url).await,
    }
}

// ========== Database connection test ==========

pub async fn test_database_connection(source: &DataSource) -> Result<bool, String> {
    match source.subtype.as_str() {
        "postgresql" => {
            if source.username.is_empty() {
                return Err("PostgreSQL 需要提供用户名".into());
            }
            let url = format!(
                "postgresql://{}:{}@{}:{}/{}",
                urlencoding_encode(&source.username),
                urlencoding_encode(&source.password),
                source.host,
                source.port,
                urlencoding_encode(&source.database),
            );
            let pool_options = sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(std::time::Duration::from_secs(10));
            pool_options.connect(&url)
                .await
                .map(|_| true)
                .map_err(|e| format!("PostgreSQL 连接失败: {}", e))
        }
        "mysql" => {
            if source.username.is_empty() {
                return Err("MySQL 需要提供用户名".into());
            }
            let url = format!(
                "mysql://{}:{}@{}:{}/{}",
                urlencoding_encode(&source.username),
                urlencoding_encode(&source.password),
                source.host,
                source.port,
                urlencoding_encode(&source.database),
            );
            let pool_options = sqlx::mysql::MySqlPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(std::time::Duration::from_secs(10));
            pool_options.connect(&url)
                .await
                .map(|_| true)
                .map_err(|e| format!("MySQL 连接失败: {}", e))
        }
        "spatialite" => {
            if source.database.is_empty() {
                return Err("SpatiaLite 需要提供数据库文件路径".into());
            }
            rusqlite::Connection::open(&source.database)
                .map(|_| true)
                .map_err(|e| format!("SpatiaLite 连接失败: {}", e))
        }
        _ => Err(format!("不支持的数据库类型: {}", source.subtype)),
    }
}

// ========== Tauri command ==========

#[tauri::command]
pub async fn test_connection(
    source: DataSource,
) -> Result<bool, String> {
    match source.ds_type.as_str() {
        "database" => test_database_connection(&source).await,
        "oss" => test_oss_connection(&source).await,
        _ => Err(format!("不支持的数据源类型: {}", source.ds_type)),
    }
}

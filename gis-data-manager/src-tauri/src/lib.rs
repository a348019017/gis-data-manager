use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::Manager;
use tokio::sync::Mutex;

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

fn default_port() -> u16 {
    5432
}

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

// ==================== 服务注册 ====================

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

// ==================== 数据管理 ====================

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
    pub tags: String, // comma-separated
}

// 数据字典条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictItem {
    pub id: String,
    pub category: String,  // 字典分类: data_type, data_source, importance
    pub label: String,     // 显示名称
    pub value: String,     // 实际值
    #[serde(default)]
    pub sort_order: i32,
}

const VECTOR_EXTENSIONS: &[&str] = &["shp", "geojson", "json", "gpkg", "kml", "kmz"];
const DOCUMENT_EXTENSIONS: &[&str] = &["pdf", "doc", "docx", "xls", "xlsx", "txt", "csv", "zip", "rar", "7z"];

fn detect_file_info(path: &str) -> (String, String, String) {
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

// ==================== GIS 工具 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GISTool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String, // spatial_analysis, data_conversion, coordinate, data_management, map_render
    pub tags: String, // comma-separated: ai, human, both
    pub params: String, // JSON array of {name, type, required, default, description}
    #[serde(default)]
    pub returns: String, // description of return value
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub example: String,
}

pub struct AppState {
    pub sources: Mutex<Vec<DataSource>>,
    pub services: Mutex<Vec<Service>>,
    pub db: Mutex<Connection>,
}

fn init_db(db_path: &str) -> Result<Connection, String> {
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }
    let conn = Connection::open(db_path).map_err(|e| format!("打开数据库失败: {}", e))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS data_sources (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            ds_type TEXT NOT NULL,
            subtype TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 5432,
            database TEXT NOT NULL,
            username TEXT NOT NULL,
            password TEXT NOT NULL DEFAULT '',
            remark TEXT NOT NULL DEFAULT '',
            connected INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .map_err(|e| format!("创建表失败: {}", e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("创建设置表失败: {}", e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS services (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            service_type TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            username TEXT NOT NULL DEFAULT '',
            password TEXT NOT NULL DEFAULT '',
            connected INTEGER NOT NULL DEFAULT 0,
            remark TEXT NOT NULL DEFAULT ''
        )",
        [],
    )
    .map_err(|e| format!("创建服务表失败: {}", e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS import_records (
            id TEXT PRIMARY KEY,
            file_name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            file_type TEXT NOT NULL,
            format TEXT NOT NULL,
            target_source_id TEXT NOT NULL,
            target_source_name TEXT NOT NULL,
            target_type TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            error_msg TEXT NOT NULL DEFAULT ''
        )",
        [],
    )
    .map_err(|e| format!("创建导入记录表失败: {}", e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS gis_tools (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            category TEXT NOT NULL,
            tags TEXT NOT NULL DEFAULT 'both',
            params TEXT NOT NULL DEFAULT '[]',
            returns TEXT NOT NULL DEFAULT '',
            example TEXT NOT NULL DEFAULT ''
        )",
        [],
    )
    .map_err(|e| format!("创建工具表失败: {}", e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS dict_items (
            id TEXT PRIMARY KEY,
            category TEXT NOT NULL,
            label TEXT NOT NULL,
            value TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .map_err(|e| format!("创建字典表失败: {}", e))?;

    // 预置标签字典
    conn.execute(
        "INSERT OR IGNORE INTO dict_items (id, category, label, value, sort_order) VALUES
            ('dict_dt_vector', 'data_type', '矢量数据', 'vector', 1),
            ('dict_dt_raster', 'data_type', '栅格数据', 'raster', 2),
            ('dict_dt_document', 'data_type', '文档资料', 'document', 3),
            ('dict_ds_field', 'data_source', '外业采集', 'field', 1),
            ('dict_ds_office', 'data_source', '内业制作', 'office', 2),
            ('dict_ds_third', 'data_source', '第三方提供', 'third_party', 3),
            ('dict_ds_history', 'data_source', '历史数据', 'historical', 4),
            ('dict_imp_high', 'importance', '重要', 'high', 1),
            ('dict_imp_normal', 'importance', '一般', 'normal', 2),
            ('dict_imp_ref', 'importance', '参考', 'reference', 3)
        ",
        [],
    )
    .map_err(|e| format!("预置字典失败: {}", e))?;

    // 为 import_records 表添加 tags 字段（兼容旧数据库）
    conn.execute("ALTER TABLE import_records ADD COLUMN tags TEXT NOT NULL DEFAULT ''", [])
        .ok(); // 忽略字段已存在的错误

    // 预置 GIS 工具
    conn.execute(
        "INSERT OR IGNORE INTO gis_tools (id, name, description, category, tags, params, returns, example) VALUES
            ('tool_buffer', '缓冲区分析', '为矢量要素创建指定距离的缓冲区', 'spatial_analysis', 'both',
             '[{\"name\":\"distance\",\"type\":\"number\",\"required\":true,\"default\":100,\"description\":\"缓冲区距离(米)\"}]',
             '生成的缓冲矢量图层', '创建河流100米缓冲带'),
            ('tool_clip', '裁剪分析', '使用裁剪边界截取输入矢量数据', 'spatial_analysis', 'both',
             '[{\"name\":\"clip_layer\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"裁剪图层ID\"}]',
             '裁剪后的矢量图层', '用行政区边界裁剪土地利用数据'),
            ('tool_intersect', '相交分析', '计算两个图层的几何交集', 'spatial_analysis', 'ai',
             '[{\"name\":\"layer_a\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"输入图层A\"},{\"name\":\"layer_b\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"输入图层B\"}]',
             '相交部分的矢量数据', '找出两个地块的公共区域'),
            ('tool_union', '并集分析', '合并两个图层的几何数据', 'spatial_analysis', 'ai',
             '[{\"name\":\"layer_a\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"输入图层A\"},{\"name\":\"layer_b\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"输入图层B\"}]',
             '合并后的矢量数据', '合并相邻的两个地块'),
            ('tool_conv2shp', '转Shapefile', '将矢量数据转换为Shapefile格式', 'data_conversion', 'both',
             '[{\"name\":\"source_layer\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源图层ID\"},{\"name\":\"encoding\",\"type\":\"string\",\"required\":false,\"default\":\"UTF-8\",\"description\":\"字符编码\"}]',
             'Shapefile文件路径', '导出GeoJSON为Shapefile'),
            ('tool_conv2geojson', '转GeoJSON', '将矢量数据转换为GeoJSON格式', 'data_conversion', 'both',
             '[{\"name\":\"source_layer\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源图层ID\"}]',
             'GeoJSON文本内容', '将数据库矢量数据转为GeoJSON'),
            ('tool_conv2gpkg', '转GeoPackage', '将矢量数据转换为GeoPackage格式', 'data_conversion', 'both',
             '[{\"name\":\"source_layer\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源图层ID\"}]',
             'GeoPackage文件路径', '导出Shapefile为GeoPackage'),
            ('tool_proj_transform', '坐标转换', '将矢量数据从一个坐标系转换到另一个坐标系', 'coordinate', 'both',
             '[{\"name\":\"source_layer\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源图层ID\"},{\"name\":\"target_crs\",\"type\":\"string\",\"required\":true,\"default\":\"EPSG:4326\",\"description\":\"目标坐标系EPSG代码\"}]',
             '转换后的矢量图层', '将WGS84数据转为CGCS2000'),
            ('tool_reproject', '投影定义', '查看或修改图层的投影信息', 'coordinate', 'human',
             '[{\"name\":\"source_layer\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源图层ID\"},{\"name\":\"target_crs\",\"type\":\"string\",\"required\":true,\"default\":\"EPSG:3857\",\"description\":\"目标投影坐标系\"}]',
             '投影后的图层', '将WGS84转为Web Mercator投影'),
            ('tool_stats', '数据统计', '计算矢量图层的面积、周长、边界框等统计信息', 'data_management', 'both',
             '[{\"name\":\"source_layer\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源图层ID\"}]',
             '统计结果JSON', '获取地块图层的面积统计'),
            ('tool_query', '属性查询', '按属性条件筛选矢量要素', 'data_management', 'both',
             '[{\"name\":\"source_layer\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源图层ID\"},{\"name\":\"filter\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"SQL过滤条件\"}]',
             '筛选后的要素列表', '查询人口>100万的城市'),
            ('tool_merge', '图层合并', '合并多个同类型矢量图层', 'data_management', 'ai',
             '[{\"name\":\"source_layers\",\"type\":\"array\",\"required\":true,\"default\":[],\"description\":\"源图层ID列表\"}]',
             '合并后的图层', '合并三个年份的土地利用数据'),
            ('tool_gdal_warp', 'GDAL 栅格重投影', '使用GDAL对栅格数据进行重投影和几何校正', 'raster_processing', 'both',
             '[{\"name\":\"source_raster\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源栅格路径\"},{\"name\":\"target_crs\",\"type\":\"string\",\"required\":true,\"default\":\"EPSG:4326\",\"description\":\"目标坐标系\"},{\"name\":\"resample\",\"type\":\"string\",\"required\":false,\"default\":\"bilinear\",\"description\":\"重采样方法\"}]',
             '重投影后的栅格文件', '将遥感影像重投影到CGCS2000'),
            ('tool_gdal_translate', 'GDAL 栅格转换', '使用GDAL进行栅格格式转换、裁剪和重采样', 'raster_processing', 'both',
             '[{\"name\":\"source_raster\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源栅格路径\"},{\"name\":\"target_format\",\"type\":\"string\",\"required\":true,\"default\":\"GTiff\",\"description\":\"输出格式\"},{\"name\":\"window\",\"type\":\"string\",\"required\":false,\"default\":\"\",\"description\":\"裁剪窗口x1,y1,x2,y2\"}]',
             '转换后的栅格文件', '将TIFF转为GeoTIFF并裁剪'),
            ('tool_gdal_info', 'GDAL 栅格信息', '获取栅格文件的元数据、投影和统计信息', 'raster_processing', 'human',
             '[{\"name\":\"source_raster\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源栅格路径\"}]',
             '栅格元数据JSON', '查看遥感影像的波段和投影信息'),
            ('tool_gdal_calc', 'GDAL 栅格计算', '使用GDAL对多波段栅格进行数学运算', 'raster_processing', 'ai',
             '[{\"name\":\"source_raster\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源栅格路径\"},{\"name\":\"expression\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"计算表达式如(A-B)/(A+B)\"}]',
             '计算结果栅格', '计算NDVI=(NIR-Red)/(NIR+Red)'),
            ('tool_gdal_polygonize', 'GDAL 栅格转矢量', '将栅格数据的像元值转换为多边形矢量', 'raster_processing', 'both',
             '[{\"name\":\"source_raster\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"源栅格路径\"},{\"name\":\"field_name\",\"type\":\"string\",\"required\":false,\"default\":\"DN\",\"description\":\"属性字段名\"}]',
             '多边形矢量图层', '将土地利用栅格转为矢量多边形'),
            ('tool_ogr_info', 'OGR 数据源信息', '获取矢量数据源的结构、图层数量和字段信息', 'data_management', 'human',
             '[{\"name\":\"source_path\",\"type\":\"string\",\"required\":true,\"default\":\"\",\"description\":\"数据源路径或连接串\"}]',
             '数据源元数据JSON', '查看Shapefile的字段和坐标系')
        ",
        [],
    )
    .map_err(|e| format!("预置工具失败: {}", e))?;

    // 预置公开 GIS 数据源
    conn.execute(
        "INSERT OR IGNORE INTO data_sources (id, name, ds_type, subtype, host, port, database, username, password, remark, connected) VALUES
            ('preset_gis_oss', '云存储 OSS', 'oss', 'aliyun', 'oss-cn-hangzhou.aliyuncs.com', 443, 'gis-data', '', '', '对象存储（公开只读）', 1),
            ('preset_local_minio', '本地 MinIO', 'oss', 'minio', '127.0.0.1', 9104, 'gis-data', 'OuR9xtys9pYwLNGAxY63', 'IzJsO74Puwhl9tusijhX7kF7QAObZs5zewo2RVkA', '本地开发 MinIO 实例', 0)
        ",
        [],
    )
    .map_err(|e| format!("预置数据源失败: {}", e))?;

    // 预置公开 GIS 服务
    conn.execute(
        "INSERT OR IGNORE INTO services (id, name, service_type, endpoint, username, password, connected, remark) VALUES
            ('svc_tiandutu_wmts', '天地图 WMTS', 'wmts', 'http://t0.tianditu.gov.cn/img_w/wmts', '', '', 1, '天地图影像 WMTS 服务'),
            ('svc_geoserver_demo', 'GeoServer 示例', 'geoserver', 'https://demo.geo-solutions.it/geoserver', '', '', 0, 'GeoServer 公开演示服务'),
            ('svc_osm_wms', 'OSM WMS', 'wms', 'https://ows.mundialis.de/services/service', '', '', 1, 'Mundialis OSM WMS 服务')
        ",
        [],
    )
    .map_err(|e| format!("预置服务失败: {}", e))?;

    Ok(conn)
}

fn load_sources_from_db(conn: &Connection) -> Vec<DataSource> {
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

fn load_services_from_db(conn: &Connection) -> Vec<Service> {
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

fn load_tools_from_db(conn: &Connection) -> Vec<GISTool> {
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

fn insert_source(conn: &Connection, source: &DataSource) -> Result<(), String> {
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

fn update_source(conn: &Connection, source: &DataSource) -> Result<(), String> {
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

fn delete_source(conn: &Connection, id: &str) -> Result<bool, String> {
    let rows = conn
        .execute("DELETE FROM data_sources WHERE id=?1", params![id])
        .map_err(|e| format!("删除失败: {}", e))?;
    Ok(rows > 0)
}

// ==================== 数据源命令 ====================

#[tauri::command]
async fn get_data_sources(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<DataSource>, String> {
    let sources = state.sources.lock().await;
    Ok(sources.clone())
}

#[tauri::command]
async fn add_data_source(
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
async fn update_data_source(
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
async fn delete_data_source(
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

// ==================== 连接测试 ====================

#[tauri::command]
async fn test_connection(
    source: DataSource,
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<bool, String> {
    match source.ds_type.as_str() {
        "database" => test_database_connection(&source).await,
        "oss" => test_oss_connection(&source).await,
        _ => Err(format!("不支持的数据源类型: {}", source.ds_type)),
    }
}

// OSS 连接测试（使用 minio-rs SDK，与前端 minio npm 包等效的认证机制）
// 对 MinIO / AWS S3 使用 MinioClient.bucket_exists 验证凭据
// 对阿里云 OSS 使用 reqwest + OSS 签名方式验证
async fn test_oss_connection(source: &DataSource) -> Result<bool, String> {
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

async fn test_database_connection(source: &DataSource) -> Result<bool, String> {
    match source.subtype.as_str() {
        "postgresql" => {
            let url = format!(
                "postgresql://{}:{}@{}:{}/{}",
                urlencoding_encode(&source.username),
                urlencoding_encode(&source.password),
                source.host,
                source.port,
                urlencoding_encode(&source.database),
            );
            sqlx::PgPool::connect(&url)
                .await
                .map(|_| true)
                .map_err(|e| format!("PostgreSQL 连接失败: {}", e))
        }
        "mysql" => {
            let url = format!(
                "mysql://{}:{}@{}:{}/{}",
                urlencoding_encode(&source.username),
                urlencoding_encode(&source.password),
                source.host,
                source.port,
                urlencoding_encode(&source.database),
            );
            sqlx::MySqlPool::connect(&url)
                .await
                .map(|_| true)
                .map_err(|e| format!("MySQL 连接失败: {}", e))
        }
        "spatialite" => {
            rusqlite::Connection::open(&source.database)
                .map(|_| true)
                .map_err(|e| format!("SpatiaLite 连接失败: {}", e))
        }
        _ => Err(format!("不支持的数据库类型: {}", source.subtype)),
    }
}


fn urlencoding_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

// ==================== 设置相关 ====================

#[tauri::command]
async fn get_settings(state: tauri::State<'_, Arc<AppState>>) -> Result<Option<ModelSettings>, String> {
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
async fn save_settings(
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
async fn test_model_connection(settings: ModelSettings) -> Result<bool, String> {
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
async fn get_app_info(state: tauri::State<'_, Arc<AppState>>) -> Result<AppInfo, String> {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[tauri::command]
async fn chat_message(
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

// ==================== 服务注册命令 ====================

#[tauri::command]
async fn get_services(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<Service>, String> {
    let services = state.services.lock().await;
    Ok(services.clone())
}

#[tauri::command]
async fn add_service(
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
async fn update_service(
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
async fn delete_service(
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
async fn test_service_connection(
    service: Service,
    _state: tauri::State<'_, Arc<AppState>>,
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
        let content_type = resp.headers().get("content-type")
            .map(|v| v.to_str().unwrap_or(""))
            .unwrap_or("");

        // GeoServer REST API returns JSON, OGC services return XML
        if content_type.contains("xml") || content_type.contains("json") || content_type.contains("text") {
            Ok(true)
        } else {
            // 有些服务器可能返回其他 content-type 但仍然是有效响应
            Ok(true)
        }
    } else {
        Err(format!("连接失败: HTTP {}", resp.status()))
    }
}

// ==================== 数据管理命令 ====================

#[tauri::command]
async fn get_import_records(
    limit: Option<i32>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<ImportRecord>, String> {
    let db = state.db.lock().await;
    let limit = limit.unwrap_or(50);
    let mut stmt = db.prepare(
        "SELECT id, file_name, file_path, file_size, file_type, format,
                target_source_id, target_source_name, target_type, status,
                created_at, error_msg, tags
         FROM import_records
         ORDER BY created_at DESC
         LIMIT ?1"
    ).map_err(|e| format!("查询失败: {}", e))?;

    let records = stmt.query_map(params![limit], |row| {
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

    Ok(records)
}

#[tauri::command]
async fn delete_import_record(
    id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.execute("DELETE FROM import_records WHERE id=?1", params![id])
        .map_err(|e| format!("删除失败: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn import_file(
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

async fn upload_to_oss(
    file_path: &str,
    source: &DataSource,
    file_name: &str,
    app: &tauri::AppHandle,
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

async fn upload_to_s3(
    file_path: &str,
    source: &DataSource,
    file_name: &str,
    app: &tauri::AppHandle,
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

    // 构建 endpoint URL
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
        // host already has port
        format!("{}{}", scheme, host_only)
    } else {
        format!("{}{}:{}", scheme, host_only, port)
    };

    let base_url = base_url.parse::<BaseUrl>()
        .map_err(|e| format!("URL 解析失败: {}", e))?;

    let provider = StaticProvider::new(access_key, secret_key, None);
    let client = MinioClient::new(base_url, Some(provider), None, None)
        .map_err(|e| format!("创建客户端失败: {}", e))?;

    // 确保 bucket 存在
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

    // 检查文件是否已存在（重复性判断）
    let existing = client.stat_object(bucket_name.clone(), ObjectKey::new(file_name)
        .map_err(|e| format!("对象键无效: {}", e))?)
        .map_err(|e| format!("对象键无效: {}", e))?
        .build()
        .send()
        .await;
    if existing.is_ok() {
        return Err(format!("文件 '{}' 已存在于 Bucket 中，请勿重复上传", file_name));
    }

    // 获取文件大小用于发送进度
    let metadata = std::fs::metadata(file_path)
        .map_err(|e| format!("获取文件信息失败: {}", e))?;
    let file_size = metadata.len();

    // 发送开始事件
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

    // 发送完成事件
    let _ = app.emit("import-progress", serde_json::json!({
        "file_name": file_name,
        "status": "success",
        "bytes_sent": file_size,
        "total_bytes": file_size,
    }));

    Ok(())
}

async fn upload_to_aliyun_oss(
    file_path: &str,
    source: &DataSource,
    file_name: &str,
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

    // 检查文件是否已存在（重复性判断）
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

#[tauri::command]
async fn download_file(
    record_id: String,
    target_source_id: String,
    save_path: String,
    state: tauri::State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let sources = state.sources.lock().await;
    let db = state.db.lock().await;

    // 查询导入记录
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

    // 使用用户选择的目标数据源
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

    // 下载文件
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

async fn download_from_s3(
    source: &DataSource,
    file_name: &str,
    save_path: &str,
    app: &tauri::AppHandle,
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

    // 获取文件信息用于进度
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

    // 确保目标目录存在
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
    source: &DataSource,
    file_name: &str,
    save_path: &str,
    app: &tauri::AppHandle,
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
async fn copy_to_data_dir(
    file_path: &str,
    file_name: &str,
    app: &tauri::AppHandle,
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

// ==================== GIS 工具命令 ====================

#[tauri::command]
async fn get_gis_tools(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<GISTool>, String> {
    let db = state.db.lock().await;
    Ok(load_tools_from_db(&db))
}

#[tauri::command]
async fn add_gis_tool(
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
async fn update_gis_tool(
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
async fn delete_gis_tool(
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
async fn execute_gis_tool(
    tool_id: String,
    params: serde_json::Value,
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    // MVP: 返回工具执行模拟结果，实际实现需要对接 GDAL/OGR
    Ok(serde_json::json!({
        "tool_id": tool_id,
        "status": "completed",
        "message": format!("工具 {} 已执行（模拟）", tool_id),
        "params": params
    }))
}

// ==================== 数据字典 ====================

#[tauri::command]
async fn get_dict_items(
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
async fn add_dict_item(
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
async fn update_dict_item(
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
async fn delete_dict_item(
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
async fn get_dict_categories(
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

// ==================== 应用入口 ====================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string();

            let db_path = std::path::Path::new(&data_dir)
                .join("gis-data-manager.db")
                .to_string_lossy()
                .to_string();

            let db = init_db(&db_path).expect("init database");
            let sources = load_sources_from_db(&db);
            let services = load_services_from_db(&db);

            app.manage(Arc::new(AppState {
                sources: Mutex::new(sources),
                services: Mutex::new(services),
                db: Mutex::new(db),
            }));

            // 系统托盘菜单
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = PredefinedMenuItem::quit(app, None)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            if let Some(tray) = app.tray_by_id("gis-tray") {
                tray.set_menu(Some(menu))?;
                tray.set_tooltip(Some("GIS Data Manager"))?;

                // 托盘图标点击事件：左键显示窗口，右键弹出菜单
                let app_handle = app.handle().clone();
                tray.on_tray_icon_event(move |_tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        if let Some(window) = app_handle.webview_windows().values().next() {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_data_sources,
            add_data_source,
            update_data_source,
            delete_data_source,
            test_connection,
            get_settings,
            save_settings,
            test_model_connection,
            get_app_info,
            chat_message,
            get_services,
            add_service,
            update_service,
            delete_service,
            test_service_connection,
            get_import_records,
            import_file,
            delete_import_record,
            download_file,
            get_gis_tools,
            add_gis_tool,
            update_gis_tool,
            delete_gis_tool,
            execute_gis_tool,
            get_dict_items,
            add_dict_item,
            update_dict_item,
            delete_dict_item,
            get_dict_categories,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

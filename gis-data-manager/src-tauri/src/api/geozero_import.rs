use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use crate::AppState;
use crate::api::data_source::DataSource;
use std::sync::Arc;

// ==================== Info structs ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoFileInfo {
    pub file_name: String,
    pub format: String,
    pub geometry_type: String,
    pub feature_count: usize,
    pub crs: Option<String>,
    pub crs_epsg: Option<i32>,
    pub fields: Vec<PropertyField>,
    pub bounding_box: Option<(f64, f64, f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyField {
    pub name: String,
    pub pg_type: String,
}

// ==================== geo_types → WKT ====================

fn geo_to_wkt(geom: &geo_types::Geometry<f64>) -> String {
    match geom {
        geo_types::Geometry::Point(ref p) => {
            format!("POINT({} {})", p.x(), p.y())
        }
        geo_types::Geometry::MultiPoint(ref mp) => {
            let pts: Vec<String> = mp.iter().map(|p| format!("{} {}", p.x(), p.y())).collect();
            format!("MULTIPOINT({})", pts.join(", "))
        }
        geo_types::Geometry::Line(ref l) => {
            format!("LINESTRING({} {}, {} {})", l.start.x, l.start.y, l.end.x, l.end.y)
        }
        geo_types::Geometry::LineString(ref ls) => {
            let coords: Vec<String> = ls.points().map(|p| format!("{} {}", p.x(), p.y())).collect();
            format!("LINESTRING({})", coords.join(", "))
        }
        geo_types::Geometry::MultiLineString(ref ml) => {
            let lines: Vec<String> = ml.iter().map(|l| {
                let coords: Vec<String> = l.points().map(|p| format!("{} {}", p.x(), p.y())).collect();
                format!("({})", coords.join(", "))
            }).collect();
            format!("MULTILINESTRING({})", lines.join(", "))
        }
        geo_types::Geometry::Polygon(ref poly) => {
            let rings: Vec<String> = std::iter::once(poly.exterior())
                .chain(poly.interiors().iter())
                .map(|ring| {
                    let coords: Vec<String> = ring.points().map(|p| format!("{} {}", p.x(), p.y())).collect();
                    format!("({})", coords.join(", "))
                }).collect();
            format!("POLYGON({})", rings.join(", "))
        }
        geo_types::Geometry::MultiPolygon(ref mp) => {
            let polys: Vec<String> = mp.iter().map(|poly| {
                let rings: Vec<String> = std::iter::once(poly.exterior())
                    .chain(poly.interiors().iter())
                    .map(|ring| {
                        let coords: Vec<String> = ring.points().map(|p| format!("{} {}", p.x(), p.y())).collect();
                        format!("({})", coords.join(", "))
                    }).collect();
                format!("({})", rings.join(", "))
            }).collect();
            format!("MULTIPOLYGON({})", polys.join(", "))
        }
        geo_types::Geometry::GeometryCollection(ref gc) => {
            let geoms: Vec<String> = gc.iter().map(|g| geo_to_wkt(g)).collect();
            format!("GEOMETRYCOLLECTION({})", geoms.join(", "))
        }
        _ => "POINT(0 0)".to_string(),
    }
}

fn geom_type_name(geom: &geo_types::Geometry<f64>) -> &'static str {
    match geom {
        geo_types::Geometry::Point(_) => "Point",
        geo_types::Geometry::MultiPoint(_) => "MultiPoint",
        geo_types::Geometry::Line(_) | geo_types::Geometry::LineString(_) => "LineString",
        geo_types::Geometry::MultiLineString(_) => "MultiLineString",
        geo_types::Geometry::Polygon(_) => "Polygon",
        geo_types::Geometry::MultiPolygon(_) => "MultiPolygon",
        geo_types::Geometry::GeometryCollection(_) => "GeometryCollection",
        _ => "Unknown",
    }
}

// ==================== GeoJSON info ====================

#[tauri::command]
pub fn read_geojson_info(file_path: String) -> Result<GeoFileInfo, String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    let file_name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let data = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("读取文件失败: {}", e))?;

    let root: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| format!("解析 GeoJSON 失败: {}", e))?;

    // Handle both FeatureCollection and single Feature
    let features = match root.get("type").and_then(|t| t.as_str()) {
        Some("FeatureCollection") => {
            root.get("features")
                .and_then(|f| f.as_array())
                .ok_or("GeoJSON FeatureCollection 缺少 features 数组")?
        }
        Some("Feature") => {
            return read_geojson_info_from_features(&[root], &file_name);
        }
        Some("Geometry") => {
            return Err("不支持直接导入 Geometry 对象，请使用 Feature 或 FeatureCollection".into());
        }
        _ => return Err("不支持的 GeoJSON 类型，需要 FeatureCollection 或 Feature".into()),
    };

    read_geojson_info_from_features(features, &file_name)
}

fn read_geojson_info_from_features(
    features: &[serde_json::Value],
    file_name: &str,
) -> Result<GeoFileInfo, String> {
    let count = features.len();

    let (geom_type, fields) = if let Some(first) = features.first() {
        let gt = first.get("geometry")
            .and_then(|g| g.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let mut props = Vec::new();
        if let Some(properties) = first.get("properties").and_then(|p| p.as_object()) {
            for (key, val) in properties {
                let pg_type = json_value_to_pg_type(val);
                props.push(PropertyField { name: key.clone(), pg_type });
            }
        }
        (gt, props)
    } else {
        (String::from("Unknown"), vec![])
    };

    Ok(GeoFileInfo {
        file_name: file_name.to_string(),
        format: "geojson".to_string(),
        geometry_type: geom_type,
        feature_count: count,
        crs: None,
        crs_epsg: None,
        fields,
        bounding_box: None,
    })
}

fn json_value_to_pg_type(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Number(n) => {
            if n.is_f64() { "DOUBLE PRECISION".to_string() }
            else if n.is_i64() {
                let v = n.as_i64().unwrap_or(0);
                if v.abs() <= 32767 { "SMALLINT".to_string() }
                else if v.abs() <= 2_147_483_647 { "INTEGER".to_string() }
                else { "BIGINT".to_string() }
            } else { "DOUBLE PRECISION".to_string() }
        }
        serde_json::Value::String(_) => "VARCHAR(255)".to_string(),
        serde_json::Value::Bool(_) => "BOOLEAN".to_string(),
        serde_json::Value::Array(_) => "VARCHAR(1024)".to_string(),
        _ => "VARCHAR(255)".to_string(),
    }
}

// ==================== FlatGeobuf info ====================

fn iter_columns<'a>(header: &'a flatgeobuf::Header<'a>) -> Vec<flatgeobuf::Column<'a>> {
    match header.columns() {
        Some(cols) => (0..cols.len()).map(|i| cols.get(i)).collect(),
        None => vec![],
    }
}

#[tauri::command]
pub fn read_flatgeobuf_info(file_path: String) -> Result<GeoFileInfo, String> {
    use flatgeobuf::FgbReader;
    use std::io::BufReader;
    use std::fs::File;

    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    let file_name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let file = File::open(&file_path)
        .map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::new(file);
    let fgb = FgbReader::open(&mut reader)
        .map_err(|e| format!("读取 FlatGeobuf 失败: {}", e))?;

    let header = fgb.header();
    let geom_type = format!("{:?}", header.geometry_type());
    let feature_count = header.features_count() as usize;

    let crs_str = header.crs().and_then(|c| {
        c.code_string().or_else(|| c.name()).map(|s| s.to_string())
    });

    let columns = iter_columns(&header);
    let fields: Vec<PropertyField> = columns.iter().map(|col| {
        let pg_type = match col.type_() {
            flatgeobuf::ColumnType::Byte => "SMALLINT",
            flatgeobuf::ColumnType::UByte => "SMALLINT",
            flatgeobuf::ColumnType::Bool => "BOOLEAN",
            flatgeobuf::ColumnType::Short => "SMALLINT",
            flatgeobuf::ColumnType::UShort => "INTEGER",
            flatgeobuf::ColumnType::Int => "INTEGER",
            flatgeobuf::ColumnType::UInt => "BIGINT",
            flatgeobuf::ColumnType::Long => "BIGINT",
            flatgeobuf::ColumnType::ULong => "BIGINT",
            flatgeobuf::ColumnType::Float => "DOUBLE PRECISION",
            flatgeobuf::ColumnType::Double => "DOUBLE PRECISION",
            flatgeobuf::ColumnType::String => "VARCHAR(255)",
            flatgeobuf::ColumnType::Json => "TEXT",
            flatgeobuf::ColumnType::DateTime => "TIMESTAMP",
            _ => "VARCHAR(255)",
        };
        PropertyField {
            name: col.name().to_string(),
            pg_type: pg_type.to_string(),
        }
    }).collect();

    let bbox = header.envelope().map(|e| (e.get(0), e.get(1), e.get(2), e.get(3)));

    Ok(GeoFileInfo {
        file_name,
        format: "flatgeobuf".to_string(),
        geometry_type: geom_type,
        feature_count,
        crs: crs_str,
        crs_epsg: None,
        fields,
        bounding_box: bbox,
    })
}

// ==================== Shared PostGIS import ====================

fn sanitize_column(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

fn create_import_record(
    state: &Arc<AppState>,
    file_path: &str,
    file_name: &str,
    file_size: u64,
    format: &str,
    target_source_id: &str,
    source_name: &str,
) -> String {
    let record_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    let db = state.db.blocking_lock();
    let _ = db.execute(
        "INSERT INTO import_records (id, file_name, file_path, file_size, file_type, format,
         target_source_id, target_source_name, target_type, status, created_at, tags)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            record_id, file_name, file_path, file_size as i64, "vector", format,
            target_source_id, source_name, "database", "processing", created_at, ""
        ],
    );
    record_id
}

fn fail_import_record(state: &Arc<AppState>, record_id: &str, error: &str) {
    let db = state.db.blocking_lock();
    let _ = db.execute(
        "UPDATE import_records SET status='failed', error_msg=?2 WHERE id=?1",
        params![record_id, error],
    );
}

fn succeed_import_record(state: &Arc<AppState>, record_id: &str) {
    let db = state.db.blocking_lock();
    let _ = db.execute(
        "UPDATE import_records SET status='success' WHERE id=?1",
        params![record_id],
    );
}

async fn connect_postgis(source: &DataSource) -> Result<tokio_postgres::Client, String> {
    let (client, connection) = tokio_postgres::Config::new()
        .host(&source.host)
        .port(source.port)
        .dbname(&source.database)
        .user(&source.username)
        .password(&source.password)
        .connect(tokio_postgres::NoTls)
        .await
        .map_err(|e| format!("连接 PostGIS 失败: {}", e))?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("PostGIS connection error: {}", e);
        }
    });

    Ok(client)
}

async fn create_postgis_table(
    client: &tokio_postgres::Client,
    table_name: &str,
    target_srid: i32,
    fields: &[PropertyField],
) -> Result<(), String> {
    let mut sql = format!(
        "CREATE TABLE {} (gid SERIAL PRIMARY KEY, geom geometry(Geometry, {})",
        table_name, target_srid
    );
    for field in fields {
        let col = sanitize_column(&field.name);
        sql.push_str(&format!(", {} {}", col, field.pg_type));
    }
    sql.push(')');
    client.execute(&sql, &[])
        .await
        .map_err(|e| format!("创建表失败: {}", e))?;

    client.execute(
        &format!("CREATE INDEX {}_geom_idx ON {} USING GIST(geom)", table_name, table_name),
        &[],
    ).await
        .map_err(|e| format!("创建空间索引失败: {}", e))?;

    Ok(())
}

async fn insert_features_to_postgis(
    client: &tokio_postgres::Client,
    table_name: &str,
    target_srid: i32,
    fields: &[PropertyField],
    geoms_and_props: &[(geo_types::Geometry<f64>, Vec<Option<String>>)],
    file_path: &str,
    app: &tauri::AppHandle,
) -> Result<usize, String> {
    let total = geoms_and_props.len();
    let file_name = std::path::Path::new(file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    for (idx, (geom, prop_values)) in geoms_and_props.iter().enumerate() {
        let wkt = geo_to_wkt(geom);

        let mut col_names = vec!["geom".to_string()];
        let mut placeholders = vec![format!("ST_GeomFromText($1, {})", target_srid)];
        let mut param_values: Vec<Option<String>> = vec![Some(wkt)];

        for (i, field) in fields.iter().enumerate() {
            col_names.push(sanitize_column(&field.name));
            let idx = i + 2;
            placeholders.push(format!("${}", idx));
            let val = prop_values.get(i).cloned().flatten();
            param_values.push(val);
        }

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            col_names.join(", "),
            placeholders.join(", ")
        );

        let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            param_values.iter().map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
        client.execute(&sql, &params_refs)
            .await
            .map_err(|e| format!("插入记录失败: {}", e))?;

        let progress = if total > 0 {
            ((idx + 1) as f64 / total as f64 * 100.0) as i32
        } else { 0 };
        let _ = app.emit("shapefile-import-progress", serde_json::json!({
            "file_name": file_name,
            "current": idx + 1,
            "total": total,
            "progress": progress,
        }));
    }

    Ok(total)
}

// ==================== GeoJSON → PostGIS ====================

#[tauri::command]
pub async fn import_geojson_to_postgis(
    file_path: String,
    target_source_id: String,
    table_name: String,
    target_srid: i32,
    state: tauri::State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    use geozero::ToGeo;

    let sources = state.sources.lock().await;
    let source = sources.iter()
        .find(|s| s.id == target_source_id)
        .ok_or("目标数据源不存在")?
        .clone();
    drop(sources);

    if source.ds_type != "database" || source.subtype != "postgresql" {
        return Err("目标数据源必须是 PostgreSQL/PostGIS 类型".into());
    }

    let file_name = std::path::Path::new(&file_path)
        .file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let file_size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

    let record_id = create_import_record(&state, &file_path, &file_name, file_size, "geojson", &target_source_id, &source.name);

    // Parse GeoJSON
    let data = std::fs::read_to_string(&file_path)
        .map_err(|e| {
            fail_import_record(&state, &record_id, &format!("读取文件失败: {}", e));
            format!("读取文件失败: {}", e)
        })?;

    let root: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| {
            fail_import_record(&state, &record_id, &format!("解析 GeoJSON 失败: {}", e));
            format!("解析 GeoJSON 失败: {}", e)
        })?;

    let features: Vec<serde_json::Value> = match root.get("type").and_then(|t| t.as_str()) {
        Some("FeatureCollection") => {
            root.get("features")
                .and_then(|f| f.as_array())
                .map(|arr| arr.clone())
                .ok_or_else(|| {
                    let err = "GeoJSON FeatureCollection 缺少 features 数组".to_string();
                    fail_import_record(&state, &record_id, &err);
                    err
                })?
        }
        Some("Feature") => vec![root],
        _ => {
            let err = "不支持的 GeoJSON 类型".to_string();
            fail_import_record(&state, &record_id, &err);
            return Err(err);
        }
    };

    // Detect property fields and types from all features
    let fields = detect_geojson_fields(&features);

    // Connect to PostGIS
    let client = connect_postgis(&source).await.map_err(|e| {
        fail_import_record(&state, &record_id, &e);
        e
    })?;

    // Create table
    create_postgis_table(&client, &table_name, target_srid, &fields).await.map_err(|e| {
        fail_import_record(&state, &record_id, &e);
        e
    })?;

    // Parse features to geometry + property values
    let mut geoms_and_props: Vec<(geo_types::Geometry<f64>, Vec<Option<String>>)> = Vec::new();
    for feature in &features {
        let geom_value = feature.get("geometry").ok_or_else(|| {
            format!("Feature 缺少 geometry: {:?}", feature)
        }).map_err(|e| {
            fail_import_record(&state, &record_id, &e);
            e
        })?;

        let geom_str = serde_json::to_string(geom_value).map_err(|e| {
            let err = format!("序列化 Geometry 失败: {}", e);
            fail_import_record(&state, &record_id, &err);
            err
        })?;
        let geojson_geom = geozero::geojson::GeoJsonString(geom_str);

        let geo_geom = geojson_geom.to_geo().map_err(|e| {
            let err = format!("转换 geometry 失败: {}", e);
            fail_import_record(&state, &record_id, &err);
            err
        })?;

        let props = feature.get("properties").and_then(|p| p.as_object());
        let prop_values: Vec<Option<String>> = fields.iter().map(|f| {
            props.and_then(|p| p.get(&f.name)).and_then(|v| match v {
                serde_json::Value::Null => None,
                serde_json::Value::String(s) => Some(s.clone()),
                other => Some(other.to_string()),
            })
        }).collect();

        geoms_and_props.push((geo_geom, prop_values));
    }

    // Insert data
    let count = insert_features_to_postgis(
        &client, &table_name, target_srid, &fields, &geoms_and_props, &file_path, &app,
    ).await.map_err(|e| {
        fail_import_record(&state, &record_id, &e);
        e
    })?;

    succeed_import_record(&state, &record_id);
    Ok(format!("成功导入 {} 条记录到表 {}", count, table_name))
}

fn detect_geojson_fields(features: &[serde_json::Value]) -> Vec<PropertyField> {
    let mut field_map: std::collections::BTreeMap<String, &serde_json::Value> = std::collections::BTreeMap::new();
    for feature in features {
        if let Some(props) = feature.get("properties").and_then(|p| p.as_object()) {
            for (key, val) in props {
                if !val.is_null() {
                    field_map.entry(key.clone()).or_insert(val);
                }
            }
        }
    }
    field_map.iter().map(|(name, sample)| {
        PropertyField {
            name: name.clone(),
            pg_type: json_value_to_pg_type(sample),
        }
    }).collect()
}

// ==================== FlatGeobuf → PostGIS ====================

#[tauri::command]
pub async fn import_flatgeobuf_to_postgis(
    file_path: String,
    target_source_id: String,
    table_name: String,
    target_srid: i32,
    state: tauri::State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    use flatgeobuf::{FgbReader, FeatureProperties};
    use geozero::ToGeo;
    use std::io::BufReader;
    use std::fs::File;

    let sources = state.sources.lock().await;
    let source = sources.iter()
        .find(|s| s.id == target_source_id)
        .ok_or("目标数据源不存在")?
        .clone();
    drop(sources);

    if source.ds_type != "database" || source.subtype != "postgresql" {
        return Err("目标数据源必须是 PostgreSQL/PostGIS 类型".into());
    }

    let file_name = std::path::Path::new(&file_path)
        .file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let file_size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

    let record_id = create_import_record(&state, &file_path, &file_name, file_size, "flatgeobuf", &target_source_id, &source.name);

    // Open FGB
    let file = File::open(&file_path).map_err(|e| {
        let err = format!("打开文件失败: {}", e);
        fail_import_record(&state, &record_id, &err);
        err
    })?;
    let mut buf_reader = BufReader::new(file);

    let fgb_reader = FgbReader::open(&mut buf_reader).map_err(|e| {
        let err = format!("读取 FlatGeobuf 失败: {}", e);
        fail_import_record(&state, &record_id, &err);
        err
    })?;

    let header = fgb_reader.header();
    let columns = iter_columns(&header);
    let fields: Vec<PropertyField> = columns.iter().map(|col| {
        let pg_type = match col.type_() {
            flatgeobuf::ColumnType::Byte => "SMALLINT",
            flatgeobuf::ColumnType::UByte => "SMALLINT",
            flatgeobuf::ColumnType::Bool => "BOOLEAN",
            flatgeobuf::ColumnType::Short => "SMALLINT",
            flatgeobuf::ColumnType::UShort => "INTEGER",
            flatgeobuf::ColumnType::Int => "INTEGER",
            flatgeobuf::ColumnType::UInt => "BIGINT",
            flatgeobuf::ColumnType::Long => "BIGINT",
            flatgeobuf::ColumnType::ULong => "BIGINT",
            flatgeobuf::ColumnType::Float => "DOUBLE PRECISION",
            flatgeobuf::ColumnType::Double => "DOUBLE PRECISION",
            flatgeobuf::ColumnType::String => "VARCHAR(255)",
            flatgeobuf::ColumnType::Json => "TEXT",
            flatgeobuf::ColumnType::DateTime => "TIMESTAMP",
            _ => "VARCHAR(255)",
        };
        PropertyField { name: col.name().to_string(), pg_type: pg_type.to_string() }
    }).collect();

    // Connect to PostGIS
    let client = connect_postgis(&source).await.map_err(|e| {
        fail_import_record(&state, &record_id, &e);
        e
    })?;

    // Create table
    create_postgis_table(&client, &table_name, target_srid, &fields).await.map_err(|e| {
        fail_import_record(&state, &record_id, &e);
        e
    })?;

    // Select all and iterate
    let mut fgb_iter = fgb_reader.select_all_seq().map_err(|e| {
        let err = format!("读取 FGB 数据失败: {}", e);
        fail_import_record(&state, &record_id, &err);
        err
    })?;

    let mut geoms_and_props: Vec<(geo_types::Geometry<f64>, Vec<Option<String>>)> = Vec::new();

    // FeatureIter implements FallibleStreamingIterator
    use flatgeobuf::FallibleStreamingIterator;
    while let Some(feature) = fgb_iter.next().map_err(|e| {
        let err = format!("读取 FGB feature 失败: {}", e);
        fail_import_record(&state, &record_id, &err);
        err
    })? {
        let geo_geom = feature.to_geo().map_err(|e| {
            let err = format!("转换 FGB geometry 失败: {}", e);
            fail_import_record(&state, &record_id, &err);
            err
        })?;

        let prop_values: Vec<Option<String>> = fields.iter().map(|f| {
            feature.property::<String>(&f.name).ok()
        }).collect();

        geoms_and_props.push((geo_geom, prop_values));
    }

    // Insert data
    let count = insert_features_to_postgis(
        &client, &table_name, target_srid, &fields, &geoms_and_props, &file_path, &app,
    ).await.map_err(|e| {
        fail_import_record(&state, &record_id, &e);
        e
    })?;

    succeed_import_record(&state, &record_id);
    Ok(format!("成功导入 {} 条记录到表 {}", count, table_name))
}


// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 MultiPolygon → WKT 转换
    #[test]
    fn test_multipolygon_to_wkt() {
        use geo_types::{polygon, MultiPolygon, Geometry};

        let poly = polygon!(
            exterior: [
                (x: 0.0, y: 0.0),
                (x: 10.0, y: 0.0),
                (x: 10.0, y: 10.0),
                (x: 0.0, y: 10.0),
                (x: 0.0, y: 0.0),
            ],
            interiors: [],
        );
        let mp = MultiPolygon(vec![poly]);
        let geom = Geometry::MultiPolygon(mp);
        let wkt = geo_to_wkt(&geom);

        assert!(wkt.starts_with("MULTIPOLYGON"), "WKT 应以 MULTIPOLYGON 开头: {}", wkt);
        assert!(wkt.contains("((0 0"), "WKT 应包含坐标点: {}", wkt);
        println!("MultiPolygon WKT: {}", wkt);
    }

    /// 验证 geometry_type 名称检测
    #[test]
    fn test_geom_type_detection() {
        use geo_types::{point, polygon, line_string, MultiPolygon, Geometry};

        assert_eq!(geom_type_name(&Geometry::Point(point!(x: 0.0, y: 0.0))), "Point");
        assert_eq!(
            geom_type_name(&Geometry::LineString(line_string![
                (x: 0.0, y: 0.0), (x: 1.0, y: 1.0)
            ])),
            "LineString"
        );

        let poly = polygon!(
            exterior: [(x: 0.0, y: 0.0), (x: 1.0, y: 0.0), (x: 0.0, y: 1.0), (x: 0.0, y: 0.0)],
            interiors: [],
        );
        assert_eq!(geom_type_name(&Geometry::MultiPolygon(MultiPolygon(vec![poly]))), "MultiPolygon");

        assert_eq!(geom_type_name(&Geometry::GeometryCollection(
            geo_types::GeometryCollection(vec![])
        )), "GeometryCollection");
    }

    /// 测试 GeoJSON → PostGIS 导入全流程
    /// 使用 sampledata/geojson/1.geojson (414 条 MultiPolygon 记录)
    /// 重点检查 geometryType 是否导入正确
    #[tokio::test]
    async fn test_geojson_to_postgis_import() {
        use geozero::ToGeo;

        // 1. 定位测试 GeoJSON 文件
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let geojson_path = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&manifest_dir)
            .join("sampledata")
            .join("geojson")
            .join("1.geojson");
        let geojson_path_str = geojson_path.to_string_lossy().to_string();

        assert!(
            geojson_path.exists(),
            "测试 GeoJSON 文件不存在: {}",
            geojson_path_str
        );

        // 2. 读取文件并解析
        let data = std::fs::read_to_string(&geojson_path_str).expect("读取 GeoJSON 失败");
        let root: serde_json::Value = serde_json::from_str(&data).expect("解析 GeoJSON 失败");

        let features: Vec<serde_json::Value> = root
            .get("features")
            .and_then(|f| f.as_array())
            .expect("应为 FeatureCollection")
            .clone();
        let total = features.len();
        assert_eq!(total, 414, "1.geojson 应包含 414 条记录");
        println!("GeoJSON 文件包含 {} 条记录", total);

        // 3. 读取元信息，验证 geometry_type
        let info = read_geojson_info(geojson_path_str.clone());
        assert!(info.is_ok(), "读取 GeoJSON 信息失败: {:?}", info.err());
        let info = info.unwrap();
        assert_eq!(info.format, "geojson");
        assert_eq!(info.feature_count, 414);
        assert_eq!(
            info.geometry_type, "MultiPolygon",
            "geometry_type 应为 MultiPolygon，实际为: {}",
            info.geometry_type
        );
        println!("元信息 geometry_type: {}", info.geometry_type);

        // 4. 检测字段
        let fields = detect_geojson_fields(&features);
        println!("检测到 {} 个字段:", fields.len());
        for f in &fields {
            println!("  {} → {}", f.name, f.pg_type);
        }

        // 5. 连接 PostGIS（参数来自 db_init.rs preset_postgis）
        let (client, connection) = tokio_postgres::Config::new()
            .host("192.168.1.2")
            .port(55433)
            .dbname("gis")
            .user("abcuser")
            .password("abc123")
            .connect(tokio_postgres::NoTls)
            .await
            .expect("连接 PostGIS 失败");

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostGIS connection error: {}", e);
            }
        });

        // 6. 创建测试表
        let table_name = format!(
            "test_geojson_import_{}",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );
        let target_srid = 4326;

        create_postgis_table(&client, &table_name, target_srid, &fields)
            .await
            .expect("创建表失败");
        println!("测试表 {} 创建成功", table_name);

        // 7. 解析所有 feature → geo_types::Geometry + 属性值
        let mut geoms_and_props: Vec<(geo_types::Geometry<f64>, Vec<Option<String>>)> = Vec::new();

        for feature in &features {
            let geom_value = feature.get("geometry").expect("Feature 缺少 geometry");
            let geom_str = serde_json::to_string(geom_value).expect("序列化 Geometry 失败");
            let geojson_geom = geozero::geojson::GeoJsonString(geom_str);
            let geo_geom = geojson_geom.to_geo().expect("转换 geometry 失败");

            // 验证每条记录都是 MultiPolygon
            assert!(
                matches!(geo_geom, geo_types::Geometry::MultiPolygon(_)),
                "每条记录的 geometry 应为 MultiPolygon"
            );

            let props = feature.get("properties").and_then(|p| p.as_object());
            let prop_values: Vec<Option<String>> = fields
                .iter()
                .map(|f| {
                    props
                        .and_then(|p| p.get(&f.name))
                        .and_then(|v| match v {
                            serde_json::Value::Null => None,
                            serde_json::Value::String(s) => Some(s.clone()),
                            other => Some(other.to_string()),
                        })
                })
                .collect();

            geoms_and_props.push((geo_geom, prop_values));
        }

        assert_eq!(geoms_and_props.len(), total);
        println!("解析完成: {} 条 MultiPolygon 记录", geoms_and_props.len());

        // 8. 逐条插入 PostGIS
        for (idx, (geom, prop_values)) in geoms_and_props.iter().enumerate() {
            let wkt = geo_to_wkt(geom);

            let mut col_names = vec!["geom".to_string()];
            let mut placeholders = vec![format!("ST_GeomFromText($1, {})", target_srid)];
            let mut param_values: Vec<Option<String>> = vec![Some(wkt)];

            for (i, field) in fields.iter().enumerate() {
                col_names.push(sanitize_column(&field.name));
                placeholders.push(format!("${}", i + 2));
                param_values.push(prop_values.get(i).cloned().flatten());
            }

            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                col_names.join(", "),
                placeholders.join(", ")
            );

            let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = param_values
                .iter()
                .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                .collect();

            client
                .execute(&sql, &params_refs)
                .await
                .unwrap_or_else(|e| panic!("插入第 {} 条记录失败: {}", idx + 1, e));
        }
        println!("导入完成: {} 条记录", total);

        // 9. 验证记录数
        let row: (i64,) = client
            .query_one(&format!("SELECT COUNT(*) FROM {}", table_name), &[])
            .await
            .map(|r| (r.get(0),))
            .expect("查询记录数失败");
        assert_eq!(row.0 as usize, total, "数据库记录数应与导入数一致");

        // 10. ★ 重点检查: geometryType 是否导入正确
        let geom_types: Vec<(String,)> = client
            .query(
                &format!(
                    "SELECT DISTINCT ST_GeometryType(geom) FROM {}",
                    table_name
                ),
                &[],
            )
            .await
            .expect("查询 geometry type 失败")
            .iter()
            .map(|r| (r.get(0),))
            .collect();

        println!("数据库中的 geometry type(s): {:?}", geom_types);
        assert_eq!(
            geom_types.len(),
            1,
            "应只有一种 geometry type，实际: {:?}",
            geom_types
        );
        assert_eq!(
            geom_types[0].0, "ST_MultiPolygon",
            "ST_GeometryType 应返回 ST_MultiPolygon，实际: {}",
            geom_types[0].0
        );
        println!("✅ geometryType 验证通过: {}", geom_types[0].0);

        // 11. 验证几何字段非空
        let null_geoms: (i64,) = client
            .query_one(
                &format!("SELECT COUNT(*) FROM {} WHERE geom IS NULL", table_name),
                &[],
            )
            .await
            .map(|r| (r.get(0),))
            .expect("查询空几何失败");
        assert_eq!(null_geoms.0, 0, "不应存在空的几何字段");

        // 12. 验证 SRID
        let srid_row: (String,) = client
            .query_one(
                &format!("SELECT ST_SRID(geom)::text FROM {} LIMIT 1", table_name),
                &[],
            )
            .await
            .map(|r| (r.get(0),))
            .expect("查询 SRID 失败");
        assert_eq!(
            srid_row.0.parse::<i32>().unwrap(),
            target_srid,
            "SRID 应与导入时指定的一致"
        );

        println!(
            "验证通过: 表 {} 包含 {} 条 MultiPolygon 记录, SRID={}, GeometryType=ST_MultiPolygon",
            table_name, total, target_srid
        );

        // 13. 清理：删除测试表
        client
            .execute(&format!("DROP TABLE IF EXISTS {} CASCADE", table_name), &[])
            .await
            .expect("删除测试表失败");
        println!("测试表 {} 已清理", table_name);
    }
}

use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use crate::AppState;

// ==================== Shapefile 导入 PostGIS ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapefileInfo {
    pub file_name: String,
    pub shape_type: String,
    pub record_count: usize,
    pub crs: Option<String>,
    pub crs_epsg: Option<i32>,
    pub fields: Vec<DbfField>,
    pub bounding_box: Option<(f64, f64, f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbfField {
    pub name: String,
    pub dbf_type: String,
    pub length: u32,
    pub decimal_count: u32,
}

#[tauri::command]
pub async fn read_shapefile_info(file_path: String) -> Result<ShapefileInfo, String> {
    if !std::path::Path::new(&file_path).exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let reader = shapefile::ShapeReader::from_path(&file_path)
        .map_err(|e| format!("读取 Shapefile 失败: {}", e))?;

    let shape_type = reader.header().shape_type.to_string();

    let crs = read_prj_crs(&file_path);
    let crs_epsg = extract_epsg_from_prj(&crs);
    let fields = read_dbf_fields(&file_path);

    let bounding_box = {
        let bbox = &reader.header().bbox;
        Some((bbox.min.x, bbox.min.y, bbox.max.x, bbox.max.y))
    };

    let record_count = reader.shape_count().unwrap_or(0);

    Ok(ShapefileInfo {
        file_name,
        shape_type,
        record_count,
        crs,
        crs_epsg,
        fields,
        bounding_box,
    })
}

fn read_prj_crs(shp_path: &str) -> Option<String> {
    let prj_path = std::path::Path::new(shp_path).with_extension("prj");
    if prj_path.exists() {
        std::fs::read_to_string(&prj_path).ok()
    } else {
        None
    }
}

fn extract_epsg_from_prj(prj: &Option<String>) -> Option<i32> {
    let prj = prj.as_ref()?;
    if let Some(pos) = prj.find("AUTHORITY") {
        let rest = &prj[pos..];
        let digits: String = rest.chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        digits.parse::<i32>().ok()
    } else {
        None
    }
}

fn read_dbf_fields(shp_path: &str) -> Vec<DbfField> {
    let dbf_path = std::path::Path::new(shp_path).with_extension("dbf");
    if !dbf_path.exists() {
        return vec![];
    }
    // Use fallback parser — it correctly reads decimal_count from the raw DBF header,
    // which the shapefile crate's FieldInfo does not expose publicly.
    parse_dbf_header_fallback(&dbf_path)
}

fn parse_dbf_header_fallback(dbf_path: &std::path::Path) -> Vec<DbfField> {
    let bytes = match std::fs::read(dbf_path) {
        Ok(b) => b,
        Err(_) => return vec![],
    };
    if bytes.len() < 33 {
        return vec![];
    }

    let header_size = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    let field_count = (header_size - 32 - 1) / 32;
    let mut fields = Vec::new();

    for i in 0..field_count {
        let offset = 32 + i * 32;
        if offset + 32 > bytes.len() {
            break;
        }
        let name_bytes = &bytes[offset..offset + 11];
        let name_len = name_bytes.iter().position(|&b| b == 0).unwrap_or(11);
        let name = String::from_utf8_lossy(&name_bytes[..name_len]).trim().to_string();
        let dbf_type = bytes[offset + 11] as char;
        let length = bytes[offset + 16] as u32;
        let decimal_count = bytes[offset + 17] as u32;
        fields.push(DbfField { name, dbf_type: dbf_type.to_string(), length, decimal_count });
    }
    fields
}

#[tauri::command]
pub async fn import_shapefile_to_postgis(
    file_path: String,
    target_source_id: String,
    table_name: String,
    target_srid: i32,
    state: tauri::State<'_, std::sync::Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let sources = state.sources.lock().await;
    let source = sources
        .iter()
        .find(|s| s.id == target_source_id)
        .ok_or("目标数据源不存在")?
        .clone();
    drop(sources);

    if source.ds_type != "database" || source.subtype != "postgresql" {
        return Err("目标数据源必须是 PostgreSQL/PostGIS 类型".into());
    }

    // 获取文件元数据
    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let file_size = std::fs::metadata(&file_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let record_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    // 创建导入记录
    {
        let db = state.db.lock().await;
        let _ = db.execute(
            "INSERT INTO import_records (id, file_name, file_path, file_size, file_type, format,
             target_source_id, target_source_name, target_type, status, created_at, tags)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                record_id, file_name, file_path, file_size as i64, "vector", "shp",
                target_source_id, source.name, "database", "processing", created_at, ""
            ],
        );
    }

    let (client, connection) = tokio_postgres::Config::new()
        .host(&source.host)
        .port(source.port)
        .dbname(&source.database)
        .user(&source.username)
        .password(&source.password)
        .connect(tokio_postgres::NoTls)
        .await
        .map_err(|e| {
            // 更新导入记录为失败
            let db = state.db.blocking_lock();
            let _ = db.execute(
                "UPDATE import_records SET status='failed', error_msg=?2 WHERE id=?1",
                params![record_id, format!("连接 PostGIS 失败: {}", e)],
            );
            format!("连接 PostGIS 失败: {}", e)
        })?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("PostGIS connection error: {}", e);
        }
    });

    let fields = read_dbf_fields(&file_path);

    // 创建表
    let mut sql = format!(
        "CREATE TABLE {} (gid SERIAL PRIMARY KEY, geom geometry(Geometry, {})",
        table_name, target_srid
    );
    for field in &fields {
        let col = sanitize_column(&field.name);
        let pg_type = dbf_to_pg(field);
        sql.push_str(&format!(", {} {}", col, pg_type));
    }
    sql.push(')');
    if let Err(e) = client.execute(&sql, &[]).await {
        let db = state.db.lock().await;
        let _ = db.execute(
            "UPDATE import_records SET status='failed', error_msg=?2 WHERE id=?1",
            params![record_id, format!("创建表失败: {}", e)],
        );
        return Err(format!("创建表失败: {}", e));
    }

    if let Err(e) = client.execute(
        &format!("CREATE INDEX {}_geom_idx ON {} USING GIST(geom)", table_name, table_name),
        &[],
    ).await {
        let db = state.db.lock().await;
        let _ = db.execute(
            "UPDATE import_records SET status='failed', error_msg=?2 WHERE id=?1",
            params![record_id, format!("创建空间索引失败: {}", e)],
        );
        return Err(format!("创建空间索引失败: {}", e));
    }

    // 读取并插入数据
    let mut reader = shapefile::Reader::from_path(&file_path)
        .map_err(|e| format!("读取 Shapefile 失败: {}", e))?;

    let total = reader.shape_count().unwrap_or(0);

    let mut count = 0;
    for result in reader.iter_shapes_and_records() {
        let (shape, record) = result.map_err(|e| format!("读取记录失败: {}", e))?;

        let geom_wkt = shape_to_wkt(&shape);

        // Build INSERT with field values from record
        let mut col_names = vec!["geom".to_string()];
        let mut placeholders = vec![format!("ST_GeomFromText($1, {})", target_srid)];
        let mut param_values: Vec<Option<String>> = Vec::new();
        param_values.push(Some(geom_wkt));

        for (i, field) in fields.iter().enumerate() {
            let col = sanitize_column(&field.name);
            col_names.push(col);
            let idx = i + 2;
            placeholders.push(format!("${}", idx));

            // Get value from record by field name
            let val: Option<String> = record.get(&field.name).map(dbf_value_to_string);
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
        if let Err(e) = client.execute(&sql, &params_refs).await {
            let db = state.db.lock().await;
            let _ = db.execute(
                "UPDATE import_records SET status='failed', error_msg=?2 WHERE id=?1",
                params![record_id, format!("插入记录失败: {}", e)],
            );
            return Err(format!("插入记录失败: {}", e));
        }

        count += 1;
        let progress = if total > 0 { (count as f64 / total as f64 * 100.0) as i32 } else { 0 };
        let _ = app.emit("shapefile-import-progress", serde_json::json!({
            "file_name": std::path::Path::new(&file_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            "current": count, "total": total, "progress": progress,
        }));
    }

    // 更新导入记录为成功
    {
        let db = state.db.lock().await;
        let _ = db.execute(
            "UPDATE import_records SET status='success' WHERE id=?1",
            params![record_id],
        );
    }

    Ok(format!("成功导入 {} 条记录到表 {}", count, table_name))
}

fn sanitize_column(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

fn dbf_to_pg(field: &DbfField) -> String {
    match field.dbf_type.as_str() {
        "N" | "F" => {
            if field.decimal_count > 0 { "DOUBLE PRECISION".to_string() }
            else if field.length <= 4 { "SMALLINT".to_string() }
            else if field.length <= 9 { "INTEGER".to_string() }
            else { "BIGINT".to_string() }
        }
        "I" => {
            if field.length <= 4 { "SMALLINT".to_string() }
            else if field.length <= 9 { "INTEGER".to_string() }
            else { "BIGINT".to_string() }
        }
        "L" => "BOOLEAN".to_string(),
        "D" => "DATE".to_string(),
        _ => format!("VARCHAR({})", field.length.max(255)),
    }
}

fn dbf_value_to_string(val: &shapefile::dbase::FieldValue) -> String {
    match val {
        shapefile::dbase::FieldValue::Character(s) => s.clone().unwrap_or_default(),
        shapefile::dbase::FieldValue::Numeric(n) => n.map(|v| v.to_string()).unwrap_or_default(),
        shapefile::dbase::FieldValue::Float(f) => f.map(|v| v.to_string()).unwrap_or_default(),
        shapefile::dbase::FieldValue::Logical(b) => b.map(|v| v.to_string()).unwrap_or_default(),
        shapefile::dbase::FieldValue::Date(d) => {
            d.map(|dt| format!("{:04}-{:02}-{:02}", dt.year(), dt.month() as u32, dt.day() as u32))
                .unwrap_or_default()
        }
        shapefile::dbase::FieldValue::Integer(i) => i.to_string(),
        shapefile::dbase::FieldValue::Double(d) => d.to_string(),
        shapefile::dbase::FieldValue::Currency(c) => c.to_string(),
        shapefile::dbase::FieldValue::DateTime(dt) => {
            format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                dt.date().year(), dt.date().month(), dt.date().day(),
                dt.time().hours(), dt.time().minutes(), dt.time().seconds())
        }
        _ => String::new(),
    }
}

fn shape_to_wkt(shape: &shapefile::Shape) -> String {
    match shape {
        shapefile::Shape::Point(pt) => {
            format!("POINT({} {})", pt.x, pt.y)
        }
        shapefile::Shape::Polyline(poly) => {
            if poly.parts().len() <= 1 {
                let coords: Vec<String> = poly.parts()[0].iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                format!("LINESTRING({})", coords.join(", "))
            } else {
                let parts: Vec<String> = poly.parts().iter().map(|part| {
                    let coords: Vec<String> = part.iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                    format!("({})", coords.join(", "))
                }).collect();
                format!("MULTILINESTRING({})", parts.join(", "))
            }
        }
        shapefile::Shape::PolylineZ(poly) => {
            if poly.parts().len() <= 1 {
                let coords: Vec<String> = poly.parts()[0].iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                format!("LINESTRING({})", coords.join(", "))
            } else {
                let parts: Vec<String> = poly.parts().iter().map(|part| {
                    let coords: Vec<String> = part.iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                    format!("({})", coords.join(", "))
                }).collect();
                format!("MULTILINESTRING({})", parts.join(", "))
            }
        }
        shapefile::Shape::PolylineM(poly) => {
            if poly.parts().len() <= 1 {
                let coords: Vec<String> = poly.parts()[0].iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                format!("LINESTRING({})", coords.join(", "))
            } else {
                let parts: Vec<String> = poly.parts().iter().map(|part| {
                    let coords: Vec<String> = part.iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                    format!("({})", coords.join(", "))
                }).collect();
                format!("MULTILINESTRING({})", parts.join(", "))
            }
        }
        shapefile::Shape::Polygon(poly) => {
            let rings: Vec<String> = poly.rings().iter().map(|ring| {
                let coords: Vec<String> = ring.points().iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                format!("({})", coords.join(", "))
            }).collect();
            format!("POLYGON({})", rings.join(", "))
        }
        shapefile::Shape::PolygonZ(poly) => {
            let rings: Vec<String> = poly.rings().iter().map(|ring| {
                let coords: Vec<String> = ring.points().iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                format!("({})", coords.join(", "))
            }).collect();
            format!("POLYGON({})", rings.join(", "))
        }
        shapefile::Shape::PolygonM(poly) => {
            let rings: Vec<String> = poly.rings().iter().map(|ring| {
                let coords: Vec<String> = ring.points().iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                format!("({})", coords.join(", "))
            }).collect();
            format!("POLYGON({})", rings.join(", "))
        }
        shapefile::Shape::Multipoint(mp) => {
            let points: Vec<String> = mp.points().iter().map(|p| format!("{} {}", p.x, p.y)).collect();
            format!("MULTIPOINT({})", points.join(", "))
        }
        shapefile::Shape::MultipointZ(mp) => {
            let points: Vec<String> = mp.points().iter().map(|p| format!("{} {}", p.x, p.y)).collect();
            format!("MULTIPOINT({})", points.join(", "))
        }
        shapefile::Shape::MultipointM(mp) => {
            let points: Vec<String> = mp.points().iter().map(|p| format!("{} {}", p.x, p.y)).collect();
            format!("MULTIPOINT({})", points.join(", "))
        }
        shapefile::Shape::PointZ(pt) => {
            format!("POINT({} {})", pt.x, pt.y)
        }
        shapefile::Shape::PointM(pt) => {
            format!("POINT({} {})", pt.x, pt.y)
        }
        _ => "POINT(0 0)".to_string(),
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 Shapefile → PostGIS 导入全流程
    /// 使用 sampledata 目录下的测试数据
    /// 数据库参数来自 db_init.rs 中的预设 PostGIS 连接
    #[tokio::test]
    async fn test_shapefile_to_postgis_import() {
        // 1. 定位测试 Shapefile
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let shp_path = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&manifest_dir)
            .join("sampledata")
            .join("shp")
            .join("1.shp");
        let shp_path_str = shp_path.to_string_lossy().to_string();

        assert!(
            shp_path.exists(),
            "测试 Shapefile 不存在: {}",
            shp_path_str
        );

        // 2. 读取 Shapefile 元信息
        let info = read_shapefile_info(shp_path_str.clone()).await;
        assert!(info.is_ok(), "读取 Shapefile 信息失败: {:?}", info.err());
        let info = info.unwrap();
        assert_eq!(info.shape_type, "Polygon", "应为 Polygon 类型 Shapefile");
        assert!(info.record_count > 0, "Shapefile 应包含至少一条记录");
        assert!(!info.fields.is_empty(), "应包含 DBF 字段");
        println!(
            "Shapefile: {} 条记录, {} 个字段, 类型={}",
            info.record_count,
            info.fields.len(),
            info.shape_type
        );

        // 3. 连接 PostGIS（参数来自 db_init.rs preset_postgis）
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

        // 4. 创建测试表
        let table_name = format!("test_shp_import_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        let target_srid = info.crs_epsg.unwrap_or(4326);
        let fields = read_dbf_fields(&shp_path_str);

        let mut sql = format!(
            "CREATE TABLE {} (gid SERIAL PRIMARY KEY, geom geometry(Geometry, {})",
            table_name, target_srid
        );
        for field in &fields {
            let col = sanitize_column(&field.name);
            let pg_type = dbf_to_pg(field);
            sql.push_str(&format!(", {} {}", col, pg_type));
        }
        sql.push(')');
        client
            .execute(&sql, &[])
            .await
            .unwrap_or_else(|e| panic!("创建表 {} 失败: {}\nSQL: {}", table_name, e, sql));

        // 5. 读取并插入数据
        let mut reader =
            shapefile::Reader::from_path(&shp_path_str).expect("读取 Shapefile 失败");
        let total = reader.shape_count().unwrap_or(0);
        let mut count = 0;

        for result in reader.iter_shapes_and_records() {
            let (shape, record) = result.expect("读取记录失败");
            let geom_wkt = shape_to_wkt(&shape);

            let mut col_names = vec!["geom".to_string()];
            let mut placeholders = vec![format!("ST_GeomFromText($1, {})", target_srid)];
            let mut param_values: Vec<Option<String>> = vec![Some(geom_wkt)];

            for (i, field) in fields.iter().enumerate() {
                let col = sanitize_column(&field.name);
                col_names.push(col);
                let idx = i + 2;
                placeholders.push(format!("${}", idx));
                let val: Option<String> = record.get(&field.name).map(dbf_value_to_string);
                param_values.push(val);
            }

            let insert_sql = format!(
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
                .execute(&insert_sql, &params_refs)
                .await
                .unwrap_or_else(|e| panic!("插入第 {} 条记录失败: {}", count + 1, e));

            count += 1;
        }

        println!("导入完成: 共 {} 条记录", count);
        assert_eq!(count, total, "导入条数应与原始记录数一致");
        assert!(count > 0, "应至少导入 1 条记录");

        // 6. 验证数据
        let row: (i64,) = client
            .query_one(
                &format!("SELECT COUNT(*) FROM {}", table_name),
                &[],
            )
            .await
            .map(|r| (r.get(0),))
            .expect("查询记录数失败");
        assert_eq!(row.0 as usize, count, "数据库中的记录数应与导入数一致");

        // 验证几何字段非空
        let null_geoms: (i64,) = client
            .query_one(
                &format!("SELECT COUNT(*) FROM {} WHERE geom IS NULL", table_name),
                &[],
            )
            .await
            .map(|r| (r.get(0),))
            .expect("查询空几何失败");
        assert_eq!(null_geoms.0, 0, "不应存在空的几何字段");

        // 验证 SRID
        let srid_row: (String,) = client
            .query_one(
                &format!(
                    "SELECT ST_SRID(geom)::text FROM {} LIMIT 1",
                    table_name
                ),
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
            "验证通过: 表 {} 包含 {} 条Polygon记录, SRID={}",
            table_name, count, target_srid
        );

        // 7. 清理：删除测试表
        client
            .execute(&format!("DROP TABLE IF EXISTS {} CASCADE", table_name), &[])
            .await
            .expect("删除测试表失败");
        println!("测试表 {} 已清理", table_name);
    }

    /// 测试 WKT 转换和字段映射（纯函数测试，无需数据库）
    #[test]
    fn test_dbf_to_pg_type_mapping() {
        // Numeric with decimal → DOUBLE PRECISION
        let f = DbfField { name: "area".into(), dbf_type: "N".into(), length: 10, decimal_count: 2 };
        assert_eq!(dbf_to_pg(&f), "DOUBLE PRECISION");

        // Numeric without decimal, small → SMALLINT
        let f = DbfField { name: "id".into(), dbf_type: "N".into(), length: 4, decimal_count: 0 };
        assert_eq!(dbf_to_pg(&f), "SMALLINT");

        // Integer type, large → BIGINT
        let f = DbfField { name: "big_id".into(), dbf_type: "I".into(), length: 10, decimal_count: 0 };
        assert_eq!(dbf_to_pg(&f), "BIGINT");

        // Logical → BOOLEAN
        let f = DbfField { name: "flag".into(), dbf_type: "L".into(), length: 1, decimal_count: 0 };
        assert_eq!(dbf_to_pg(&f), "BOOLEAN");

        // Date → DATE
        let f = DbfField { name: "dt".into(), dbf_type: "D".into(), length: 8, decimal_count: 0 };
        assert_eq!(dbf_to_pg(&f), "DATE");

        // Char → VARCHAR with length
        let f = DbfField { name: "name".into(), dbf_type: "C".into(), length: 50, decimal_count: 0 };
        assert_eq!(dbf_to_pg(&f), "VARCHAR(255)");
    }

    #[test]
    fn test_sanitize_column() {
        assert_eq!(sanitize_column("Hello World"), "hello_world");
        assert_eq!(sanitize_column("ABC"), "abc");
        assert_eq!(sanitize_column("a_b"), "a_b");
        assert_eq!(sanitize_column("123test"), "123test");
    }
}

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
    // Try to use dbase reader to get field info
    match shapefile::dbase::Reader::from_path(&dbf_path) {
        Ok(dbf_reader) => {
            dbf_reader.fields().iter().map(|fi| {
                let field_char = match fi.field_type() {
                    shapefile::dbase::FieldType::Character => "C",
                    shapefile::dbase::FieldType::Numeric => "N",
                    shapefile::dbase::FieldType::Float => "F",
                    shapefile::dbase::FieldType::Logical => "L",
                    shapefile::dbase::FieldType::Date => "D",
                    shapefile::dbase::FieldType::Integer => "I",
                    _ => "C",
                };
                DbfField {
                    name: fi.name().to_string(),
                    dbf_type: field_char.to_string(),
                    length: fi.length() as u32,
                    decimal_count: 0,
                }
            }).collect()
        }
        Err(_) => {
            // Fallback: parse DBF header manually
            parse_dbf_header_fallback(&dbf_path)
        }
    }
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

    let fields = read_dbf_fields(&file_path);

    // 创建表
    let mut sql = format!(
        "CREATE TABLE {} (gid SERIAL PRIMARY KEY, geom geometry(Geometry, {}))",
        table_name, target_srid
    );
    for field in &fields {
        let col = sanitize_column(&field.name);
        let pg_type = dbf_to_pg(field);
        sql.push_str(&format!(", {} {}", col, pg_type));
    }
    client.execute(&sql, &[])
        .await
        .map_err(|e| format!("创建表失败: {}", e))?;

    client.execute(
        &format!("CREATE INDEX {}_geom_idx ON {} USING GIST(geom)", table_name, table_name),
        &[],
    ).await
        .map_err(|e| format!("创建空间索引失败: {}", e))?;

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
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> = Vec::new();
        params.push(Box::new(geom_wkt));

        for (i, field) in fields.iter().enumerate() {
            let col = sanitize_column(&field.name);
            col_names.push(col);
            let idx = i + 2;
            placeholders.push(format!("${}", idx));

            // Get value from record by field name
            let val: Option<String> = record.get(&field.name).map(dbf_value_to_string);
            params.push(Box::new(val));
        }

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            col_names.join(", "),
            placeholders.join(", ")
        );

        let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();
        client.execute(&sql, &params_refs)
            .await
            .map_err(|e| format!("插入记录失败: {}", e))?;

        count += 1;
        let progress = if total > 0 { (count as f64 / total as f64 * 100.0) as i32 } else { 0 };
        let _ = app.emit("shapefile-import-progress", serde_json::json!({
            "file_name": std::path::Path::new(&file_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            "current": count, "total": total, "progress": progress,
        }));
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
        "L" | "I" => "BOOLEAN".to_string(),
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

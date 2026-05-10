use rusqlite::Connection;

pub fn init_db(db_path: &str) -> Result<Connection, String> {
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
            ('preset_local_minio', '本地 MinIO', 'oss', 'minio', '127.0.0.1', 9104, 'gis-data', 'OuR9xtys9pYwLNGAxY63', 'IzJsO74Puwhl9tusijhX7kF7QAObZs5zewo2RVkA', '本地开发 MinIO 实例', 0),
            ('preset_postgis', 'PostGIS 测试库', 'database', 'postgresql', '192.168.1.2', 55433, 'postgres', 'abcuser', 'abc123', 'PostGIS 测试数据库', 0)
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

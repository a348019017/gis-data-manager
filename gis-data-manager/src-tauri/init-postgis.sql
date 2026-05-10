-- 启用 PostGIS 扩展
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS postgis_raster;

-- 创建示例空间表
CREATE TABLE IF NOT EXISTS sample_locations (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100),
    location GEOMETRY(Point, 4326),
    created_at TIMESTAMP DEFAULT NOW()
);

INSERT INTO sample_locations (name, location) VALUES
    ('北京', ST_SetSRID(ST_MakePoint(116.4074, 39.9042), 4326)),
    ('上海', ST_SetSRID(ST_MakePoint(121.4737, 31.2304), 4326)),
    ('广州', ST_SetSRID(ST_MakePoint(113.2644, 23.1291), 4326));

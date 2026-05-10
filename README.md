# GIS Data Manager 项目

基于 Tauri 2 + Vue 3 的桌面 GIS 数据管理工具。

## 快速开始

```bash
cd gis-data-manager
npm install
npm run tauri:dev
```

## 测试环境

使用 Docker Compose 一键搭建本地测试依赖服务（PostGIS + MinIO）：

```bash
# 启动所有服务
docker-compose up -d

# 查看状态
docker-compose ps

# 停止服务
docker-compose down
```

| 服务 | 端口 | 说明 |
| --- | --- | --- |
| PostGIS | 5433 | PostgreSQL 16 + PostGIS 3.4（数据库 `gis_data`，用户 `gis_admin`，密码 `gis_secret_2024`） |
| MinIO | 9000 (API) / 9001 (控制台) | S3 兼容对象存储（AccessKey `minio_admin`，SecretKey `minio_secret_2024`） |

启动后可访问 MinIO 控制台 `http://localhost:9001` 查看对象存储。

## 项目结构

- `gis-data-manager/` — Tauri 应用主体（前端 + Rust 后端）
- `project-design.md` — 项目设计文档
- `screenshots/` — 截图素材

详细功能说明和技术文档见 [gis-data-manager/README.md](gis-data-manager/README.md) 和 [project-design.md](project-design.md)。

## License

MIT

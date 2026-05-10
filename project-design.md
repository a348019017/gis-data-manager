# GIS 数据管理工具 - 项目设计文档

## 项目概述

- **项目类型**：桌面应用
- **技术栈**：Tauri 2（前端 Vue 3 + 后端 Rust）
- **项目定位**：GIS 数据管理工具
- **版本**：0.1.0

## 目标用户

- 通用用户（测绘、规划、开发等各领域 GIS 使用者）

## 核心功能

### 1. 数据源管理

支持多种数据源类型的统一管理和连接：

- **数据库数据源**
  - PostgreSQL/PostGIS
  - MySQL（空间扩展）
  - SpatiaLite
  - 其他关系型数据库

- **OSS 远程数据源**
  - 阿里云 OSS
  - AWS S3（兼容）
  - MinIO 等 S3 兼容存储服务
  - 用于远端数据存储和同步

- **功能**：数据源的增删改查、一键连接测试、状态展示
- **持久化**：SQLite `data_sources` 表

### 2. 数据管理

支持文件导入到已配置的数据源（数据库或OSS）：

- **矢量文件导入**（shp、geojson、gpkg、kml、kmz）
  - 可导入到数据库数据源（复制到本地 data_dir/imports）
  - 可导入到 OSS 数据源（上传到 bucket）

- **文档文件导入**（pdf、doc、docx、xls、xlsx、txt、csv、zip、rar、7z）
  - 只能导入到 OSS 数据源

- **导入历史记录**：列表展示最近导入的文件，包含文件名、类型、目标数据源、状态、时间
- **OSS 上传**：使用 AWS Signature V4 签名认证，支持 MinIO/S3 兼容服务
- **Tauri 命令**：`import_file`、`get_import_records`、`delete_import_record`

### 3. 服务注册

管理 GIS 地图服务的注册、发现和预览：

- **支持的服务类型**
  - WMTS（Web Map Tile Service）
  - TMS（Tile Map Service）
  - WMS（Web Map Service）
  - WFS（Web Feature Service）
  - GeoServer REST API
  - ArcGIS REST API

- **功能**
  - 服务的新增、编辑、删除、查询
  - 连接测试（发送 GetCapabilities 或 REST 请求验证端点）
  - 预置公开服务（天地图 WMTS、GeoServer 示例、OSM WMS）
- **Tauri 命令**：`get_services`、`add_service`、`update_service`、`delete_service`、`test_service_connection`

### 4. GIS 工具箱

提供预置的 GIS 分析和管理工具，支持 AI 辅助执行：

- **空间分析**
  - 缓冲区分析
  - 裁剪分析
  - 相交分析
  - 并集分析

- **数据转换**
  - 转 Shapefile
  - 转 GeoJSON
  - 转 GeoPackage

- **坐标处理**
  - 坐标转换
  - 投影定义

- **数据管理**
  - 数据统计
  - 属性查询
  - 图层合并
  - OGR 数据源信息

- **栅格处理（GDAL）**
  - 栅格重投影
  - 栅格转换/裁剪
  - 栅格信息查看
  - 栅格计算（如 NDVI）
  - 栅格转矢量

- **标签系统**：每个工具标记为 `ai`（需 AI 辅助）、`human`（手动操作）、`both`（两者皆可）
- **执行状态**：当前为模拟实现，返回工具执行结果占位，后续对接 GDAL/OGR 实际执行
- **持久化**：SQLite `gis_tools` 表，启动时预置 18 个工具
- **Tauri 命令**：`get_gis_tools`、`add_gis_tool`、`update_gis_tool`、`delete_gis_tool`、`execute_gis_tool`

### 5. AI Agent 设置

提供大语言模型（LLM）配置能力，支持 GIS 数据操作场景下的 AI 辅助功能：

- **模型提供商**
  - OpenAI（兼容接口，如 SiliconFlow、智谱等）
  - Anthropic（Claude 系列）
  - Ollama（本地部署）
  - 自定义 Endpoint

- **配置项**
  - API 地址（Base URL）
  - API Key（加密存储）
  - 模型名称（如 gpt-4o、claude-sonnet-4-6）
  - 最大 Token 数
  - Temperature 参数

- **连接测试**：一键验证模型配置是否可用
- **AI 对话**：内置聊天接口，系统角色设定为 GIS 助手，支持多轮对话和历史记录
- **持久化**：SQLite `settings` 表（key-value 存储）
- **Tauri 命令**：`get_settings`、`save_settings`、`test_model_connection`、`chat_message`

### 6. 通用设置

- 数据库路径查看
- 数据目录查看
- 应用版本信息
- **Tauri 命令**：`get_app_info`

## 前端架构

- **框架**：Vue 3 + Vue Router 5
- **UI 组件库**：Element Plus
  - 生态成熟，社区活跃，遇到问题容易找到解决方案
  - 组件丰富，覆盖 GIS 数据管理所需（表格、表单、树形控件等）
  - 可通过自定义主题实现 Clash Verge 风格的紧凑卡片式布局
- **图标**：@element-plus/icons-vue
- **地图预览**：OpenLayers（ol）
  - 支持 WMTS、WMS、TMS、XYZ 等多种地图服务加载
  - 用于服务注册页面的地图预览功能

### 页面路由

| 路由 | 组件 | 说明 |
| ------ | ------ | ------ |
| `/` | `HomeChat.vue` | 首页（AI 聊天界面） |
| `/dashboard` | `DashboardStats.vue` | 数据概览仪表盘 |
| `/datasources` | `DataSources.vue` | 数据源管理 |
| `/datamanagement` | `DataManagement.vue` | 数据导入与管理 |
| `/serviceregistry` | `ServiceRegistry.vue` | GIS 服务注册与预览 |
| `/gistools` | `GISTools.vue` | GIS 工具箱 |
| `/settings` | `Settings.vue` | 系统设置 |

## 后端（Rust/Tauri）

- **桌面框架**：Tauri 2
- **数据库（本地存储）**：SQLite（rusqlite），存储数据源配置、系统设置、服务注册、导入记录、GIS 工具
- **数据源连接测试**：
  - PostgreSQL/PostGIS：sqlx（postgres）
  - MySQL：sqlx（mysql）
  - SpatiaLite：rusqlite
- **OSS 连接/上传**：reqwest（HTTP HEAD/PUT + AWS Signature V4 签名认证）
- **AI 模型连接测试 & 对话**：reqwest（调用各提供商 API 验证配置和发送聊天请求）
- **GIS 服务连接测试**：reqwest（发送 OGC GetCapabilities 或 REST API 请求）
- **异步运行时**：tokio
- **系统托盘**：Tauri 2 内置 tray-icon，支持最小化隐藏和托盘菜单
- **设置存储**：SQLite key-value 表
- **文件对话框**：tauri-plugin-dialog
- **文件系统访问**：tauri-plugin-fs
- **UUID 生成**：uuid crate

### 数据库表结构

| 表名 | 用途 |
| ------ | ------ |
| `data_sources` | 数据源配置（数据库/OSS） |
| `settings` | 系统设置（key-value） |
| `services` | GIS 服务注册信息 |
| `import_records` | 文件导入历史记录 |
| `gis_tools` | GIS 工具定义 |

### Tauri 命令汇总

| 分类 | 命令 | 说明 |
| ------ | ------ | ------ |
| 数据源 | `get_data_sources` | 获取所有数据源 |
| 数据源 | `add_data_source` | 新增数据源 |
| 数据源 | `update_data_source` | 更新数据源 |
| 数据源 | `delete_data_source` | 删除数据源 |
| 连接测试 | `test_connection` | 测试数据源连接 |
| 服务注册 | `get_services` | 获取所有服务 |
| 服务注册 | `add_service` | 新增服务 |
| 服务注册 | `update_service` | 更新服务 |
| 服务注册 | `delete_service` | 删除服务 |
| 服务测试 | `test_service_connection` | 测试 GIS 服务连接 |
| 数据管理 | `import_file` | 导入文件到数据源 |
| 数据管理 | `get_import_records` | 获取导入记录 |
| 数据管理 | `delete_import_record` | 删除导入记录 |
| AI 设置 | `get_settings` | 获取模型配置 |
| AI 设置 | `save_settings` | 保存模型配置 |
| AI 设置 | `test_model_connection` | 测试模型连接 |
| AI 对话 | `chat_message` | 发送 AI 聊天消息 |
| GIS 工具 | `get_gis_tools` | 获取工具列表 |
| GIS 工具 | `add_gis_tool` | 新增工具 |
| GIS 工具 | `update_gis_tool` | 更新工具 |
| GIS 工具 | `delete_gis_tool` | 删除工具 |
| GIS 工具 | `execute_gis_tool` | 执行工具（模拟） |
| 系统信息 | `get_app_info` | 获取应用信息 |

### 应用生命周期

- **启动**：初始化 SQLite 数据库和表 → 预置 GIS 工具和数据源 → 加载数据到内存 → 设置系统托盘
- **关闭**：窗口关闭时最小化到托盘（非退出）

## 本地 OSS 测试环境

- MinIO（Docker）：S3 兼容对象存储，端口 9104（API）/ 9105（Console）

## 界面风格

- 参考 Clash Verge：左侧导航 + 卡片式仪表盘布局
- 浅色主题为主，蓝色作为主色调
- 紧凑、信息密度高的桌面工具风格
- 卡片化内容区域，轻量阴影分隔
- 支持侧边栏折叠

---

> 本文档最后更新于 2026-05-10

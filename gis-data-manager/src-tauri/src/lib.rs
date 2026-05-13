mod db_init;
mod shapefile_import;
mod api;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::Manager;
use tokio::sync::Mutex;

pub use db_init::init_db;
pub use shapefile_import::{ShapefileInfo, DbfField};

// ==================== 分页响应 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub total: i64,
    pub items: Vec<T>,
}

// ==================== 应用状态 ====================

pub struct AppState {
    pub sources: Mutex<Vec<api::DataSource>>,
    pub services: Mutex<Vec<api::Service>>,
    pub db: Mutex<Connection>,
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

            let default_db_path = std::path::Path::new(&data_dir)
                .join("gis-data-manager.db")
                .to_string_lossy()
                .to_string();

            // Check for custom db path saved from previous session
            let db_path = if let Ok(conn) = rusqlite::Connection::open(&default_db_path) {
                let custom: Option<String> = conn
                    .query_row(
                        "SELECT value FROM settings WHERE key='custom_db_path'",
                        [],
                        |r| r.get(0),
                    )
                    .ok()
                    .flatten();
                // Verify the custom path exists and is a valid SQLite file
                match custom {
                    Some(p) if std::path::Path::new(&p).exists() => p,
                    _ => default_db_path,
                }
            } else {
                default_db_path
            };

            let db = db_init::init_db(&db_path).expect("init database");
            let sources = api::load_sources_from_db(&db);
            let services = api::load_services_from_db(&db);

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
            // Data sources
            api::data_source::get_data_sources,
            api::data_source::add_data_source,
            api::data_source::update_data_source,
            api::data_source::delete_data_source,
            // Connection testing
            api::connection::test_connection,
            // Settings
            api::settings::get_settings,
            api::settings::save_settings,
            api::settings::test_model_connection,
            api::settings::get_app_info,
            api::settings::chat_message,
            api::settings::preview_db_tables,
            api::settings::switch_database,
            // Services
            api::service::get_services,
            api::service::add_service,
            api::service::update_service,
            api::service::delete_service,
            api::service::test_service_connection,
            // Import / data management
            api::import::get_import_records,
            api::import::import_file,
            api::import::delete_import_record,
            api::import::download_file,
            // GIS tools
            api::tools::get_gis_tools,
            api::tools::add_gis_tool,
            api::tools::update_gis_tool,
            api::tools::delete_gis_tool,
            api::tools::execute_gis_tool,
            // Dictionary
            api::dict::get_dict_items,
            api::dict::add_dict_item,
            api::dict::update_dict_item,
            api::dict::delete_dict_item,
            api::dict::get_dict_categories,
            // Shapefile import
            shapefile_import::read_shapefile_info,
            shapefile_import::import_shapefile_to_postgis,
            // GeoJSON / FlatGeobuf import (geozero)
            api::geozero_import::read_geojson_info,
            api::geozero_import::read_flatgeobuf_info,
            api::geozero_import::import_geojson_to_postgis,
            api::geozero_import::import_flatgeobuf_to_postgis,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

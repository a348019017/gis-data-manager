import { invoke } from '@tauri-apps/api/core'

export function getImportRecords(keyword = '', offset = 0, limit = 10) {
  return invoke('get_import_records', {
    keyword: keyword || null,
    offset,
    limit,
  })
}

export function importFile(filePath, targetSourceId, tags = '') {
  return invoke('import_file', {
    filePath,
    targetSourceId,
    tags,
  })
}

export function deleteImportRecord(id) {
  return invoke('delete_import_record', { id })
}

export function downloadFile(recordId, targetSourceId, savePath) {
  return invoke('download_file', {
    recordId,
    targetSourceId,
    savePath,
  })
}

export function readShapefileInfo(filePath) {
  return invoke('read_shapefile_info', { filePath })
}

export function importShapefileToPostgis(filePath, targetSourceId, tableName, targetSrid) {
  return invoke('import_shapefile_to_postgis', {
    filePath,
    targetSourceId,
    tableName,
    targetSrid,
  })
}

import { invoke } from '@tauri-apps/api/core'

export function getDataSources(keyword = '', offset = 0, limit = 10) {
  return invoke('get_data_sources', {
    keyword: keyword || null,
    offset,
    limit,
  })
}

export function addDataSource(source) {
  return invoke('add_data_source', { source })
}

export function updateDataSource(source) {
  return invoke('update_data_source', { source })
}

export function deleteDataSource(id) {
  return invoke('delete_data_source', { id })
}

export function testConnection(source) {
  return invoke('test_connection', { source })
}

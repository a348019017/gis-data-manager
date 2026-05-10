import { invoke } from '@tauri-apps/api/core'

export function getServices(keyword = '', offset = 0, limit = 10) {
  return invoke('get_services', {
    keyword: keyword || null,
    offset,
    limit,
  })
}

export function addService(service) {
  return invoke('add_service', { service })
}

export function updateService(service) {
  return invoke('update_service', { service })
}

export function deleteService(id) {
  return invoke('delete_service', { id })
}

export function testServiceConnection(service) {
  return invoke('test_service_connection', { service })
}

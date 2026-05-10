import { invoke } from '@tauri-apps/api/core'

export function getGISTools() {
  return invoke('get_gis_tools')
}

export function addGISTool(tool) {
  return invoke('add_gis_tool', { tool })
}

export function updateGISTool(tool) {
  return invoke('update_gis_tool', { tool })
}

export function deleteGISTool(id) {
  return invoke('delete_gis_tool', { id })
}

export function executeGISTool(toolId, params) {
  return invoke('execute_gis_tool', { toolId, params })
}

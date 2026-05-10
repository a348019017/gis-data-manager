import { invoke } from '@tauri-apps/api/core'

export function getSettings() {
  return invoke('get_settings')
}

export function saveSettings(settings) {
  return invoke('save_settings', { settings })
}

export function testModelConnection(settings) {
  return invoke('test_model_connection', { settings })
}

export function getAppInfo() {
  return invoke('get_app_info')
}

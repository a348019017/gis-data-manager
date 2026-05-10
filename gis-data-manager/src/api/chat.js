import { invoke } from '@tauri-apps/api/core'

export function chatMessage(settings, message, history = []) {
  return invoke('chat_message', { settings, message, history })
}

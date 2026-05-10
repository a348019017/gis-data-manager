import { invoke } from '@tauri-apps/api/core'

export function getDictItems(category = null) {
  return invoke('get_dict_items', { category })
}

export function addDictItem(item) {
  return invoke('add_dict_item', { item })
}

export function updateDictItem(item) {
  return invoke('update_dict_item', { item })
}

export function deleteDictItem(id) {
  return invoke('delete_dict_item', { id })
}

export function getDictCategories() {
  return invoke('get_dict_categories')
}

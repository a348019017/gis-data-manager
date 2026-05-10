<template>
  <div class="data-management">
    <div class="page-header">
      <h2>数据管理</h2>
      <div class="header-actions">
        <el-button :icon="Refresh" @click="loadRecords" :loading="loading">
          刷新
        </el-button>
        <el-button type="primary" :icon="Upload" @click="showImportDialog">
          导入文件
        </el-button>
      </div>
    </div>

    <!-- 导入记录列表 -->
    <el-table :data="records" size="small" stripe style="width: 100%">
      <el-table-column prop="file_name" label="文件名" min-width="160" show-overflow-tooltip />
      <el-table-column label="类型" width="80">
        <template #default="{ row }">
          <el-tag size="small" :type="row.file_type === 'vector' ? '' : 'warning'">
            {{ row.file_type === 'vector' ? '矢量' : '文档' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="格式" width="90">
        <template #default="{ row }">
          <el-tag size="small" effect="plain">{{ row.format }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="大小" width="90">
        <template #default="{ row }">
          {{ formatFileSize(row.file_size) }}
        </template>
      </el-table-column>
      <el-table-column prop="target_source_name" label="目标数据源" min-width="140" show-overflow-tooltip />
      <el-table-column label="目标类型" width="90">
        <template #default="{ row }">
          <el-tag size="small" :type="row.target_type === 'oss' ? 'warning' : 'info'">
            {{ row.target_type === 'oss' ? 'OSS' : '数据库' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="状态" width="90">
        <template #default="{ row }">
          <el-tag size="small" :type="statusTagType(row.status)">
            {{ statusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="时间" width="170">
        <template #default="{ row }">
          {{ formatTime(row.created_at) }}
        </template>
      </el-table-column>
      <el-table-column label="标签" width="160">
        <template #default="{ row }">
          <el-tag
            v-for="tag in (row.tags || '').split(',').filter(Boolean)"
            :key="tag"
            size="small"
            effect="plain"
            style="margin-right: 4px"
          >{{ dictLabel(tag) }}</el-tag>
          <span v-if="!row.tags" class="no-data">-</span>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="160" fixed="right">
        <template #default="{ row }">
          <el-tooltip v-if="row.status === 'failed'" :content="row.error_msg" placement="top">
            <el-icon class="error-icon"><WarningFilled /></el-icon>
          </el-tooltip>
          <el-button v-if="row.status === 'success' && row.target_type === 'oss'" link type="primary" size="small" @click="downloadFile(row)">
            下载
          </el-button>
          <el-button v-if="row.status === 'failed'" link type="warning" size="small" @click="retryImport(row)">
            重试
          </el-button>
          <el-button link type="danger" size="small" @click="removeRecord(row)">
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-empty v-if="records.length === 0 && !loading" description="暂无导入记录" />

    <!-- 下载进度提示 -->
  <el-dialog v-model="downloading" title="正在下载" width="400px" :close-on-click-modal="false" :show-close="false">
    <div v-for="(status, name) in downloadProgress" :key="name" class="progress-item">
      <el-progress
        v-if="typeof status === 'number'"
        :percentage="status"
        :stroke-width="14"
        style="flex: 1"
      />
      <el-icon v-else-if="status === 'success'" :size="16" color="#67c23a"><CircleCheck /></el-icon>
      <el-icon v-else-if="status === 'failed'" :size="16" color="#f56c6c"><CircleClose /></el-icon>
      <el-icon v-else :size="16" class="is-loading"><Loading /></el-icon>
      <span class="progress-label">{{ name }}</span>
    </div>
  </el-dialog>

  <!-- 导入对话框 -->
    <el-dialog v-model="dialogVisible" title="导入文件" width="600px" @close="resetImport">
      <div v-if="selectedFiles.length === 0" class="file-picker-area" @click="pickFiles">
        <el-icon :size="48"><FolderOpened /></el-icon>
        <p>点击选择文件或拖拽到此处</p>
        <p class="hint">支持矢量文件: shp, geojson, gpkg, kml, kmz</p>
        <p class="hint">支持文档文件: pdf, doc, docx, xls, xlsx, txt, csv, zip</p>
      </div>

      <template v-else>
        <div v-for="file in selectedFiles" :key="file.path" class="import-file-item">
          <span class="file-name">{{ file.name }}</span>
          <span class="file-size">{{ formatFileSize(file.size) }}</span>
          <el-tag size="small" :type="file.targetType === 'vector' ? '' : 'warning'">
            {{ file.targetType === 'vector' ? '矢量' : '文档' }}
          </el-tag>
          <el-select
            v-model="file.targetSourceId"
            placeholder="选择目标"
            style="width: 160px"
            size="small"
          >
            <el-option
              v-for="src in ossSources"
              :key="src.id"
              :label="src.name"
              :value="src.id"
            />
          </el-select>
          <el-tag type="warning" size="small">仅OSS</el-tag>
        </div>

        <!-- 标签选择 -->
        <div class="tags-section">
          <span class="tags-label">标签：</span>
          <el-checkbox-group v-model="selectedTags" size="small">
            <template v-for="cat in dictCategories" :key="cat.value">
              <span class="tag-group-label">{{ cat.label }}</span>
              <el-checkbox
                v-for="item in dictItemsByCategory[cat.value]"
                :key="item.id"
                :label="item.value"
              >{{ item.label }}</el-checkbox>
            </template>
          </el-checkbox-group>
        </div>

        <div v-if="Object.keys(importProgress).length > 0" class="import-progress">
          <div v-for="(status, name) in importProgress" :key="name" class="progress-item">
            <el-progress
              v-if="typeof status === 'number'"
              :percentage="status"
              :stroke-width="14"
              style="flex: 1"
            />
            <el-icon v-else-if="status === 'success'" :size="16" color="#67c23a"><CircleCheck /></el-icon>
            <el-icon v-else-if="status === 'failed'" :size="16" color="#f56c6c"><CircleClose /></el-icon>
            <el-icon v-else :size="16" class="is-loading"><Loading /></el-icon>
            <span class="progress-label">{{ name }}</span>
          </div>
        </div>
      </template>

      <template #footer>
        <el-button @click="dialogVisible = false" :disabled="importing">取消</el-button>
        <el-button type="primary" @click="startImport" :loading="importing" :disabled="selectedFiles.length === 0">
          开始导入 ({{ selectedFiles.length }} 个文件)
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Upload, Refresh, FolderOpened, WarningFilled, CircleCheck, CircleClose, Loading, Download } from '@element-plus/icons-vue'

async function openDialog() {
  const { open } = await import('@tauri-apps/plugin-dialog')
  return open
}

const VECTOR_EXTENSIONS = ['shp', 'geojson', 'json', 'gpkg', 'kml', 'kmz']
const DOCUMENT_EXTENSIONS = ['pdf', 'doc', 'docx', 'xls', 'xlsx', 'txt', 'csv', 'zip', 'rar', '7z']

const records = ref([])
const sources = ref([])
const dialogVisible = ref(false)
const selectedFiles = ref([])
const importing = ref(false)
const importProgress = ref({})
const loading = ref(false)
const downloading = ref(false)
const downloadProgress = ref({})
const selectedTags = ref([])
const dictCategories = ref([])
const dictItemsByCategory = ref({})
const dictLabelMap = ref({})

const ossSources = computed(() => sources.value.filter(s => s.ds_type === 'oss'))

async function loadRecords() {
  loading.value = true
  try {
    records.value = await invoke('get_import_records', { limit: 50 })
  } catch (err) {
    console.error('加载导入记录失败:', err)
  } finally {
    loading.value = false
  }
}

async function loadSources() {
  try {
    sources.value = await invoke('get_data_sources')
  } catch (err) {
    console.error('加载数据源失败:', err)
  }
}

async function loadDictItems() {
  try {
    const items = await invoke('get_dict_items')
    const grouped = {}
    const labelMap = {}
    const cats = {}
    for (const item of items) {
      if (!grouped[item.category]) grouped[item.category] = []
      grouped[item.category].push(item)
      labelMap[item.value] = item.label
      cats[item.category] = true
    }
    dictItemsByCategory.value = grouped
    dictLabelMap.value = labelMap
    dictCategories.value = Object.keys(cats).map(k => ({ value: k, label: categoryLabel(k) }))
  } catch (err) {
    console.error('加载字典失败:', err)
  }
}

function categoryLabel(cat) {
  return { data_type: '数据类型', data_source: '数据来源', importance: '重要程度' }[cat] || cat
}

function dictLabel(value) {
  return dictLabelMap.value[value] || value
}

async function pickFiles() {
  const open = await openDialog()
  const result = await open({
    multiple: true,
    filters: [{
      name: 'GIS & Document Files',
      extensions: ['shp', 'geojson', 'json', 'gpkg', 'kml', 'kmz', 'pdf', 'doc', 'docx', 'xls', 'xlsx', 'txt', 'csv', 'zip', 'rar', '7z']
    }]
  })
  let filePaths = []
  if (typeof result === 'string') {
    filePaths = [result]
  } else if (Array.isArray(result)) {
    filePaths = result
  }
  if (filePaths.length === 0) return

  selectedFiles.value = filePaths.map(path => {
    const name = path.split('/').pop()?.split('\\').pop() || path
    const format = (name.split('.').pop() || '').toLowerCase()
    const targetType = VECTOR_EXTENSIONS.includes(format) ? 'vector' : DOCUMENT_EXTENSIONS.includes(format) ? 'document' : 'unknown'
    return { path, name, format, targetType, targetSourceId: '', size: 0 }
  }).filter(f => f.targetType !== 'unknown')
}

async function startImport() {
  for (const file of selectedFiles.value) {
    if (!file.targetSourceId) {
      ElMessage.warning(`请为文件 "${file.name}" 选择目标数据源`)
      return
    }
  }

  importing.value = true
  importProgress.value = {}
  const tags = selectedTags.value.join(',')
  for (const file of selectedFiles.value) {
    importProgress.value[file.name] = 'pending'
    try {
      await invoke('import_file', {
        filePath: file.path,
        targetSourceId: file.targetSourceId,
        tags,
      })
      importProgress.value[file.name] = 'success'
      ElMessage.success(`${file.name} 导入成功`)
    } catch (err) {
      importProgress.value[file.name] = 'failed'
      ElMessage.error(`${file.name} 导入失败: ${err}`)
    }
  }
  importing.value = false
  dialogVisible.value = false
  selectedTags.value = []
  await loadRecords()
}

async function retryImport(row) {
  if (importing.value) return
  const source = sources.value.find(s => s.id === row.target_source_id)
  if (!source) {
    ElMessage.error('目标数据源不存在，无法重试')
    return
  }
  importing.value = true
  importProgress.value = {}
  importProgress.value[row.file_name] = 'pending'
  try {
    await invoke('import_file', {
      filePath: row.file_path,
      targetSourceId: row.target_source_id,
    })
    importProgress.value[row.file_name] = 'success'
    ElMessage.success(`${row.file_name} 重试成功`)
    await loadRecords()
  } catch (err) {
    importProgress.value[row.file_name] = 'failed'
    ElMessage.error(`${row.file_name} 重试失败: ${err}`)
  } finally {
    importing.value = false
  }
}

async function removeRecord(row) {
  await ElMessageBox.confirm(`确定删除导入记录 "${row.file_name}" 吗？`, '确认删除', { type: 'warning' })
  try {
    await invoke('delete_import_record', { id: row.id })
    records.value = records.value.filter(r => r.id !== row.id)
    ElMessage.success('记录已删除')
  } catch (err) {
    ElMessage.error('删除失败: ' + err)
  }
}

async function downloadFile(row) {
  if (downloading.value) return
  const { save } = await import('@tauri-apps/plugin-dialog')
  const result = await save({
    filters: [{ name: 'All Files', extensions: ['*'] }],
    defaultPath: row.file_name,
  })
  if (!result) return

  downloading.value = true
  downloadProgress.value = {}
  downloadProgress.value[row.file_name] = 'pending'
  try {
    const savedPath = await invoke('download_file', {
      recordId: row.id,
      targetSourceId: row.target_source_id,
      savePath: result,
    })
    downloadProgress.value[row.file_name] = 'success'
    ElMessage.success(`文件已保存到 ${savedPath}`)
    await loadRecords()
  } catch (err) {
    downloadProgress.value[row.file_name] = 'failed'
    ElMessage.error(`下载失败: ${err}`)
  } finally {
    downloading.value = false
  }
}

function resetImport() {
  selectedFiles.value = []
  importing.value = false
  importProgress.value = {}
  selectedTags.value = []
}

function formatFileSize(bytes) {
  if (!bytes || bytes < 1024) return (bytes || 0) + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

function formatTime(iso) {
  if (!iso) return ''
  const d = new Date(iso)
  return d.toLocaleString('zh-CN')
}

function statusTagType(status) {
  return { success: 'success', pending: 'info', failed: 'danger' }[status] || 'info'
}

function statusLabel(status) {
  return { success: '成功', pending: '处理中', failed: '失败' }[status] || status
}

function showImportDialog() {
  loadSources()
  dialogVisible.value = true
}

onMounted(async () => {
  loadRecords()
  loadSources()
  loadDictItems()

  // 监听后端推送的上传进度
  await listen('import-progress', (event) => {
    const { file_name, status, bytes_sent, total_bytes } = event.payload
    if (status === 'uploading') {
      const pct = total_bytes > 0 ? Math.round((bytes_sent / total_bytes) * 100) : 0
      importProgress.value[file_name] = pct
    } else if (status === 'success') {
      importProgress.value[file_name] = 'success'
    }
  })

  // 监听后端推送的下载进度
  await listen('download-progress', (event) => {
    const { file_name, status, bytes_received, total_bytes } = event.payload
    if (status === 'downloading') {
      const pct = total_bytes > 0 ? Math.round((bytes_received / total_bytes) * 100) : 0
      downloadProgress.value[file_name] = pct
    } else if (status === 'success') {
      downloadProgress.value[file_name] = 'success'
    }
  })
})
</script>

<style scoped>
.data-management {
  width: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  flex-wrap: wrap;
  gap: 8px;
}

.page-header h2 {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.header-actions {
  display: flex;
  gap: 8px;
}

@media (max-width: 768px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }
  .data-management {
    max-width: 100%;
  }
  .file-picker-area {
    padding: 24px 12px;
  }
}

@media (max-width: 480px) {
  .import-file-item {
    flex-wrap: wrap;
  }
  .file-name {
    width: 100%;
  }
}

.file-picker-area {
  text-align: center;
  padding: 40px 20px;
  border: 2px dashed #dcdfe6;
  border-radius: 8px;
  cursor: pointer;
  transition: border-color 0.2s;
}

.file-picker-area:hover {
  border-color: #409eff;
}

.file-picker-area .el-icon {
  color: #909399;
}

.file-picker-area p {
  margin-top: 8px;
  color: #606266;
}

.file-picker-area .hint {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}

.import-file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 0;
  border-bottom: 1px solid #f0f0f0;
}

.file-name {
  flex: 1;
  font-size: 14px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-size {
  font-size: 12px;
  color: #909399;
  white-space: nowrap;
}

.import-progress {
  margin-top: 16px;
}

.progress-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 13px;
}

.progress-label {
  min-width: 60px;
  text-align: right;
  color: #909399;
}

.error-icon {
  color: #f56c6c;
  cursor: pointer;
}

.no-data { color: #c0c4cc; font-size: 12px; }

.tags-section {
  padding: 12px 0;
  border-bottom: 1px solid #f0f0f0;
}

.tags-label {
  font-size: 14px;
  color: #606266;
  margin-right: 8px;
}

.tag-group-label {
  font-size: 12px;
  color: #909399;
  margin-right: 4px;
  margin-left: 8px;
}

:deep(.el-empty) {
  padding: 24px 0;
}
</style>

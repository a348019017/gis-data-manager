<template>
  <div class="data-management">
    <div class="page-header">
      <h2>数据管理</h2>
      <div class="header-actions">
        <el-input
          v-model="searchText"
          placeholder="搜索文件名、数据源"
          clearable
          style="width: 220px"
          @clear="page = 1"
          @keyup.enter="page = 1"
        />
        <el-button :icon="Refresh" @click="loadRecords" :loading="loading">
          刷新
        </el-button>
        <el-button type="primary" :icon="Upload" @click="showImportDialog">
          导入文件
        </el-button>
      </div>
    </div>

    <!-- 导入记录列表 -->
    <div class="table-wrapper">
      <el-table :data="records" size="small" stripe :loading="loading" style="width: 100%">
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
    </div>

    <el-pagination
      v-if="total > 0"
      v-model:current-page="page"
      v-model:page-size="pageSize"
      :total="total"
      :page-sizes="[10, 20, 50]"
      layout="total, sizes, prev, pager, next"
      style="margin-top: 12px; justify-content: flex-end"
    />

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
        <p class="hint">支持矢量文件: shp, geojson, gpkg, fgb, kml, kmz</p>
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
            style="width: 180px"
            size="small"
          >
            <el-option-group v-if="ossSources.length > 0" label="OSS 存储">
              <el-option
                v-for="src in ossSources"
                :key="src.id"
                :label="src.name"
                :value="src.id"
              />
            </el-option-group>
            <el-option-group v-if="dbSources.length > 0" label="PostgreSQL 数据库">
              <el-option
                v-for="src in dbSources"
                :key="src.id"
                :label="src.name"
                :value="src.id"
              />
            </el-option-group>
          </el-select>
          <el-tag v-if="file.targetSourceId && ossSources.find(s => s.id === file.targetSourceId)" type="warning" size="small">OSS</el-tag>
          <el-tag v-else-if="file.targetSourceId && dbSources.find(s => s.id === file.targetSourceId)" type="success" size="small">数据库</el-tag>
          <el-tag v-else type="info" size="small">未选择</el-tag>
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

    <!-- 矢量导入配置对话框（Shapefile / GeoJSON / FlatGeobuf → PostGIS） -->
    <el-dialog v-model="vectorDialogVisible" :title="vectorDialogTitle" width="700px" :close-on-click-modal="false">
      <div v-if="vectorInfo" class="vector-import">
        <!-- 文件基本信息 -->
        <div class="section">
          <h4>文件信息</h4>
          <el-descriptions :column="2" border size="small">
            <el-descriptions-item label="文件名">{{ vectorInfo.file_name }}</el-descriptions-item>
            <el-descriptions-item label="几何类型">{{ vectorInfo.geometry_type || vectorInfo.shape_type || '-' }}</el-descriptions-item>
            <el-descriptions-item label="记录数">{{ vectorInfo.feature_count || vectorInfo.record_count || 0 }}</el-descriptions-item>
            <el-descriptions-item label="格式">{{ formatLabel(vectorFormat) }}</el-descriptions-item>
            <el-descriptions-item v-if="vectorInfo.bounding_box" label="范围" :span="2">
              {{ vectorInfo.bounding_box[0].toFixed(4) }}, {{ vectorInfo.bounding_box[1].toFixed(4) }} ~
              {{ vectorInfo.bounding_box[2].toFixed(4) }}, {{ vectorInfo.bounding_box[3].toFixed(4) }}
            </el-descriptions-item>
          </el-descriptions>
        </div>

        <!-- 坐标系统配置 -->
        <div class="section">
          <h4>坐标系统</h4>
          <el-row :gutter="16">
            <el-col :span="12">
              <div class="crs-item">
                <span class="crs-label">源坐标系 (EPSG):</span>
                <el-input v-model="importSourceCrs" placeholder="自动检测" size="small" />
              </div>
            </el-col>
            <el-col :span="12">
              <div class="crs-item">
                <span class="crs-label">目标坐标系 (SRID):</span>
                <el-select v-model="importTargetCrs" placeholder="选择目标坐标系" size="small" filterable style="width: 100%">
                  <el-option v-for="crs in COMMON_CRS" :key="crs.value" :label="crs.label" :value="crs.value" />
                  <el-option label="自定义 (手动输入)" :value="'custom'" />
                </el-select>
                <el-input v-if="importTargetCrs === 'custom'" v-model="customCrsInput" placeholder="输入 EPSG 代码" size="small" style="margin-top: 4px" />
              </div>
            </el-col>
          </el-row>
        </div>

        <!-- 目标表名 -->
        <div class="section">
          <h4>目标设置</h4>
          <el-form label-width="80px" size="small">
            <el-form-item label="目标表名">
              <el-input v-model="importTargetTable" placeholder="输入目标表名" />
            </el-form-item>
            <el-form-item label="目标数据源">
              <el-tag size="small">{{ dbSources.find(s => s.id === importTargetSourceId)?.name || '-' }}</el-tag>
            </el-form-item>
          </el-form>
        </div>

        <!-- 字段列表 -->
        <div class="section" v-if="vectorFields.length > 0">
          <h4>属性字段 ({{ vectorFields.length }})</h4>
          <el-table :data="vectorFields" size="small" border max-height="200">
            <el-table-column prop="name" label="字段名" min-width="120" />
            <el-table-column label="PG 类型" width="140">
              <template #default="{ row }">
                <el-tag size="small" effect="plain">{{ row.pg_type }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column v-if="vectorFormat === 'shp'" label="源类型" width="90">
              <template #default="{ row }">
                <el-tag size="small" effect="plain">{{ getDbfTypeLabel(row.dbf_type) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column v-if="vectorFormat === 'shp'" prop="length" label="长度" width="70" align="center" />
          </el-table>
        </div>

        <!-- 导入进度 -->
        <div class="section" v-if="vectorImporting">
          <h4>导入进度</h4>
          <el-progress
            :percentage="vectorImportProgress.total > 0 ? Math.round(vectorImportProgress.current / vectorImportProgress.total * 100) : 0"
            :stroke-width="18"
            :text-inside="true"
          />
          <div class="progress-text">
            {{ vectorImportProgress.current }} / {{ vectorImportProgress.total }} 条记录
          </div>
        </div>
      </div>

      <template #footer>
        <el-button @click="vectorDialogVisible = false" :disabled="vectorImporting">取消</el-button>
        <el-button type="primary" @click="confirmVectorImport" :loading="vectorImporting" :disabled="vectorImporting">
          确认导入
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, watch, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Upload, Refresh, FolderOpened, WarningFilled, CircleCheck, CircleClose, Loading } from '@element-plus/icons-vue'

async function openDialog() {
  const { open } = await import('@tauri-apps/plugin-dialog')
  return open
}

const VECTOR_EXTENSIONS = ['shp', 'dbf', 'prj', 'geojson', 'json', 'gpkg', 'kml', 'kmz', 'fgb', 'fgb2']
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
const searchText = ref('')
const page = ref(1)
const pageSize = ref(10)
const total = ref(0)

const ossSources = computed(() => sources.value.filter(s => s.type === 'oss'))
const dbSources = computed(() => sources.value.filter(s => s.type === 'database' && s.subtype === 'postgresql'))

const vectorDialogTitle = computed(() => ({
  shp: 'Shapefile 导入配置',
  geojson: 'GeoJSON 导入配置',
  fgb: 'FlatGeobuf 导入配置',
  fgb2: 'FlatGeobuf 导入配置',
}[vectorFormat.value] || '矢量导入配置'))

const vectorFields = computed(() => {
  if (!vectorInfo.value) return []
  if (vectorFormat.value === 'shp') {
    return (vectorInfo.value.fields || []).map(f => ({
      ...f,
      pg_type: f.pg_type || dbfTypeToPg(f),
    }))
  }
  return vectorInfo.value.fields || []
})

function formatLabel(fmt) {
  return { shp: 'ESRI Shapefile', geojson: 'GeoJSON', fgb: 'FlatGeobuf', fgb2: 'FlatGeobuf' }[fmt] || fmt.toUpperCase()
}

// 矢量导入状态（Shapefile / GeoJSON / FlatGeobuf → PostGIS）
const vectorDialogVisible = ref(false)
const vectorInfo = ref(null)       // ShapefileInfo or GeoFileInfo
const vectorFormat = ref('')       // 'shp' | 'geojson' | 'flatgeobuf'
const vectorImporting = ref(false)
const vectorImportProgress = reactive({ current: 0, total: 0 })
const importTargetTable = ref('')
const importSourceCrs = ref('')
const importTargetCrs = ref('4326')
const customCrsInput = ref('')
const importTargetSourceId = ref('')
const vectorFilePath = ref('')     // the file being imported

const COMMON_CRS = [
  { label: 'WGS 84 (EPSG:4326)', value: '4326' },
  { label: 'Web Mercator (EPSG:3857)', value: '3857' },
  { label: 'CGCS2000 (EPSG:4490)', value: '4490' },
  { label: 'CGCS2000 / 3-degree Gauss (EPSG:4527)', value: '4527' },
  { label: 'Beijing 54 (EPSG:4214)', value: '4214' },
  { label: 'Xian 80 (EPSG:4610)', value: '4610' },
  { label: 'UTM Zone 50N (EPSG:32650)', value: '32650' },
]

onMounted(() => {
  loadRecords()
  loadSources()
  loadDictItems()
  setupEventListeners()
})

watch([searchText, pageSize], () => {
  page.value = 1
  loadRecords()
})

watch(page, () => {
  loadRecords()
})

async function loadRecords() {
  loading.value = true
  try {
    const offset = (page.value - 1) * pageSize.value
    const result = await invoke('get_import_records', {
      keyword: searchText.value || null,
      offset,
      limit: pageSize.value,
    })
    records.value = result.items || []
    total.value = result.total || 0
  } catch (err) {
    console.error('加载导入记录失败:', err)
  } finally {
    loading.value = false
  }
}

async function loadSources() {
  try {
    const result = await invoke('get_data_sources')
    sources.value = result.items || []
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
      extensions: ['shp', 'dbf', 'prj', 'geojson', 'json', 'gpkg', 'kml', 'kmz', 'fgb', 'fgb2', 'pdf', 'doc', 'docx', 'xls', 'xlsx', 'txt', 'csv', 'zip', 'rar', '7z']
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

  // 检查是否有矢量文件 → 数据库的导入
  const dbVector = selectedFiles.value.find(f => {
    if (!['shp', 'geojson', 'fgb', 'fgb2'].includes(f.format)) return false
    const source = sources.value.find(s => s.id === f.targetSourceId)
    return source && source.type === 'database' && source.subtype === 'postgresql'
  })

  if (dbVector) {
    await openVectorImportDialog(dbVector)
    return
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
  if (row.target_type !== 'oss') {
    ElMessage.warning('数据库导入的重试暂不支持，请通过"导入文件"按钮重新导入')
    return
  }
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

async function openVectorImportDialog(file) {
  try {
    const format = file.format
    vectorFormat.value = format
    vectorFilePath.value = file.path
    importTargetSourceId.value = file.targetSourceId
    importTargetTable.value = file.name.replace(/\.[^.]+$/, '').toLowerCase().replace(/[^a-z0-9_]/g, '_')
    customCrsInput.value = ''
    importTargetCrs.value = '4326'
    vectorImportProgress.current = 0
    vectorImportProgress.total = 0

    if (format === 'shp') {
      const info = await invoke('read_shapefile_info', { filePath: file.path })
      vectorInfo.value = { ...info, fields: (info.fields || []).map(f => ({ name: f.name, pg_type: dbfTypeToPg(f), dbf_type: f.dbf_type, length: f.length, decimal_count: f.decimal_count })) }
      vectorImportProgress.total = info.record_count
      importSourceCrs.value = info.crs_epsg ? String(info.crs_epsg) : '4326'
      if (info.crs_epsg && !COMMON_CRS.find(c => c.value === String(info.crs_epsg))) {
        importTargetCrs.value = 'custom'
        customCrsInput.value = String(info.crs_epsg)
      } else {
        importTargetCrs.value = info.crs_epsg ? String(info.crs_epsg) : '4326'
      }
    } else if (format === 'geojson') {
      const info = await invoke('read_geojson_info', { filePath: file.path })
      vectorInfo.value = info
      vectorImportProgress.total = info.feature_count
      importSourceCrs.value = info.crs_epsg ? String(info.crs_epsg) : '4326'
    } else if (format === 'fgb' || format === 'fgb2') {
      const info = await invoke('read_flatgeobuf_info', { filePath: file.path })
      vectorInfo.value = info
      vectorImportProgress.total = info.feature_count
      importSourceCrs.value = info.crs_epsg ? String(info.crs_epsg) : '4326'
    }

    vectorDialogVisible.value = true
  } catch (err) {
    ElMessage.error('读取文件信息失败: ' + err)
  }
}

function dbfTypeToPg(field) {
  const t = field.dbf_type
  if (t === 'N' || t === 'F') {
    return field.decimal_count > 0 ? 'DOUBLE PRECISION'
      : field.length <= 4 ? 'SMALLINT'
      : field.length <= 9 ? 'INTEGER'
      : 'BIGINT'
  }
  if (t === 'I') return field.length <= 4 ? 'SMALLINT' : field.length <= 9 ? 'INTEGER' : 'BIGINT'
  if (t === 'L') return 'BOOLEAN'
  if (t === 'D') return 'DATE'
  return `VARCHAR(${Math.max(field.length, 255)})`
}

async function confirmVectorImport() {
  if (!importTargetTable.value) {
    ElMessage.warning('请输入目标表名')
    return
  }
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(importTargetTable.value)) {
    ElMessage.warning('表名只能包含字母、数字和下划线，且不能以数字开头')
    return
  }

  const targetSrid = importTargetCrs.value === 'custom'
    ? parseInt(customCrsInput.value)
    : parseInt(importTargetCrs.value)
  if (!targetSrid || isNaN(targetSrid)) {
    ElMessage.warning('请输入有效的坐标系 SRID')
    return
  }

  vectorImporting.value = true
  vectorImportProgress.current = 0

  const cmdMap = {
    shp: 'import_shapefile_to_postgis',
    geojson: 'import_geojson_to_postgis',
    fgb: 'import_flatgeobuf_to_postgis',
    fgb2: 'import_flatgeobuf_to_postgis',
  }
  const command = cmdMap[vectorFormat.value]

  try {
    const result = await invoke(command, {
      filePath: vectorFilePath.value,
      targetSourceId: importTargetSourceId.value,
      tableName: importTargetTable.value,
      targetSrid,
    })
    ElMessage.success(result)
    vectorDialogVisible.value = false
    dialogVisible.value = false
    await loadRecords()
  } catch (err) {
    ElMessage.error('导入失败: ' + err)
  } finally {
    vectorImporting.value = false
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

async function showImportDialog() {
  await loadSources()
  dialogVisible.value = true
}

function getDbfTypeLabel(type) {
  return { C: '字符', N: '数值', F: '浮点', L: '逻辑', D: '日期', I: '整型' }[type] || type
}

async function setupEventListeners() {
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

  // 监听后端推送的矢量导入进度
  await listen('shapefile-import-progress', (event) => {
    const { current, total } = event.payload
    vectorImportProgress.current = current
    vectorImportProgress.total = total
  })
}
</script>

<style scoped>
.data-management {
  width: 100%;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  flex-wrap: wrap;
  gap: 8px;
  flex-shrink: 0;
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

.table-wrapper {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.table-wrapper :deep(.el-table) {
  height: 100%;
}

.table-wrapper :deep(.el-table .el-table__body-wrapper) {
  overflow-y: auto;
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
  :deep(.el-pagination) {
    flex-wrap: wrap;
    gap: 4px;
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

.vector-import .section {
  margin-bottom: 16px;
}

.shapefile-import .section h4 {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 8px 0;
  padding-bottom: 6px;
  border-bottom: 1px solid #ebeef5;
}

.shapefile-import .crs-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.shapefile-import .crs-label {
  font-size: 13px;
  color: #606266;
}

.shapefile-import .progress-text {
  text-align: center;
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}
</style>

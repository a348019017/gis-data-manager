<template>
  <div class="service-registry">
    <div class="page-header">
      <h2>服务注册</h2>
      <el-button type="primary" :icon="Plus" @click="showAddDialog">
        注册服务
      </el-button>
    </div>

    <!-- 服务列表 -->
    <el-table :data="serviceList" size="small" stripe style="width: 100%">
      <el-table-column prop="name" label="服务名称" min-width="140" />
      <el-table-column label="类型" width="110">
        <template #default="{ row }">
          <el-tag size="small" :type="typeTagColor(row.type)">
            {{ typeLabel(row.type) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="endpoint" label="端点地址" min-width="250" show-overflow-tooltip />
      <el-table-column label="状态" width="100">
        <template #default="{ row }">
          <el-tag size="small" :type="row.connected ? 'success' : 'info'">
            {{ row.connected ? '已连接' : '未连接' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="220" fixed="right">
        <template #default="{ row }">
          <el-button link type="primary" size="small" @click="previewService(row)">
            预览
          </el-button>
          <el-button link type="primary" size="small" @click="testConnection(row)">
            测试
          </el-button>
          <el-button link type="primary" size="small" @click="showEditDialog(row)">
            编辑
          </el-button>
          <el-button link type="danger" size="small" @click="removeService(row)">
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-empty v-if="serviceList.length === 0" description="暂无注册的服务" />

    <!-- 注册/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑服务' : '注册服务'"
      width="500px"
      @close="resetForm"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="80px" label-position="right">
        <el-form-item label="服务名称" prop="name">
          <el-input v-model="formData.name" placeholder="输入服务名称" />
        </el-form-item>
        <el-form-item label="服务类型" prop="type">
          <el-select v-model="formData.type" placeholder="选择服务类型" style="width: 100%">
            <el-option label="WMTS" value="wmts" />
            <el-option label="TMS" value="tms" />
            <el-option label="WMS" value="wms" />
            <el-option label="WFS" value="wfs" />
            <el-option label="GeoServer REST" value="geoserver" />
            <el-option label="ArcGIS REST" value="arcgis" />
          </el-select>
        </el-form-item>
        <el-form-item label="端点地址" prop="endpoint">
          <el-input v-model="formData.endpoint" placeholder="http://localhost:8080/geoserver/gwc/service/wmts" />
        </el-form-item>
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form-item label="用户名" prop="username">
              <el-input v-model="formData.username" placeholder="可选" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="密码" prop="password">
              <el-input v-model="formData.password" type="password" show-password placeholder="可选" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="formData.remark" type="textarea" :rows="2" placeholder="可选" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="handleSave">
          {{ isEdit ? '保存' : '注册并测试' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 地图预览对话框 -->
    <el-dialog v-model="previewVisible" title="地图预览" width="900px" :close-on-click-modal="false">
      <div class="preview-info">
        <span>服务: {{ previewServiceName }}</span>
        <span class="preview-url">{{ previewEndpoint }}</span>
      </div>
      <div id="map-preview-container" class="map-preview"></div>
      <template #footer>
        <el-button @click="closePreview">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Plus } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import 'ol/ol.css'
import Map from 'ol/Map'
import View from 'ol/View'
import TileLayer from 'ol/layer/Tile'
import OSM from 'ol/source/OSM'
import TileWMS from 'ol/source/TileWMS'
import XYZ from 'ol/source/XYZ'

const serviceList = ref([])
const dialogVisible = ref(false)
const previewVisible = ref(false)
const isEdit = ref(false)
const saving = ref(false)
const formRef = ref(null)
let previewMap = null

const formData = reactive({
  id: '',
  name: '',
  type: 'wmts',
  endpoint: '',
  username: '',
  password: '',
  remark: '',
  connected: false,
})

const previewServiceName = ref('')
const previewEndpoint = ref('')

const formRules = {
  name: [{ required: true, message: '请输入服务名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择服务类型', trigger: 'change' }],
  endpoint: [{ required: true, message: '请输入端点地址', trigger: 'blur' }],
}

const typeLabels = {
  wmts: 'WMTS',
  tms: 'TMS',
  wms: 'WMS',
  wfs: 'WFS',
  geoserver: 'GeoServer',
  arcgis: 'ArcGIS',
}

const typeColors = {
  wmts: '',
  tms: 'success',
  wms: 'warning',
  wfs: 'info',
  geoserver: 'danger',
  arcgis: '',
}

function typeLabel(type) {
  return typeLabels[type] || type
}

function typeTagColor(type) {
  return typeColors[type] || 'info'
}

function generateId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

async function loadServices() {
  try {
    serviceList.value = await invoke('get_services')
  } catch (err) {
    console.error('加载服务列表失败:', err)
  }
}

function showAddDialog() {
  isEdit.value = false
  dialogVisible.value = true
}

function showEditDialog(row) {
  isEdit.value = true
  Object.assign(formData, {
    id: row.id,
    name: row.name,
    type: row.type,
    endpoint: row.endpoint,
    username: row.username || '',
    password: row.password || '',
    remark: row.remark || '',
    connected: row.connected,
  })
  dialogVisible.value = true
}

function resetForm() {
  formRef.value?.resetFields()
  Object.assign(formData, {
    id: '',
    name: '',
    type: 'wmts',
    endpoint: '',
    username: '',
    password: '',
    remark: '',
    connected: false,
  })
}

async function handleSave() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return

  saving.value = true
  try {
    const service = { ...formData, id: formData.id || generateId() }
    const cmd = isEdit.value ? 'update_service' : 'add_service'
    const result = await invoke(cmd, { service })

    dialogVisible.value = false
    ElMessage.success(isEdit.value ? '服务已更新' : '服务已注册')

    // 自动测试连接
    await testConnection(result)
  } catch (err) {
    ElMessage.error('保存失败: ' + err)
  } finally {
    saving.value = false
  }
}

async function testConnection(row) {
  ElMessage.info(`正在测试连接: ${row.name}`)
  try {
    const connected = await invoke('test_service_connection', { service: row })
    row.connected = connected

    // 更新连接状态到后端
    await invoke('update_service', { service: row })

    ElMessage[connected ? 'success' : 'error'](
      connected ? '连接成功' : '连接失败，请检查配置'
    )
  } catch (err) {
    row.connected = false
    ElMessage.error('连接测试失败: ' + err)
  }
}

async function removeService(row) {
  ElMessageBox.confirm(`确定删除服务 "${row.name}" 吗？`, '确认删除', { type: 'warning' })
    .then(async () => {
      try {
        await invoke('delete_service', { id: row.id })
        serviceList.value = serviceList.value.filter(s => s.id !== row.id)
        ElMessage.success('已删除')
      } catch (err) {
        ElMessage.error('删除失败: ' + err)
      }
    })
    .catch(() => {})
}

function buildPreviewLayers(service) {
  const endpoint = service.endpoint
  const layers = []

  switch (service.type) {
    case 'wmts': {
      // WMTS 用简化 XYZ 模板，需用户根据实际服务调整
      const wmtsUrl = endpoint.endsWith('/') ? endpoint + '{TileMatrix}/{TileRow}/{TileCol}.png' : endpoint + '/{TileMatrix}/{TileRow}/{TileCol}.png'
      layers.push(new TileLayer({
        source: new XYZ({ url: wmtsUrl, tileSize: 256 }),
        visible: true,
      }))
      break
    }
    case 'wms': {
      layers.push(new TileLayer({
        source: new TileWMS({
          url: endpoint,
          params: { 'LAYERS': 'topp:states', 'TILED': true, 'VERSION': '1.1.1' },
          serverType: 'geoserver',
          crossOrigin: 'anonymous',
        }),
        visible: true,
      }))
      break
    }
    case 'tms': {
      const tmsUrl = endpoint.endsWith('/') ? endpoint + '{z}/{x}/{y}.png' : endpoint + '/{z}/{x}/{y}.png'
      layers.push(new TileLayer({
        source: new XYZ({ url: tmsUrl, tileSize: 256 }),
        visible: true,
      }))
      break
    }
    case 'geoserver': {
      const baseUrl = endpoint.replace(/\/rest.*/, '')
      layers.push(new TileLayer({
        source: new TileWMS({
          url: `${baseUrl}/gwc/service/wms`,
          params: { 'LAYERS': 'topp:states', 'TILED': true, 'VERSION': '1.1.1' },
          serverType: 'geoserver',
          crossOrigin: 'anonymous',
        }),
        visible: true,
      }))
      break
    }
    case 'arcgis': {
      const arcUrl = endpoint.endsWith('/') ? endpoint + 'tile/{z}/{y}/{x}' : endpoint + '/tile/{z}/{y}/{x}'
      layers.push(new TileLayer({
        source: new XYZ({ url: arcUrl, tileSize: 256 }),
        visible: true,
      }))
      break
    }
  }

  return layers
}

function previewService(row) {
  previewServiceName.value = row.name
  previewEndpoint.value = row.endpoint
  previewVisible.value = true

  nextTick(() => {
    initMap(row)
  })
}

function initMap(service) {
  if (previewMap) {
    previewMap.setTarget(null)
    previewMap = null
  }

  const layers = buildPreviewLayers(service)

  // 添加底图
  const osmLayer = new TileLayer({
    source: new OSM(),
    visible: true,
    opacity: 0.3,
  })

  previewMap = new Map({
    target: 'map-preview-container',
    layers: [osmLayer, ...layers],
    view: new View({
      center: [12900000, 4900000], // China center
      zoom: 4,
      projection: 'EPSG:3857',
    }),
  })
}

function closePreview() {
  if (previewMap) {
    previewMap.setTarget(null)
    previewMap = null
  }
  previewVisible.value = false
}

onMounted(() => {
  loadServices()
})
</script>

<style scoped>
.service-registry {
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

.preview-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 8px;
  font-size: 13px;
  color: #606266;
}

.preview-url {
  color: #409eff;
  word-break: break-all;
}

.map-preview {
  width: 100%;
  height: 500px;
  border: 1px solid #dcdfe6;
  border-radius: 4px;
}

:deep(.el-empty) {
  padding: 24px 0;
}

@media (max-width: 768px) {
  .page-header {
    flex-direction: column;
    align-items: flex-start;
  }
  .map-preview {
    height: 350px;
  }
  .service-registry {
    max-width: 100%;
  }
}

@media (max-width: 480px) {
  .map-preview {
    height: 250px;
  }
}
</style>

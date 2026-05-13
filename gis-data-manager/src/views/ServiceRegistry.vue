<template>
  <div class="flex flex-col h-full min-h-0">
    <div class="flex items-center justify-between mb-3 flex-wrap gap-2 shrink-0">
      <h2 class="text-lg font-semibold">服务注册</h2>
      <div class="flex gap-2 items-center">
        <el-input
          v-model="searchText"
          placeholder="搜索服务名称、端点"
          clearable
          style="width: 220px"
          @clear="page = 1"
          @keyup.enter="page = 1"
        />
        <button class="btn btn-ghost btn-sm" @click="loadServices">
          <Icon icon="mdi:refresh" width="18" />刷新
        </button>
        <button class="btn btn-primary btn-sm" @click="showAddDialog">
          <Icon icon="mdi:plus" width="18" />注册服务
        </button>
      </div>
    </div>

    <div class="flex-1 min-h-0">
      <AppTable :data="serviceList" :columns="serviceColumns" :loading="loading" :show-pagination="false" />
    </div>

    <!-- 注册/编辑对话框 -->
    <AppModal v-model="dialogVisible" :title="isEdit ? '编辑服务' : '注册服务'" @closed="resetForm">
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
        <div class="grid grid-cols-2 gap-3">
          <el-form-item label="用户名" prop="username">
            <el-input v-model="formData.username" placeholder="可选" />
          </el-form-item>
          <el-form-item label="密码" prop="password">
            <el-input v-model="formData.password" type="password" show-password placeholder="可选" />
          </el-form-item>
        </div>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="formData.remark" type="textarea" :rows="2" placeholder="可选" />
        </el-form-item>
      </el-form>
      <template #footer>
        <button class="btn btn-ghost" @click="dialogVisible = false">取消</button>
        <button class="btn btn-primary" :disabled="saving" @click="handleSave">
          <span v-if="saving" class="loading loading-spinner loading-xs"></span>
          {{ isEdit ? '保存' : '注册并测试' }}
        </button>
      </template>
    </AppModal>

    <!-- 地图预览对话框 -->
    <AppModal v-model="previewVisible" title="地图预览" :close-on-backdrop="false" wide>
      <div class="flex flex-col gap-1 mb-2 text-xs text-base-content/70">
        <span>服务: {{ previewServiceName }}</span>
        <span class="text-primary break-all">{{ previewEndpoint }}</span>
      </div>
      <div id="map-preview-container" class="w-full h-[500px] max-sm:h-[350px] max-[480px]:h-[250px] border border-base-300 rounded"></div>
      <template #footer>
        <button class="btn" @click="closePreview">关闭</button>
      </template>
    </AppModal>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, nextTick, watch, h } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Icon } from '@iconify/vue'
import AppTable from '@/components/AppTable.vue'
import AppModal from '@/components/AppModal.vue'
import { useToast } from '@/components/AppToast'
import { useConfirm } from '@/components/AppConfirm'
import 'ol/ol.css'
import Map from 'ol/Map'
import View from 'ol/View'
import TileLayer from 'ol/layer/Tile'
import OSM from 'ol/source/OSM'
import TileWMS from 'ol/source/TileWMS'
import XYZ from 'ol/source/XYZ'

const toast = useToast()
const confirm = useConfirm()

const serviceList = ref([])
const dialogVisible = ref(false)
const previewVisible = ref(false)
const isEdit = ref(false)
const saving = ref(false)
const formRef = ref(null)
const searchText = ref('')
const page = ref(1)
const pageSize = ref(10)
const total = ref(0)
const loading = ref(false)
let previewMap = null

const typeLabels = { wmts: 'WMTS', tms: 'TMS', wms: 'WMS', wfs: 'WFS', geoserver: 'GeoServer', arcgis: 'ArcGIS' }

const serviceColumns = [
  { accessorKey: 'name', header: '服务名称' },
  {
    accessorKey: 'type',
    header: '类型',
    cell: (info) => h('span', { class: `badge badge-sm ${badgeType(info.getValue())}` }, typeLabels[info.getValue()] || info.getValue()),
  },
  { accessorKey: 'endpoint', header: '端点地址' },
  {
    accessorKey: 'connected',
    header: '状态',
    cell: (info) => h('span', { class: `badge badge-sm ${info.getValue() ? 'badge-success' : 'badge-ghost'}` }, info.getValue() ? '已连接' : '未连接'),
  },
  {
    id: 'actions',
    header: '操作',
    cell: (info) => h('div', { class: 'flex gap-1 flex-wrap' }, [
      h('button', { class: 'btn btn-xs btn-ghost text-primary', onClick: () => previewService(info.row.original) }, '预览'),
      h('button', { class: 'btn btn-xs btn-ghost text-primary', onClick: () => testConnection(info.row.original) }, '测试'),
      h('button', { class: 'btn btn-xs btn-ghost text-primary', onClick: () => showEditDialog(info.row.original) }, '编辑'),
      h('button', { class: 'btn btn-xs btn-ghost text-error', onClick: () => removeService(info.row.original) }, '删除'),
    ]),
  },
]

const badgeTypes = { wmts: 'badge-ghost', tms: 'badge-success', wms: 'badge-warning', wfs: 'badge-info', geoserver: 'badge-error', arcgis: 'badge-ghost' }
function badgeType(type) { return badgeTypes[type] || 'badge-ghost' }

const formData = reactive({ id: '', name: '', type: 'wmts', endpoint: '', username: '', password: '', remark: '', connected: false })
const previewServiceName = ref('')
const previewEndpoint = ref('')

const formRules = {
  name: [{ required: true, message: '请输入服务名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择服务类型', trigger: 'change' }],
  endpoint: [{ required: true, message: '请输入端点地址', trigger: 'blur' }],
}

function generateId() { return Date.now().toString(36) + Math.random().toString(36).slice(2, 8) }

watch([searchText, pageSize], () => { page.value = 1; loadServices() })
watch(page, () => { loadServices() })

async function loadServices() {
  loading.value = true
  try {
    const offset = (page.value - 1) * pageSize.value
    const result = await invoke('get_services', { keyword: searchText.value || null, offset, limit: pageSize.value })
    serviceList.value = result.items || []
    total.value = result.total || 0
  } catch (err) { console.error('加载服务列表失败:', err) }
  finally { loading.value = false }
}

function showAddDialog() { isEdit.value = false; dialogVisible.value = true }
function showEditDialog(row) {
  isEdit.value = true
  Object.assign(formData, { id: row.id, name: row.name, type: row.type, endpoint: row.endpoint, username: row.username || '', password: row.password || '', remark: row.remark || '', connected: row.connected })
  dialogVisible.value = true
}
function resetForm() {
  formRef.value?.resetFields()
  Object.assign(formData, { id: '', name: '', type: 'wmts', endpoint: '', username: '', password: '', remark: '', connected: false })
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
    toast.success(isEdit.value ? '服务已更新' : '服务已注册')
    await loadServices()
    await testConnection(result)
  } catch (err) { toast.error('保存失败: ' + err) }
  finally { saving.value = false }
}

async function testConnection(row) {
  toast.info(`正在测试连接: ${row.name}`)
  try {
    const connected = await invoke('test_service_connection', { service: row })
    row.connected = connected
    await invoke('update_service', { service: row })
    toast[connected ? 'success' : 'error'](connected ? '连接成功' : '连接失败，请检查配置')
  } catch (err) { row.connected = false; toast.error('连接测试失败: ' + err) }
}

async function removeService(row) {
  const ok = await confirm('确认删除', `确定删除服务 "${row.name}" 吗？`)
  if (!ok) return
  try {
    await invoke('delete_service', { id: row.id })
    toast.success('已删除')
    loadServices()
  } catch (err) { toast.error('删除失败: ' + err) }
}

function buildPreviewLayers(service) {
  const endpoint = service.endpoint
  const layers = []
  switch (service.type) {
    case 'wmts': {
      const wmtsUrl = endpoint.endsWith('/') ? endpoint + '{TileMatrix}/{TileRow}/{TileCol}.png' : endpoint + '/{TileMatrix}/{TileRow}/{TileCol}.png'
      layers.push(new TileLayer({ source: new XYZ({ url: wmtsUrl, tileSize: 256 }), visible: true }))
      break
    }
    case 'wms': {
      layers.push(new TileLayer({ source: new TileWMS({ url: endpoint, params: { 'LAYERS': 'topp:states', 'TILED': true, 'VERSION': '1.1.1' }, serverType: 'geoserver', crossOrigin: 'anonymous' }), visible: true }))
      break
    }
    case 'tms': {
      const tmsUrl = endpoint.endsWith('/') ? endpoint + '{z}/{x}/{y}.png' : endpoint + '/{z}/{x}/{y}.png'
      layers.push(new TileLayer({ source: new XYZ({ url: tmsUrl, tileSize: 256 }), visible: true }))
      break
    }
    case 'geoserver': {
      const baseUrl2 = endpoint.replace(/\/rest.*/, '')
      layers.push(new TileLayer({ source: new TileWMS({ url: `${baseUrl2}/gwc/service/wms`, params: { 'LAYERS': 'topp:states', 'TILED': true, 'VERSION': '1.1.1' }, serverType: 'geoserver', crossOrigin: 'anonymous' }), visible: true }))
      break
    }
    case 'arcgis': {
      const arcUrl = endpoint.endsWith('/') ? endpoint + 'tile/{z}/{y}/{x}' : endpoint + '/tile/{z}/{y}/{x}'
      layers.push(new TileLayer({ source: new XYZ({ url: arcUrl, tileSize: 256 }), visible: true }))
      break
    }
  }
  return layers
}

function previewService(row) {
  previewServiceName.value = row.name
  previewEndpoint.value = row.endpoint
  previewVisible.value = true
  nextTick(() => { initMap(row) })
}

function initMap(service) {
  if (previewMap) { previewMap.setTarget(null); previewMap = null }
  const layers = buildPreviewLayers(service)
  const osmLayer = new TileLayer({ source: new OSM(), visible: true, opacity: 0.3 })
  previewMap = new Map({
    target: 'map-preview-container',
    layers: [osmLayer, ...layers],
    view: new View({ center: [12900000, 4900000], zoom: 4, projection: 'EPSG:3857' }),
  })
}

function closePreview() {
  if (previewMap) { previewMap.setTarget(null); previewMap = null }
  previewVisible.value = false
}

onMounted(() => { loadServices() })
</script>

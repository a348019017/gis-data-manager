<template>
  <div>
    <div class="flex items-center justify-between mb-4 flex-wrap gap-2">
      <div class="flex items-center gap-3">
        <h2 class="text-lg font-semibold">GIS 工具</h2>
        <span class="text-xs text-base-content/50 bg-base-200 px-2.5 py-0.5 rounded-full">{{ filteredTools.length }} 个工具</span>
      </div>
    </div>

    <!-- 分类标签 -->
    <div class="flex gap-1 mb-4 flex-wrap">
      <button v-for="cat in categories" :key="cat.value"
              class="btn btn-sm" :class="activeCategory === cat.value ? 'btn-primary' : 'btn-ghost'"
              @click="activeCategory = cat.value">{{ cat.label }}</button>
    </div>

    <!-- 适用对象标签 -->
    <div class="flex items-center gap-2 mb-5">
      <span class="text-xs text-base-content/50">适用对象：</span>
      <el-radio-group v-model="activeTag" size="small">
        <el-radio-button value="all">全部</el-radio-button>
        <el-radio-button value="ai">AI 智能体</el-radio-button>
        <el-radio-button value="human">人工操作</el-radio-button>
        <el-radio-button value="both">两者通用</el-radio-button>
      </el-radio-group>
    </div>

    <!-- 工具卡片网格 -->
    <div class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4">
      <div v-for="tool in filteredTools" :key="tool.id"
           class="card bg-base-100 border border-base-300 rounded-xl p-5 transition-all hover:border-primary hover:shadow-md hover:-translate-y-0.5 relative overflow-hidden group">
        <div class="absolute top-0 left-0 right-0 h-0.75 bg-gradient-to-r from-primary to-success opacity-0 group-hover:opacity-100 transition-opacity"
             :class="{ 'from-warning to-error': tool.tags.includes('ai') && !tool.tags.includes('human') }"></div>
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 rounded-lg flex items-center justify-center shrink-0"
               :class="iconBgClass(tool.category)">
            <Icon :icon="categoryIcon(tool.category)" width="20" :class="iconColorClass(tool.category)" />
          </div>
          <h3 class="text-sm font-semibold flex-1 m-0">{{ tool.name }}</h3>
          <div class="flex gap-1">
            <span v-for="t in parseTags(tool.tags)" :key="t"
                  class="badge badge-sm" :class="badgeClass(t)">
              {{ tagLabel(t) }}
            </span>
          </div>
        </div>
        <p class="text-xs text-base-content/70 leading-relaxed mb-3">{{ tool.description }}</p>
        <div class="flex items-center gap-1.5 flex-wrap mb-3">
          <span class="text-sm text-base-content/50">参数：</span>
          <span v-for="p in parseParams(tool.params)" :key="p.name"
                class="badge badge-sm" :class="p.required ? 'badge-ghost' : 'badge-ghost text-base-content/50'">
            {{ p.name }}{{ p.required ? '*' : '' }}
          </span>
        </div>
        <div class="flex justify-between items-center pt-3 border-t border-base-200">
          <span class="text-xs text-base-content/50">{{ tool.returns }}</span>
          <button class="btn btn-primary btn-sm" @click="showToolDialog(tool)">使用</button>
        </div>
      </div>
    </div>

    <div v-if="filteredTools.length === 0" class="text-center py-12 text-base-content/50">
      该分类下暂无工具
    </div>

    <!-- 工具使用对话框 -->
    <AppModal v-model="dialogVisible" :title="currentTool?.name">
      <p class="text-sm text-base-content/70 mb-4">{{ currentTool?.description }}</p>
      <el-form v-if="currentTool" label-position="top" label-width="100%">
        <el-form-item
          v-for="param in currentToolParams" :key="param.name"
          :label="`${param.description} (${param.name})`"
          :required="param.required">
          <el-input v-if="param.type === 'string'" v-model="toolParams[param.name]"
                    :placeholder="param.default || '请输入'" />
          <el-input-number v-else-if="param.type === 'number'" v-model="toolParams[param.name]"
                           :placeholder="String(param.default || '')" style="width: 100%" />
          <el-select v-else-if="param.type === 'select'" v-model="toolParams[param.name]"
                     :placeholder="param.default || '请选择'" style="width: 100%">
            <el-option v-for="opt in param.options" :key="opt" :label="opt" :value="opt" />
          </el-select>
          <el-input v-else v-model="toolParams[param.name]" :placeholder="param.default || '请输入'" />
        </el-form-item>
      </el-form>
      <template #footer>
        <button class="btn btn-ghost" @click="dialogVisible = false">取消</button>
        <button class="btn btn-primary" :disabled="executing" @click="executeTool">
          <span v-if="executing" class="loading loading-spinner loading-xs"></span>
          执行工具
        </button>
      </template>
    </AppModal>

    <!-- 执行结果对话框 -->
    <AppModal v-model="resultVisible" title="执行结果">
      <pre class="bg-base-200 p-4 rounded-lg text-xs font-mono overflow-auto max-h-96 whitespace-pre-wrap break-all">{{ executionResult }}</pre>
      <template #footer>
        <button class="btn btn-primary" @click="resultVisible = false">关闭</button>
      </template>
    </AppModal>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Icon } from '@iconify/vue'
import AppModal from '@/components/AppModal.vue'
import { useToast } from '@/components/AppToast'

const toast = useToast()

const tools = ref([])
const activeCategory = ref('all')
const activeTag = ref('all')
const dialogVisible = ref(false)
const currentTool = ref(null)
const currentToolParams = ref([])
const toolParams = ref({})
const executing = ref(false)
const resultVisible = ref(false)
const executionResult = ref('')

const categories = [
  { value: 'all', label: '全部' },
  { value: 'spatial_analysis', label: '空间分析' },
  { value: 'data_conversion', label: '格式转换' },
  { value: 'coordinate', label: '坐标投影' },
  { value: 'data_management', label: '数据管理' },
  { value: 'raster_processing', label: '栅格处理 (GDAL)' },
]

const filteredTools = computed(() => {
  return tools.value.filter(tool => {
    const catMatch = activeCategory.value === 'all' || tool.category === activeCategory.value
    const tagMatch = activeTag.value === 'all' ||
      tool.tags.includes(activeTag.value) ||
      tool.tags.includes('both')
    return catMatch && tagMatch
  })
})

function parseTags(tagsStr) { return tagsStr.split(',').map(t => t.trim()).filter(Boolean) }
function parseParams(paramsStr) { try { return JSON.parse(paramsStr) } catch { return [] } }

function badgeClass(tag) {
  return { ai: 'badge-warning', human: 'badge-info', both: 'badge-success' }[tag] || 'badge-ghost'
}
function tagLabel(tag) {
  return { ai: 'AI', human: '人工', both: '通用' }[tag] || tag
}

const iconMap = {
  spatial_analysis: 'mdi:compass',
  data_conversion: 'mdi:file-swap',
  coordinate: 'mdi:crosshairs-gps',
  data_management: 'mdi:chart-bar',
  raster_processing: 'mdi:image',
}
const iconBgMap = {
  spatial_analysis: 'bg-error/10',
  data_conversion: 'bg-warning/10',
  coordinate: 'bg-primary/10',
  data_management: 'bg-success/10',
  raster_processing: 'bg-base-200',
}
const iconColorMap = {
  spatial_analysis: 'text-error',
  data_conversion: 'text-warning',
  coordinate: 'text-primary',
  data_management: 'text-success',
  raster_processing: 'text-base-content/50',
}

function categoryIcon(category) { return iconMap[category] || 'mdi:file-document' }
function iconBgClass(category) { return iconBgMap[category] || 'bg-base-200' }
function iconColorClass(category) { return iconColorMap[category] || 'text-base-content/50' }

async function loadTools() {
  try {
    tools.value = await invoke('get_gis_tools')
  } catch (err) {
    console.error('加载工具列表失败:', err)
    toast.error('加载工具列表失败')
  }
}

function showToolDialog(tool) {
  currentTool.value = tool
  currentToolParams.value = parseParams(tool.params)
  toolParams.value = {}
  for (const p of currentToolParams.value) {
    toolParams.value[p.name] = p.default !== undefined ? p.default : ''
  }
  dialogVisible.value = true
}

async function executeTool() {
  executing.value = true
  try {
    const result = await invoke('execute_gis_tool', {
      toolId: currentTool.value.id,
      params: toolParams.value,
    })
    executionResult.value = JSON.stringify(result, null, 2)
    dialogVisible.value = false
    resultVisible.value = true
    toast.success('工具执行成功')
  } catch (err) {
    toast.error('工具执行失败: ' + err)
  } finally {
    executing.value = false
  }
}

onMounted(() => { loadTools() })
</script>

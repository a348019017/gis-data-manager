<template>
  <div class="gis-tools">
    <div class="page-header">
      <div class="header-left">
        <h2>GIS 工具</h2>
        <span class="tool-count">{{ filteredTools.length }} 个工具</span>
      </div>
    </div>

    <!-- 分类标签 -->
    <div class="category-tabs">
      <span
        v-for="cat in categories"
        :key="cat.value"
        :class="['tab', { active: activeCategory === cat.value }]"
        @click="activeCategory = cat.value"
      >
        {{ cat.label }}
      </span>
    </div>

    <!-- 使用对象标签 -->
    <div class="tag-filter">
      <span class="filter-label">适用对象：</span>
      <el-radio-group v-model="activeTag" size="small">
        <el-radio-button value="all">全部</el-radio-button>
        <el-radio-button value="ai">AI 智能体</el-radio-button>
        <el-radio-button value="human">人工操作</el-radio-button>
        <el-radio-button value="both">两者通用</el-radio-button>
      </el-radio-group>
    </div>

    <!-- 工具卡片网格 -->
    <div class="tools-grid">
      <div
        v-for="tool in filteredTools"
        :key="tool.id"
        class="tool-card"
        :class="{ 'is-ai': tool.tags.includes('ai') && !tool.tags.includes('human') }"
      >
        <div class="card-header">
          <div class="card-icon" :class="categoryIconClass(tool.category)">
            <el-icon :size="20"><component :is="categoryIcon(tool.category)" /></el-icon>
          </div>
          <h3>{{ tool.name }}</h3>
          <div class="card-tags">
            <el-tag
              v-for="t in parseTags(tool.tags)"
              :key="t"
              size="small"
              :type="tagType(t)"
              effect="plain"
            >
              {{ tagLabel(t) }}
            </el-tag>
          </div>
        </div>
        <p class="card-desc">{{ tool.description }}</p>
        <div class="card-params">
          <span class="param-label">参数：</span>
          <el-tag
            v-for="p in parseParams(tool.params)"
            :key="p.name"
            size="small"
            :type="p.required ? '' : 'info'"
            effect="plain"
          >
            {{ p.name }}{{ p.required ? '*' : '' }}
          </el-tag>
        </div>
        <div class="card-footer">
          <span class="card-returns">{{ tool.returns }}</span>
          <el-button size="small" type="primary" @click="showToolDialog(tool)">
            使用
          </el-button>
        </div>
      </div>
    </div>

    <el-empty v-if="filteredTools.length === 0" description="该分类下暂无工具" />

    <!-- 工具使用对话框 -->
    <el-dialog v-model="dialogVisible" :title="currentTool?.name" width="560px">
      <p class="dialog-desc">{{ currentTool?.description }}</p>

      <el-form v-if="currentTool" label-position="top" label-width="100%">
        <el-form-item
          v-for="param in currentToolParams"
          :key="param.name"
          :label="`${param.description} (${param.name})`"
          :required="param.required"
        >
          <el-input
            v-if="param.type === 'string'"
            v-model="toolParams[param.name]"
            :placeholder="param.default || '请输入'"
          />
          <el-input-number
            v-else-if="param.type === 'number'"
            v-model="toolParams[param.name]"
            :placeholder="String(param.default || '')"
            style="width: 100%"
          />
          <el-select
            v-else-if="param.type === 'select'"
            v-model="toolParams[param.name]"
            :placeholder="param.default || '请选择'"
            style="width: 100%"
          >
            <el-option
              v-for="opt in param.options"
              :key="opt"
              :label="opt"
              :value="opt"
            />
          </el-select>
          <el-input
            v-else
            v-model="toolParams[param.name]"
            :placeholder="param.default || '请输入'"
          />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="executing" @click="executeTool">
          执行工具
        </el-button>
      </template>
    </el-dialog>

    <!-- 执行结果对话框 -->
    <el-dialog v-model="resultVisible" title="执行结果" width="560px">
      <pre class="result-content">{{ executionResult }}</pre>
      <template #footer>
        <el-button type="primary" @click="resultVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import {
  Scissor,
  Files,
  Position,
  DataAnalysis,
  Document,
  Picture,
  Compass,
} from '@element-plus/icons-vue'

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

function parseTags(tagsStr) {
  return tagsStr.split(',').map(t => t.trim()).filter(Boolean)
}

function parseParams(paramsStr) {
  try {
    return JSON.parse(paramsStr)
  } catch {
    return []
  }
}

function tagType(tag) {
  return { ai: 'warning', human: 'info', both: 'success' }[tag] || 'info'
}

function tagLabel(tag) {
  return { ai: 'AI', human: '人工', both: '通用' }[tag] || tag
}

function categoryIcon(category) {
  return {
    spatial_analysis: Compass,
    data_conversion: Files,
    coordinate: Position,
    data_management: DataAnalysis,
    raster_processing: Picture,
  }[category] || Document
}

function categoryIconClass(category) {
  return {
    spatial_analysis: 'icon-analysis',
    data_conversion: 'icon-conversion',
    coordinate: 'icon-coordinate',
    data_management: 'icon-management',
    raster_processing: 'icon-raster',
  }[category] || 'icon-default'
}

async function loadTools() {
  try {
    tools.value = await invoke('get_gis_tools')
  } catch (err) {
    console.error('加载工具列表失败:', err)
    ElMessage.error('加载工具列表失败')
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
    ElMessage.success('工具执行成功')
  } catch (err) {
    ElMessage.error('工具执行失败: ' + err)
  } finally {
    executing.value = false
  }
}

onMounted(() => {
  loadTools()
})
</script>

<style scoped>
.gis-tools {
  width: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.page-header h2 {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.tool-count {
  font-size: 13px;
  color: #909399;
  background: #f5f7fa;
  padding: 2px 10px;
  border-radius: 10px;
}

.category-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.tab {
  padding: 6px 16px;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
  color: #606266;
  background: #f5f7fa;
  transition: all 0.2s;
  user-select: none;
}

.tab:hover {
  background: #ecf5ff;
  color: #409eff;
}

.tab.active {
  background: #409eff;
  color: #fff;
}

.tag-filter {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 20px;
}

.filter-label {
  font-size: 13px;
  color: #909399;
}

.tools-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 16px;
}

.tool-card {
  background: #fff;
  border: 1px solid #ebeef5;
  border-radius: 10px;
  padding: 20px;
  transition: all 0.2s;
  position: relative;
  overflow: hidden;
}

.tool-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(90deg, #409eff, #67c23a);
  opacity: 0;
  transition: opacity 0.2s;
}

.tool-card:hover {
  border-color: #409eff;
  box-shadow: 0 4px 16px rgba(64, 158, 255, 0.1);
  transform: translateY(-2px);
}

.tool-card:hover::before {
  opacity: 1;
}

.tool-card.is-ai::before {
  background: linear-gradient(90deg, #e6a23c, #f56c6c);
}

.card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.card-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.icon-analysis { background: #fef0f0; color: #f56c6c; }
.icon-conversion { background: #fdf6ec; color: #e6a23c; }
.icon-coordinate { background: #ecf5ff; color: #409eff; }
.icon-management { background: #f0f9eb; color: #67c23a; }
.icon-raster { background: #f5f7fa; color: #909399; }
.icon-default { background: #f5f7fa; color: #909399; }

.card-header h3 {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  margin: 0;
  flex: 1;
}

.card-tags {
  display: flex;
  gap: 4px;
}

.card-desc {
  font-size: 13px;
  color: #606266;
  line-height: 1.5;
  margin-bottom: 12px;
}

.card-params {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.param-label {
  font-size: 12px;
  color: #909399;
}

.card-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-top: 12px;
  border-top: 1px solid #f0f0f0;
}

.card-returns {
  font-size: 12px;
  color: #909399;
}

.dialog-desc {
  font-size: 14px;
  color: #606266;
  margin-bottom: 16px;
}

.result-content {
  background: #f5f7fa;
  padding: 16px;
  border-radius: 8px;
  font-size: 13px;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 400px;
  overflow-y: auto;
}

@media (max-width: 768px) {
  .tools-grid {
    grid-template-columns: 1fr;
  }
  .category-tabs {
    gap: 2px;
  }
  .tab {
    padding: 4px 12px;
    font-size: 12px;
  }
  .tag-filter {
    flex-wrap: wrap;
    gap: 8px;
  }
}
</style>

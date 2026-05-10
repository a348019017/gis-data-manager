<template>
  <div class="settings">
    <h2>设置</h2>

    <el-tabs v-model="activeTab">
      <!-- AI 模型配置 -->
      <el-tab-pane label="AI 模型" name="model">
        <el-card class="settings-card">
          <el-form
            ref="modelFormRef"
            :model="modelConfig"
            :rules="modelRules"
            label-width="120px"
            label-position="right"
          >
            <el-form-item label="模型提供商" prop="provider">
              <el-select v-model="modelConfig.provider" placeholder="选择模型提供商" style="width: 100%">
                <el-option label="OpenAI (兼容)" value="openai" />
                <el-option label="Anthropic (Claude)" value="anthropic" />
                <el-option label="Ollama (本地)" value="ollama" />
                <el-option label="硅基流动 (SiliconFlow)" value="siliconflow" />
                <el-option label="自定义" value="custom" />
              </el-select>
            </el-form-item>

            <el-form-item label="API 地址" prop="apiUrl">
              <el-input v-model="modelConfig.apiUrl" placeholder="https://api.openai.com/v1" />
            </el-form-item>

            <el-form-item label="API Key" prop="apiKey">
              <el-input v-model="modelConfig.apiKey" type="password" show-password placeholder="sk-..." />
            </el-form-item>

            <el-form-item label="模型名称" prop="modelName">
              <el-input v-model="modelConfig.modelName" placeholder="gpt-4o / claude-sonnet-4-6" />
            </el-form-item>

            <el-form-item label="最大 Token">
              <el-input-number v-model="modelConfig.maxTokens" :min="256" :max="64000" :step="256" />
            </el-form-item>

            <el-form-item label="Temperature">
              <el-slider v-model="modelConfig.temperature" :min="0" :max="1" :step="0.1" style="width: 200px" />
              <span class="slider-value">{{ modelConfig.temperature }}</span>
            </el-form-item>

            <el-form-item>
              <el-button type="primary" @click="saveModelConfig">保存配置</el-button>
              <el-button @click="testModelConnection" :loading="testingModel">测试连接</el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-tab-pane>

      <!-- 数据字典 -->
      <el-tab-pane label="数据字典" name="dict">
        <el-card class="settings-card">
          <div class="dict-layout">
            <!-- 左侧分类树 -->
            <div class="dict-sidebar">
              <div class="dict-sidebar-header">
                <span>分类</span>
                <el-button type="primary" link size="small" @click="showAddCategoryDialog">+ 新增</el-button>
              </div>
              <el-menu
                :default-active="dictCategory"
                class="dict-menu"
                @select="onCategorySelect"
              >
                <el-menu-item v-for="cat in dictCategoryTree" :key="cat.value" :index="cat.value">
                  <span>{{ cat.label }}</span>
                  <span class="item-count">{{ cat.count }}</span>
                </el-menu-item>
              </el-menu>
            </div>

            <!-- 右侧表格 -->
            <div class="dict-content">
              <div class="dict-content-header">
                <span class="dict-content-title">{{ currentCategoryLabel }}</span>
                <el-button type="primary" size="small" @click="showAddDictDialog">新增标签</el-button>
              </div>

              <el-table :data="dictItems" size="small" stripe>
                <el-table-column prop="label" label="名称" min-width="120" />
                <el-table-column prop="value" label="值" width="150" />
                <el-table-column prop="sort_order" label="排序" width="80" />
                <el-table-column label="操作" width="140" fixed="right">
                  <template #default="{ row }">
                    <el-button link type="primary" size="small" @click="editDictItem(row)">编辑</el-button>
                    <el-button link type="danger" size="small" @click="deleteDictItemConfirm(row)">删除</el-button>
                  </template>
                </el-table-column>
              </el-table>
            </div>
          </div>
        </el-card>
      </el-tab-pane>

      <!-- 通用设置 -->
      <el-tab-pane label="通用" name="general">
        <el-card class="settings-card">
          <el-form label-width="120px" label-position="right">
            <el-form-item label="数据库路径">
              <el-input :model-value="generalConfig.dbPath" disabled />
            </el-form-item>
            <el-form-item label="数据目录">
              <el-input :model-value="generalConfig.dataDir" disabled />
            </el-form-item>
            <el-form-item label="应用版本">
              <span>{{ generalConfig.version }}</span>
            </el-form-item>
          </el-form>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <!-- 新增分类对话框 -->
    <el-dialog v-model="categoryDialogVisible" title="新增分类" width="400px">
      <el-form label-width="80px" label-position="right">
        <el-form-item label="分类标识" required>
          <el-input v-model="newCategoryValue" placeholder="英文标识，如 project_type" />
        </el-form-item>
        <el-form-item label="分类名称" required>
          <el-input v-model="newCategoryLabel" placeholder="显示名称" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="categoryDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveNewCategory">保存</el-button>
      </template>
    </el-dialog>

    <!-- 数据字典编辑对话框 -->
    <el-dialog v-model="dictDialogVisible" :title="dictEditing ? '编辑标签' : '新增标签'" width="480px">
      <el-form :model="dictForm" label-width="80px" label-position="right">
        <el-form-item label="分类">
          <el-select v-model="dictForm.category" placeholder="选择分类" style="width: 100%">
            <el-option v-for="cat in dictCategoryTree" :key="cat.value" :label="cat.label" :value="cat.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="名称" required>
          <el-input v-model="dictForm.label" placeholder="显示名称" />
        </el-form-item>
        <el-form-item label="值" required>
          <el-input v-model="dictForm.value" placeholder="英文标识" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="dictForm.sortOrder" :min="0" :max="999" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dictDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveDictItem">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage, ElMessageBox } from 'element-plus'

const activeTab = ref('model')

// --- AI 模型配置 ---
const modelFormRef = ref(null)
const testingModel = ref(false)

const modelConfig = reactive({
  provider: 'openai',
  apiUrl: 'https://api.openai.com/v1',
  apiKey: '',
  modelName: 'gpt-4o',
  maxTokens: 4096,
  temperature: 0.7,
})

const modelRules = {
  provider: [{ required: true, message: '请选择模型提供商', trigger: 'change' }],
  apiUrl: [{ required: true, message: '请输入 API 地址', trigger: 'blur' }],
  modelName: [{ required: true, message: '请输入模型名称', trigger: 'blur' }],
}

const generalConfig = reactive({
  dbPath: '',
  dataDir: '',
  version: '0.1.0',
})

async function saveModelConfig() {
  const valid = await modelFormRef.value?.validate().catch(() => false)
  if (!valid) return
  try {
    await invoke('save_settings', { settings: { ...modelConfig } })
    ElMessage.success('配置已保存')
  } catch (err) {
    ElMessage.error('保存失败: ' + err)
  }
}

async function testModelConnection() {
  if (!modelConfig.apiUrl || !modelConfig.apiKey || !modelConfig.modelName) {
    ElMessage.warning('请先填写完整的模型配置信息')
    return
  }
  testingModel.value = true
  try {
    const result = await invoke('test_model_connection', { settings: { ...modelConfig } })
    ElMessage[result ? 'success' : 'error'](result ? '连接成功' : '连接失败')
  } catch (err) {
    ElMessage.error('连接测试失败: ' + err)
  } finally {
    testingModel.value = false
  }
}

// --- 数据字典 ---
const dictCategory = ref('data_type')
const dictItems = ref([])
const dictDialogVisible = ref(false)
const dictEditing = ref(null)
const dictForm = reactive({ id: '', category: '', label: '', value: '', sortOrder: 0 })

// 新增分类
const categoryDialogVisible = ref(false)
const newCategoryValue = ref('')
const newCategoryLabel = ref('')

// 分类名称映射（用户自定义标签存储在这里）
const categoryNames = reactive({})

// 左侧分类树（含条目数量）
const dictCategoryTree = ref([])
const currentCategoryLabel = ref('')

async function loadCategories() {
  try {
    const items = await invoke('get_dict_items')
    const grouped = {}
    for (const item of items) {
      if (!grouped[item.category]) grouped[item.category] = []
      grouped[item.category].push(item)
    }
    dictCategoryTree.value = Object.keys(grouped).map(k => ({
      value: k,
      label: categoryNames[k] || k,
      count: grouped[k].length,
    }))
    // 确保当前选中分类有效
    if (dictCategoryTree.value.length > 0) {
      const hasCurrent = dictCategoryTree.value.some(c => c.value === dictCategory.value)
      if (!hasCurrent) dictCategory.value = dictCategoryTree.value[0].value
      currentCategoryLabel.value = dictCategoryTree.value.find(c => c.value === dictCategory.value)?.label || ''
    }
  } catch (err) {
    console.error('加载分类失败:', err)
  }
}

async function loadDictItems() {
  try {
    dictItems.value = await invoke('get_dict_items', { category: dictCategory.value })
    currentCategoryLabel.value = dictCategoryTree.value.find(c => c.value === dictCategory.value)?.label || ''
  } catch (err) {
    console.error('加载字典项失败:', err)
  }
}

function onCategorySelect(key) {
  dictCategory.value = key
  loadDictItems()
}

function showAddCategoryDialog() {
  newCategoryValue.value = ''
  newCategoryLabel.value = ''
  categoryDialogVisible.value = true
}

async function saveNewCategory() {
  if (!newCategoryValue.value || !newCategoryLabel.value) {
    ElMessage.warning('分类标识和名称不能为空')
    return
  }
  // 插入一条占位字典项以创建分类
  try {
    const id = 'dict_' + Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
    await invoke('add_dict_item', {
      item: { id, category: newCategoryValue.value, label: newCategoryLabel.value + ' (占位)', value: newCategoryValue.value, sort_order: 0 }
    })
    categoryNames[newCategoryValue.value] = newCategoryLabel.value
    categoryDialogVisible.value = false
    ElMessage.success('分类已添加')
    await loadCategories()
    dictCategory.value = newCategoryValue.value
    await loadDictItems()
  } catch (err) {
    ElMessage.error('添加失败: ' + err)
  }
}

function showAddDictDialog() {
  dictEditing.value = null
  Object.assign(dictForm, { id: uuid(), category: dictCategory.value, label: '', value: '', sortOrder: 0 })
  dictDialogVisible.value = true
}

function editDictItem(row) {
  dictEditing.value = row.id
  Object.assign(dictForm, { id: row.id, category: row.category, label: row.label, value: row.value, sortOrder: row.sort_order })
  dictDialogVisible.value = true
}

async function saveDictItem() {
  if (!dictForm.label || !dictForm.value) {
    ElMessage.warning('名称和值不能为空')
    return
  }
  try {
    const item = { id: dictForm.id, category: dictForm.category, label: dictForm.label, value: dictForm.value, sort_order: dictForm.sortOrder }
    if (dictEditing.value) {
      await invoke('update_dict_item', { item })
      ElMessage.success('已更新')
    } else {
      await invoke('add_dict_item', { item })
      ElMessage.success('已添加')
    }
    dictDialogVisible.value = false
    await loadCategories()
    await loadDictItems()
  } catch (err) {
    ElMessage.error('保存失败: ' + err)
  }
}

async function deleteDictItemConfirm(row) {
  await ElMessageBox.confirm(`确定删除标签 "${row.label}" 吗？`, '确认删除', { type: 'warning' })
  try {
    await invoke('delete_dict_item', { id: row.id })
    ElMessage.success('已删除')
    await loadDictItems()
  } catch (err) {
    ElMessage.error('删除失败: ' + err)
  }
}

function uuid() {
  return 'dict_' + Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

// --- 初始化 ---
async function loadSettings() {
  try {
    const settings = await invoke('get_settings')
    if (settings) Object.assign(modelConfig, settings)
  } catch (err) {
    console.log('无已保存的模型配置')
  }
  try {
    const info = await invoke('get_app_info')
    if (info) Object.assign(generalConfig, info)
  } catch (err) {
    console.error('获取应用信息失败:', err)
  }
  await loadCategories()
  await loadDictItems()
}

onMounted(() => {
  loadSettings()
})
</script>

<style scoped>
.settings { width: 100%; }
.settings h2 {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 12px;
}
.settings-card { margin-bottom: 12px; }
.slider-value {
  margin-left: 12px;
  color: #606266;
  font-size: 14px;
  min-width: 36px;
  display: inline-block;
}

/* 数据字典左右布局 */
.dict-layout {
  display: flex;
  height: 500px;
  gap: 0;
}
.dict-sidebar {
  width: 220px;
  border-right: 1px solid #ebeef5;
  display: flex;
  flex-direction: column;
}
.dict-sidebar-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  border-bottom: 1px solid #ebeef5;
}
.dict-menu {
  border: none;
  flex: 1;
  overflow-y: auto;
}
.dict-menu :deep(.el-menu-item) {
  height: 40px;
  line-height: 40px;
}
.dict-menu :deep(.el-menu-item.is-active) {
  background-color: #ecf5ff;
  color: #409eff;
}
.item-count {
  font-size: 12px;
  color: #909399;
  margin-left: auto;
}
.dict-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.dict-content-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 0 12px;
}
.dict-content-title {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}
.dict-content :deep(.el-table) {
  flex: 1;
}

@media (max-width: 768px) {
  .settings { max-width: 100%; }
  .settings h2 { font-size: 16px; }
  :deep(.el-form-item__label) { font-size: 13px !important; }
  .slider-value { margin-left: 8px; font-size: 13px; }
  .dict-sidebar { width: 160px; }
}
</style>

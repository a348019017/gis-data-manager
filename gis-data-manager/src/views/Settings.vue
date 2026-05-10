<template>
  <div class="settings">
    <h2>设置</h2>

    <!-- AI 模型配置 -->
    <el-card class="settings-card" header="AI 模型配置">
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

    <!-- 通用设置 -->
    <el-card class="settings-card" header="通用设置">
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
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'

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

async function loadSettings() {
  try {
    const settings = await invoke('get_settings')
    if (settings) {
      Object.assign(modelConfig, settings)
    }
  } catch (err) {
    console.log('无已保存的模型配置')
  }

  try {
    const info = await invoke('get_app_info')
    if (info) {
      Object.assign(generalConfig, info)
    }
  } catch (err) {
    console.error('获取应用信息失败:', err)
  }
}

onMounted(() => {
  loadSettings()
})
</script>

<style scoped>
.settings {
  width: 100%;
}

.settings h2 {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 12px;
}

.settings-card {
  margin-bottom: 12px;
}

.slider-value {
  margin-left: 12px;
  color: #606266;
  font-size: 14px;
  min-width: 36px;
  display: inline-block;
}

@media (max-width: 768px) {
  .settings {
    max-width: 100%;
  }
  .settings h2 {
    font-size: 16px;
  }
  :deep(.el-form-item__label) {
    font-size: 13px !important;
  }
  .slider-value {
    margin-left: 8px;
    font-size: 13px;
  }
}
</style>

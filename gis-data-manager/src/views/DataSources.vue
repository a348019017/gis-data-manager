<template>
  <div class="datasources">
    <div class="page-header">
      <h2>数据源管理</h2>
      <div class="header-actions">
        <el-input
          v-model="searchText"
          placeholder="搜索名称、地址"
          clearable
          style="width: 220px"
          @clear="page = 1"
          @keyup.enter="page = 1"
        />
        <el-button :icon="Refresh" @click="loadSources">刷新</el-button>
        <el-button type="primary" :icon="Plus" @click="showAddDialog">
          添加数据源
        </el-button>
      </div>
    </div>

    <!-- 数据源列表 -->
    <div class="table-wrapper">
      <el-table :data="dataSourceList" size="small" stripe :loading="loading" style="width: 100%">
      <el-table-column prop="name" label="名称" min-width="150" />
      <el-table-column label="类型" width="100">
        <template #default="{ row }">
          <el-tag size="small" :type="row.type === 'oss' ? 'warning' : 'info'">
            {{ getTypeLabel(row.type) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="子类型" width="120">
        <template #default="{ row }">
          <span class="sub-type">{{ getSubtypeLabel(row.subtype) }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="host" label="地址" min-width="140" show-overflow-tooltip />
      <el-table-column label="端口" width="80">
        <template #default="{ row }">
          <span class="sub-type">{{ row.port || '-' }}</span>
        </template>
      </el-table-column>
      <el-table-column label="状态" width="100">
        <template #default="{ row }">
          <el-tag size="small" :type="row.connected ? 'success' : 'info'">
            {{ row.connected ? '已连接' : '未连接' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="200" fixed="right">
        <template #default="{ row }">
          <el-button link type="primary" size="small" @click="testConnection(row)">
            测试连接
          </el-button>
          <el-button link type="primary" size="small" @click="showEditDialog(row)">
            编辑
          </el-button>
          <el-button link type="danger" size="small" @click="removeDataSource(row)">
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-empty v-if="dataSourceList.length === 0 && !loading" description="暂无数据源" />
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

    <!-- 添加/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑数据源' : '添加数据源'"
      width="520px"
      @close="resetForm"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="80px"
        label-position="top"
      >
        <el-form-item label="数据源名称" prop="name">
          <el-input v-model="formData.name" placeholder="输入数据源名称" />
        </el-form-item>

        <!-- 数据源类型 -->
        <el-form-item label="数据源类型" prop="type">
          <el-radio-group v-model="formData.type">
            <el-radio-button value="database">数据库</el-radio-button>
            <el-radio-button value="oss">OSS 对象存储</el-radio-button>
          </el-radio-group>
        </el-form-item>

        <!-- 数据库配置 -->
        <template v-if="formData.type === 'database'">
          <el-form-item label="数据库类型" prop="subtype">
            <el-select v-model="formData.subtype" placeholder="选择数据库类型" style="width: 100%">
              <el-option label="PostgreSQL / PostGIS" value="postgresql" />
              <el-option label="MySQL" value="mysql" />
              <el-option label="SpatiaLite" value="spatialite" />
            </el-select>
          </el-form-item>
          <el-row :gutter="12">
            <el-col :span="16">
              <el-form-item label="主机地址" prop="host">
                <el-input v-model="formData.host" placeholder="localhost" />
              </el-form-item>
            </el-col>
            <el-col :span="8">
              <el-form-item label="端口" prop="port">
                <el-input-number v-model="formData.port" :min="1" :max="65535" style="width: 100%" />
              </el-form-item>
            </el-col>
          </el-row>
          <el-form-item label="数据库名" prop="database">
            <el-input v-model="formData.database" placeholder="数据库名称" />
          </el-form-item>
          <el-row :gutter="12">
            <el-col :span="12">
              <el-form-item label="用户名" prop="username">
                <el-input v-model="formData.username" placeholder="用户名" />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="密码" prop="password">
                <el-input v-model="formData.password" type="password" show-password placeholder="密码" />
              </el-form-item>
            </el-col>
          </el-row>
        </template>

        <!-- OSS 配置 -->
        <template v-if="formData.type === 'oss'">
          <el-form-item label="存储类型" prop="subtype">
            <el-select v-model="formData.subtype" placeholder="选择存储服务" style="width: 100%" @change="onOssSubtypeChange">
              <el-option label="阿里云 OSS" value="aliyun" />
              <el-option label="AWS S3" value="aws" />
              <el-option label="MinIO (S3兼容)" value="minio" />
            </el-select>
          </el-form-item>
          <el-row :gutter="12">
            <el-col :span="16">
              <el-form-item label="Endpoint" prop="host">
                <el-input v-model="formData.host" placeholder="oss-cn-hangzhou.aliyuncs.com" />
              </el-form-item>
            </el-col>
            <el-col :span="8">
              <el-form-item label="端口" prop="port">
                <el-input-number v-model="formData.port" :min="1" :max="65535" style="width: 100%" />
              </el-form-item>
            </el-col>
          </el-row>
          <el-form-item label="Bucket" prop="database">
            <el-input v-model="formData.database" placeholder="bucket 名称（选填）" />
          </el-form-item>
          <el-row :gutter="12">
            <el-col :span="12">
              <el-form-item label="AccessKey" prop="username">
                <el-input v-model="formData.username" placeholder="AccessKey ID" />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="SecretKey" prop="password">
                <el-input v-model="formData.password" type="password" show-password placeholder="SecretKey" />
              </el-form-item>
            </el-col>
          </el-row>
        </template>

        <el-form-item label="备注" prop="remark">
          <el-input v-model="formData.remark" type="textarea" :rows="2" placeholder="可选" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="testing" @click="handleSave">
          {{ isEdit ? '保存' : '添加并测试连接' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Plus, Refresh } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'

const dataSourceList = ref([])
const dialogVisible = ref(false)
const isEdit = ref(false)
const testing = ref(false)
const formRef = ref(null)
const searchText = ref('')
const page = ref(1)
const pageSize = ref(10)
const total = ref(0)
const loading = ref(false)

onMounted(() => {
  loadSources()
})

watch([searchText, pageSize], () => {
  page.value = 1
  loadSources()
})

watch(page, () => {
  loadSources()
})

const formData = reactive({
  id: '',
  name: '',
  type: 'database',
  subtype: '',
  host: '',
  port: 5432,
  database: '',
  username: '',
  password: '',
  remark: '',
  connected: false,
})

watch(() => formData.type, (newType) => {
  if (newType === 'oss' && formData.port === 5432) {
    const defaults = { minio: 9000, aws: 443, aliyun: 443 }
    formData.port = defaults[formData.subtype] || 443
  } else if (newType === 'database' && formData.port !== 5432) {
    if ([9000].includes(formData.port)) {
      formData.port = 5432
    }
  }
})

const formRules = {
  name: [{ required: true, message: '请输入数据源名称', trigger: 'blur' }],
  subtype: [{ required: true, message: '请选择类型', trigger: 'change' }],
  host: [{ required: true, message: '请输入地址', trigger: 'blur' }],
}

const typeMap = {
  postgresql: 'PostgreSQL',
  mysql: 'MySQL',
  spatialite: 'SpatiaLite',
  aliyun: '阿里云 OSS',
  aws: 'AWS S3',
  minio: 'MinIO',
}

function getTypeLabel(type) {
  return typeMap[type] || type
}

function getSubtypeLabel(subtype) {
  return typeMap[subtype] || subtype
}

function onOssSubtypeChange(subtype) {
  const defaults = { minio: 9000, aws: 443, aliyun: 443 }
  if (defaults[subtype] && formData.port === 5432) {
    formData.port = defaults[subtype]
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
    subtype: row.subtype,
    host: row.host,
    port: row.port,
    database: row.database,
    username: row.username,
    password: row.password,
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
    type: 'database',
    subtype: '',
    host: '',
    port: 5432,
    database: '',
    username: '',
    password: '',
    remark: '',
    connected: false,
  })
}

function generateId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8)
}

async function loadSources() {
  loading.value = true
  try {
    const offset = (page.value - 1) * pageSize.value
    const result = await invoke('get_data_sources', {
      keyword: searchText.value || null,
      offset,
      limit: pageSize.value,
    })
    dataSourceList.value = result.items || []
    total.value = result.total || 0
  } catch (err) {
    console.error('加载数据源失败:', err)
  } finally {
    loading.value = false
  }
}

async function handleSave() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return

  testing.value = true
  try {
    const source = { ...formData, id: formData.id || generateId() }
    const cmd = isEdit.value ? 'update_data_source' : 'add_data_source'
    const result = await invoke(cmd, { source })
    dialogVisible.value = false
    ElMessage.success(isEdit.value ? '数据源已更新' : '数据源已添加')
    await loadSources()
    await testConnection(result)
  } catch (err) {
    ElMessage.error('保存失败: ' + err)
  } finally {
    testing.value = false
  }
}

async function testConnection(row) {
  ElMessage.info(`正在测试连接: ${row.name}`)
  try {
    const connected = await invoke('test_connection', { source: row })
    row.connected = connected
    ElMessage[connected ? 'success' : 'error'](
      connected ? '连接成功' : '连接失败，请检查配置'
    )
    await invoke('update_data_source', { source: row })
  } catch (err) {
    row.connected = false
    ElMessage.error('连接测试失败: ' + err)
  }
}

function removeDataSource(row) {
  ElMessageBox.confirm(`确定删除数据源 "${row.name}" 吗？`, '确认删除', {
    type: 'warning',
  }).then(async () => {
    try {
      await invoke('delete_data_source', { id: row.id })
      ElMessage.success('已删除')
      loadSources()
    } catch (err) {
      ElMessage.error('删除失败: ' + err)
    }
  }).catch(() => {})
}
</script>

<style scoped>
.datasources {
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
  align-items: center;
}

.sub-type {
  font-size: 13px;
  color: #606266;
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
    gap: 8px;
  }
  .datasources {
    max-width: 100%;
  }
}

@media (max-width: 480px) {
  .page-header h2 {
    font-size: 16px;
  }
  :deep(.el-pagination) {
    flex-wrap: wrap;
    gap: 4px;
  }
}
</style>

<template>
  <div class="flex flex-col h-full min-h-0">
    <div class="flex items-center justify-between mb-3 flex-wrap gap-2 shrink-0">
      <h2 class="text-lg font-semibold">数据源管理</h2>
      <div class="flex gap-2 items-center">
        <el-input v-model="searchText" placeholder="搜索名称、地址" clearable style="width: 220px"
                  @clear="page = 1" @keyup.enter="page = 1" />
        <button class="btn btn-ghost btn-sm" @click="loadSources">
          <Icon icon="mdi:refresh" width="18" />刷新
        </button>
        <button class="btn btn-primary btn-sm" @click="showAddDialog">
          <Icon icon="mdi:plus" width="18" />添加数据源
        </button>
      </div>
    </div>

    <div class="flex-1 min-h-0">
      <AppTable :data="dataSourceList" :columns="sourceColumns" :loading="loading" :show-pagination="false" />
    </div>

    <!-- 添加/编辑对话框 -->
    <AppModal v-model="dialogVisible" :title="isEdit ? '编辑数据源' : '添加数据源'" @closed="resetForm">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="80px" label-position="top">
        <el-form-item label="数据源名称" prop="name">
          <el-input v-model="formData.name" placeholder="输入数据源名称" />
        </el-form-item>
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
          <div class="grid grid-cols-[2fr_1fr] gap-3">
            <el-form-item label="主机地址" prop="host">
              <el-input v-model="formData.host" placeholder="localhost" />
            </el-form-item>
            <el-form-item label="端口" prop="port">
              <el-input-number v-model="formData.port" :min="1" :max="65535" style="width: 100%" />
            </el-form-item>
          </div>
          <el-form-item label="数据库名" prop="database">
            <el-input v-model="formData.database" placeholder="数据库名称" />
          </el-form-item>
          <div class="grid grid-cols-2 gap-3">
            <el-form-item label="用户名" prop="username">
              <el-input v-model="formData.username" placeholder="用户名" />
            </el-form-item>
            <el-form-item label="密码" prop="password">
              <el-input v-model="formData.password" type="password" show-password placeholder="密码" />
            </el-form-item>
          </div>
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
          <div class="grid grid-cols-[2fr_1fr] gap-3">
            <el-form-item label="Endpoint" prop="host">
              <el-input v-model="formData.host" placeholder="oss-cn-hangzhou.aliyuncs.com" />
            </el-form-item>
            <el-form-item label="端口" prop="port">
              <el-input-number v-model="formData.port" :min="1" :max="65535" style="width: 100%" />
            </el-form-item>
          </div>
          <el-form-item label="Bucket" prop="database">
            <el-input v-model="formData.database" placeholder="bucket 名称（选填）" />
          </el-form-item>
          <div class="grid grid-cols-2 gap-3">
            <el-form-item label="AccessKey" prop="username">
              <el-input v-model="formData.username" placeholder="AccessKey ID" />
            </el-form-item>
            <el-form-item label="SecretKey" prop="password">
              <el-input v-model="formData.password" type="password" show-password placeholder="SecretKey" />
            </el-form-item>
          </div>
        </template>

        <el-form-item label="备注" prop="remark">
          <el-input v-model="formData.remark" type="textarea" :rows="2" placeholder="可选" />
        </el-form-item>
      </el-form>
      <template #footer>
        <button class="btn btn-ghost" @click="dialogVisible = false">取消</button>
        <button class="btn btn-primary" :disabled="testing" @click="handleSave">
          <span v-if="testing" class="loading loading-spinner loading-xs"></span>
          {{ isEdit ? '保存' : '添加并测试连接' }}
        </button>
      </template>
    </AppModal>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, watch, h } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Icon } from '@iconify/vue'
import AppTable from '@/components/AppTable.vue'
import AppModal from '@/components/AppModal.vue'
import { useToast } from '@/components/AppToast'
import { useConfirm } from '@/components/AppConfirm'

const toast = useToast()
const confirm = useConfirm()

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

const typeMap = { postgresql: 'PostgreSQL', mysql: 'MySQL', spatialite: 'SpatiaLite', aliyun: '阿里云 OSS', aws: 'AWS S3', minio: 'MinIO' }

const sourceColumns = [
  { accessorKey: 'name', header: '名称' },
  {
    accessorKey: 'type',
    header: '类型',
    cell: (info) => h('span', { class: `badge badge-sm ${info.getValue() === 'oss' ? 'badge-warning' : 'badge-info'}` }, typeMap[info.getValue()] || info.getValue()),
  },
  {
    accessorKey: 'subtype',
    header: '子类型',
    meta: { hideOnMobile: true },
    cell: (info) => h('span', { class: 'text-xs text-base-content/70' }, typeMap[info.getValue()] || info.getValue()),
  },
  { accessorKey: 'host', header: '地址', meta: { hideOnMobile: true } },
  {
    accessorKey: 'port',
    header: '端口',
    cell: (info) => info.getValue() || '-',
  },
  {
    accessorKey: 'connected',
    header: '状态',
    cell: (info) => h('span', { class: `badge badge-sm ${info.getValue() ? 'badge-success' : 'badge-ghost'}` }, info.getValue() ? '已连接' : '未连接'),
  },
  {
    id: 'actions',
    header: '操作',
    cell: (info) => h('div', { class: 'flex gap-1 flex-wrap' }, [
      h('button', { class: 'btn btn-xs btn-ghost text-primary', onClick: () => testConnection(info.row.original) }, '测试连接'),
      h('button', { class: 'btn btn-xs btn-ghost text-primary', onClick: () => showEditDialog(info.row.original) }, '编辑'),
      h('button', { class: 'btn btn-xs btn-ghost text-error', onClick: () => removeDataSource(info.row.original) }, '删除'),
    ]),
  },
]

const formData = reactive({ id: '', name: '', type: 'database', subtype: '', host: '', port: 5432, database: '', username: '', password: '', remark: '', connected: false })

watch(() => formData.type, (newType) => {
  if (newType === 'oss' && formData.port === 5432) {
    const defaults = { minio: 9000, aws: 443, aliyun: 443 }
    formData.port = defaults[formData.subtype] || 443
  } else if (newType === 'database' && formData.port !== 5432) {
    if ([9000].includes(formData.port)) formData.port = 5432
  }
})

const formRules = {
  name: [{ required: true, message: '请输入数据源名称', trigger: 'blur' }],
  subtype: [{ required: true, message: '请选择类型', trigger: 'change' }],
  host: [{ required: true, message: '请输入地址', trigger: 'blur' }],
}

function onOssSubtypeChange(subtype) {
  const defaults = { minio: 9000, aws: 443, aliyun: 443 }
  if (defaults[subtype] && formData.port === 5432) formData.port = defaults[subtype]
}

function generateId() { return Date.now().toString(36) + Math.random().toString(36).slice(2, 8) }

watch([searchText, pageSize], () => { page.value = 1; loadSources() })
watch(page, () => { loadSources() })

async function loadSources() {
  loading.value = true
  try {
    const offset = (page.value - 1) * pageSize.value
    const result = await invoke('get_data_sources', { keyword: searchText.value || null, offset, limit: pageSize.value })
    dataSourceList.value = result.items || []
    total.value = result.total || 0
  } catch (err) { console.error('加载数据源失败:', err) }
  finally { loading.value = false }
}

function showAddDialog() { isEdit.value = false; dialogVisible.value = true }
function showEditDialog(row) {
  isEdit.value = true
  Object.assign(formData, { id: row.id, name: row.name, type: row.type, subtype: row.subtype, host: row.host, port: row.port, database: row.database, username: row.username, password: row.password, remark: row.remark || '', connected: row.connected })
  dialogVisible.value = true
}
function resetForm() {
  formRef.value?.resetFields()
  Object.assign(formData, { id: '', name: '', type: 'database', subtype: '', host: '', port: 5432, database: '', username: '', password: '', remark: '', connected: false })
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
    toast.success(isEdit.value ? '数据源已更新' : '数据源已添加')
    await loadSources()
    await testConnection(result)
  } catch (err) { toast.error('保存失败: ' + err) }
  finally { testing.value = false }
}

async function testConnection(row) {
  toast.info(`正在测试连接: ${row.name}`)
  try {
    const connected = await invoke('test_connection', { source: row })
    row.connected = connected
    toast[connected ? 'success' : 'error'](connected ? '连接成功' : '连接失败，请检查配置')
    await invoke('update_data_source', { source: row })
  } catch (err) { row.connected = false; toast.error('连接测试失败: ' + err) }
}

async function removeDataSource(row) {
  const ok = await confirm('确认删除', `确定删除数据源 "${row.name}" 吗？`)
  if (!ok) return
  try {
    await invoke('delete_data_source', { id: row.id })
    toast.success('已删除')
    loadSources()
  } catch (err) { toast.error('删除失败: ' + err) }
}

onMounted(() => { loadSources() })
</script>

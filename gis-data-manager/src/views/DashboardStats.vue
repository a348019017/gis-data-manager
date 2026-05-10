<template>
  <div class="dashboard">
    <!-- 统计卡片 -->
    <div class="stats-row">
      <el-card class="stat-card">
        <div class="stat-content">
          <div class="stat-icon" style="background-color: #ecf5ff;">
            <el-icon :size="24" color="#409eff"><Connection /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-value">{{ totalSources }}</div>
            <div class="stat-label">数据源总数</div>
          </div>
        </div>
      </el-card>
      <el-card class="stat-card">
        <div class="stat-content">
          <div class="stat-icon" style="background-color: #f0f9eb;">
            <el-icon :size="24" color="#67c23a"><Check /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-value">{{ connectedSources }}</div>
            <div class="stat-label">已连接</div>
          </div>
        </div>
      </el-card>
      <el-card class="stat-card">
        <div class="stat-content">
          <div class="stat-icon" style="background-color: #fef0f0;">
            <el-icon :size="24" color="#f56c6c"><CircleClose /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-value">{{ disconnectedSources }}</div>
            <div class="stat-label">未连接</div>
          </div>
        </div>
      </el-card>
      <el-card class="stat-card">
        <div class="stat-content">
          <div class="stat-icon" style="background-color: #fdf6ec;">
            <el-icon :size="24" color="#e6a23c"><Link /></el-icon>
          </div>
          <div class="stat-info">
            <div class="stat-value">{{ totalServices }}</div>
            <div class="stat-label">服务总数</div>
          </div>
        </div>
      </el-card>
    </div>

    <!-- 快速导航 -->
    <el-card class="nav-menu" header="功能导航">
      <div class="menu-grid">
        <div class="menu-card" @click="$router.push('/datasources')">
          <div class="menu-icon" style="background-color: #ecf5ff;">
            <el-icon :size="28" color="#409eff"><Connection /></el-icon>
          </div>
          <div class="menu-label">数据源管理</div>
          <div class="menu-desc">管理数据库和OSS连接</div>
        </div>
        <div class="menu-card" @click="$router.push('/datamanagement')">
          <div class="menu-icon" style="background-color: #f0f9eb;">
            <el-icon :size="28" color="#67c23a"><FolderOpened /></el-icon>
          </div>
          <div class="menu-label">数据管理</div>
          <div class="menu-desc">导入文件到数据源</div>
        </div>
        <div class="menu-card" @click="$router.push('/serviceregistry')">
          <div class="menu-icon" style="background-color: #fdf6ec;">
            <el-icon :size="28" color="#e6a23c"><Link /></el-icon>
          </div>
          <div class="menu-label">服务注册</div>
          <div class="menu-desc">注册和预览GIS服务</div>
        </div>
        <div class="menu-card" @click="$router.push('/settings')">
          <div class="menu-icon" style="background-color: #f5f7fa;">
            <el-icon :size="28" color="#909399"><Setting /></el-icon>
          </div>
          <div class="menu-label">系统设置</div>
          <div class="menu-desc">AI模型和通用配置</div>
        </div>
      </div>
    </el-card>

    <!-- 快速操作 -->
    <el-card class="quick-actions" header="快速操作">
      <el-space :size="12" wrap>
        <el-button type="primary" :icon="Plus" @click="$router.push('/datasources')">
          添加数据源
        </el-button>
        <el-button :icon="Refresh" @click="refreshAll" :loading="refreshing">
          刷新全部连接
        </el-button>
      </el-space>
    </el-card>

    <!-- 最近数据源 -->
    <el-card class="recent-sources" header="最近数据源">
      <el-empty v-if="recentSources.length === 0" description="暂无数据源" />
      <el-table v-else :data="recentSources" size="small" style="width: 100%">
        <el-table-column prop="name" label="名称" />
        <el-table-column label="类型" width="120">
          <template #default="{ row }">
            <el-tag size="small" :type="row.type === 'oss' ? 'warning' : 'info'">
              {{ row.type === 'oss' ? 'OSS' : '数据库' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag size="small" :type="row.connected ? 'success' : 'danger'">
              {{ row.connected ? '已连接' : '未连接' }}
            </el-tag>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Plus, Refresh } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'

const allSources = ref([])
const allServices = ref([])
const refreshing = ref(false)

const totalSources = computed(() => allSources.value.length)
const connectedSources = computed(() => allSources.value.filter(s => s.connected).length)
const disconnectedSources = computed(() => allSources.value.filter(s => !s.connected).length)
const totalServices = computed(() => allServices.value.length)
const recentSources = computed(() => allSources.value.slice(-5).reverse())

async function loadSources() {
  try {
    allSources.value = await invoke('get_data_sources')
  } catch (err) {
    console.error('加载数据源失败:', err)
  }
}

async function loadServices() {
  try {
    allServices.value = await invoke('get_services')
  } catch (err) {
    console.error('加载服务列表失败:', err)
  }
}

async function refreshAll() {
  refreshing.value = true
  try {
    const sources = await invoke('get_data_sources')
    for (const source of sources) {
      try {
        const connected = await invoke('test_connection', { source })
        source.connected = connected
        if (connected) {
          await invoke('update_data_source', { source })
        }
      } catch (err) {
        source.connected = false
      }
    }
    allSources.value = sources
    ElMessage.success('刷新完成')
  } catch (err) {
    ElMessage.error('刷新失败: ' + err)
  } finally {
    refreshing.value = false
  }
}

onMounted(() => {
  loadSources()
  loadServices()
})
</script>

<style scoped>
.dashboard {
  width: 100%;
}

.stats-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
  margin-bottom: 12px;
}

@media (max-width: 900px) {
  .stats-row {
    grid-template-columns: repeat(2, 1fr);
  }
  .menu-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 600px) {
  .stats-row {
    grid-template-columns: 1fr;
  }
  .menu-grid {
    grid-template-columns: 1fr;
  }
}

.stat-card :deep(.el-card__body) {
  padding: 12px;
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}

.stat-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.stat-value {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  line-height: 1.2;
}

.stat-label {
  font-size: 13px;
  color: #909399;
  margin-top: 2px;
}

.quick-actions,
.recent-sources,
.nav-menu {
  margin-bottom: 12px;
}

.menu-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}

@media (max-width: 768px) {
  .stats-row {
    grid-template-columns: repeat(2, 1fr);
  }
  .menu-grid {
    grid-template-columns: repeat(2, 1fr);
  }
  .stat-value {
    font-size: 22px;
  }
  .stat-icon {
    width: 40px;
    height: 40px;
  }
}

@media (max-width: 480px) {
  .stats-row {
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .menu-grid {
    grid-template-columns: 1fr;
    gap: 12px;
  }
  .stat-value {
    font-size: 20px;
  }
  .stat-label {
    font-size: 12px;
  }
}

.menu-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 14px 12px;
  border-radius: 8px;
  border: 1px solid #ebeef5;
  cursor: pointer;
  transition: all 0.2s;
}

.menu-card:hover {
  border-color: #409eff;
  box-shadow: 0 2px 12px rgba(64, 158, 255, 0.15);
  transform: translateY(-2px);
}

.menu-icon {
  width: 52px;
  height: 52px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 10px;
}

.menu-label {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
}

.menu-desc {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}
</style>

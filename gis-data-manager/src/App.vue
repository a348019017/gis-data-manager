<template>
  <div class="app-layout">
    <!-- 左侧导航 -->
    <aside class="sidebar">
      <div class="sidebar-header">
        <el-icon :size="24"><DataLine /></el-icon>
        <span class="app-title">GIS Data</span>
      </div>
      <el-menu
        :default-active="activeMenu"
        class="sidebar-menu"
        @select="handleMenuSelect"
      >
        <el-menu-item index="/">
          <el-icon><ChatDotRound /></el-icon>
          <span>首页</span>
        </el-menu-item>
        <el-menu-item index="/dashboard">
          <el-icon><DataAnalysis /></el-icon>
          <span>概览</span>
        </el-menu-item>
        <el-menu-item index="/datasources">
          <el-icon><Connection /></el-icon>
          <span>数据源</span>
        </el-menu-item>
        <el-menu-item index="/datamanagement">
          <el-icon><FolderOpened /></el-icon>
          <span>数据管理</span>
        </el-menu-item>
        <el-menu-item index="/serviceregistry">
          <el-icon><Link /></el-icon>
          <span>服务注册</span>
        </el-menu-item>
        <el-menu-item index="/gistools">
          <el-icon><Tools /></el-icon>
          <span>GIS工具</span>
        </el-menu-item>
        <el-menu-item index="/settings">
          <el-icon><Setting /></el-icon>
          <span>设置</span>
        </el-menu-item>
      </el-menu>
    </aside>

    <!-- 主内容区 -->
    <main class="main-content">
      <router-view />
    </main>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

const route = useRoute()
const router = useRouter()

const activeMenu = computed(() => route.path)

function handleMenuSelect(index) {
  router.push(index)
}
</script>

<style>
/* 全局重置 */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body, #app {
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  background-color: #f5f7fa;
  color: #303133;
}

/* 整体布局 */
.app-layout {
  display: flex;
  height: 100vh;
}

/* 左侧导航 */
.sidebar {
  width: 200px;
  min-width: 200px;
  background-color: #ffffff;
  border-right: 1px solid #e4e7ed;
  display: flex;
  flex-direction: column;
  transition: width 0.3s, min-width 0.3s;
}

.sidebar.collapsed {
  width: 64px;
  min-width: 64px;
}

.sidebar-header {
  height: 56px;
  display: flex;
  align-items: center;
  padding: 0 16px;
  gap: 8px;
  border-bottom: 1px solid #e4e7ed;
  color: #409eff;
  font-weight: 600;
  font-size: 16px;
  overflow: hidden;
}

.sidebar.collapsed .sidebar-header .app-title {
  display: none;
}

.sidebar.collapsed .sidebar-menu .el-menu-item span {
  display: none;
}

.sidebar.collapsed .sidebar-menu .el-menu-item {
  justify-content: center;
}

/* 主内容区 */
.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
}

@media (max-width: 900px) {
  .sidebar {
    width: 64px;
    min-width: 64px;
  }
  .sidebar .app-title { display: none; }
  .sidebar .el-menu-item span { display: none; }
  .sidebar .el-menu-item { justify-content: center; }
}
</style>

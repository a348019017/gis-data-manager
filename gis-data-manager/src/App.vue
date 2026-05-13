<template>
  <div class="flex flex-col h-screen">
    <!-- 顶部 Header -->
    <header class="navbar bg-base-100 border-b border-base-300 px-4 h-14 shrink-0 relative">
      <!-- 移动端菜单按钮（绝对定位左侧） -->
      <button class="btn btn-ghost btn-square lg:hidden absolute left-2" @click="mobileMenuOpen = !mobileMenuOpen">
        <Icon :icon="mobileMenuOpen ? 'mdi:close' : 'mdi:menu'" width="24" />
      </button>
      <!-- 桌面端菜单按钮占位（对称） -->
      <div class="hidden lg:flex flex-1 items-center gap-2">
        <Icon icon="mdi:earth" width="28" class="text-primary" />
        <span class="text-lg font-bold">GIS 数据管理</span>
      </div>
      <!-- 标题（移动端居中） -->
      <div class="flex lg:hidden items-center gap-2 mx-auto">
        <Icon icon="mdi:earth" width="24" class="text-primary" />
        <span class="text-base font-bold">GIS 数据管理</span>
      </div>
      <!-- 桌面端右侧占位 -->
      <div class="hidden lg:flex flex-none" />
    </header>

    <!-- 下方：侧边栏 + 主内容 -->
    <div class="flex flex-1 overflow-hidden">
      <!-- 桌面端侧边栏 -->
      <aside class="hidden lg:flex w-52 shrink-0 bg-base-200 border-r border-base-300 overflow-y-auto">
        <ul class="menu p-4 gap-1 w-full">
          <li v-for="item in navItems" :key="item.path">
            <router-link :to="item.path" class="flex items-center gap-3"
                         :class="{ active: route.path === item.path }">
              <Icon :icon="item.icon" width="20" />
              <span>{{ item.label }}</span>
            </router-link>
          </li>
        </ul>
      </aside>

      <!-- 主内容区 -->
      <main class="flex-1 overflow-y-auto p-4">
        <router-view />
      </main>
    </div>

    <!-- 移动端遮罩 -->
    <div
      v-if="mobileMenuOpen"
      class="lg:hidden fixed inset-0 z-40 bg-black/50 transition-opacity"
      @click="mobileMenuOpen = false"
    />

    <!-- 移动端侧滑菜单 -->
    <aside
      class="lg:hidden fixed top-0 left-0 z-50 h-full w-full bg-base-200 flex flex-col transition-transform duration-250"
      :class="mobileMenuOpen ? 'translate-x-0' : '-translate-x-full'"
    >
      <div class="flex items-center justify-between px-4 py-3 border-b border-base-300">
        <div class="flex items-center gap-2">
          <Icon icon="mdi:earth" width="24" class="text-primary" />
          <span class="text-lg font-bold">GIS 数据管理</span>
        </div>
        <button class="btn btn-ghost btn-square" @click="mobileMenuOpen = false">
          <Icon icon="mdi:close" width="24" />
        </button>
      </div>
      <ul class="menu p-4 gap-2 flex-1 text-lg">
        <li v-for="item in navItems" :key="item.path">
          <router-link
            :to="item.path"
            class="flex items-center gap-4 py-2"
            :class="{ active: route.path === item.path }"
            @click="mobileMenuOpen = false"
          >
            <Icon :icon="item.icon" width="22" />
            <span>{{ item.label }}</span>
          </router-link>
        </li>
      </ul>
    </aside>

    <!-- Toast 容器 -->
    <div id="toast-container" class="toast toast-top toast-end z-[100]"></div>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'
import { Icon } from '@iconify/vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const mobileMenuOpen = ref(false)

watch(() => route.path, () => {
  mobileMenuOpen.value = false
})

const navItems = [
  { path: '/',              label: 'AI 助手',  icon: 'mdi:chat' },
  { path: '/dashboard',     label: '概览',      icon: 'mdi:view-dashboard' },
  { path: '/datasources',   label: '数据源',    icon: 'mdi:database' },
  { path: '/datamanagement', label: '数据管理',  icon: 'mdi:folder-open' },
  { path: '/serviceregistry', label: '服务注册', icon: 'mdi:link' },
  { path: '/gistools',      label: 'GIS 工具',  icon: 'mdi:tools' },
  { path: '/settings',      label: '设置',      icon: 'mdi:cog' },
]
</script>

<style scoped>
.transition-transform {
  transition: transform 0.25s ease;
}
</style>

import { createRouter, createWebHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    name: 'Home',
    component: () => import('@/views/HomeChat.vue'),
  },
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('@/views/DashboardStats.vue'),
  },
  {
    path: '/datasources',
    name: 'DataSources',
    component: () => import('@/views/DataSources.vue'),
  },
  {
    path: '/settings',
    name: 'Settings',
    component: () => import('@/views/Settings.vue'),
  },
  {
    path: '/datamanagement',
    name: 'DataManagement',
    component: () => import('@/views/DataManagement.vue'),
  },
  {
    path: '/serviceregistry',
    name: 'ServiceRegistry',
    component: () => import('@/views/ServiceRegistry.vue'),
  },
  {
    path: '/gistools',
    name: 'GISTools',
    component: () => import('@/views/GISTools.vue'),
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

export default router

<script setup>
import { useVueTable, getCoreRowModel, getPaginationRowModel, getSortedRowModel } from '@tanstack/vue-table'
import { watch, computed, ref, onMounted, onUnmounted, h } from 'vue'

const props = defineProps({
  data: { type: Array, required: true },
  columns: { type: Array, required: true },
  loading: Boolean,
  pageSize: { type: Number, default: 20 },
  total: Number,
  manualPagination: Boolean,
  showPagination: { type: Boolean, default: true },
  compact: { type: Boolean, default: true },
})

const emit = defineEmits(['page-change', 'page-size-change'])

const isMobile = ref(false)
const tableWrapper = ref(null)

function checkMobile() {
  isMobile.value = window.innerWidth < 640
}

onMounted(() => {
  checkMobile()
  window.addEventListener('resize', checkMobile)
})
onUnmounted(() => {
  window.removeEventListener('resize', checkMobile)
})

function safeVNode(val) {
  if (val === null || val === undefined) return h('span')
  if (typeof val === 'string' || typeof val === 'number' || typeof val === 'boolean') return h('span', String(val))
  return val
}

function cellClass(colDef) {
  const cls = []
  if (colDef.meta?.hideOnMobile) cls.push('hidden sm:table-cell')
  return cls.join(' ')
}

const table = useVueTable({
  get data() { return props.data },
  get columns() { return props.columns },
  getCoreRowModel: getCoreRowModel(),
  getPaginationRowModel: getPaginationRowModel(),
  getSortedRowModel: getSortedRowModel(),
  manualPagination: props.manualPagination,
  pageCount: props.total ? Math.ceil(props.total / props.pageSize) : undefined,
  initialState: { pagination: { pageSize: props.pageSize } },
})

watch(() => table.getState().pagination, (val) => {
  emit('page-change', { page: val.pageIndex + 1, pageSize: val.pageSize })
}, { deep: true })

const visiblePages = computed(() => {
  const total = table.getPageCount()
  const current = table.getState().pagination.pageIndex
  const maxVisible = 7
  if (total <= maxVisible) return Array.from({ length: total }, (_, i) => i)

  const pages = []
  const start = Math.max(0, Math.min(current - 3, total - maxVisible))
  const end = Math.min(total, start + maxVisible)
  for (let i = start; i < end; i++) pages.push(i)
  return pages
})
</script>

<template>
  <div>
    <div ref="tableWrapper" class="overflow-x-auto rounded-box scrollbar-thin">
      <table class="table table-sm" :class="{ 'table-xs': isMobile }">
        <thead>
          <tr v-for="headerGroup in table.getHeaderGroups()" :key="headerGroup.id">
            <th v-for="header in headerGroup.headers" :key="header.id"
                :class="[
                  header.column.getCanSort() ? 'cursor-pointer select-none' : '',
                  cellClass(header.column.columnDef),
                ]"
                @click="header.column.getToggleSortingHandler()?.($event)">
              <template v-if="!header.isPlaceholder">
                <span v-if="typeof header.column.columnDef.header === 'function'">
                  <component :is="safeVNode(header.column.columnDef.header(header.getContext()))" />
                </span>
                <span v-else>{{ header.column.columnDef.header }}</span>
                <span v-if="header.column.getIsSorted()" class="ml-1 text-xs opacity-60">
                  {{ header.column.getIsSorted() === 'asc' ? '↑' : '↓' }}
                </span>
              </template>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in table.getRowModel().rows" :key="row.id">
            <td v-for="cell in row.getVisibleCells()" :key="cell.id"
                :class="cellClass(cell.column.columnDef)">
              <template v-if="cell.column.columnDef.cell">
                <component :is="safeVNode(cell.column.columnDef.cell(cell.getContext()))" />
              </template>
              <template v-else>
                {{ cell.getValue() }}
              </template>
            </td>
          </tr>
          <tr v-if="data.length === 0 && !loading">
            <td :colspan="columns.length" class="text-center py-8 text-base-content/50">
              暂无数据
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 加载状态 -->
    <div v-if="loading" class="flex justify-center py-4">
      <span class="loading loading-spinner loading-md text-primary"></span>
    </div>

    <!-- 分页器 -->
    <div v-if="showPagination && data.length > 0" class="flex items-center justify-between mt-4 gap-2 flex-wrap">
      <select class="select select-sm select-bordered w-24 max-sm:w-20"
              :value="table.getState().pagination.pageSize"
              @change="table.setPageSize(Number($event.target.value)); $emit('page-size-change', Number($event.target.value))">
        <option :value="10">10 条</option>
        <option :value="20">20 条</option>
        <option :value="50">50 条</option>
        <option :value="100">100 条</option>
      </select>
      <div class="join">
        <button class="join-item btn btn-xs sm:btn-sm" :disabled="!table.getCanPreviousPage()"
                @click="table.previousPage()">«</button>
        <button v-for="page in visiblePages" :key="page"
                class="join-item btn btn-xs sm:btn-sm"
                :class="{ 'btn-active': page === table.getState().pagination.pageIndex }"
                @click="table.setPageIndex(page)">
          {{ page + 1 }}
        </button>
        <button class="join-item btn btn-xs sm:btn-sm" :disabled="!table.getCanNextPage()"
                @click="table.nextPage()">»</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.scrollbar-thin {
  scrollbar-width: thin;
  scrollbar-color: #c0c4cc transparent;
}
.scrollbar-thin::-webkit-scrollbar {
  height: 4px;
}
.scrollbar-thin::-webkit-scrollbar-thumb {
  border-radius: 2px;
  background: #c0c4cc;
}

/* 操作列右对齐 */
td:last-child,
th:last-child {
  text-align: right;
}
</style>

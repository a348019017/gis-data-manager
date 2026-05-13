<template>
  <div class="flex flex-col h-[calc(100vh-40px)] w-full max-w-4xl mx-auto bg-base-100 rounded-xl shadow-sm overflow-hidden">
    <!-- 聊天头部 -->
    <div class="flex items-center justify-between px-5 py-3 border-b border-base-300">
      <div class="flex items-center gap-2 font-semibold text-base">
        <Icon icon="mdi:chat" width="20" class="text-primary" />
        GIS 智能助手
      </div>
      <button class="btn btn-ghost btn-sm" @click="clearChat">
        <Icon icon="mdi:refresh" width="18" />新对话
      </button>
    </div>

    <!-- 消息列表 -->
    <div class="flex-1 overflow-y-auto px-4 py-3" ref="messagesRef">
      <div v-if="messages.length === 0" class="flex flex-col items-center justify-center py-8 text-center">
        <div class="w-16 h-16 rounded-2xl bg-primary/10 flex items-center justify-center mb-4">
          <Icon icon="mdi:earth" width="32" class="text-primary" />
        </div>
        <h2 class="text-xl font-semibold mb-2">你好，我是 GIS 数据助手</h2>
        <p class="text-base-content/50 text-sm mb-6">你可以问我关于数据管理、地图服务、GIS 数据处理等问题</p>
        <div class="flex flex-col sm:flex-row flex-wrap gap-2 justify-center">
          <button v-for="s in suggestions" :key="s" class="btn btn-sm btn-outline"
                  @click="sendSuggestion(s)">{{ s }}</button>
        </div>
      </div>

      <div v-for="(msg, i) in messages" :key="i" class="flex gap-2.5 mb-3.5"
           :class="{ 'flex-row-reverse': msg.role === 'user' }">
        <div class="w-8 h-8 rounded-lg bg-base-200 flex items-center justify-center shrink-0">
          <Icon v-if="msg.role === 'assistant'" icon="mdi:chat" width="20" class="text-primary" />
          <Icon v-else icon="mdi:account" width="20" class="text-base-content/50" />
        </div>
        <div class="max-w-[75%] min-w-0">
          <div class="px-3.5 py-2.5 rounded-xl text-sm leading-relaxed break-words"
               :class="msg.role === 'user'
                 ? 'bg-primary/10 rounded-tr-sm'
                 : 'bg-base-200 rounded-tl-sm'"
               v-html="formatText(msg.content)"></div>
          <div v-if="msg.error" class="mt-1.5 px-3 py-2 bg-error/10 text-error rounded-md text-xs">
            {{ msg.error }}
          </div>
        </div>
      </div>

      <div v-if="loading" class="flex gap-2.5 mb-3.5">
        <div class="w-8 h-8 rounded-lg bg-base-200 flex items-center justify-center shrink-0">
          <Icon icon="mdi:chat" width="20" class="text-primary" />
        </div>
        <div class="flex gap-1 px-3.5 py-3 bg-base-200 rounded-xl rounded-tl-sm">
          <span class="w-1.5 h-1.5 rounded-full bg-base-content/40 animate-bounce [animation-delay:0ms]"></span>
          <span class="w-1.5 h-1.5 rounded-full bg-base-content/40 animate-bounce [animation-delay:150ms]"></span>
          <span class="w-1.5 h-1.5 rounded-full bg-base-content/40 animate-bounce [animation-delay:300ms]"></span>
        </div>
      </div>
    </div>

    <!-- 输入区域 -->
    <div class="px-5 py-3 border-t border-base-300 bg-base-200/50 relative transition-colors"
         :class="{ 'bg-primary/5': isDragOver }"
         @dragover.prevent="isDragOver = true"
         @dragleave="isDragOver = false"
         @drop.prevent="onDrop">
      <!-- 拖放提示 -->
      <div v-if="isDragOver" class="absolute inset-0 flex flex-col items-center justify-center bg-base-100/92 border-2 border-dashed border-primary rounded-lg z-10 pointer-events-none">
        <Icon icon="mdi:upload" width="32" class="text-primary" />
        <p class="mt-2 text-sm font-medium text-primary">释放以添加文件</p>
      </div>

      <!-- 已添加的文件 -->
      <div v-if="attachedFiles.length > 0" class="flex flex-wrap gap-1.5 mb-2">
        <div v-for="(file, i) in attachedFiles" :key="i"
             class="badge badge-info badge-sm gap-1">
          <Icon icon="mdi:file-document-outline" width="12" />
          {{ file.name }}
          <button class="btn btn-ghost btn-xs p-0 min-h-0 h-auto" @click="removeFile(i)">✕</button>
        </div>
      </div>

      <div class="flex items-end gap-1 bg-base-100 border border-base-300 rounded-xl px-2 py-1.5 focus-within:border-primary transition-colors">
        <button class="btn btn-ghost btn-sm text-base-content/50 hover:text-primary shrink-0"
                @click="pickFile" title="选择文件">
          <Icon icon="mdi:paperclip" width="20" />
        </button>
        <textarea
          v-model="inputText"
          class="textarea textarea-ghost w-full text-sm py-0.5 px-1 min-h-[28px] max-h-[150px] resize-none border-none focus:outline-none focus:ring-0 bg-transparent placeholder:text-base-content/30"
          placeholder="输入你的问题，或直接拖拽文件到这里..."
          rows="1"
          @keydown.enter.exact.prevent="sendMessage"
          @input="autoResize"
          ref="textareaRef"
        ></textarea>
        <button
          class="btn btn-primary btn-circle btn-sm shrink-0"
          :disabled="!inputText.trim() && attachedFiles.length === 0"
          @click="sendMessage">
          <span v-if="loading" class="loading loading-spinner loading-xs"></span>
          <Icon v-else icon="mdi:send" width="18" />
        </button>
      </div>
      <div class="text-center text-xs text-base-content/30 mt-2">
        按 Enter 发送 · Shift+Enter 换行 · 拖拽或点击添加文件
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Icon } from '@iconify/vue'
import { useToast } from '@/components/AppToast'

const toast = useToast()

const messages = ref([])
const inputText = ref('')
const loading = ref(false)
const messagesRef = ref(null)
const textareaRef = ref(null)
const isDragOver = ref(false)
const attachedFiles = ref([])

const suggestions = [
  '如何导入 Shapefile 数据？',
  '帮我列出所有数据源',
  '如何注册 WMTS 服务？',
]

const ALLOWED_EXTENSIONS = [
  'shp', 'geojson', 'json', 'gpkg', 'kml', 'kmz',
  'pdf', 'doc', 'docx', 'xls', 'xlsx', 'txt', 'csv',
  'zip', 'png', 'jpg', 'jpeg', 'tif', 'tiff',
]

function autoResize() {
  nextTick(() => {
    const el = textareaRef.value
    if (el) {
      el.style.height = 'auto'
      el.style.height = Math.min(el.scrollHeight, 150) + 'px'
    }
  })
}

function scrollToBottom() {
  nextTick(() => {
    if (messagesRef.value) {
      messagesRef.value.scrollTop = messagesRef.value.scrollHeight
    }
  })
}

function formatText(text) {
  if (!text) return ''
  return text
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/\n/g, '<br>')
}

function sendSuggestion(text) {
  inputText.value = text
  sendMessage()
}

function clearChat() {
  messages.value = []
  attachedFiles.value = []
  toast.success('已开启新对话')
}

async function pickFile() {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const filePaths = await open({ multiple: true })
    if (filePaths && Array.isArray(filePaths)) {
      for (const path of filePaths) {
        const name = path.split('/').pop()?.split('\\').pop() || path
        const ext = name.split('.').pop().toLowerCase()
        if (ALLOWED_EXTENSIONS.includes(ext)) {
          attachedFiles.value.push({ name, path })
        } else {
          toast.warning(`不支持的文件类型: ${name}`)
        }
      }
    }
  } catch (err) {
    console.error('文件选择失败:', err)
  }
}

function onDrop(e) {
  isDragOver.value = false
  const files = e.dataTransfer?.files
  if (!files) return

  for (const file of files) {
    const name = file.name
    const ext = name.split('.').pop().toLowerCase()
    if (ALLOWED_EXTENSIONS.includes(ext)) {
      attachedFiles.value.push({
        name,
        path: file.webkitRelativePath || name,
        size: file.size,
        type: file.type,
      })
    } else {
      toast.warning(`不支持的文件类型: ${name}`)
    }
  }
}

function removeFile(index) {
  attachedFiles.value.splice(index, 1)
}

async function sendMessage() {
  const text = inputText.value.trim()
  const files = [...attachedFiles.value]
  if ((!text && files.length === 0) || loading.value) return

  const userMsg = { role: 'user', content: text, files: [] }
  if (files.length > 0) {
    userMsg.files = files.map(f => f.name)
    userMsg.content = text
      ? `${text}\n\n📎 附件: ${files.map(f => f.name).join(', ')}`
      : `📎 文件: ${files.map(f => f.name).join(', ')}\n\n请分析这些文件。`
  }

  messages.value.push(userMsg)
  inputText.value = ''
  attachedFiles.value = []
  loading.value = true
  nextTick(() => { if (textareaRef.value) textareaRef.value.style.height = 'auto' })
  scrollToBottom()

  try {
    const settings = await invoke('get_settings')
    if (!settings || !settings.api_key) {
      messages.value.push({
        role: 'assistant',
        content: '',
        error: '请先在「设置」页面中配置 AI 模型'
      })
      loading.value = false
      scrollToBottom()
      return
    }

    const response = await invoke('chat_message', {
      settings,
      message: userMsg.content,
      history: messages.value.filter(m => m.content).slice(-8).map(m => ({
        role: m.role,
        content: m.content
      }))
    })
    messages.value.push({ role: 'assistant', content: response })
  } catch (err) {
    messages.value.push({
      role: 'assistant',
      content: '',
      error: '请求失败: ' + err
    })
  } finally {
    loading.value = false
    scrollToBottom()
  }
}
</script>

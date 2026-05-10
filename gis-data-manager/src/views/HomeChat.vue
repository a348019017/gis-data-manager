<template>
  <div class="chat-home">
    <!-- 聊天头部 -->
    <div class="chat-header">
      <div class="header-info">
        <el-icon :size="20" color="#409eff"><ChatDotRound /></el-icon>
        <span>GIS 智能助手</span>
      </div>
      <el-button size="small" :icon="Refresh" @click="clearChat" text>
        新对话
      </el-button>
    </div>

    <!-- 消息列表 -->
    <div class="chat-messages" ref="messagesRef">
      <div v-if="messages.length === 0" class="welcome">
        <div class="welcome-icon">
          <el-icon :size="32"><DataLine /></el-icon>
        </div>
        <h2>你好，我是 GIS 数据助手</h2>
        <p>你可以问我关于数据管理、地图服务、GIS 数据处理等问题</p>
        <div class="suggestions">
          <el-button size="small" @click="sendSuggestion('如何导入 Shapefile 数据？')">
            如何导入 Shapefile 数据？
          </el-button>
          <el-button size="small" @click="sendSuggestion('帮我列出所有数据源')">
            列出所有数据源
          </el-button>
          <el-button size="small" @click="sendSuggestion('如何注册 WMTS 服务？')">
            如何注册 WMTS 服务？
          </el-button>
        </div>
      </div>

      <div v-for="(msg, i) in messages" :key="i" :class="['message', msg.role]">
        <div class="message-avatar">
          <el-icon :size="20" v-if="msg.role === 'assistant'" color="#409eff"><ChatDotRound /></el-icon>
          <el-icon :size="20" v-else color="#606266"><User /></el-icon>
        </div>
        <div class="message-content">
          <div class="message-text" v-html="formatText(msg.content)"></div>
          <div v-if="msg.error" class="message-error">{{ msg.error }}</div>
        </div>
      </div>

      <div v-if="loading" class="message assistant">
        <div class="message-avatar">
          <el-icon :size="20" color="#409eff"><ChatDotRound /></el-icon>
        </div>
        <div class="message-content">
          <div class="typing-indicator">
            <span></span><span></span><span></span>
          </div>
        </div>
      </div>
    </div>

    <!-- 输入区域 -->
    <div
      class="chat-input-area"
      :class="{ 'drag-over': isDragOver }"
      @dragover.prevent="isDragOver = true"
      @dragleave="isDragOver = false"
      @drop.prevent="onDrop"
    >
      <!-- 文件拖放提示 -->
      <div v-if="isDragOver" class="drop-overlay">
        <el-icon :size="32" color="#409eff"><UploadFilled /></el-icon>
        <p>释放以添加文件</p>
      </div>

      <!-- 已添加的文件标签 -->
      <div v-if="attachedFiles.length > 0" class="attached-files">
        <el-tag
          v-for="(file, i) in attachedFiles"
          :key="i"
          closable
          size="small"
          type="info"
          @close="removeFile(i)"
        >
          <el-icon :size="12"><Document /></el-icon>
          {{ file.name }}
        </el-tag>
      </div>

      <div class="input-wrapper">
        <el-button
          size="small"
          :icon="Paperclip"
          text
          @click="pickFile"
          class="attach-btn"
          title="选择文件"
        />
        <el-input
          v-model="inputText"
          type="textarea"
          :autosize="{ minRows: 1, maxRows: 6 }"
          placeholder="输入你的问题，或直接拖拽文件到这里..."
          @keydown.enter.exact.prevent="sendMessage"
          resize="none"
        />
        <el-button
          type="primary"
          :icon="Promotion"
          circle
          :loading="loading"
          :disabled="!inputText.trim() && attachedFiles.length === 0"
          @click="sendMessage"
        />
      </div>
      <div class="input-hint">
        按 Enter 发送 · Shift+Enter 换行 · 拖拽或点击添加文件
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { ChatDotRound, User, Promotion, Refresh, DataLine, UploadFilled, Document, Paperclip } from '@element-plus/icons-vue'

const messages = ref([])
const inputText = ref('')
const loading = ref(false)
const messagesRef = ref(null)
const isDragOver = ref(false)
const attachedFiles = ref([])

const ALLOWED_EXTENSIONS = [
  'shp', 'geojson', 'json', 'gpkg', 'kml', 'kmz',
  'pdf', 'doc', 'docx', 'xls', 'xlsx', 'txt', 'csv',
  'zip', 'png', 'jpg', 'jpeg', 'tif', 'tiff',
]

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
  ElMessage.success('已开启新对话')
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
          ElMessage.warning(`不支持的文件类型: ${name}`)
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
      // Tauri WebView2 中 file.path 可能不可用，使用文件名作为标识
      attachedFiles.value.push({
        name,
        path: file.webkitRelativePath || name,
        size: file.size,
        type: file.type,
      })
    } else {
      ElMessage.warning(`不支持的文件类型: ${name}`)
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

  // 构建用户消息
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

<style scoped>
.chat-home {
  display: flex;
  flex-direction: column;
  height: calc(100vh - 40px);
  width: 100%;
  max-width: 960px;
  margin: 0 auto;
  background: #ffffff;
  border-radius: 12px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  overflow: hidden;
}

.chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  border-bottom: 1px solid #ebeef5;
}

.header-info {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 15px;
  color: #303133;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
}

.welcome {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 30px 20px;
  text-align: center;
}

.welcome-icon {
  width: 64px;
  height: 64px;
  border-radius: 16px;
  background: #ecf5ff;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #409eff;
  margin-bottom: 16px;
}

.welcome h2 {
  font-size: 22px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 8px;
}

.welcome p {
  color: #909399;
  font-size: 14px;
  margin-bottom: 24px;
}

.suggestions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
}

.message {
  display: flex;
  gap: 10px;
  margin-bottom: 14px;
}

.message.user {
  flex-direction: row-reverse;
}

.message-avatar {
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f5f7fa;
}

.message-content {
  max-width: 75%;
  min-width: 0;
}

.message-text {
  padding: 10px 14px;
  border-radius: 12px;
  font-size: 14px;
  line-height: 1.6;
  word-break: break-word;
}

.message.user .message-text {
  background: #ecf5ff;
  color: #303133;
  border-top-right-radius: 4px;
}

.message.assistant .message-text {
  background: #f5f7fa;
  color: #303133;
  border-top-left-radius: 4px;
}

.message-error {
  margin-top: 6px;
  padding: 8px 12px;
  background: #fef0f0;
  color: #f56c6c;
  border-radius: 6px;
  font-size: 13px;
}

.typing-indicator {
  display: flex;
  gap: 4px;
  padding: 10px 14px;
  background: #f5f7fa;
  border-radius: 12px;
  border-top-left-radius: 4px;
}

.typing-indicator span {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #909399;
  animation: typing 1.2s ease-in-out infinite;
}

.typing-indicator span:nth-child(2) { animation-delay: 0.2s }
.typing-indicator span:nth-child(3) { animation-delay: 0.4s }

@keyframes typing {
  0%, 60%, 100% { opacity: 0.3; transform: translateY(0) }
  30% { opacity: 1; transform: translateY(-4px) }
}

.chat-input-area {
  padding: 12px 20px;
  border-top: 1px solid #ebeef5;
  background: #fafafa;
  position: relative;
  transition: background-color 0.2s;
}

.chat-input-area.drag-over {
  background: #ecf5ff;
}

.drop-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.92);
  border: 2px dashed #409eff;
  border-radius: 8px;
  z-index: 10;
  pointer-events: none;
  animation: fadeIn 0.15s ease;
}

.drop-overlay p {
  margin-top: 8px;
  font-size: 14px;
  font-weight: 500;
  color: #409eff;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.attached-files {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 8px;
  padding: 0 2px;
}

.attached-files .el-tag {
  cursor: default;
}

.input-wrapper {
  display: flex;
  align-items: flex-end;
  gap: 4px;
  background: #ffffff;
  border: 1px solid #dcdfe6;
  border-radius: 12px;
  padding: 6px 8px;
  transition: border-color 0.2s;
}

.input-wrapper:focus-within {
  border-color: #409eff;
}

.attach-btn {
  flex-shrink: 0;
  color: #909399;
  padding: 4px;
  min-width: unset;
}

.attach-btn:hover {
  color: #409eff;
}

.input-wrapper :deep(.el-textarea__inner) {
  border: none;
  box-shadow: none;
  padding: 2px 4px;
  font-size: 14px;
  resize: none;
}

.input-wrapper .el-button {
  flex-shrink: 0;
}

.input-hint {
  margin-top: 8px;
  text-align: center;
  font-size: 12px;
  color: #c0c4cc;
}

@media (max-width: 768px) {
  .chat-home {
    max-width: 100%;
    border-radius: 8px;
  }
  .welcome {
    padding: 40px 16px;
  }
  .welcome h2 {
    font-size: 18px;
  }
  .welcome p {
    font-size: 13px;
  }
  .suggestions {
    flex-direction: column;
    width: 100%;
  }
  .suggestions .el-button {
    width: 100%;
  }
  .message-content {
    max-width: 85%;
  }
  .chat-messages {
    padding: 12px;
  }
  .chat-input-area {
    padding: 8px 12px;
  }
}

@media (max-width: 480px) {
  .header-info span {
    font-size: 13px;
  }
  .welcome-icon {
    width: 48px;
    height: 48px;
  }
  .input-hint {
    font-size: 11px;
  }
}
</style>

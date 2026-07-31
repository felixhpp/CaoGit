<template>
  <Teleport to="body">
    <div v-if="show" class="modal-overlay" @click.self="close">
      <div class="modal-container">
        <div class="modal-header">
          <h2>发布管理</h2>
          <button class="close-btn" @click="close">×</button>
        </div>

        <div class="modal-body">
          <!-- Loading State -->
          <div v-if="loading" class="loading">
            <div class="spinner"></div>
            <p>加载中...</p>
          </div>

          <!-- Error State -->
          <div v-else-if="error" class="error-box">
            <p>{{ error }}</p>
            <button @click="loadReleaseInfo">重试</button>
          </div>

          <!-- Main Content -->
          <div v-else-if="releaseInfo" class="content">
            <!-- Publish Section -->
            <div class="section">
              <h3>发布新版本</h3>
              <div class="publish-form">
                <div class="form-group">
                  <label>当前版本:</label>
                  <span class="current-version">{{ releaseInfo.current_version }}</span>
                </div>

                <div class="form-group">
                  <label>新版本号:</label>
                  <div class="version-input">
                    <input v-model="newVersion" type="text" placeholder="v0.2.2" />
                    <button @click="incrementPatch">+补丁</button>
                    <button @click="incrementMinor">+次版本</button>
                    <button @click="incrementMajor">+主版本</button>
                  </div>
                </div>

                <div class="form-group">
                  <label>发布说明:</label>
                  <textarea v-model="releaseMessage" rows="6" placeholder="发布说明..."></textarea>
                  <button class="generate-btn" :disabled="generating" @click="generateNotes">
                    <svg v-if="!generating" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M12 2v20M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"></path>
                    </svg>
                    <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="spinning">
                      <polyline points="23 4 23 10 17 10"></polyline>
                      <polyline points="1 20 1 14 7 14"></polyline>
                      <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
                    </svg>
                    {{ generating ? '生成中...' : '自动生成' }}
                  </button>
                </div>

                <button class="publish-btn" :disabled="publishing" @click="publishRelease">
                  <span v-if="publishing">发布中...</span>
                  <span v-else>发布到 GitHub</span>
                </button>
              </div>
            </div>

            <!-- Releases List -->
            <div class="section">
              <h3>最近发布 ({{ releaseInfo.releases.length }})</h3>
              <div class="releases-list">
                <div v-for="release in releaseInfo.releases.slice(0, 5)" :key="release.id" class="release-item">
                  <div class="release-header">
                    <span class="release-tag">{{ release.tag_name }}</span>
                    <span class="release-date">{{ formatDate(release.created_at) }}</span>
                  </div>
                  <div class="release-body">
                    <p>{{ release.name }}</p>
                    <div class="release-assets">
                      <span>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                          <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path>
                        </svg>
                        {{ release.assets.length }} 个文件
                      </span>
                      <span>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                          <polyline points="7 10 12 15 17 10"></polyline>
                          <line x1="12" y1="15" x2="12" y2="3"></line>
                        </svg>
                        {{ totalDownloads(release.assets) }} 次下载
                      </span>
                    </div>
                  </div>
                  <a :href="release.html_url" target="_blank" class="view-link">查看详情 →</a>
                </div>
              </div>
            </div>

            <!-- Workflow Runs -->
            <div class="section">
              <div class="section-header">
                <h3>构建状态</h3>
                <button class="refresh-btn" @click="loadReleaseInfo" :disabled="loading" title="刷新构建状态">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" :class="{ spinning: loading }">
                    <polyline points="23 4 23 10 17 10"></polyline>
                    <polyline points="1 20 1 14 7 14"></polyline>
                    <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
                  </svg>
                </button>
              </div>
              <div class="workflows-list">
                <div v-for="run in releaseInfo.workflow_runs.slice(0, 5)" :key="run.id" class="workflow-item">
                  <div class="workflow-status" :class="getStatusClass(run.status, run.conclusion)">
                    <!-- Success -->
                    <svg v-if="run.conclusion === 'success'" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
                      <polyline points="22 4 12 14.01 9 11.01"></polyline>
                    </svg>
                    <!-- Failure -->
                    <svg v-else-if="run.conclusion === 'failure'" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <circle cx="12" cy="12" r="10"></circle>
                      <line x1="15" y1="9" x2="9" y2="15"></line>
                      <line x1="9" y1="9" x2="15" y2="15"></line>
                    </svg>
                    <!-- Running -->
                    <svg v-else-if="run.status === 'in_progress'" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="spinning">
                      <circle cx="12" cy="12" r="10"></circle>
                      <path d="M12 6v6l4 2"></path>
                    </svg>
                    <!-- Pending/Other -->
                    <svg v-else width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <circle cx="12" cy="12" r="10"></circle>
                      <line x1="12" y1="8" x2="12" y2="12"></line>
                      <line x1="12" y1="16" x2="12.01" y2="16"></line>
                    </svg>
                  </div>
                  <div class="workflow-info">
                    <div class="workflow-name">{{ run.name }}</div>
                    <div class="workflow-meta">
                      <span>{{ formatDate(run.created_at) }}</span>
                      <span v-if="run.conclusion">{{ run.conclusion }}</span>
                    </div>
                  </div>
                  <div class="workflow-actions">
                    <a :href="run.html_url" target="_blank" class="view-link-small">查看</a>
                    <button v-if="run.conclusion === 'failure'" @click="rerunWorkflow(run.id)" class="rerun-btn">
                      重新运行
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
  show: boolean
  repoPath: string | null
  githubToken: string | null
}>()

const emit = defineEmits<{
  close: []
  success: [message: string]
}>()

const loading = ref(false)
const error = ref<string | null>(null)
const releaseInfo = ref<any>(null)
const newVersion = ref('')
const releaseMessage = ref('')
const publishing = ref(false)
const generating = ref(false)

// Watch for modal open
watch(() => props.show, (show) => {
  if (show && props.repoPath) {
    loadReleaseInfo()
  }
})

async function loadReleaseInfo() {
  if (!props.repoPath) return

  loading.value = true
  error.value = null

  try {
    releaseInfo.value = await invoke('get_release_info', {
      repoPath: props.repoPath,
      githubToken: props.githubToken || undefined
    })

    // Set default new version
    newVersion.value = releaseInfo.value.current_version
  } catch (e: any) {
    error.value = e.toString()
  } finally {
    loading.value = false
  }
}

async function generateNotes() {
  if (!props.repoPath || !releaseInfo.value) return

  generating.value = true
  try {
    const notes = await invoke('generate_release_notes', {
      repoPath: props.repoPath,
      fromVersion: releaseInfo.value.current_version,
      toVersion: newVersion.value || 'HEAD'
    })
    releaseMessage.value = notes as string
  } catch (e) {
    console.error('生成发布说明失败:', e)
    releaseMessage.value = `生成失败: ${e}`
  } finally {
    generating.value = false
  }
}

async function publishRelease() {
  if (!props.repoPath || !newVersion.value) return

  // 清理发布说明内容，避免 Git tag 创建失败
  const cleanedMessage = cleanReleaseMessage(releaseMessage.value || `发布 ${newVersion.value}`)

  publishing.value = true
  try {
    await invoke('publish_new_release', {
      repoPath: props.repoPath,
      config: {
        version: newVersion.value,
        message: cleanedMessage,
        createTag: true,
        pushTag: true
      },
      githubToken: props.githubToken || undefined
    })

    emit('success', `发布成功！构建已触发`)

    // 清空输入框，为下次发布做准备
    releaseMessage.value = ''

    // Reload release info
    await loadReleaseInfo()
  } catch (e: any) {
    error.value = e.toString()
  } finally {
    publishing.value = false
  }
}

function cleanReleaseMessage(message: string): string {
  if (!message) return ''

  // 1. 移除 Markdown 标题符号（只移除行首的 # 号）
  let cleaned = message.replace(/^#+\s+/gm, '')

  // 2. 移除反引号（代码标记）
  cleaned = cleaned.replace(/`/g, '')

  // 3. 移除多余的空行（连续超过2个换行符的情况）
  cleaned = cleaned.replace(/\n{3,}/g, '\n\n')

  // 4. 去除首尾空白
  cleaned = cleaned.trim()

  // 5. 限制长度，避免 Git tag message 过长（建议不超过 5000 字符）
  if (cleaned.length > 5000) {
    cleaned = cleaned.substring(0, 5000) + '...'
  }

  return cleaned
}

async function incrementVersion(part: string) {
  try {
    newVersion.value = await invoke('increment_version', {
      version: newVersion.value || releaseInfo.value.current_version,
      part
    })
  } catch (e) {
    console.error('Failed to increment version:', e)
  }
}

function incrementPatch() { incrementVersion('patch') }
function incrementMinor() { incrementVersion('minor') }
function incrementMajor() { incrementVersion('major') }

async function rerunWorkflow(runId: number) {
  if (!props.githubToken) {
    error.value = '需要 GitHub Token 才能重新运行构建'
    return
  }

  try {
    await invoke('rerun_failed_build', {
      repoPath: props.repoPath,
      runId,
      githubToken: props.githubToken
    })

    emit('success', '重新触发构建成功')
    await loadReleaseInfo()
  } catch (e: any) {
    error.value = e.toString()
  }
}

function formatDate(dateStr: string) {
  const date = new Date(dateStr)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)

  if (diffMins < 60) return `${diffMins} 分钟前`
  if (diffMins < 1440) return `${Math.floor(diffMins / 60)} 小时前`
  return `${Math.floor(diffMins / 1440)} 天前`
}

function totalDownloads(assets: any[]) {
  return assets.reduce((sum, asset) => sum + asset.download_count, 0)
}

function getStatusClass(status: string, conclusion: string | null) {
  if (status === 'in_progress') return 'status-running'
  if (conclusion === 'success') return 'status-success'
  if (conclusion === 'failure') return 'status-failure'
  return 'status-pending'
}

function close() {
  emit('close')
}
</script>

<style scoped src="./ReleaseManagerModal.css"></style>

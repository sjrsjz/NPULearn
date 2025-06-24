<template>
  <div class="settings-container">
    <div class="settings-header">
      <h2>设置</h2>
      <button class="close-button" @click="$emit('close')">✕</button>
    </div>

    <!-- 加载状态 -->
    <div v-if="isLoading" class="loading-container">
      <div class="loading-spinner"></div>
      <p>正在加载设置...</p>
    </div>    <!-- 设置内容 -->
    <div v-else class="settings-content">
      <!-- 基础设置 -->
      <div class="setting-section">
        <h3>基础设置</h3>

        <div class="setting-item">
          <label>主题</label>
          <select v-model="settings.theme">
            <option v-for="option in themeOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>

        <div class="setting-item">
          <label>字体大小</label>
          <select v-model="settings.font_size">
            <option v-for="option in fontSizeOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>
      </div> <!-- 模型管理 -->
      <div class="setting-section">
        <h3>模型管理</h3>
        <p class="section-description">为每种AI服务选择要使用的模型</p>
        <div v-for="apiType in getAllApiKeyTypes()" :key="apiType" class="model-selection-item">          <div class="model-header">
            <h4>{{ getDisplayName(apiType) }}</h4>
            <span v-if="isCurrentModelReasoning(apiType)" class="reasoning-badge">推理模型</span>
            
            <!-- Gemini模型加载状态显示在标题右边 -->
            <div v-if="apiType === 'Gemini' && isLoadingGeminiModels" class="model-loading-inline">
              <div class="loading-spinner-small"></div>
              <span>获取模型列表中...</span>
            </div>
          </div>          <div class="model-selector">
            <select v-model="settings.model_selection[apiType]"
              @change="updateModelSelection(apiType, ($event.target as HTMLSelectElement).value)"
              :disabled="apiType === 'Gemini' && isLoadingGeminiModels">
              <option v-for="model in getAvailableModels(apiType)" :key="model.name" :value="model.name">
                {{ model.displayName }}
              </option>
            </select>

            <!-- 显示Gemini模型错误信息 -->
            <div v-if="apiType === 'Gemini' && geminiModelsError" class="model-error">
              {{ geminiModelsError }}
            </div>
          </div>

          <!-- 显示模型描述 - 独立行显示 -->
          <div v-if="getSelectedModelInfo(apiType)?.description" class="model-description">
            {{ getSelectedModelInfo(apiType)?.description }}
          </div>
        </div>
      </div> <!-- API密钥管理 -->
      <div class="setting-section">
        <h3>API 密钥管理</h3>

        <!-- API Keys 列表 -->
        <div class="api-keys-list">
          <div v-if="apiKeys.keys.length === 0" class="empty-state">
            暂无 API 密钥，点击下方按钮添加
          </div>

          <div v-else class="api-key-items">
            <div v-for="(key, index) in apiKeys.keys" :key="index" class="api-key-item">
              <div class="api-key-info">
                <div class="api-key-name">{{ key.name }}</div>
                <div class="api-key-type">{{ key.key_type }}</div>
                <div class="api-key-value">{{ key.key.substring(0, 4) + '••••••••' +
                  key.key.substring(key.key.length - 4) }}</div>
              </div>
              <!-- 在API密钥列表中修改删除按钮 -->
              <button class="delete-key-button" @click="confirmDeleteApiKey(key)">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
                  stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <!-- 垃圾桶盖子 -->
                  <path d="M3 6h18"></path>
                  <!-- 垃圾桶主体 -->
                  <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"></path>
                  <!-- 垃圾桶内部分割线 -->
                  <line x1="10" y1="11" x2="10" y2="17"></line>
                  <line x1="14" y1="11" x2="14" y2="17"></line>
                </svg>
              </button>

              <!-- 添加API密钥删除确认对话框 -->
              <div v-if="showConfirmDeleteKey" class="modal-overlay" @click.self="cancelDeleteApiKey">
                <div class="modal-content">
                  <div class="modal-header">
                    <h3>删除API密钥</h3>
                  </div>
                  <div class="modal-body">
                    <p>确定要删除 "{{ keyToDelete?.name }}" 吗？此操作不可撤销。</p>
                  </div>
                  <div class="modal-footer">
                    <button class="modal-button cancel" @click="cancelDeleteApiKey">取消</button>
                    <button class="modal-button delete" @click="submitDeleteApiKey">删除</button>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- 添加新 API Key 表单 -->
          <div v-if="isAddingKey" class="add-key-form">
            <div class="form-header">
              <h4>添加新密钥</h4>
              <button class="close-form" @click="isAddingKey = false">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
                  stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18"></line>
                  <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
              </button>
            </div>

            <div class="form-group">
              <label for="keyName">名称</label>
              <input type="text" id="keyName" v-model="newApiKey.name" placeholder="例如: 我的 Gemini API 密钥">
            </div>

            <div class="form-group">
              <label for="keyType">类型</label>
              <select id="keyType" v-model="newApiKey.key_type">
                <option v-for="type in apiKeyTypes" :key="type" :value="type">{{ type }}</option>
              </select>
            </div>

            <div class="form-group">
              <label for="apiKeyValue">密钥</label>
              <input type="text" id="apiKeyValue" v-model="newApiKey.key" placeholder="输入 API 密钥">
            </div>

            <div class="form-actions">
              <button class="cancel-button" @click="isAddingKey = false">取消</button>
              <button class="add-key-button" @click="addApiKey">添加</button>
            </div>
          </div>

          <!-- 添加 API Key 按钮 -->
          <button v-if="!isAddingKey" class="add-api-key" @click="isAddingKey = true">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="12" y1="5" x2="12" y2="19"></line>
              <line x1="5" y1="12" x2="19" y2="12"></line>
            </svg>
            添加 API 密钥
          </button>
        </div>
      </div>

      <!-- 模型配置 -->
      <div class="setting-section">
        <h3>模型配置</h3>

        <div class="setting-item">
          <label>温度参数 ({{ settings.model_config.temperature }})</label>
          <input type="range" min="0.1" max="1.0" step="0.1" v-model.number="settings.model_config.temperature">
          <div class="range-labels">
            <span>保守</span>
            <span>创意</span>
          </div>
        </div>

        <div class="setting-item">
          <label>最大令牌数</label>
          <input type="number" min="100" max="8192" v-model.number="settings.model_config.max_tokens">
        </div>
      </div>
    </div>

    <!-- 底部操作按钮 -->
    <div class="settings-footer">
      <button @click="resetSettings" class="reset-btn">重置设置</button>
      <div class="footer-actions">
        <button @click="handleCancel" class="cancel-btn">取消</button>
        <button @click="handleSaveAndClose" class="save-btn">保存</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, watch, ref } from 'vue';
import { useSettingsProvider, ApiKeyType, type ModelInfo } from '../composables/useSettings';
import { applyTheme, applyFontSize } from '../themeUtils';
import { AppEvents } from '../App/eventBus';

const emit = defineEmits(['close']);

// 添加加载状态
const isLoading = ref(true);

// 使用 settings composable
const {
  settings,
  themeOptions,
  fontSizeOptions,
  apiKeys,
  newApiKey,
  isAddingKey,
  isLoadingGeminiModels,
  geminiModelsError,
  loadApiKeys,
  addApiKey,
  deleteApiKey,
  saveSettings,
  resetSettings,
  cancelSettings,
  backupCurrentSettings,
  getConfigurableApiKeyTypes,
  getAllApiKeyTypes,
  isCurrentModelReasoning,
  updateModelSelection,
  getAvailableModels,
  getDisplayName,
  initAppSettings,
  fetchGeminiModels
} = useSettingsProvider();

// 本地通知函数，使用事件总线
const showNotification = (message: string, type: 'success' | 'error' | 'info' = 'success') => {
  AppEvents.showNotification(message, type);
};

// API 密钥类型选项
const apiKeyTypes = Object.values(ApiKeyType);

// 获取指定类型的密钥
function getKeysForType(keyType: ApiKeyType) {
  return apiKeys.value.filterByType(keyType).keys;
}

// 掩码显示API密钥
function maskApiKey(key: string): string {
  if (key.length <= 8) return '****';
  return key.substring(0, 4) + '****' + key.substring(key.length - 4);
}

// 开始添加密钥
function startAddingKey(keyType: ApiKeyType) {
  newApiKey.key_type = keyType;
  newApiKey.key = '';
  newApiKey.name = '';
  isAddingKey.value = true;
}

// 取消添加密钥
function cancelAddingKey() {
  isAddingKey.value = false;
  newApiKey.key = '';
  newApiKey.name = '';
}

// 添加用于API密钥删除确认的状态
const showConfirmDeleteKey = ref(false);
const keyToDelete = ref<any>(null);

// 显示删除确认对话框
function confirmDeleteApiKey(key: any) {
  keyToDelete.value = key;
  showConfirmDeleteKey.value = true;
}

// 执行真正的删除操作
async function submitDeleteApiKey() {
  if (!keyToDelete.value) {
    return;
  }

  try {
    await deleteApiKey(keyToDelete.value);
    showNotification('API 密钥已删除', 'info');
  } catch (error) {
    console.error('删除 API 密钥失败:', error);
    showNotification('删除 API 密钥失败', 'error');
  } finally {
    // 关闭对话框并清除状态
    showConfirmDeleteKey.value = false;
    keyToDelete.value = null;
  }
}

// 取消删除操作
function cancelDeleteApiKey() {
  showConfirmDeleteKey.value = false;
  keyToDelete.value = null;
}

// 获取选中模型的信息
function getSelectedModelInfo(apiType: ApiKeyType): ModelInfo | undefined {
  const selectedModel = settings.value.model_selection[apiType];
  const models = getAvailableModels(apiType);
  return models.find((m: ModelInfo) => m.name === selectedModel);
}

// 处理取消操作
function handleCancel() {
  // 调用原有的取消设置功能（恢复主题和字体）
  cancelSettings();

  // 刷新全局样式以确保界面状态正确
  setTimeout(() => {
    applyTheme(settings.value.theme as 'system' | 'light' | 'dark');
    applyFontSize(settings.value.font_size as 'small' | 'medium' | 'large');
  }, 100);

  // 关闭设置界面
  emit('close');
}

// 处理保存并关闭
async function handleSaveAndClose() {
  try {
    await saveSettings();
    
    showNotification('设置已保存', 'success');

    // 确保样式已经应用
    setTimeout(() => {
      applyTheme(settings.value.theme as 'system' | 'light' | 'dark');
      applyFontSize(settings.value.font_size as 'small' | 'medium' | 'large');
    }, 100);

    emit('close');
  } catch (error) {
    console.error('保存设置失败:', error);
    showNotification('保存设置失败', 'error');
  }
}

// 初始化
onMounted(async () => {
  try {
    console.log('设置界面开始初始化...');

    // 首先确保应用设置已经初始化（这会从后端加载设置）
    await initAppSettings();
    console.log('应用设置初始化完成，当前模型选择:', settings.value.model_selection);

    // 备份当前设置状态，以便取消时恢复
    backupCurrentSettings();

    // 加载API密钥
    await loadApiKeys();
    console.log('API密钥加载完成');

    // 检查是否有Gemini密钥，如果有则自动获取最新模型列表
    const geminiKeys = apiKeys.value.filterByType(ApiKeyType.Gemini);
    if (geminiKeys.keys.length > 0) {
      console.log('检测到Gemini API密钥，自动获取最新模型列表...');
      fetchGeminiModels().catch(error => {
        console.error('自动获取Gemini模型失败:', error);
      });
    } else {
      console.log('未检测到Gemini API密钥，跳过模型列表获取');
    }

    // 设置加载完成，可以显示界面
    isLoading.value = false;
    console.log('设置界面初始化完成');

  } catch (error) {
    console.error('设置界面初始化失败:', error);
    isLoading.value = false; // 即使失败也要显示界面
  }

  // 监听模型选择的变化
  watch(() => settings.value.model_selection, (newSelection: any) => {
    console.log('设置界面检测到模型选择变化:', newSelection);
  }, { deep: true });
});
</script>

<style scoped>
.settings-container {
  background: var(--card-bg);
  border-radius: 12px;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
  min-height: 0;
  /* 确保flex子元素可以缩小 */
}

.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid var(--border-color);
}

.settings-header h2 {
  margin: 0;
  color: var(--text-color);
}

.close-button {
  background: none;
  border: none;
  font-size: 20px;
  cursor: pointer;
  color: var(--text-color);
  padding: 4px;
}

.settings-content {
  padding: 24px;
  overflow-y: auto;
  flex: 1;
  min-height: 0;
  /* 允许内容区域缩小并显示滚动条 */
}

.loading-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  padding: 60px 24px;
}

.loading-spinner {
  width: 40px;
  height: 40px;
  border: 4px solid var(--border-color);
  border-top: 4px solid var(--primary-color);
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 16px;
}

@keyframes spin {
  0% {
    transform: rotate(0deg);
  }

  100% {
    transform: rotate(360deg);
  }
}

.loading-container p {
  color: var(--text-secondary);
  margin: 0;
}

.setting-section {
  margin-bottom: 32px;
}

.setting-section h3 {
  margin: 0 0 16px 0;
  color: var(--text-color);
  font-size: 18px;
}

.section-description {
  color: var(--text-secondary);
  font-size: 14px;
  margin-bottom: 16px;
}

.setting-item {
  margin-bottom: 16px;
}

.setting-item label {
  display: block;
  margin-bottom: 6px;
  color: var(--text-color);
  font-weight: 500;
}

.setting-item input,
.setting-item select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--card-bg);
  color: var(--text-color);
  font-size: 14px;
  box-sizing: border-box;
}

.setting-item select {
  -webkit-appearance: none;
  -moz-appearance: none;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='%236b7280' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  background-size: 16px;
  padding-right: 32px;
}


.setting-item input:focus,
.setting-item select:focus {
  border-color: var(--primary-color);
  outline: none;
  box-shadow: 0 0 0 2px rgba(79, 70, 229, 0.1);
}

.path-input-group {
  display: flex;
  gap: 8px;
}

.path-input-group input {
  flex: 1;
}

.path-input-group button {
  padding: 8px 16px;
  background: var(--primary-color);
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
}

.model-selection-item {
  margin-bottom: 20px;
  padding: 16px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
}

.model-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.model-header h4 {
  margin: 0;
  color: var(--text-color);
}

.model-loading-inline {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
  font-size: 12px;
  color: var(--primary-color);
  background: rgba(79, 70, 229, 0.1);
  padding: 4px 8px;
  border-radius: 12px;
}

.loading-spinner-small {
  width: 12px;
  height: 12px;
  border: 2px solid var(--primary-color);
  border-top: 2px solid transparent;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  flex-shrink: 0;
}

.refresh-models-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  color: var(--text-color);
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s ease;
  margin-left: auto;
}

.refresh-models-btn:hover:not(:disabled) {
  background: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.refresh-models-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.refresh-models-btn .loading-icon {
  animation: spin 1s linear infinite;
}

.reasoning-badge {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
}

.model-selector select {
  width: 100%;
  margin-bottom: 8px;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--card-bg);
  color: var(--text-color);
  font-size: 14px;
  box-sizing: border-box;
  -webkit-appearance: none;
  -moz-appearance: none;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='%236b7280' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  background-size: 16px;
  padding-right: 32px;
}

.model-selector select:focus {
  border-color: var(--primary-color);
  outline: none;
  box-shadow: 0 0 0 2px rgba(79, 70, 229, 0.1);
}

.model-selector select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.model-loading {
  font-size: 14px;
  color: var(--primary-color);
  padding: 8px 12px;
  background: rgba(79, 70, 229, 0.1);
  border-radius: 6px;
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.model-loading::before {
  content: '';
  width: 16px;
  height: 16px;
  min-width: 16px;
  min-height: 16px;
  border: 2px solid var(--primary-color);
  border-top: 2px solid transparent;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  flex-shrink: 0;
}

.model-error {
  font-size: 14px;
  color: #dc3545;
  padding: 8px 12px;
  background: rgba(220, 53, 69, 0.1);
  border-radius: 6px;
  margin-bottom: 8px;
  border-left: 3px solid #dc3545;
}

.model-description {
  font-size: 13px;
  color: var(--text-secondary);
  background: var(--background-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  margin-top: 12px;
  line-height: 1.5;
  position: relative;
  transition: all 0.2s ease;
}

.model-description::before {
  content: "💡";
  font-size: 12px;
  margin-right: 6px;
  opacity: 0.8;
}

.model-description:hover {
  background: var(--background-hover);
  border-color: var(--border-hover);
}

[data-theme="dark"] .model-description {
  background: rgba(30, 41, 59, 0.4);
  border-color: rgba(71, 85, 105, 0.5);
}

[data-theme="dark"] .model-description:hover {
  background: rgba(30, 41, 59, 0.6);
  border-color: rgba(71, 85, 105, 0.7);
}

.key-type-section {
  margin-bottom: 24px;
}

.key-type-section h4 {
  margin: 0 0 12px 0;
  color: var(--text-color);
}

/* API Key 管理样式 */
.api-keys-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.empty-state {
  padding: 20px;
  text-align: center;
  color: var(--text-secondary);
  border: 1px dashed var(--border-color);
  border-radius: 8px;
  margin-bottom: 16px;
}

.api-key-items {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.api-key-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background-color: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  transition: all 0.2s ease;
}

.api-key-item:hover {
  border-color: var(--primary-color);
}

.api-key-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.api-key-name {
  font-weight: 500;
  color: var(--text-color);
}

.api-key-type {
  font-size: 12px;
  color: var(--text-secondary);
}

.api-key-value {
  font-family: monospace;
  background-color: rgba(128, 128, 128, 0.1);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
  color: var(--text-secondary);
}

.delete-key-button {
  background: none;
  border: none;
  color: var(--text-color);
  opacity: 0.5;
  cursor: pointer;
  padding: 8px;
  border-radius: 4px;
  transition: all 0.2s ease;
}

.delete-key-button:hover {
  opacity: 1;
  color: #ef4444;
  background-color: rgba(239, 68, 68, 0.1);
}

.add-api-key {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 12px;
  border: 1px dashed var(--border-color);
  border-radius: 8px;
  background: none;
  color: var(--primary-color);
  cursor: pointer;
  width: 100%;
  transition: all 0.2s ease;
  font-weight: 500;
}

.add-api-key:hover {
  background-color: rgba(79, 70, 229, 0.05);
  border-color: var(--primary-color);
}

.add-key-form {
  padding: 16px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background-color: var(--card-bg);
  margin-bottom: 16px;
}

.form-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.form-header h4 {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
  color: var(--text-color);
}

.close-form {
  background: none;
  border: none;
  color: var(--text-color);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  border-radius: 4px;
  transition: all 0.2s ease;
}

.close-form:hover {
  background-color: rgba(128, 128, 128, 0.1);
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: block;
  font-weight: 500;
  margin-bottom: 6px;
  color: var(--text-color);
}

.form-group input,
.form-group select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--card-bg);
  color: var(--text-color);
  font-size: 14px;
  box-sizing: border-box;
}

.form-group input:focus,
.form-group select:focus {
  border-color: var(--primary-color);
  outline: none;
  box-shadow: 0 0 0 2px rgba(79, 70, 229, 0.1);
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 20px;
}

.cancel-button,
.add-key-button {
  padding: 8px 16px;
  border-radius: 6px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  font-size: 14px;
}

.cancel-button {
  background-color: transparent;
  border: 1px solid var(--border-color);
  color: var(--text-color);
}

.add-key-button {
  background-color: var(--primary-color);
  border: none;
  color: white;
}

.cancel-button:hover {
  background-color: rgba(128, 128, 128, 0.1);
  border-color: var(--text-color);
}

.add-key-button:hover {
  background-color: var(--primary-hover);
}

/* 模态框样式 */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background-color: var(--card-bg);
  border-radius: 8px;
  padding: 24px;
  max-width: 400px;
  width: 90%;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
  border: 1px solid var(--border-color);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.modal-header h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-color);
}

.modal-close {
  background: none;
  border: none;
  color: var(--text-color);
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  transition: all 0.2s ease;
}

.modal-close:hover {
  background-color: rgba(128, 128, 128, 0.1);
}

.modal-body {
  margin-bottom: 20px;
}

.modal-body p {
  margin: 0;
  color: var(--text-color);
  line-height: 1.5;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.modal-button {
  padding: 8px 16px;
  border-radius: 6px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  font-size: 14px;
}

.modal-button.cancel {
  background-color: transparent;
  border: 1px solid var(--border-color);
  color: var(--text-color);
}

.modal-button.delete {
  background-color: #ef4444;
  border: none;
  color: white;
}

.modal-button.cancel:hover {
  background-color: rgba(128, 128, 128, 0.1);
  border-color: var(--text-color);
}

.modal-button.delete:hover {
  background-color: #dc2626;
}

.range-labels {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 4px;
}

.settings-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-top: 1px solid var(--border-color);
}

.footer-actions {
  display: flex;
  gap: 12px;
}

.reset-btn,
.cancel-btn,
.save-btn,
.confirm-btn {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-weight: 500;
}

.reset-btn {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-color);
}

.cancel-btn {
  background: transparent;
  color: var(--text-color);
  border: 1px solid var(--border-color);
}

.save-btn,
.confirm-btn {
  background: var(--primary-color);
  color: white;
}

.reset-btn:hover,
.cancel-btn:hover {
  background: var(--hover-background);
}

.save-btn:hover,
.confirm-btn:hover {
  background: var(--primary-hover);
}
</style>

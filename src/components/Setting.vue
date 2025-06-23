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
    </div>

    <!-- 设置内容 -->
    <div v-else class="settings-content">
        <!-- 通知 -->
        <div v-if="notification.visible" :class="['notification', notification.type]">
          {{ notification.message }}
        </div>        <!-- 基础设置 -->
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
        </div>        <!-- 模型管理 -->
        <div class="setting-section">
          <h3>模型管理</h3>
          <p class="section-description">为每种AI服务选择要使用的模型</p>
            <div v-for="apiType in getAllApiKeyTypes()" :key="apiType" class="model-selection-item">            <div class="model-header">
              <h4>{{ getDisplayName(apiType) }}</h4>
              <span v-if="isCurrentModelReasoning(apiType)" class="reasoning-badge">推理模型</span>
            </div>
              <div class="model-selector">
              <select 
                v-model="settings.model_selection[apiType]" 
                @change="updateModelSelection(apiType, ($event.target as HTMLSelectElement).value)"
                :disabled="apiType === 'Gemini' && isLoadingGeminiModels"
              >
                <option v-for="model in getAvailableModels(apiType)" :key="model.name" :value="model.name">
                  {{ model.displayName }}
                </option>
              </select>
              
              <!-- 显示Gemini模型加载状态和错误信息 -->
              <div v-if="apiType === 'Gemini' && isLoadingGeminiModels" class="model-loading">
                正在获取最新的Gemini模型列表...
              </div>
              <div v-if="apiType === 'Gemini' && geminiModelsError" class="model-error">
                {{ geminiModelsError }}
              </div>
              
              <div v-if="getSelectedModelInfo(apiType)?.description" class="model-description">
                {{ getSelectedModelInfo(apiType)?.description }}
              </div>
            </div>
          </div>
        </div>

        <!-- API密钥管理 -->
        <div class="setting-section">
          <h3>API密钥管理</h3>
          <p class="section-description">配置各AI服务的API密钥（Coze使用内置密钥无需配置）</p>
          
          <!-- 现有密钥列表 -->
          <div class="api-keys-list">
            <div v-for="keyType in getConfigurableApiKeyTypes()" :key="keyType" class="key-type-section">
              <h4>{{ getDisplayName(keyType) }}</h4>
              
              <div v-if="getKeysForType(keyType).length === 0" class="no-keys">
                暂无配置的密钥
              </div>
              
              <div v-else class="keys-grid">
                <div v-for="key in getKeysForType(keyType)" :key="key.key" class="key-item">
                  <div class="key-info">
                    <div class="key-name">{{ key.name }}</div>
                    <div class="key-preview">{{ maskApiKey(key.key) }}</div>
                  </div>
                  <button @click="deleteApiKey(key)" class="delete-key-btn">删除</button>
                </div>
              </div>

              <button @click="startAddingKey(keyType)" class="add-key-btn">
                添加{{ getDisplayName(keyType) }}密钥
              </button>
            </div>
          </div>

          <!-- 添加新密钥表单 -->
          <div v-if="isAddingKey" class="add-key-form">
            <h4>添加新密钥</h4>
            <div class="form-group">
              <label>密钥类型</label>
              <select v-model="newApiKey.key_type" disabled>
                <option :value="newApiKey.key_type">{{ getDisplayName(newApiKey.key_type) }}</option>
              </select>
            </div>
            <div class="form-group">
              <label>密钥名称</label>
              <input type="text" v-model="newApiKey.name" placeholder="为这个密钥起个名字">
            </div>
            <div class="form-group">
              <label>API密钥</label>
              <input type="password" v-model="newApiKey.key" placeholder="输入API密钥">
            </div>
            <div class="form-actions">
              <button @click="cancelAddingKey" class="cancel-btn">取消</button>
              <button @click="addApiKey" class="confirm-btn">添加</button>
            </div>
          </div>
        </div>

        <!-- 模型配置 -->
        <div class="setting-section">
          <h3>模型配置</h3>
          
          <div class="setting-item">
            <label>温度参数 ({{ settings.model_config.temperature }})</label>
            <input 
              type="range" 
              min="0.1" 
              max="1.0" 
              step="0.1" 
              v-model.number="settings.model_config.temperature"
            >
            <div class="range-labels">
              <span>保守</span>
              <span>创意</span>
            </div>
          </div>

          <div class="setting-item">
            <label>最大令牌数</label>
            <input 
              type="number" 
              min="100" 
              max="8192" 
              v-model.number="settings.model_config.max_tokens"
            >
          </div>
        </div>
      </div>

      <!-- 底部操作按钮 -->
      <div class="settings-footer">
        <button @click="resetSettings" class="reset-btn">重置设置</button>
        <div class="footer-actions">
          <button @click="handleCancel" class="cancel-btn">取消</button>
          <button @click="saveSettings" class="save-btn">保存</button>
        </div>      </div>
    </div>
</template>

<script setup lang="ts">
import { onMounted, watch, ref } from 'vue';
import { useSettingsProvider, ApiKeyType, type ModelInfo } from '../composables/useSettings';

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
  notification,
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
  // 关闭设置界面
  emit('close');
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
  background: var(--background-color);
  border-radius: 12px;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
  min-height: 0; /* 确保flex子元素可以缩小 */
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
  min-height: 0; /* 允许内容区域缩小并显示滚动条 */
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
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.loading-container p {
  color: var(--text-secondary);
  margin: 0;
}

.notification {
  padding: 12px;
  border-radius: 6px;
  margin-bottom: 20px;
}

.notification.success {
  background-color: #d4edda;
  color: #155724;
  border: 1px solid #c3e6cb;
}

.notification.error {
  background-color: #f8d7da;
  color: #721c24;
  border: 1px solid #f5c6cb;
}

.notification.info {
  background-color: #d1ecf1;
  color: #0c5460;
  border: 1px solid #bee5eb;
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

.setting-item input, .setting-item select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--background-color);
  color: var(--text-color);
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
  border: 2px solid var(--primary-color);
  border-top: 2px solid transparent;
  border-radius: 50%;
  animation: spin 1s linear infinite;
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
  font-size: 14px;
  color: var(--text-secondary);
  font-style: italic;
}

.key-type-section {
  margin-bottom: 24px;
}

.key-type-section h4 {
  margin: 0 0 12px 0;
  color: var(--text-color);
}

.no-keys {
  color: var(--text-secondary);
  font-style: italic;
  margin-bottom: 12px;
}

.keys-grid {
  display: grid;
  gap: 8px;
  margin-bottom: 12px;
}

.key-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px;
  background: var(--card-background);
  border: 1px solid var(--border-color);
  border-radius: 6px;
}

.key-info {
  flex: 1;
}

.key-name {
  font-weight: 500;
  color: var(--text-color);
}

.key-preview {
  font-size: 12px;
  color: var(--text-secondary);
  font-family: monospace;
}

.delete-key-btn, .add-key-btn {
  padding: 6px 12px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.delete-key-btn {
  background: #dc3545;
  color: white;
}

.add-key-btn {
  background: var(--primary-color);
  color: white;
}

.add-key-form {
  padding: 16px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--card-background);
}

.add-key-form h4 {
  margin: 0 0 16px 0;
  color: var(--text-color);
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: block;
  margin-bottom: 6px;
  color: var(--text-color);
  font-weight: 500;
}

.form-group input, .form-group select {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--background-color);
  color: var(--text-color);
}

.form-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
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

.reset-btn, .cancel-btn, .save-btn, .confirm-btn {
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

.save-btn, .confirm-btn {
  background: var(--primary-color);
  color: white;
}

.reset-btn:hover, .cancel-btn:hover {
  background: var(--hover-background);
}

.save-btn:hover, .confirm-btn:hover {
  background: var(--primary-hover);
}
</style>

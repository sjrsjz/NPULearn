<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue';
import { invoke } from "@tauri-apps/api/core";
// 导入 Vue 的 watch 函数
import { watch } from 'vue';
import { applyFontSize, applyTheme } from '../themeUtils';

const emit = defineEmits(['close']);

// 设置选项
const settings = ref({
    theme: 'system', // 主题: system, light, dark
    font_size: 'medium', // 字体大小: small, medium, large
    auto_save: true, // 自动保存对话
    save_path: '', // 保存路径
    api_model: 'Gemini', // AI 模型
    model_config: {
        temperature: 0.7, // 温度参数 0.1-1.0
        max_tokens: 2048, // 最大生成令牌数
    },
});

// 主题选项
const themeOptions = [
    { value: 'system', label: '跟随系统' },
    { value: 'light', label: '浅色模式' },
    { value: 'dark', label: '深色模式' },
];

// 字体大小选项
const fontSizeOptions = [
    { value: 'small', label: '小' },
    { value: 'medium', label: '中' },
    { value: 'large', label: '大' },
];

const theme_before_save = ref('system');
const font_size_before_save = ref('medium');

// 定义 ApiKeyType 枚举
enum ApiKeyType {
    Gemini = "Gemini",
    DeepSeek = "DeepSeek",
    Coze = "Coze"
}

function get_display_name(key_type: ApiKeyType): string {
    switch (key_type) {
        case ApiKeyType.Gemini:
            return "Gemini";
        case ApiKeyType.DeepSeek:
            return "DeepSeek";
        case ApiKeyType.Coze:
            return "Coze";
        default:
            return "未知类型";
    }
}

// 模型选项
const modelOptions = ref<{ value: ApiKeyType; label: string; keyType: ApiKeyType }[]>([]);

function loadModelOptions() {
    // 根据ApiKeyType 枚举动态加载模型选项
    modelOptions.value = Object.values(ApiKeyType).map((keyType) => {
        return {
            value: keyType,
            label: get_display_name(keyType),
            keyType: keyType
        };
    });
}

// 定义 ApiKey 接口
interface ApiKey {
    key: string;
    name: string;
    key_type: ApiKeyType;
}


// 实现 APIKeyList 类
class APIKeyList {
    keys: ApiKey[];

    constructor() {
        this.keys = [];
    }

    // 添加 API 密钥
    addKey(key: ApiKey): void {
        this.keys.push(key);
    }

    // 移除 API 密钥
    removeKey(key: ApiKey): void {
        this.keys = this.keys.filter(k => k.key !== key.key);
    }

    // 根据类型过滤 API 密钥
    filterByType(keyType: ApiKeyType): APIKeyList {
        const result = new APIKeyList();
        result.keys = this.keys.filter(key => key.key_type === keyType);
        return result;
    }

    // 获取随机 API 密钥
    randomKey(): ApiKey | null {
        if (this.keys.length === 0) {
            return null;
        }
        const randomIndex = Math.floor(Math.random() * this.keys.length);
        return this.keys[randomIndex];
    }
}


// API Key 管理
const apiKeys = ref<APIKeyList>(new APIKeyList());
const newApiKey = reactive({
    key: '',
    name: '',
    key_type: ApiKeyType.Gemini
});
const isAddingKey = ref(false);
const apiKeyTypes = Object.values(ApiKeyType).filter(type => type !== ApiKeyType.Coze);
const apiKeyConfigFile = 'api_keys.json';




async function getApiKeyListOrCreate(config_name: string): Promise<APIKeyList> {
    try {
        const response = await invoke("get_api_key_list_or_create", { configName: config_name });
        // 将响应转换为 APIKeyList 类的实例
        const keyList = new APIKeyList();
        if (response && typeof response === 'object' && 'keys' in response) {
            const keys = (response as any).keys;
            if (Array.isArray(keys)) {
                keys.forEach((key: any) => {
                    if (key.key && key.name && key.key_type) {
                        keyList.addKey({
                            key: key.key,
                            name: key.name,
                            key_type: key.key_type as ApiKeyType
                        });
                    }
                });
            }
        }
        return keyList;
    } catch (error) {
        console.error("获取 API 密钥列表失败:", error);
        return new APIKeyList();
    }
}

async function saveApiKeyList(config_file: string, apiKeyList: APIKeyList): Promise<void> {
    try {
        await invoke("try_save_api_key_list", { configName: config_file, list: apiKeyList });
        showNotification("API 密钥列表已保存", "success");
    } catch (error) {
        console.error("保存 API 密钥列表失败:", error);
        showNotification("保存 API 密钥列表失败", "error");
    }
}


// 加载 API Keys
async function loadApiKeys() {
    try {
        const keys = await getApiKeyListOrCreate(apiKeyConfigFile);
        if (keys) {
            apiKeys.value = keys;
        }
    } catch (error) {
        console.error("加载 API 密钥失败:", error);
        showNotification("加载 API 密钥失败", "error");
    }
}

// 添加新的 API Key
async function addApiKey() {
    if (!newApiKey.key || !newApiKey.name) {
        showNotification("密钥和名称不能为空", "error");
        return;
    }

    const key: ApiKey = {
        key: newApiKey.key,
        name: newApiKey.name,
        key_type: newApiKey.key_type
    };

    apiKeys.value.addKey(key);
    await saveApiKeyList(apiKeyConfigFile, apiKeys.value);

    // 重置表单
    newApiKey.key = '';
    newApiKey.name = '';
    isAddingKey.value = false;

    showNotification("API 密钥已添加", "success");
}

// 添加用于API密钥删除确认的状态
const showConfirmDeleteKey = ref(false);
const keyToDelete = ref<ApiKey | null>(null);

// 修改为显示删除确认对话框
function confirmDeleteApiKey(key: ApiKey) {
    keyToDelete.value = key;
    showConfirmDeleteKey.value = true;
}

// 执行真正的删除操作
async function submitDeleteApiKey() {
    if (!keyToDelete.value) {
        showNotification("无效的 API 密钥", "error");
        return;
    }

    try {
        apiKeys.value.removeKey(keyToDelete.value);
        await saveApiKeyList(apiKeyConfigFile, apiKeys.value);
        showNotification("API 密钥已删除", "info");
    } catch (error) {
        console.error("删除 API 密钥失败:", error);
        showNotification("删除 API 密钥失败", "error");
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
// 保存设置
async function saveSettings() {
    try {
        await invoke("save_settings", { settings: settings.value });
        theme_before_save.value = settings.value.theme as 'system' | 'light' | 'dark';
        font_size_before_save.value = settings.value.font_size as 'small' | 'medium' | 'large';
        showNotification("设置已保存", "success");
    } catch (error) {
        console.error("保存设置失败:", error);
        showNotification("保存设置失败", "error");
    }
}

// 重置设置
async function resetSettings() {
    try {
        const defaultSettings = await invoke("get_default_settings");
        settings.value = defaultSettings as any;
        showNotification("设置已重置", "info");
    } catch (error) {
        console.error("重置设置失败:", error);
        showNotification("重置设置失败", "error");
    }
}


// 通知
const notification = ref({
    visible: false,
    message: '',
    type: 'success'
});

// 显示通知
function showNotification(message: string, type: string = 'success', duration: number = 3000) {
    notification.value = {
        visible: true,
        message,
        type
    };

    setTimeout(() => {
        notification.value.visible = false;
    }, duration);
}


// 监听主题变化
watch(
    () => settings.value.theme,
    (newTheme: string) => {
        applyTheme(newTheme as 'system' | 'light' | 'dark');
    }
);

watch(
    () => settings.value.font_size,
    (newFontSize: string) => {
        applyFontSize(newFontSize as 'small' | 'medium' | 'large');
    }
);


// 加载设置
async function loadSettings() {
    try {
        const savedSettings = await invoke("get_settings");
        if (savedSettings) {
            settings.value = { ...settings.value, ...savedSettings };
            applyTheme(settings.value.theme as 'system' | 'light' | 'dark');
            applyFontSize(settings.value.font_size as 'small' | 'medium' | 'large');
            theme_before_save.value = settings.value.theme as 'system' | 'light' | 'dark';
            font_size_before_save.value = settings.value.font_size as 'small' | 'medium' | 'large';
        }
    } catch (error) {
        console.error("加载设置失败:", error);
    }
}


// 关闭设置界面
function closeSettings() {
    applyTheme(theme_before_save.value as 'system' | 'light' | 'dark');
    applyFontSize(font_size_before_save.value as 'small' | 'medium' | 'large');
    emit('close');
}

// 初始化应用设置
async function initAppSettings() {
    try {
        const savedSettings = await invoke("get_settings");
        if (savedSettings) {
            settings.value = { ...settings.value, ...savedSettings };
            // 应用主题
            if (settings.value.theme === 'system') {
                document.documentElement.removeAttribute('data-theme');
            } else {
                document.documentElement.setAttribute('data-theme', settings.value.theme);
            }

            // 应用字体大小
            document.documentElement.setAttribute('data-font-size', settings.value.font_size);
        }
    } catch (error) {
        console.error("初始化应用设置失败:", error);
    }
}


// 组件挂载时加载设置
onMounted(() => {
    initAppSettings();
    loadSettings();
    loadApiKeys();
    loadModelOptions();
});

</script>

<template>
    <div class="settings-container">
        <!-- 通知组件 -->
        <div v-if="notification.visible" class="notification" :class="notification.type">
            <div class="notification-content">
                <svg v-if="notification.type === 'success'" xmlns="http://www.w3.org/2000/svg" width="16" height="16"
                    viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round">
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
                    <polyline points="22 4 12 14.01 9 11.01"></polyline>
                </svg>
                <svg v-else-if="notification.type === 'error'" xmlns="http://www.w3.org/2000/svg" width="16" height="16"
                    viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round">
                    <circle cx="12" cy="12" r="10"></circle>
                    <line x1="15" y1="9" x2="9" y2="15"></line>
                    <line x1="9" y1="9" x2="15" y2="15"></line>
                </svg>
                <svg v-else-if="notification.type === 'info'" xmlns="http://www.w3.org/2000/svg" width="16" height="16"
                    viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                    stroke-linejoin="round">
                    <circle cx="12" cy="12" r="10"></circle>
                    <line x1="12" y1="16" x2="12" y2="12"></line>
                    <line x1="12" y1="8" x2="12.01" y2="8"></line>
                </svg>
                <span>{{ notification.message }}</span>
            </div>
        </div>
        <div class="settings-header">
            <h2>设置</h2>
            <button class="close-settings" @click="closeSettings">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none"
                    stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                    <!-- 现代化的关闭图标：圆角X -->
                    <path d="M6 6l12 12"></path>
                    <path d="M18 6l-12 12"></path>
                </svg>
            </button>
        </div>

        <div class="settings-content">
            <!-- 外观设置 -->
            <div class="settings-section">
                <h3>外观设置</h3>

                <div class="settings-item">
                    <label for="theme">主题</label>
                    <div class="settings-controls">
                        <select id="theme" v-model="settings.theme">
                            <option v-for="option in themeOptions" :key="option.value" :value="option.value">
                                {{ option.label }}
                            </option>
                        </select>
                    </div>
                </div>

                <div class="settings-item">
                    <label for="fontSize">字体大小</label>
                    <div class="settings-controls">
                        <select id="fontSize" v-model="settings.font_size">
                            <option v-for="option in fontSizeOptions" :key="option.value" :value="option.value">
                                {{ option.label }}
                            </option>
                        </select>
                    </div>
                </div>
            </div>
            <!-- 

            <div class="settings-section">
                <h3>保存设置</h3>

                <div class="settings-item auto-save-item">
                    <label for="autoSave">自动保存对话</label>
                    <div class="settings-controls">
                        <label class="switch">
                            <input type="checkbox" id="autoSave" v-model="settings.auto_save">
                            <span class="slider round"></span>
                        </label>
                    </div>
                </div>

                <div class="settings-item">
                    <label for="savePath">保存路径</label>
                    <div class="settings-controls path-selection">
                        <input type="text" id="savePath" v-model="settings.save_path" readonly
                            placeholder="点击右侧按钮选择保存路径">
                        <button @click="selectSavePath" class="directory-button">
                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
                                fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                                stroke-linejoin="round">
                                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z">
                                </path>
                            </svg>
                        </button>
                    </div>
                </div>
            </div>

            <div class="settings-section">
                <h3>模型设置</h3>

                <div class="settings-item">
                    <label for="apiModel">AI 模型</label>
                    <div class="settings-controls">
                        <select id="apiModel" v-model="settings.api_model">
                            <option v-for="option in modelOptions" :key="option.value" :value="option.value">
                                {{ option.label }}
                            </option>
                        </select>
                    </div>
                </div>

                <div class="settings-item">
                    <label for="temperature">温度参数</label>
                    <div class="settings-controls range-slider">
                        <input type="range" id="temperature" v-model.number="settings.model_config.temperature"
                            min="0.1" max="1" step="0.1">
                        <span class="range-value">{{ settings.model_config.temperature.toFixed(1) }}</span>
                    </div>
                </div>

                <div class="settings-item">
                    <label for="maxTokens">最大令牌数</label>
                    <div class="settings-controls">
                        <input type="number" id="maxTokens" v-model.number="settings.model_config.max_tokens" min="512"
                            max="8192" step="512">
                    </div>
                </div>
            </div> -->

            <div class="settings-section">
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
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
                                    fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                                    stroke-linejoin="round">
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
                                        <button class="modal-close" @click="cancelDeleteApiKey">
                                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"
                                                viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                                                stroke-linecap="round" stroke-linejoin="round">
                                                <line x1="18" y1="6" x2="6" y2="18"></line>
                                                <line x1="6" y1="6" x2="18" y2="18"></line>
                                            </svg>
                                        </button>
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
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
                                    fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                                    stroke-linejoin="round">
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
        </div>

        <!-- 设置操作按钮 -->
        <div class="settings-actions">
            <button @click="resetSettings" class="reset-button">重置设置</button>
            <button @click="saveSettings" class="save-button">保存设置</button>
            <button @click="closeSettings" class="cancel-button">取消</button>
        </div>
    </div>
</template>

<style>
/* 全局 CSS 变量 */
:root {
    /* 浅色主题变量 */
    --bg-color: #ffffff;
    --text-color: #1a202c;
    --text-secondary: #4a5568;
    --card-bg: #f8fafc;
    --border-color: #e2e8f0;
    --primary-color: #4f46e5;
    --primary-hover: #4338ca;
    --radius: 8px;
    --radius-sm: 4px;
    --shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06);
    --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
    --transition: all 0.2s ease;

    /* 默认字体大小 */
    --font-size-base: 16px;
    --font-size-sm: 14px;
    --font-size-lg: 18px;
}

/* 深色主题 */
:root[data-theme="dark"] {
    --bg-color: #1a202c;
    --text-color: #f1f5f9;
    --text-secondary: #94a3b8;
    --card-bg: #2d3748;
    --border-color: #4a5568;
    --shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.4), 0 1px 2px 0 rgba(0, 0, 0, 0.2);
    --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.15);
}

/* 字体大小设置 */
:root[data-font-size="small"] {
    --font-size-base: 14px;
    --font-size-sm: 12px;
    --font-size-lg: 16px;
    --font-size-heading: 18px;
}

:root[data-font-size="medium"] {
    --font-size-base: 16px;
    --font-size-sm: 14px;
    --font-size-lg: 18px;
    --font-size-heading: 20px;
}

:root[data-font-size="large"] {
    --font-size-base: 18px;
    --font-size-sm: 16px;
    --font-size-lg: 20px;
    --font-size-heading: 24px;
}

/* 应用字体大小 */
body {
    font-size: var(--font-size-base);
}

.small-text {
    font-size: var(--font-size-sm);
}

.large-text {
    font-size: var(--font-size-lg);
}

.settings-container {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background-color: var(--bg-color);
    color: var(--text-color);
    padding: 20px;
    overflow-y: auto;
    /* 已有的滚动条设置 */
    max-height: 90vh;
    /* 限制最大高度 */
    scrollbar-width: thin;
    /* Firefox 滚动条样式 */
}

/* 为 Webkit 浏览器添加自定义滚动条样式 */
.settings-container::-webkit-scrollbar {
    width: 6px;
}

.settings-container::-webkit-scrollbar-track {
    background: transparent;
}

.settings-container::-webkit-scrollbar-thumb {
    background-color: rgba(0, 0, 0, 0.2);
    border-radius: 3px;
}

/* 设置内容区域也添加滚动条样式 */
.settings-content {
    flex: 1;
    overflow-y: auto;
    scrollbar-width: thin;
}

.settings-content::-webkit-scrollbar {
    width: 6px;
}

.settings-content::-webkit-scrollbar-track {
    background: transparent;
}

.settings-content::-webkit-scrollbar-thumb {
    background-color: rgba(0, 0, 0, 0.2);
    border-radius: 3px;
}


.settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border-color);
}

.settings-header h2 {
    font-size: calc(var(--font-size-heading) * 1.1 * 1.1);
    font-weight: 600;
}

.settings-content {
    flex: 1;
    overflow-y: auto;
}

.settings-section {
    margin-bottom: 32px;
    padding: 20px;
    background-color: var(--card-bg);
    border-radius: var(--radius);
    box-shadow: var(--shadow-sm);
    border: 1px solid var(--border-color);
}

.settings-section h3 {
    font-size: calc(var(--font-size-heading) * 1.1);
    font-weight: 600;
    margin-bottom: 16px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-color);
}

.settings-item {
    display: flex;
    align-items: center;
    margin-bottom: 16px;
    flex-wrap: wrap;
}

.settings-item:last-child {
    margin-bottom: 0;
}

.settings-item label {
    flex: 0 0 140px;
    font-weight: 500;
    margin-right: 16px;
}

.settings-controls {
    flex: 1;
    min-width: 200px;
}

.close-settings {
    background: none;
    border: none;
    color: var(--text-color);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    transition: var(--transition);
}

.close-settings:hover {
    background-color: rgba(0, 0, 0, 0.05);
}

select,
input[type="text"],
input[type="number"] {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    background-color: var(--card-bg);
    color: var(--text-color);
    font-size: var(--font-size-base);
    transition: var(--transition);
}

select:focus,
input:focus {
    border-color: var(--primary-color);
    outline: none;
    box-shadow: 0 0 0 2px rgba(79, 70, 229, 0.1);
}

.switch-container {
    display: flex;
    align-items: center;
    justify-content: flex-start;
}

.switch-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    cursor: pointer;
}

.switch {
    position: relative;
    display: inline-block;
    width: 48px;
    height: 24px;
}

.switch input {
    opacity: 0;
    width: 0;
    height: 0;
}

.slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: #cbd5e1;
    transition: .4s;
}

.slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    transition: .4s;
}

input:checked+.slider {
    background-color: var(--primary-color);
}

input:focus+.slider {
    box-shadow: 0 0 1px var(--primary-color);
}

input:checked+.slider:before {
    transform: translateX(24px);
}

.slider.round {
    border-radius: 24px;
    width: 48px;
}

.slider.round:before {
    border-radius: 50%;
}

.auto-save-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.auto-save-item .settings-controls {
    flex: 0 0 auto;
    min-width: auto;
    display: flex;
    justify-content: flex-end;
}

.auto-save-item label {
    flex: 1;
}

.auto-save-item .switch {
    margin-left: auto;
}

.path-selection {
    display: flex;
    align-items: center;
}

.path-selection input {
    flex: 1;
    margin-right: 8px;
}

.directory-button {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    background-color: var(--card-bg);
    color: var(--text-color);
    cursor: pointer;
    transition: var(--transition);
}

.directory-button:hover {
    background-color: rgba(0, 0, 0, 0.05);
    border-color: var(--primary-color);
}

.range-slider {
    display: flex;
    align-items: center;
}

.range-slider input[type="range"] {
    flex: 1;
    margin-right: 12px;
}

.range-value {
    min-width: 40px;
    text-align: center;
    font-weight: 500;
}

.settings-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 24px;
    gap: 12px;
}

.reset-button,
.save-button {
    padding: 10px 20px;
    border-radius: var(--radius);
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
}

.reset-button {
    background-color: transparent;
    border: 1px solid var(--border-color);
    color: var(--text-color);
}

.reset-button:hover {
    background-color: rgba(0, 0, 0, 0.05);
    border-color: var(--text-color);
}

.save-button {
    background-color: var(--primary-color);
    border: none;
    color: white;
}

.save-button:hover {
    background-color: var(--primary-hover);
}

/* 通知样式 */
.notification {
    position: fixed;
    top: 16px;
    right: 16px;
    padding: 12px 16px;
    border-radius: var(--radius);
    background-color: white;
    box-shadow: var(--shadow);
    z-index: 1000;
    max-width: 400px;
    animation: slide-in 0.3s ease forwards;
    border-left: 4px solid;
}

.notification.success {
    border-left-color: #10b981;
}

.notification.error {
    border-left-color: #ef4444;
}

.notification.info {
    border-left-color: #3b82f6;
}

.notification-content {
    display: flex;
    align-items: center;
    gap: 8px;
}

.notification-content svg {
    flex-shrink: 0;
}

.notification.success svg {
    color: #10b981;
}

.notification.error svg {
    color: #ef4444;
}

.notification.info svg {
    color: #3b82f6;
}

@keyframes slide-in {
    from {
        transform: translateX(100%);
        opacity: 0;
    }

    to {
        transform: translateX(0);
        opacity: 1;
    }
}

/* 响应式样式 */
@media (max-width: 768px) {
    .settings-item {
        flex-direction: column;
        align-items: flex-start;
    }

    .settings-item label {
        margin-bottom: 8px;
        flex: 0 0 auto;
    }

    .settings-controls {
        width: 100%;
    }

    /* 在小屏幕下，确保开关居左对齐 */
    .switch-container {
        justify-content: flex-start;
    }

    .auto-save-item {
        flex-direction: row;
        align-items: center;
    }

    .auto-save-item label {
        margin-bottom: 0;
    }

    .auto-save-item .settings-controls {
        width: auto;
    }
}


.settings-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 24px;
    gap: 12px;
}

.reset-button,
.save-button,
.cancel-button {
    padding: 10px 20px;
    border-radius: var(--radius);
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
}

.cancel-button {
    background-color: transparent;
    border: 1px solid var(--border-color);
    color: var(--text-color);
}

.cancel-button:hover {
    background-color: rgba(0, 0, 0, 0.05);
}



select {
    /* 保留必要的自定义样式 */
    -webkit-appearance: none;
    -moz-appearance: none;
    appearance: none;

    /* 添加下拉箭头背景 */
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='%236b7280' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 8px center;
    background-size: 16px;

    /* 添加右侧内边距，避免文本与图标重叠 */
    padding-right: 32px;

    /* 移除可能导致延迟的性能优化属性 */
    will-change: initial;
    transform: none;

    /* 简化过渡效果 */
    transition: border-color 0.2s ease-in-out;

    /* 确保覆盖浏览器默认样式 */
    text-rendering: optimizeLegibility;
}

select option {
    background-color: var(--card-bg);
    color: var(--text-color);
    padding: 8px;
}

/* 移除Firefox的下拉箭头 */
select::-ms-expand {
    display: none;
}

/* 优化选择框的聚焦效果 */
select:focus {
    border-color: var(--primary-color);
    outline: none;
    box-shadow: 0 0 0 2px rgba(79, 70, 229, 0.1);
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

/* 修复自动保存开关的布局问题 */
.settings-item .switch-container {
    display: flex;
    align-items: center;
    justify-content: space-between;
}

.settings-item .switch-container label {
    flex: 0 0 auto;
    margin-right: auto;
}

.switch-container .switch {
    margin-left: auto;
}

/* 修复输入框和选择框的初始渲染 */
input,
select {
    backface-visibility: hidden;
    -webkit-font-smoothing: subpixel-antialiased;
}

/* 响应式调整 */
@media (max-width: 768px) {
    .settings-item .switch-container {
        justify-content: space-between;
        width: 100%;
    }

    .settings-item .switch-container label {
        margin-right: 0;
    }
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
    border-radius: var(--radius);
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
    background-color: var(--bg-color);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    transition: var(--transition);
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
}

.api-key-type {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
}

.api-key-value {
    font-family: monospace;
    background-color: rgba(0, 0, 0, 0.03);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: var(--font-size-sm);
}

.delete-key-button {
    background: none;
    border: none;
    color: var(--text-color);
    opacity: 0.5;
    cursor: pointer;
    padding: 8px;
    border-radius: var(--radius-sm);
    transition: var(--transition);
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
    padding: 10px;
    border: 1px dashed var(--border-color);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--primary-color);
    cursor: pointer;
    width: 100%;
    transition: var(--transition);
}

.add-api-key:hover {
    background-color: rgba(79, 70, 229, 0.05);
}

.add-key-form {
    padding: 16px;
    border: 1px solid var(--border-color);
    border-radius: var(--radius);
    background-color: var(--card-bg);
}

.form-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
}

.form-header h4 {
    font-size: calc(var(--font-size-heading));
    font-weight: 600;
    margin: 0;
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
    border-radius: var(--radius-sm);
    transition: var(--transition);
}

.close-form:hover {
    background-color: rgba(0, 0, 0, 0.05);
}

.form-group {
    margin-bottom: 12px;
}

.form-group label {
    display: block;
    font-weight: 500;
    margin-bottom: 6px;
}

.form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
}

.cancel-button,
.add-key-button {
    padding: 8px 16px;
    border-radius: var(--radius-sm);
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
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
    background-color: rgba(0, 0, 0, 0.05);
}

.add-key-button:hover {
    background-color: var(--primary-hover);
}


</style>
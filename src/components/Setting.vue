<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from "@tauri-apps/api/core";
// 导入 Vue 的 watch 函数
import { watch } from 'vue';

const emit = defineEmits(['close']);

// 设置选项
const settings = ref({
    theme: 'system', // 主题: system, light, dark
    fontSize: 'medium', // 字体大小: small, medium, large
    autoSave: true, // 自动保存对话
    savePath: '', // 保存路径
    apiModel: 'qwen', // 模型选择: qwen, gpt, claude
    modelConfig: {
        temperature: 0.7, // 温度参数 0.1-1.0
        maxTokens: 2048, // 最大生成令牌数
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

// 模型选项
const modelOptions = [
    { value: 'qwen', label: '通义千问' },
    { value: 'gpt', label: 'GPT' },
    { value: 'claude', label: 'Claude' },
];

// 保存设置
async function saveSettings() {
    try {
        await invoke("save_settings", { settings: settings.value });
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

// 选择保存路径
async function selectSavePath() {
    try {
        const path = await invoke("select_save_directory");
        if (path) {
            settings.value.savePath = path as string;
        }
    } catch (error) {
        console.error("选择路径失败:", error);
        showNotification("选择路径失败", "error");
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

// 应用主题
function applyTheme(theme: string) {
    if (theme === 'system') {
        document.documentElement.removeAttribute('data-theme');
    } else {
        document.documentElement.setAttribute('data-theme', theme);
    }
}

// 监听主题变化
watch(
    () => settings.value.theme,
    (newTheme: string) => {
        applyTheme(newTheme);
    }
);

// 加载设置
async function loadSettings() {
    try {
        const savedSettings = await invoke("get_settings");
        if (savedSettings) {
            settings.value = { ...settings.value, ...savedSettings };
            applyTheme(settings.value.theme);
        }
    } catch (error) {
        console.error("加载设置失败:", error);
    }
}

// 关闭设置界面
function closeSettings() {
    emit('close');
}

// 组件挂载时加载设置
onMounted(() => {
    loadSettings();
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
                    stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
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
                        <select id="fontSize" v-model="settings.fontSize">
                            <option v-for="option in fontSizeOptions" :key="option.value" :value="option.value">
                                {{ option.label }}
                            </option>
                        </select>
                    </div>
                </div>
            </div>

            <!-- 保存设置 -->
            <div class="settings-section">
                <h3>保存设置</h3>

                <div class="settings-item auto-save-item">
                    <label for="autoSave">自动保存对话</label>
                    <div class="settings-controls">
                        <label class="switch">
                            <input type="checkbox" id="autoSave" v-model="settings.autoSave">
                            <span class="slider round"></span>
                        </label>
                    </div>
                </div>

                <div class="settings-item">
                    <label for="savePath">保存路径</label>
                    <div class="settings-controls path-selection">
                        <input type="text" id="savePath" v-model="settings.savePath" readonly
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

            <!-- 模型设置 -->
            <div class="settings-section">
                <h3>模型设置</h3>

                <div class="settings-item">
                    <label for="apiModel">AI 模型</label>
                    <div class="settings-controls">
                        <select id="apiModel" v-model="settings.apiModel">
                            <option v-for="option in modelOptions" :key="option.value" :value="option.value">
                                {{ option.label }}
                            </option>
                        </select>
                    </div>
                </div>

                <div class="settings-item">
                    <label for="temperature">温度参数</label>
                    <div class="settings-controls range-slider">
                        <input type="range" id="temperature" v-model.number="settings.modelConfig.temperature" min="0.1"
                            max="1" step="0.1">
                        <span class="range-value">{{ settings.modelConfig.temperature.toFixed(1) }}</span>
                    </div>
                </div>

                <div class="settings-item">
                    <label for="maxTokens">最大令牌数</label>
                    <div class="settings-controls">
                        <input type="number" id="maxTokens" v-model.number="settings.modelConfig.maxTokens" min="512"
                            max="8192" step="512">
                    </div>
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

<style scoped>
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

/* 暗黑模式下的滚动条适配 */
@media (prefers-color-scheme: dark) {

    .settings-container::-webkit-scrollbar-thumb,
    .settings-content::-webkit-scrollbar-thumb {
        background-color: rgba(255, 255, 255, 0.2);
    }
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
    font-size: 1.5rem;
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
    font-size: 1.1rem;
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
    font-size: 0.95rem;
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

/* 暗黑模式适配 */
@media (prefers-color-scheme: dark) {
    .reset-button:hover {
        background-color: rgba(255, 255, 255, 0.1);
    }

    .directory-button:hover {
        background-color: rgba(255, 255, 255, 0.1);
    }

    .notification {
        background-color: #1e293b;
        color: #f1f5f9;
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

/* 暗色模式适配 */
@media (prefers-color-scheme: dark) {
    .close-settings:hover {
        background-color: rgba(255, 255, 255, 0.1);
    }

    .cancel-button:hover {
        background-color: rgba(255, 255, 255, 0.1);
    }
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

/* 暗黑模式下的选择框箭头颜色 */
@media (prefers-color-scheme: dark) {
    select {
        background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='%23e2e8f0' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    }
}
</style>
import { ref, reactive, inject, provide, watch } from 'vue';
import { invoke } from "@tauri-apps/api/core";
import { applyFontSize, applyTheme } from '../themeUtils';

// 定义Settings类型
export interface Settings {
    theme: 'system' | 'light' | 'dark';
    font_size: 'small' | 'medium' | 'large';
    auto_save: boolean;
    save_path: string;
    api_model: string;
    model_config: {
        temperature: number;
        max_tokens: number;
    };
}

// 定义 ApiKeyType 枚举
export enum ApiKeyType {
    Gemini = "Gemini"
}

// 定义 ApiKey 接口
export interface ApiKey {
    key: string;
    name: string;
    key_type: ApiKeyType;
}

// 实现 APIKeyList 类
export class APIKeyList {
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

// 通知类型
export interface Notification {
    visible: boolean;
    message: string;
    type: 'success' | 'error' | 'info';
}

// 定义provide/inject的key
const SettingsKey = Symbol('settings');

export function useSettingsProvider() {
    // 设置选项
    const settings = ref<Settings>({
        theme: 'system',
        font_size: 'medium',
        auto_save: true,
        save_path: '',
        api_model: 'Gemini',
        model_config: {
            temperature: 0.7,
            max_tokens: 2048,
        },
    });

    // 记录保存前的主题和字体大小，用于关闭设置时恢复
    const theme_before_save = ref<'system' | 'light' | 'dark'>('system');
    const font_size_before_save = ref<'small' | 'medium' | 'large'>('medium');

    // 模型选项
    const modelOptions = ref<{ value: ApiKeyType; label: string; keyType: ApiKeyType }[]>([]);

    // API Key 管理
    const apiKeys = ref<APIKeyList>(new APIKeyList());
    const newApiKey = reactive({
        key: '',
        name: '',
        key_type: ApiKeyType.Gemini
    });

    const isAddingKey = ref(false);
    const apiKeyConfigFile = 'api_keys.json';

    // 通知
    const notification = ref<Notification>({
        visible: false,
        message: '',
        type: 'success'
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

    // 加载模型选项
    function loadModelOptions() {
        modelOptions.value = Object.values(ApiKeyType).map((keyType) => {
            return {
                value: keyType,
                label: getDisplayName(keyType),
                keyType: keyType
            };
        });
    }

    function getDisplayName(key_type: ApiKeyType): string {
        switch (key_type) {
            case ApiKeyType.Gemini:
                return "Gemini";
            default:
                return "未知类型";
        }
    }

    // 显示通知
    function showNotification(message: string, type: 'success' | 'error' | 'info' = 'success', duration: number = 3000) {
        notification.value = {
            visible: true,
            message,
            type
        };

        setTimeout(() => {
            notification.value.visible = false;
        }, duration);
    }

    // 获取API密钥列表
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

    // 保存API密钥列表
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

    // 删除 API Key
    async function deleteApiKey(key: ApiKey) {
        if (confirm(`确定要删除 ${key.name} 吗?`)) {
            apiKeys.value.removeKey(key);
            await saveApiKeyList(apiKeyConfigFile, apiKeys.value);
            showNotification("API 密钥已删除", "info");
        }
    }

    // 保存设置
    async function saveSettings() {
        try {
            await invoke("save_settings", { settings: settings.value });
            theme_before_save.value = settings.value.theme;
            font_size_before_save.value = settings.value.font_size;
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
            settings.value = defaultSettings as Settings;
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
                settings.value.save_path = path as string;
            }
        } catch (error) {
            console.error("选择路径失败:", error);
            showNotification("选择路径失败", "error");
        }
    }

    // 加载设置
    async function loadSettings() {
        try {
            const savedSettings = await invoke("get_settings");
            if (savedSettings) {
                settings.value = { ...settings.value, ...savedSettings as Settings };
                applyTheme(settings.value.theme);
                applyFontSize(settings.value.font_size);
                theme_before_save.value = settings.value.theme;
                font_size_before_save.value = settings.value.font_size;
            }
        } catch (error) {
            console.error("加载设置失败:", error);
        }
    }

    // 关闭设置界面但不保存
    function cancelSettings() {
        applyTheme(theme_before_save.value);
        applyFontSize(font_size_before_save.value);
    }

    // 初始化应用设置
    async function initAppSettings() {
        try {
            const savedSettings = await invoke("get_settings");
            if (savedSettings) {
                settings.value = { ...settings.value, ...savedSettings as Settings };
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

    // 监听主题变化
    watch(
        () => settings.value.theme,
        (newTheme) => {
            applyTheme(newTheme);
        }
    );

    // 监听字体大小变化
    watch(
        () => settings.value.font_size,
        (newFontSize) => {
            applyFontSize(newFontSize);
        }
    );

    // 提供全局状态和方法
    provide(SettingsKey, {
        // 状态
        settings,
        theme_before_save,
        font_size_before_save,
        modelOptions,
        apiKeys,
        newApiKey,
        isAddingKey,
        notification,
        themeOptions,
        fontSizeOptions,
        
        // 方法
        loadModelOptions,
        getDisplayName,
        showNotification,
        loadApiKeys,
        addApiKey,
        deleteApiKey,
        saveSettings,
        resetSettings,
        selectSavePath,
        loadSettings,
        cancelSettings,
        initAppSettings
    });

    return {
        // 状态
        settings,
        theme_before_save,
        font_size_before_save,
        modelOptions,
        apiKeys,
        newApiKey,
        isAddingKey,
        notification,
        themeOptions,
        fontSizeOptions,
        
        // 方法
        loadModelOptions,
        getDisplayName,
        showNotification,
        loadApiKeys,
        addApiKey,
        deleteApiKey,
        saveSettings,
        resetSettings,
        selectSavePath,
        loadSettings,
        cancelSettings,
        initAppSettings
    };
}

// 在子组件中使用settings
export function useSettings() {
    const settings = inject(SettingsKey);
    if (!settings) {
        throw new Error('useSettings() must be used after useSettingsProvider()');
    }
    return settings;
}
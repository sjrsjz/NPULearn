import { ref, reactive, inject, provide, watch } from 'vue';
import { invoke } from "@tauri-apps/api/core";
import { applyFontSize, applyTheme } from '../themeUtils';

// 定义 ApiKeyType 枚举
export enum ApiKeyType {
    Gemini = "Gemini",
    DeepSeek = "DeepSeek",
    Coze = "Coze"
}

// 定义模型信息接口
export interface ModelInfo {
    name: string;
    displayName: string;
    isReasoning: boolean;
    description?: string;
}

// 定义默认（静态）支持的模型，作为动态获取失败时的备选
const DEFAULT_GEMINI_MODELS: ModelInfo[] = [
    { name: 'gemini-2.0-flash', displayName: 'Gemini 2.0 Flash', isReasoning: false },
    { name: 'gemini-1.5-pro', displayName: 'Gemini 1.5 Pro', isReasoning: false },
    { name: 'gemini-1.5-flash', displayName: 'Gemini 1.5 Flash', isReasoning: false },
    { name: 'gemini-2.5-pro', displayName: 'Gemini 2.5 Pro', isReasoning: true, description: '推理模型，具备思维链展示能力' },
    { name: 'gemini-2.5-flash', displayName: 'Gemini 2.5 Flash', isReasoning: true, description: '推理模型，具备思维链展示能力' },
];

// 定义每种API密钥类型支持的模型
export const SUPPORTED_MODELS: Record<ApiKeyType, ModelInfo[]> = {
    [ApiKeyType.Gemini]: [...DEFAULT_GEMINI_MODELS], // 初始使用默认模型，后续会动态更新
    [ApiKeyType.DeepSeek]: [
        { name: 'deepseek-chat', displayName: 'DeepSeek Chat(V3)', isReasoning: false },
        { name: 'deepseek-reasoner', displayName: 'DeepSeek Reasoner(R1)', isReasoning: true, description: '推理模型，具备强化思维链能力' },
    ],
    [ApiKeyType.Coze]: [
        { name: 'coze-bot', displayName: 'Coze Bot', isReasoning: false, description: '使用内置Bot ID' },
    ],
};

// 定义人格配置接口
export interface PersonaConfig {
    use_custom: boolean;
    preset_persona: string;
    custom_persona: string;
}

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
    model_selection: {
        [key in ApiKeyType]: string;
    };
    persona_config: PersonaConfig;
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

// 全局状态实例 - 确保单例模式
let globalSettingsInstance: any = null;

// 重置全局实例（用于测试或完全重新初始化）
export function resetGlobalSettingsInstance() {
    globalSettingsInstance = null;
    console.log('🔄 Settings global instance has been reset');
}

export function useSettingsProvider() {
    // 如果已经存在实例，直接返回
    if (globalSettingsInstance) {
        console.log('♻️ Reusing existing settings instance');
        return globalSettingsInstance;
    }

    console.log('🆕 Creating new settings instance');

    // 设置选项
    const settings = ref<Settings>({
        theme: 'system',
        font_size: 'medium',
        auto_save: true,
        save_path: '',
        api_model: 'Gemini',
        model_config: {
            temperature: 0.7,
            max_tokens: 8192,
        }, model_selection: {
            [ApiKeyType.Gemini]: 'gemini-2.0-flash',
            [ApiKeyType.DeepSeek]: 'deepseek-chat',
            [ApiKeyType.Coze]: 'coze-bot',
        },
        persona_config: {
            use_custom: false,
            preset_persona: 'academic',
            custom_persona: '',
        },
    });    // 记录保存前的主题和字体大小，用于关闭设置时恢复
    const theme_before_save = ref<'system' | 'light' | 'dark'>('system');
    const font_size_before_save = ref<'small' | 'medium' | 'large'>('medium');

    // 记录打开设置前的完整设置状态，用于取消时恢复
    const settings_before_edit = ref<Settings | null>(null);    // 模型选项
    const modelOptions = ref<{ value: ApiKeyType; label: string; keyType: ApiKeyType }[]>([]);

    // 动态模型列表状态
    const isLoadingGeminiModels = ref(false);
    const geminiModelsError = ref<string | null>(null);

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
    } function getDisplayName(key_type: ApiKeyType): string {
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

    // 获取当前选择的模型是否为推理模型
    function isCurrentModelReasoning(apiKeyType: ApiKeyType): boolean {
        const selectedModel = settings.value.model_selection[apiKeyType];
        const models = SUPPORTED_MODELS[apiKeyType];
        const modelInfo = models.find(m => m.name === selectedModel);
        return modelInfo?.isReasoning || false;
    }    // 更新模型选择
    function updateModelSelection(apiKeyType: ApiKeyType, modelName: string) {
        console.log(`更新模型选择 - API类型: ${apiKeyType}, 新模型: ${modelName}`);
        settings.value.model_selection[apiKeyType] = modelName;
        console.log('更新后的模型选择:', settings.value.model_selection);
        showNotification(`${getDisplayName(apiKeyType)} 模型已更新为 ${getAvailableModels(apiKeyType).find(m => m.name === modelName)?.displayName || modelName}`, 'success');
        saveSettings();
    }    // 获取指定API密钥类型的可用模型
    function getAvailableModels(apiKeyType: ApiKeyType): ModelInfo[] {
        return SUPPORTED_MODELS[apiKeyType] || [];
    }

    // 动态获取Gemini模型列表
    async function fetchGeminiModels(): Promise<void> {
        if (isLoadingGeminiModels.value) {
            return; // 如果正在加载，避免重复请求
        }

        isLoadingGeminiModels.value = true;
        geminiModelsError.value = null; try {
            console.log('🔄 [DEBUG] Fetching Gemini model list...');
            const models = await invoke("get_gemini_models", { keyType: "Gemini" }) as string[];
            console.log('📦 [DEBUG] Model list returned from backend:', models);

            if (models && models.length > 0) {
                console.log('✅ [DEBUG] Successfully fetched Gemini models, count:', models.length);
                console.log('📋 [DEBUG] Model details:', models);

                // Convert model names to ModelInfo objects, using original names directly
                const modelInfos: ModelInfo[] = models.map(modelName => {
                    // Check if it's a reasoning model (contains specific keywords)
                    const isReasoning = modelName.includes('thinking') ||
                        modelName.includes('reasoning') ||
                        modelName.includes('2.5');

                    return {
                        name: modelName,
                        displayName: modelName, // Use original names directly, no aliases
                        isReasoning: isReasoning,
                        description: isReasoning ? '推理模型，具备强化思维链能力' : undefined
                    };
                });

                console.log('🔄 [DEBUG] Converted ModelInfo objects:', modelInfos);

                // Output model list before update
                console.log('📝 [DEBUG] SUPPORTED_MODELS[Gemini] before update:', SUPPORTED_MODELS[ApiKeyType.Gemini]);

                // Update Gemini model list in SUPPORTED_MODELS
                SUPPORTED_MODELS[ApiKeyType.Gemini] = modelInfos;

                // Verify model list after update
                console.log('🎯 [DEBUG] SUPPORTED_MODELS[Gemini] after update:', SUPPORTED_MODELS[ApiKeyType.Gemini]);
                console.log('📊 [DEBUG] Model list length:', SUPPORTED_MODELS[ApiKeyType.Gemini].length);

                // Verify global SUPPORTED_MODELS object
                console.log('🌐 [DEBUG] Complete SUPPORTED_MODELS object:', SUPPORTED_MODELS);

            } else {
                console.warn('⚠️ [DEBUG] No Gemini models fetched, using default list');
                geminiModelsError.value = 'Failed to fetch model list, using default models';
            }
        } catch (error) {
            console.error('❌ [DEBUG] Failed to fetch Gemini models:', error);
            geminiModelsError.value = error instanceof Error ? error.message : 'Failed to fetch model list';
        } finally {
            isLoadingGeminiModels.value = false;
        }
    }

    // 刷新Gemini模型列表
    async function refreshGeminiModels(): Promise<void> {
        await fetchGeminiModels();
    }

    // 获取选中的预设人格信息
    function getSelectedPresetInfo() {
        return PERSONA_PRESETS.find(preset => preset.value === settings.value.persona_config.preset_persona);
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
    }    // 添加新的 API Key
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

        // 如果添加的是Gemini密钥，自动刷新模型列表
        const isGeminiKey = newApiKey.key_type === ApiKeyType.Gemini;

        // 重置表单
        newApiKey.key = '';
        newApiKey.name = '';
        isAddingKey.value = false;

        showNotification("API 密钥已添加", "success");
        if (isGeminiKey) {
            console.log('🔍 [DEBUG] Detected new Gemini key added, auto-fetching latest model list...');
            fetchGeminiModels().catch(error => {
                console.error('❌ [DEBUG] Auto-fetch Gemini models failed:', error);
            });
        }
    }

    // 删除 API Key
    async function deleteApiKey(key: ApiKey) {
        apiKeys.value.removeKey(key);
        await saveApiKeyList(apiKeyConfigFile, apiKeys.value);
        showNotification("API 密钥已删除", "info");
    }    // 保存设置
    async function saveSettings() {
        try {
            console.log('准备保存设置，当前模型选择:', settings.value.model_selection);
            await invoke("save_settings", { settings: settings.value });
            theme_before_save.value = settings.value.theme;
            font_size_before_save.value = settings.value.font_size;
            console.log('设置保存请求已发送');
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
    }    // 关闭设置界面但不保存
    function cancelSettings() {
        // 如果有备份的设置，恢复到打开设置前的状态
        if (settings_before_edit.value) {
            settings.value = { ...settings_before_edit.value };
            console.log('已恢复设置到打开前的状态:', settings.value.model_selection);
        } else {
            // 至少恢复主题和字体大小
            applyTheme(theme_before_save.value);
            applyFontSize(font_size_before_save.value);
        }
    }

    // 备份当前设置状态（在打开设置界面时调用）
    function backupCurrentSettings() {
        settings_before_edit.value = { ...settings.value };
        theme_before_save.value = settings.value.theme;
        font_size_before_save.value = settings.value.font_size;
        console.log('已备份当前设置状态');
    }// 初始化应用设置
    async function initAppSettings() {
        try {
            console.log('开始初始化应用设置...');
            const savedSettings = await invoke("get_settings");
            console.log('从后端获取的设置:', savedSettings);

            if (savedSettings) {
                const settingsData = savedSettings as any;
                console.log('设置更新前的模型选择:', settings.value.model_selection);

                // 更新各个字段，但保持响应式
                if (settingsData.theme) settings.value.theme = settingsData.theme;
                if (settingsData.font_size) settings.value.font_size = settingsData.font_size;
                if (typeof settingsData.auto_save === 'boolean') settings.value.auto_save = settingsData.auto_save;
                if (settingsData.save_path) settings.value.save_path = settingsData.save_path;
                if (settingsData.api_model) settings.value.api_model = settingsData.api_model;

                // 更新模型配置
                if (settingsData.model_config) {
                    if (typeof settingsData.model_config.temperature === 'number') {
                        settings.value.model_config.temperature = settingsData.model_config.temperature;
                    }
                    if (typeof settingsData.model_config.max_tokens === 'number') {
                        settings.value.model_config.max_tokens = settingsData.model_config.max_tokens;
                    }
                }

                // 特别处理model_selection字段
                if (settingsData.model_selection) {
                    console.log('后端返回的模型选择数据:', settingsData.model_selection);

                    // 确保每个API类型都有对应的模型选择
                    Object.values(ApiKeyType).forEach(apiType => {
                        const key = apiType.toString(); // 转换为字符串键
                        if (settingsData.model_selection[key]) {
                            settings.value.model_selection[apiType] = settingsData.model_selection[key];
                            console.log(`设置 ${apiType} 模型为: ${settingsData.model_selection[key]}`);
                        } else {
                            console.log(`${apiType} 模型选择不存在，使用默认值`);
                        }
                    });
                } else {
                    console.log('model_selection字段不存在，保持默认值');
                }

                console.log('设置更新后的模型选择:', settings.value.model_selection);

                // 特别处理persona_config字段
                if (settingsData.persona_config) {
                    console.log('后端返回的人格配置数据:', settingsData.persona_config);
                    
                    if (typeof settingsData.persona_config.use_custom === 'boolean') {
                        settings.value.persona_config.use_custom = settingsData.persona_config.use_custom;
                    }
                    if (settingsData.persona_config.preset_persona) {
                        settings.value.persona_config.preset_persona = settingsData.persona_config.preset_persona;
                    }
                    if (settingsData.persona_config.custom_persona) {
                        settings.value.persona_config.custom_persona = settingsData.persona_config.custom_persona;
                    }
                    console.log('人格配置更新完成:', settings.value.persona_config);
                } else {
                    console.log('persona_config字段不存在，保持默认值');
                }

                // 应用主题
                if (settings.value.theme === 'system') {
                    document.documentElement.removeAttribute('data-theme');
                } else {
                    document.documentElement.setAttribute('data-theme', settings.value.theme);
                }                // 应用字体大小
                document.documentElement.setAttribute('data-font-size', settings.value.font_size);
            } else {
                console.log('没有获取到保存的设置，使用默认值');
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
    );    // 获取需要配置API密钥的类型（排除Coze）
    function getConfigurableApiKeyTypes(): ApiKeyType[] {
        return Object.values(ApiKeyType).filter(type => type !== ApiKeyType.Coze);
    }

    // 获取所有API类型（用于模型选择）
    function getAllApiKeyTypes(): ApiKeyType[] {
        return Object.values(ApiKeyType);
    }    // 提供全局状态和方法
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
        isLoadingGeminiModels,
        geminiModelsError,

        // 方法
        loadModelOptions,
        getDisplayName,
        showNotification,
        loadApiKeys,
        addApiKey,
        deleteApiKey,
        saveSettings,
        resetSettings,
        selectSavePath, loadSettings, cancelSettings,
        backupCurrentSettings,
        initAppSettings,
        getConfigurableApiKeyTypes,
        getAllApiKeyTypes,
        isCurrentModelReasoning,
        updateModelSelection,
        getAvailableModels,
        fetchGeminiModels,
        refreshGeminiModels
    }); 

    // 创建全局实例对象
    const instance = {
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
        isLoadingGeminiModels,
        geminiModelsError,

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
        backupCurrentSettings,
        initAppSettings,
        getConfigurableApiKeyTypes,
        getAllApiKeyTypes,
        isCurrentModelReasoning,
        updateModelSelection,
        getAvailableModels,
        fetchGeminiModels,
        refreshGeminiModels,
        getSelectedPresetInfo
    };    // 保存全局实例
    globalSettingsInstance = instance;
    console.log('✅ Settings global instance created and cached');

    return instance;
}

// 在子组件中使用settings
export function useSettings() {
    // 优先从全局实例获取
    if (globalSettingsInstance) {
        return globalSettingsInstance;
    }
    
    // 如果全局实例不存在，尝试从inject获取
    const settings = inject(SettingsKey);
    if (!settings) {
        throw new Error('useSettings() must be used after useSettingsProvider(). Please call useSettingsProvider() first in the parent component.');
    }
    return settings;
}

// 定义人格预设选项
export const PERSONA_PRESETS = [
    {
        value: 'academic',
        label: '学术助手（航小天）',
        description: '专业的学术AI伙伴，提供学业支持与科研辅助',
        prompt: `你是航小天，西北工业大学的AI学习伙伴，致力于为不同学习阶段与需求的学生提供学业支持与科研辅助。

## 核心身份
- **Name**: 航小天
- **Identity**: 西北工业大学AI学习伙伴
- **Mission**: 为不同学习阶段与需求的学生提供学业支持与科研辅助

## 核心能力
- **学科知识**: 解答数学、物理、计算机科学、电子工程、机械工程、航空航天等理工科问题
- **数学辅助**: 进行符号运算、数值计算、公式推导、解方程、绘制函数图像
- **编程支持**: 理解和生成多种编程语言代码，辅助调试和算法设计
- **学术写作**: 提供论文选题、结构规划、文献综述、语言润色等支持
- **学习规划**: 协助制定学习计划，推荐学习资源`
    },
    {
        value: 'professional',
        label: '专业助手',
        description: '专业、客观、正式的回答风格，提供准确可靠的信息',
        prompt: `你是一个专业的AI助手，始终以准确、客观、正式的语言回答问题。提供详细和有用的信息，避免不必要的装饰性语言。

核心原则：
- 保持专业、客观的态度
- 提供准确、可靠的信息
- 使用正式但易懂的语言
- 结构化地组织回答
- 承认知识的局限性`
    },
    {
        value: 'friendly',
        label: '友好助手',
        description: '温和、友好、易于交流的风格，充满关怀和耐心',
        prompt: `你是一个友好温和的AI助手，以亲切、平易近人的语言与用户交流。你总是保持耐心和理解，让每次对话都感觉自然舒适。

性格特点：
- 温暖友善，富有同理心
- 耐心细致，不厌其烦
- 积极正面，充满鼓励
- 善于倾听，理解用户需求
- 用词亲切但不失专业`
    },
    {
        value: 'creative',
        label: '创意伙伴',
        description: '富有创造力和想象力，提供创新的想法和独特视角',
        prompt: `你是一个富有创造力和想象力的AI伙伴，擅长提供创新的想法和解决方案。你用生动有趣的语言回答问题，总是能从独特的角度思考问题。

创意特质：
- 思维发散，善于联想
- 富有想象力和创新精神
- 乐于探索新的可能性
- 善用比喻和类比
- 鼓励用户跳出常规思维`
    },
    {
        value: 'teacher',
        label: '教学导师',
        description: '耐心教学，善于将复杂概念分解为易懂的部分',
        prompt: `你是一位经验丰富的教师，擅长将复杂的概念分解为易于理解的部分。你善用教学技巧，如类比、例子和逐步解释来帮助学生理解。

教学理念：
- 循序渐进，由浅入深
- 因材施教，适应不同水平
- 启发式教学，引导思考
- 理论联系实际
- 耐心细致，反复确认理解`
    },
    {
        value: 'researcher',
        label: '研究员',
        description: '严谨分析，提供基于证据的深入研究性回答',
        prompt: `你是一位严谨的研究员，总是提供基于证据的深入分析。你善于多角度思考问题，引用相关资料，并诚实地承认不确定性和知识的局限。

研究态度：
- 严谨客观，基于证据
- 多角度分析，全面考虑
- 批判性思维，质疑假设
- 诚实坦率，承认局限
- 持续学习，更新认知`
    }
];
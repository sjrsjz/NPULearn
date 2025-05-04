import { ref } from 'vue';
import type { ChatMessage, ChatHistoryItem } from './types';

// 导出直接可访问的响应式状态
export const isLoading = ref(false);
export const isStreaming = ref(false);
export const chatContent = ref<ChatMessage[]>([]);
export const chatHistory = ref<ChatHistoryItem[]>([]);

// 只有真正需要事件通信的功能使用事件总线
import mitt from 'mitt';

type Events = {
    'notification:show': { message: string, type: string };
    'chart:open': { svgContent: string, diagramContent: string };
    'history:autoHide': void;
    'message:send': string;
    'content:update': { messages: ChatMessage[] };
};

export const eventBus = mitt<Events>();

// 提供辅助函数，用于事件通知类型的功能
export const AppEvents = {
    // 通知相关 (事件型)
    showNotification: (message: string, type: string) => {
        eventBus.emit('notification:show', { message, type });
    },

    // 图表查看器相关 (事件型)
    openChartViewer: (svgContent: string, diagramContent: string) => {
        eventBus.emit('chart:open', { svgContent, diagramContent });
    },

    // 自动隐藏历史 (事件型)
    autoHideHistory: () => {
        eventBus.emit('history:autoHide');
    },

    // 直接发送消息 (事件型)
    sendStreamMessageDirect: async (message: string) => {
        eventBus.emit('message:send', message);
    },

    updateChatContent: (messages: ChatMessage[]) => {
        eventBus.emit('content:update', { messages });
    }
};

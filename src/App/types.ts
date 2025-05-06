
// 定义聊天历史的类型
interface ChatHistoryItem {
    id: number;
    title: string;
    time: string;
}

// 定义完整的聊天历史结构
interface ChatHistory {
    id: number;
    title: string;
    time: string;
    content: ChatMessage[];
}

// 定义聊天消息的类型
interface ChatMessage {
    msgtype: 'User' | 'System' | 'Assistant';
    time: string;
    content: string;
}

export type { ChatHistoryItem, ChatHistory, ChatMessage };
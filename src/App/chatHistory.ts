import { AppEvents, chatHistory, isLoading, isStreaming } from "./eventBus";
import { invoke } from "@tauri-apps/api/core";
import { ChatMessage } from "./types";

// 创建新对话
async function createNewChat() {
    // 如果正在流式输出消息，禁止创建新聊天
    if (isStreaming.value) {
        AppEvents.showNotification("请等待当前消息输出完成", "error");
        return;
    }

    isLoading.value = true;
    try {
        // 调用后端创建新对话API
        const chatContent = await invoke("create_new_chat") as ChatMessage[];
        // 更新聊天内容显示
        AppEvents.updateChatContent(chatContent);
        // 重新加载历史记录以显示新创建的对话
        await loadChatHistory();
        AppEvents.showNotification("已创建新对话", "success");
    } catch (error) {
        console.error("创建新对话失败:", error);
        AppEvents.showNotification("创建新对话失败", "error");
    } finally {
        isLoading.value = false;
    }
}


// 选择历史对话
async function selectHistory(id: number) {
    // 如果正在流式输出消息，禁止切换聊天
    // 调用后端加载特定对话
    console.log(`加载对话 ${id}`);

    isLoading.value = true;
    let chatContent = [] as ChatMessage[];
    try {
        // 调用 Rust 函数加载特定对话内容
        chatContent = await invoke("select_chat_by_id", { id }) as ChatMessage[];
    } catch (error) {
        console.error("加载对话失败:", error);
        AppEvents.showNotification("加载对话失败", "error");
    } finally {
        isLoading.value = false;
        // 更新聊天内容，确保样式隔离
        AppEvents.updateChatContent(chatContent);

        // 在移动设备上选择后自动关闭侧边栏
        AppEvents.autoHideHistory();
    }
}



// 从后端加载聊天历史
async function loadChatHistory() {
    try {
        // 从后端API获取聊天历史列表
        chatHistory.value = await invoke("get_chat_history_items");
        console.log("已加载聊天历史:", chatHistory.value);
        const chatContent = await invoke("get_chat_html") as ChatMessage[];
        AppEvents.updateChatContent(chatContent); // 确保在加载历史后更新内容
    } catch (error) {
        console.error("加载聊天历史失败:", error);
        AppEvents.showNotification("加载聊天历史失败", "error");
    }
}

export { loadChatHistory, selectHistory, createNewChat };
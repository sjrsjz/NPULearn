<script setup lang="ts">
// 保持脚本部分不变
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

// 定义聊天历史的类型
interface ChatHistoryItem {
  id: number;
  title: string;
  time: string;
}

// 改为空数组，将从后端加载
const chatHistory = ref<ChatHistoryItem[]>([]);
const windowWidth = ref(window.innerWidth);
const isHistoryOpen = ref(windowWidth.value >= 768);
const inputMessage = ref("");
const chatContent = ref("");
const isLoading = ref(false);

// 切换历史列表显示
function toggleHistory() {
  isHistoryOpen.value = !isHistoryOpen.value;
}

// 选择历史对话
async function selectHistory(id: number) {
  // 调用后端加载特定对话
  console.log(`加载对话 ${id}`);

  isLoading.value = true;
  try {
    // 调用 Rust 函数加载特定对话内容
    chatContent.value = await invoke("get_chat_by_id", { id });
  } catch (error) {
    console.error("加载对话失败:", error);
  } finally {
    isLoading.value = false;
    // 在移动设备上选择后自动关闭侧边栏
    if (windowWidth.value < 768) {
      isHistoryOpen.value = false;
    }
  }
}

// 处理窗口大小变化
function handleResize() {
  windowWidth.value = window.innerWidth;
  if (windowWidth.value >= 768) {
    isHistoryOpen.value = true;
  } else {
    isHistoryOpen.value = false;
  }
}

// 从后端加载聊天历史
async function loadChatHistory() {
  try {
    chatHistory.value = await invoke("get_chat_history");
  } catch (error) {
    console.error("加载聊天历史失败:", error);
    // 如果失败，使用一些默认数据
    chatHistory.value = [
      { id: 0, title: "新对话", time: "现在" }
    ];
  }
}

// 从后端加载聊天内容
async function loadChatContent() {
  isLoading.value = true;
  try {
    chatContent.value = await invoke("get_chat_html");
  } catch (error) {
    console.error("加载聊天内容失败:", error);
  } finally {
    isLoading.value = false;
  }
}

// 发送消息
async function sendMessage() {
  if (!inputMessage.value.trim()) return;

  isLoading.value = true;
  try {
    // 调用 Rust 函数处理用户输入并返回更新的 HTML
    chatContent.value = await invoke("process_message", { message: inputMessage.value });
    inputMessage.value = ""; // 清空输入框

    // 重新加载聊天历史（如果当前对话是新建的，它可能被添加到历史记录中）
    await loadChatHistory();
  } catch (error) {
    console.error("发送消息失败:", error);
  } finally {
    isLoading.value = false;
  }
}

// 创建新对话
async function createNewChat() {
  isLoading.value = true;
  try {
    // 调用后端创建新对话
    chatContent.value = await invoke("create_new_chat");
    // 重新加载历史记录
    await loadChatHistory();
  } catch (error) {
    console.error("创建新对话失败:", error);
  } finally {
    isLoading.value = false;
  }
}

// 组件加载时初始化对话内容
onMounted(async () => {
  // 先加载历史记录，再加载当前对话内容
  await loadChatHistory();
  await loadChatContent();
  window.addEventListener('resize', handleResize);
});

// 组件卸载时清理事件监听
onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
});
</script>

<template>
  <div class="app-container">
    <!-- 遮罩层 - 仅在小屏幕且历史栏打开时显示 -->
    <div v-if="isHistoryOpen && windowWidth < 768" class="history-overlay" @click="toggleHistory"></div>

    <!-- 左侧历史列表 -->
    <aside class="history-sidebar" :class="{ 'history-open': isHistoryOpen }">
      <div class="history-header">
        <h3>对话历史</h3>
        <!-- 小屏幕时在历史栏中添加关闭按钮 -->
        <button class="close-history" @click="toggleHistory">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>
      <div class="history-actions">
        <button class="new-chat-button" @click="createNewChat">
          <svg class="icon" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
          </svg>
          新对话
        </button>
      </div>
      <div class="history-list">
        <div v-for="item in chatHistory" :key="item.id" @click="selectHistory(item.id)" class="history-item">
          <div class="history-item-content">
            <svg class="history-icon" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
              fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
            </svg>
            <div class="history-text">
              <div class="history-title">{{ item.title }}</div>
              <div class="history-time">{{ item.time }}</div>
            </div>
          </div>
        </div>
      </div>
    </aside>

    <!-- 主要聊天区域 -->
    <main class="chat-container" :class="{ 'sidebar-open': isHistoryOpen }">
      <!-- 顶部导航栏 -->
      <header class="chat-header">
        <button class="toggle-history" @click="toggleHistory">
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="3" y1="12" x2="21" y2="12"></line>
            <line x1="3" y1="6" x2="21" y2="6"></line>
            <line x1="3" y1="18" x2="21" y2="18"></line>
          </svg>
        </button>
        <h1>AI 助手</h1>
      </header>

      <!-- 聊天内容区域 -->
      <div class="chat-content">
        <div v-if="isLoading" class="loading">
          <div class="loading-spinner"></div>
          <div>加载中...</div>
        </div>
        <div v-html="chatContent" class="chat-messages"></div>
      </div>

      <!-- 底部输入区 -->
      <div class="chat-input-area">
        <form @submit.prevent="sendMessage" class="input-form">
          <input v-model="inputMessage" type="text" placeholder="输入消息..." class="message-input" />
          <button type="submit" class="send-button">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="send-icon">
              <line x1="22" y1="2" x2="11" y2="13"></line>
              <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
            </svg>
          </button>
        </form>
      </div>
    </main>
  </div>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&display=swap');

:root {
  --primary-color: #4f46e5;
  --primary-hover: #4338ca;
  --bg-color: #f9fafb;
  --dark-bg-color: #111827;
  --text-color: #1f2937;
  --text-secondary: #6b7280;
  --dark-text-color: #f3f4f6;
  --dark-text-secondary: #9ca3af;
  --border-color: #e5e7eb;
  --dark-border-color: #374151;
  --card-bg: #ffffff;
  --dark-card-bg: #1f2937;
  --sidebar-width: 280px;
  --header-height: 64px;
  --input-area-height: 80px;
  --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
  --shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
  --radius-sm: 0.375rem;
  --radius: 0.5rem;
  --radius-lg: 0.75rem;
  --transition: all 0.2s ease;
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  line-height: 1.5;
  background-color: var(--bg-color);
  color: var(--text-color);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.app-container {
  display: flex;
  height: 100vh;
  width: 100%;
  overflow: hidden;
  position: relative;
  background-color: var(--bg-color);
}

/* 遮罩层 */
.history-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  z-index: 90;
  cursor: pointer;
  backdrop-filter: blur(2px);
  transition: opacity 0.3s ease;
}

/* 历史侧边栏 */
.history-sidebar {
  width: var(--sidebar-width);
  background-color: var(--card-bg);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  position: fixed;
  top: 0;
  left: 0;
  bottom: 0;
  z-index: 100;
  transform: translateX(-100%);
  box-shadow: var(--shadow);
}

.history-open {
  transform: translateX(0);
}

.history-header {
  height: var(--header-height);
  padding: 0 16px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.history-header h3 {
  font-size: 1.125rem;
  font-weight: 600;
}

.close-history {
  background: none;
  border: none;
  color: var(--text-color);
  cursor: pointer;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  transition: var(--transition);
}

.close-history:hover {
  background-color: rgba(0, 0, 0, 0.05);
}

.history-actions {
  padding: 16px;
  border-bottom: 1px solid var(--border-color);
}

.new-chat-button {
  width: 100%;
  padding: 10px;
  background-color: var(--primary-color);
  color: white;
  border: none;
  border-radius: var(--radius);
  cursor: pointer;
  font-weight: 500;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: var(--transition);
  font-size: 0.95rem;
  box-shadow: var(--shadow-sm);
}

.new-chat-button:hover {
  background-color: var(--primary-hover);
  box-shadow: var(--shadow);
}

.new-chat-button svg.icon {
  margin-right: 8px;
}

.history-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
  scrollbar-width: thin;
}

.history-list::-webkit-scrollbar {
  width: 5px;
}

.history-list::-webkit-scrollbar-thumb {
  background-color: #d1d5db;
  border-radius: 3px;
}

.history-list::-webkit-scrollbar-track {
  background-color: transparent;
}

.history-item {
  padding: 10px 12px;
  border-radius: var(--radius);
  cursor: pointer;
  margin-bottom: 4px;
  transition: var(--transition);
  border: 1px solid transparent;
}

.history-item:hover {
  background-color: rgba(0, 0, 0, 0.03);
  border-color: var(--border-color);
}

.history-item-content {
  display: flex;
  align-items: center;
}

.history-icon {
  color: var(--text-secondary);
  margin-right: 10px;
  flex-shrink: 0;
}

.history-text {
  flex: 1;
  min-width: 0;
}

.history-title {
  font-weight: 500;
  font-size: 0.95rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.history-time {
  font-size: 0.8rem;
  color: var(--text-secondary);
  margin-top: 2px;
}

/* 聊天区域 */
.chat-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  width: 100%;
  transition: margin-left 0.3s cubic-bezier(0.16, 1, 0.3, 1), width 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  margin-left: 0;
  min-height: 0;
}

.chat-header {
  height: var(--header-height);
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  padding: 0 16px;
  background-color: var(--card-bg);
}

.chat-header h1 {
  font-size: 1.25rem;
  font-weight: 600;
}

.toggle-history {
  background: none;
  border: none;
  color: var(--text-color);
  cursor: pointer;
  margin-right: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: var(--radius);
  transition: var(--transition);
}

.toggle-history:hover {
  background-color: rgba(0, 0, 0, 0.05);
}

.chat-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 16px;
  background-color: var(--bg-color);
  scrollbar-width: thin;
  min-height: 0;
}

.chat-content::-webkit-scrollbar {
  width: 5px;
}

.chat-content::-webkit-scrollbar-thumb {
  background-color: #d1d5db;
  border-radius: 3px;
}

.chat-content::-webkit-scrollbar-track {
  background-color: transparent;
}

.chat-messages {
  max-width: 800px;
  margin: 0 auto;
  padding-bottom: 20px;
}

.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 120px;
}

.loading-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid rgba(79, 70, 229, 0.2);
  border-top: 3px solid var(--primary-color);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: 12px;
}

@keyframes spin {
  0% {
    transform: rotate(0deg);
  }

  100% {
    transform: rotate(360deg);
  }
}

.chat-input-area {
  height: var(--input-area-height);
  border-top: 1px solid var(--border-color);
  padding: 12px 16px;
  background-color: var(--card-bg);
}

.input-form {
  display: flex;
  height: 100%;
  max-width: 900px;
  margin: 0 auto;
  position: relative;
}

.message-input {
  flex: 1;
  padding: 12px 16px;
  padding-right: 50px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius);
  font-size: 1rem;
  outline: none;
  transition: var(--transition);
  font-family: inherit;
  box-shadow: var(--shadow-sm);
  background-color: var(--card-bg);
  color: var(--text-color);
}

.message-input:focus {
  border-color: var(--primary-color);
  box-shadow: 0 0 0 2px rgba(79, 70, 229, 0.1);
}

.send-button {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  width: 40px;
  height: 40px;
  background-color: var(--primary-color);
  color: white;
  border: none;
  border-radius: var(--radius);
  cursor: pointer;
  font-weight: 500;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: var(--transition);
}

.send-button:hover {
  background-color: var(--primary-hover);
  transform: translateY(-50%) scale(1.05);
}

.send-icon {
  stroke-width: 2;
}

/* 响应式设计 */
@media (min-width: 768px) {
  .history-sidebar {
    transform: translateX(0);
    position: relative;
    box-shadow: none;
  }

  .chat-container {
    margin-left: 0;
    width: calc(100% - var(--sidebar-width));
  }

  .toggle-history {
    display: none;
    /* 在大屏幕上隐藏菜单按钮 */
  }

  .close-history {
    display: none;
    /* 在大屏幕上隐藏侧边栏关闭按钮 */
  }

  .chat-content {
    padding: 24px;
  }
}

/* 小屏幕模式 */
@media (max-width: 767px) {
  .chat-header h1 {
    font-size: 1.1rem;
  }

  .sidebar-open {
    margin-left: 0;
  }

  .chat-content {
    padding: 16px 12px;
  }
}

/* 暗色模式 */
@media (prefers-color-scheme: dark) {
  :root {
    --bg-color: #0f172a;
    --text-color: #f1f5f9;
    --text-secondary: #94a3b8;
    --border-color: #334155;
  }

  body {
    background-color: var(--bg-color);
    color: var(--text-color);
  }

  .history-sidebar,
  .chat-header,
  .chat-input-area {
    background-color: #1e293b;
    color: var(--text-color);
  }

  .message-input {
    background-color: #1e293b;
    color: var(--text-color);
    border-color: #475569;
  }

  .message-input:focus {
    border-color: var(--primary-color);
    box-shadow: 0 0 0 2px rgba(79, 70, 229, 0.2);
  }

  .history-item:hover {
    background-color: rgba(255, 255, 255, 0.05);
    border-color: #475569;
  }

  .close-history:hover,
  .toggle-history:hover {
    background-color: rgba(255, 255, 255, 0.1);
  }

  .history-list::-webkit-scrollbar-thumb,
  .chat-content::-webkit-scrollbar-thumb {
    background-color: #475569;
  }

  .loading-spinner {
    border-color: rgba(79, 70, 229, 0.3);
  }
}

/* 自定义滚动条样式 */
::-webkit-scrollbar {
  width: 5px;
  height: 5px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: #d1d5db;
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: #9ca3af;
}
</style>
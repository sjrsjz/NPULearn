<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { listen } from '@tauri-apps/api/event';
import hljs from 'highlight.js';
import 'highlight.js/styles/github.min.css';
import LoadingLogo from './components/LoadingLogo.vue';
import Setting from './components/Setting.vue';
import { refreshGlobalStyles } from './themeUtils.ts';
import { getMarkdownStyles, MarkdownStyleOptions } from './markdownStyles';
import { useSettingsProvider } from './composables/useSettings';
import { Window } from '@tauri-apps/api/window';

// 初始化全局设置，在整个应用中提供设置
const {
  notification,
  showNotification,
  initAppSettings
} = useSettingsProvider();

const isAppLoading = ref(true);

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

// 改为空数组，将从后端加载
const chatHistory = ref<ChatHistoryItem[]>([]);
const windowWidth = ref(window.innerWidth);
const isHistoryOpen = ref(windowWidth.value >= 768);
const inputMessage = ref("");
const chatContent = ref<ChatMessage[]>([]);
const isLoading = ref(false);

const showSettings = ref(false);

// 添加流式消息处理需要的状态变量
const isStreaming = ref(false);

// 切换设置界面的显示
function toggleSettings() {
  showSettings.value = !showSettings.value;
  // 如果在小屏幕上打开了历史栏，同时关闭它
  if (showSettings.value && isHistoryOpen.value && windowWidth.value < 768) {
    isHistoryOpen.value = false;
  }
}

// 加载 MathJax
function loadMathJax() {
  return new Promise<void>((resolve) => {
    // 如果已经加载过，直接返回
    if (window.MathJax) {
      resolve();
      return;
    }

    // 配置 MathJax
    window.MathJax = {
      tex: {
        inlineMath: [['$', '$'], ['\\(', '\\)']],
        displayMath: [['$$', '$$'], ['\\[', '\\]']]
      },
      svg: {
        fontCache: 'global'
      },
      startup: {
        pageReady: () => {
          return window.MathJax.startup.defaultPageReady().then(() => {
            resolve();
          });
        },
        defaultPageReady: () => {
          // 这里可以添加其他初始化代码
          return Promise.resolve();
        }
      }
    };

    // 创建脚本元素
    const script = document.createElement('script');
    script.src = 'https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-svg.js';
    script.async = true;
    script.id = 'mathjax-script';
    document.head.appendChild(script);
  });
}

// 在需要时渲染数学公式
function renderMathInElement() {
  if (window.MathJax && window.MathJax.typesetPromise) {
    window.MathJax.typesetPromise([document.querySelector('.chat-messages') as HTMLElement]).catch((err: Error) => {
      console.error('MathJax 渲染错误:', err);
    });
  }
}

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
  // 更新聊天内容，确保样式隔离
  updateChatContent(chatContent.value);
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
    // 从后端API获取聊天历史列表
    chatHistory.value = await invoke("get_chat_history");
    console.log("已加载聊天历史:", chatHistory.value);
  } catch (error) {
    console.error("加载聊天历史失败:", error);
    showNotification("加载聊天历史失败", "error");
  }
  updateChatContent(chatContent.value); // 确保在加载历史后更新内容
}

// 处理聊天内容，隔离样式
const processedChatContent = ref("");

function applyHighlight() {
  nextTick(() => {
    // 查找所有代码块并应用高亮
    document.querySelectorAll('.chat-messages pre code').forEach((el) => {
      hljs.highlightElement(el as HTMLElement);
    });
  });
}



// 修改链接处理函数
function setupExternalLinks() {
  nextTick(() => {
    document.querySelectorAll('.chat-messages a').forEach(link => {
      link.addEventListener('click', async (e) => {
        e.preventDefault();
        const href = link.getAttribute('href');
        if (href) {
          try {
            await writeText(href); // 将链接复制到剪贴板
            showNotification(`链接已复制: ${href}`, 'success');
          } catch (error) {
            console.error('复制链接失败:', error);
            showNotification('复制链接失败', 'error');
          }
        }
      });
    });
  });
}

// 修改 updateChatContent 函数，使其处理ChatMessage数组
function updateChatContent(messages: ChatMessage[]) {
  if (!messages || messages.length === 0) {
    processedChatContent.value = '';
    return;
  }

  // 获取当前主题和字体大小
  const currentTheme = document.documentElement.getAttribute('data-theme') || 'system';
  const currentFontSize = document.documentElement.getAttribute('data-font-size') || 'medium';

  // 创建样式配置
  const styleOptions: MarkdownStyleOptions = {
    theme: currentTheme === 'system' ? 'auto' : (currentTheme as 'light' | 'dark'),
    fontSize: currentFontSize as 'small' | 'medium' | 'large'
  };

  // 获取生成的新样式
  const newStyleContent = getMarkdownStyles(styleOptions);

  // 生成消息HTML
  let messagesHtml = '';


  for (const msg of messages) {
    const messageClass = msg.msgtype.toLowerCase();
    messagesHtml += `
    <div class="message-wrapper ${messageClass}">
      <div class="message-avatar">
        <div class="avatar-icon">
          ${msg.msgtype === 'User' ?
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path><circle cx="12" cy="7" r="4"></circle></svg>' :
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><path d="M8.56 2.75c4.37 6.03 6.02 9.42 8.03 17.72m2.54-15.38c-3.72 4.35-8.94 5.66-16.88 5.85m19.5 1.9c-3.5-.93-6.63-.82-8.94 0-2.58.92-5.01 2.86-7.44 6.32"></path></svg>'
      }
        </div>
        <div class="message-time">${msg.time}</div>
      </div>
      <div class="message-bubble ${messageClass}">
        <div class="message-content markdown-body">
          ${msg.content}
        </div>
      </div>
    </div>
  `;
  }

  // 在updateChatContent函数中更新CSS部分
  processedChatContent.value = `
  <div class="scoped-content">
    ${messagesHtml}
    <style>
      ${newStyleContent}
    </style>
    <style>
      .message-wrapper {
        display: flex;
        margin-bottom: 24px;
        position: relative;
        gap: 8px;
      }
      
      .message-wrapper.user {
        flex-direction: row-reverse;
      }
      
      .message-avatar {
        display: flex;
        flex-direction: column;
        align-items: center;
        margin-top: 4px;
        flex-shrink: 0;
        width: 42px;
      }
      
      .avatar-icon {
        width: 32px;
        height: 32px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        background-color: var(--border-color);
        color: var(--text-color);
        overflow: hidden;
        margin-bottom: 4px;
      }
      
      .message-wrapper.user .avatar-icon {
        background-color: var(--primary-color);
        color: white;
      }
      
      .message-wrapper.assistant .avatar-icon,
      .message-wrapper.system .avatar-icon {
        background-color: #e2e8f0;
        color: #475569;
      }
      
      .avatar-icon svg {
        width: 20px;
        height: 20px;
      }
      
      .message-time {
        font-size: 10px;
        color: var(--text-secondary);
        text-align: center;
        white-space: nowrap;
        margin-top: 2px;
      }
      
      .message-bubble {
        max-width: calc(85% - 42px);
        display: flex;
        flex-direction: column;
        position: relative;
        transform: translateY(15px);
      }
      
      .message-content {
        padding: 12px 16px;
        border-radius: 18px;
        overflow-wrap: break-word;
        overflow: hidden;
      }
      
      .message-wrapper.user .message-content {
        background-color: var(--primary-color);
        color: white;
        border-top-right-radius: 4px;
      }
      
      .message-wrapper.assistant .message-content,
      .message-wrapper.system .message-content {
        background-color: var(--card-bg);
        border: 1px solid var(--border-color);
        border-top-left-radius: 4px;
      }
      
      /* 暗黑模式适配 */
      @media (prefers-color-scheme: dark) {
        .message-wrapper.assistant .message-content,
        .message-wrapper.system .message-content {
          background-color: #2d3748;
          border-color: #4a5568;
        }
        
        .message-wrapper.assistant .avatar-icon,
        .message-wrapper.system .avatar-icon {
          background-color: #4a5568;
          color: #e2e8f0;
        }
      }
      
      /* 移动端优化 */
      @media (max-width: 767px) {
        .message-bubble {
          max-width: calc(90% - 42px);
        }
        
        .message-content {
          padding: 10px 14px;
        }
        
        .avatar-icon {
          width: 28px;
          height: 28px;
        }
        
        .avatar-icon svg {
          width: 16px;
          height: 16px;
        }
      }
    </style>
  </div>
`;

  // 下一个 tick 后处理样式和代码高亮
  nextTick(() => {
    // 应用代码高亮
    applyHighlight();

    // 渲染数学公式
    renderMathInElement();

    // 设置外部链接处理
    setupExternalLinks();

    // 滚动到底部
    scrollToBottom();
  });
}


// 流式消息处理相关函数
async function setupStreamListeners() {
  // 监听流式消息事件
  const unlistenStream = await listen('stream-message', (event) => {
    // 将后端发送的聊天历史更新到前端
    const chatData = event.payload as ChatHistory;
    chatContent.value = chatData.content;
    // 更新聊天内容显示
    updateChatContent(chatContent.value);
  });

  // 监听流完成事件
  const unlistenComplete = await listen('stream-complete', async () => {
    isStreaming.value = false;
    isLoading.value = false;

    // 重新加载聊天历史
    await loadChatHistory();
  });

  // 在组件卸载时清理事件监听
  onUnmounted(() => {
    unlistenStream();
    unlistenComplete();
  });
}

// 流式发送消息
async function sendStreamMessage() {
  if (!inputMessage.value.trim()) return;

  isStreaming.value = true;
  isLoading.value = true;

  try {
    // 清空输入框但保存消息内容
    const message = inputMessage.value;
    inputMessage.value = "";

    // 调用后端的流式处理函数
    await invoke("process_message_stream", { message });

    // 处理将在事件监听器中完成
  } catch (error) {
    console.error("消息发送失败:", error);
    showNotification("消息发送失败", "error");
    isStreaming.value = false;
    isLoading.value = false;
  }
}

// 自动滚动到底部
function scrollToBottom() {
  nextTick(() => {
    const chatContent = document.querySelector('.chat-content');
    if (chatContent) {
      chatContent.scrollTop = chatContent.scrollHeight;
    }
  });
}

// 创建新对话
async function createNewChat() {
  isLoading.value = true;
  try {
    // 调用后端创建新对话API
    chatContent.value = await invoke("create_new_chat");
    // 更新聊天内容显示
    updateChatContent(chatContent.value);
    // 重新加载历史记录以显示新创建的对话
    await loadChatHistory();
    showNotification("已创建新对话", "success");
  } catch (error) {
    console.error("创建新对话失败:", error);
    showNotification("创建新对话失败", "error");
  } finally {
    isLoading.value = false;
  }
}

// 监听 chatContent 变化，确保 MathJax 重新渲染
watch(chatContent, () => {
  nextTick(() => {
    console.log("聊天内容变化:", chatContent.value);
    refreshGlobalStyles();
    renderMathInElement();
  });
});

// 监听主题变化，更新聊天内容
watch(() => document.documentElement.getAttribute('data-theme'), (newTheme) => {
  console.log("主题变化:", newTheme);
  // 当主题变化时，重新应用样式
  if (chatContent.value) {
    updateChatContent(chatContent.value);
  }
});


// 组件加载时初始化对话内容
onMounted(async () => {
  try {
    // 初始化应用设置
    await initAppSettings();

    // 加载 MathJax
    await loadMathJax();

    // 设置流式消息监听器
    await setupStreamListeners();

    // 加载聊天历史和当前对话内容
    await loadChatHistory();

    // 尝试获取当前活跃的聊天内容
    const content = await invoke("get_chat_html");
    chatContent.value = content as ChatMessage[];
    updateChatContent(chatContent.value);

    // 所有内容加载完成后，隐藏启动logo
    setTimeout(() => {
      isAppLoading.value = false;
    }, 1500); // 添加短暂延迟，让过渡更平滑
  } catch (error) {
    console.error("初始化失败:", error);
    // 即使出错，也需要隐藏加载动画
    isAppLoading.value = false;
  }

  window.addEventListener('resize', handleResize);

  // 添加事件监听器以响应主题和字体大小变化
  window.addEventListener('themeChanged', (e: Event) => {
    const customEvent = e as CustomEvent;
    console.log('主题已变更:', customEvent.detail);
    if (chatContent.value) {
      updateChatContent(chatContent.value);
    }
  });

  window.addEventListener('fontSizeChanged', (e: Event) => {
    const customEvent = e as CustomEvent;
    console.log('字体大小已变更:', customEvent.detail);
    if (chatContent.value) {
      updateChatContent(chatContent.value);
    }
  });

});

// 组件卸载时清理事件监听
onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  // 清除主题和字体大小变化的事件监听
  window.removeEventListener('themeChanged', (_: Event) => { });
  window.removeEventListener('fontSizeChanged', (_: Event) => { });
});


const minimizeWindow = () => Window.getCurrent().minimize();
const toggleMaximize = async () => {
  const currentWindow = Window.getCurrent();
  const isMaximized = await currentWindow.isMaximized();
  isMaximized ? currentWindow.unmaximize() : currentWindow.maximize();
};
const closeWindow = () => Window.getCurrent().close();
</script>

<template>
  <div class="app-container">
    <!-- 自定义标题栏 - 移到最外层，作为整个应用的顶部 -->
    <div class="custom-titlebar" data-tauri-drag-region>
      <div class="app-icon">
        <img src="./assets/logo.png" alt="NPULearn" />
      </div>
      <div class="title" data-tauri-drag-region>NPULearn</div>
      <div class="window-controls">
        <button class="minimize" @click="minimizeWindow">
          <svg viewBox="0 0 12 12">
            <rect x="2" y="5.5" width="8" height="1" />
          </svg>
        </button>
        <button class="maximize" @click="toggleMaximize">
          <svg viewBox="0 0 12 12">
            <rect x="2" y="2" width="8" height="8" style="fill:none;stroke-width:1" />
          </svg>
        </button>
        <button class="close" @click="closeWindow">
          <svg viewBox="0 0 12 12">
            <line x1="2" y1="2" x2="10" y2="10" />
            <line x1="10" y1="2" x2="2" y2="10" />
          </svg>
        </button>
      </div>
    </div>

    <div class="app-content">
      <LoadingLogo :show="isAppLoading" />

      <div v-if="showSettings" class="settings-modal">
        <div class="settings-modal-overlay" @click="toggleSettings"></div>
        <div class="settings-modal-content">
          <Setting @close="toggleSettings" />
        </div>
      </div>

      <!-- 通知组件 -->
      <div v-if="notification.visible" class="notification" :class="notification.type">
        <div class="notification-content">
          <svg v-if="notification.type === 'success'" xmlns="http://www.w3.org/2000/svg" width="16" height="16"
            viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
            <polyline points="22 4 12 14.01 9 11.01"></polyline>
          </svg>
          <span>{{ notification.message }}</span>
        </div>
      </div>

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
        <!-- 历史列表其余部分保持不变 -->
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

        <div class="history-footer">
          <button @click="toggleSettings" class="settings-button">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="3"></circle>
              <path
                d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1-2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z">
              </path>
            </svg>
            设置
          </button>
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
          <h1>NPULearn</h1>
        </header>

        <!-- 聊天内容区域 -->
        <div class="chat-content">
          <div v-if="isLoading" class="loading">
            <div class="loading-spinner"></div>
            <div class="loading-text">加载中...</div>
          </div>
          <div v-else-if="!processedChatContent" class="empty-chat">
            <div class="empty-chat-icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
              </svg>
            </div>
            <h3>开始一个新对话</h3>
            <p>在下方输入框中提问，开始与AI助手交流</p>
          </div>
          <div v-html="processedChatContent" class="chat-messages"></div>
        </div>
        <!-- 底部输入区 -->
        <div class="chat-input-area">
          <form @submit.prevent="sendStreamMessage" class="input-form">
            <input v-model="inputMessage" type="text" placeholder="输入消息..." class="message-input" />
            <button type="submit" class="send-button" :disabled="isStreaming">
              <svg v-if="!isStreaming" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"
                fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                class="send-icon">
                <line x1="22" y1="2" x2="11" y2="13"></line>
                <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
              </svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                class="loading-icon">
                <circle cx="12" cy="12" r="10"></circle>
                <path d="M12 6v6l4 2"></path>
              </svg>
            </button>
          </form>
        </div>
      </main>
    </div>
  </div>
</template>



<style>
.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100%;
  overflow: hidden;
  position: relative;
  background-color: var(--bg-color);
  margin: 0;
  padding: 0;
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--border-color);
}

.app-content {
  display: flex;
  flex: 1;
  position: relative;
  overflow: hidden;
}

.custom-titlebar {
  height: 32px;
  background-color: var(--card-bg);
  display: flex;
  align-items: center;
  padding: 0 8px;
  user-select: none;
  width: 100%;
  z-index: 101;
  /* 确保标题栏在最上层 */
}

.close {
  color: var(--text-color);
}

.minimize {
  color: var(--text-color);
}

.maximize {
  color: var(--text-color);
}

.app-icon {
  display: flex;
  align-items: center;
  margin-right: 8px;
}

.app-icon img {
  width: 16px;
  height: 16px;
}

.title {
  flex: 1;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-color);
}

.window-controls {
  display: flex;
}

.window-controls button {
  width: 32px;
  height: 32px;
  background: transparent;
  border: none;
  outline: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.window-controls button:hover {
  background-color: rgba(0, 0, 0, 0.05);
}

.window-controls button.close:hover {
  background-color: #e81123;
  color: white;
}

.window-controls svg {
  width: 10px;
  height: 10px;
  stroke: currentColor;
  stroke-width: 1;
  fill: none;
}

/* 暗色模式 */
@media (prefers-color-scheme: dark) {
  .window-controls button:hover {
    background-color: rgba(255, 255, 255, 0.1);
  }
}
</style>


<style>
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&display=swap');

html,
body {
  margin: 0;
  padding: 0;
  overflow: hidden;
  height: 100%;
  width: 100%;
}


:root {
  --primary-color: #3b82f6;
  /* 更新为蓝色系 */
  --light-primary-color: #60a5fa;
  --primary-hover: #2563eb;
  --bg-color: #f9fafb;
  --dark-bg-color: #0f172a;
  --text-color: #1f2937;
  --text-secondary: #64748b;
  --dark-text-color: #f3f4f6;
  --dark-text-secondary: #9ca3af;
  --border-color: #e5e7eb;
  --dark-border-color: #334155;
  --card-bg: #ffffff;
  --dark-card-bg: #1e293b;
  --sidebar-width: 280px;
  --header-height: 64px;
  --input-area-height: 80px;
  --shadow-sm: 0 2px 4px rgba(0, 0, 0, 0.05);
  --shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
  --radius-sm: 6px;
  --radius: 8px;
  --radius-lg: 12px;
  --transition: all 0.2s ease;
}


* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
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

body {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  line-height: 1.5;
  background-color: var(--bg-color);
  color: var (--text-color);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.empty-chat {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 60px 20px;
  height: 100%;
  color: var(--text-secondary);
}

.empty-chat-icon {
  margin-bottom: 20px;
  color: var(--text-secondary);
  background-color: var(--card-bg);
  width: 80px;
  height: 80px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: var(--shadow-sm);
  border: 1px solid var(--border-color);
}

.empty-chat h3 {
  margin-bottom: 8px;
  font-weight: 600;
  font-size: var(--font-size-lg);
  color: var(--text-color);
}

.empty-chat p {
  max-width: 320px;
  font-size: var(--font-size-base);
}

/* 暗色模式适配 */
@media (prefers-color-scheme: dark) {
  .empty-chat-icon {
    background-color: #1e293b;
    border-color: #334155;
  }
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

.history-sidebar {
  width: var(--sidebar-width);
  background-color: var(--card-bg);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  position: fixed;
  top: 32px;
  /* 调整顶部位置，留出标题栏的高度 */
  left: 0;
  bottom: 0;
  z-index: 100;
  transform: translateX(-100%);
  box-shadow: var(--shadow);
}

.chat-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  width: 100%;
  transition: margin-left 0.3s cubic-bezier(0.16, 1, 0.3, 1), width 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  margin-left: 0;
  min-height: 0;
  height: calc(100vh - 32px);
  /* 减去标题栏高度 */
  overflow: hidden;
}

/* 响应式设计调整 */
@media (min-width: 768px) {
  .history-sidebar {
    transform: translateX(0);
    position: relative;
    box-shadow: none;
    top: 0;
    /* 在大屏幕上不需要相对于顶部定位 */
  }

  .chat-container {
    margin-left: 0;
    width: calc(100% - var(--sidebar-width));
  }
}


.history-open {
  transform: translateX(0);
}

.history-header {
  height: var(--header-height);
  padding: 0 16px;
  border-bottom: 0px solid var(--border-color);
  display: flex;
  justify-content: space-between;
  align-items: center;
  position: relative;
}

.history-header h3 {
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--text-color);
  margin: 0;
  padding: 0;
  flex: 1;
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
  border-bottom: 0px solid var (--border-color);
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
  font-size: var(--font-size-base);
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
  color: var(--text-color);
  font-weight: 500;
  font-size: var(--font-size-base);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.history-time {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  margin-top: 2px;
}


.settings-modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.settings-modal-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(2px);
}

.settings-modal-content {
  position: relative;
  width: 90%;
  max-width: 800px;
  max-height: 90vh;
  border-radius: var(--radius-lg);
  background-color: var(--card-bg);
  box-shadow: var(--shadow);
  overflow: hidden;
  animation: modal-in 0.3s ease forwards;
  z-index: 1001;
}

@keyframes modal-in {
  from {
    opacity: 0;
    transform: scale(0.95);
  }

  to {
    opacity: 1;
    transform: scale(1);
  }
}

/* 历史栏底部的设置按钮 */
.history-footer {
  padding: 12px 16px;
  border-top: 1px solid var(--border-color);
  margin-top: auto;
}

.settings-button {
  width: 100%;
  padding: 10px;
  background-color: transparent;
  color: var(--text-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius);
  cursor: pointer;
  font-weight: 500;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: var(--transition);
  font-size: var(--font-size-base);
  gap: 8px;
}

.settings-button:hover {
  background-color: rgba(0, 0, 0, 0.05);
  border-color: var(--text-color);
}

/* 头部设置按钮 */
.header-settings-button {
  background: none;
  border: none;
  color: var(--text-color);
  cursor: pointer;
  margin-left: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: var(--radius);
  transition: var(--transition);
}

.header-settings-button:hover {
  background-color: rgba(0, 0, 0, 0.05);
}

/* 暗色模式下的设置按钮样式 */
@media (prefers-color-scheme: dark) {
  .settings-button:hover {
    background-color: rgba(255, 255, 255, 0.1);
    border-color: var(--text-color);
  }

  .header-settings-button:hover {
    background-color: rgba(255, 255, 255, 0.1);
  }
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
  /* 固定高度为视口高度 */
  overflow: hidden;
  /* 防止整体溢出 */
}

.chat-header {
  grid-row: 1;
  border-bottom: 0px solid var(--border-color);
  display: flex;
  align-items: center;
  padding: 0 16px;
  background-color: var(--card-bg);
  z-index: 10;
  /* 绝对固定高度，防止挤压 */
  height: var(--header-height);
  min-height: var(--header-height);
  /* 确保顶部固定 */
  position: sticky;
  top: 0;
}

.chat-header h1 {
  color: var(--text-color);
  font-size: var(--font-size-lg);
  font-weight: 600;
  line-height: 1;
  /* 固定行高 */
  margin: 0;
  padding: 0;
  display: flex;
  align-items: center;
  height: 100%;
  /* 填充父容器 */
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
  /* 确保内容可以被压缩 */
  position: relative;
  overscroll-behavior: contain;
  /* 防止滚动传播 */
}

.chat-content::-webkit-scrollbar {
  width: 5px;
}

.chat-content::-webkit-scrollbar-thumb {
  background-color: #d1d5db;
  border-radius: 3px;
}

chat-content::-webkit-scrollbar-track {
  background-color: transparent;
}

chat-messages .scoped-content {
  all: initial;
  /* 重置所有样式 */
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  color: var(--text-color);
  line-height: 1.5;
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
  font-size: var(--font-size-lg);
  color: var(--text-color);

}

.loading-text {
  font-size: var(--font-size-base);
  color: var(--text-secondary);
  font-weight: 500;
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
  grid-row: 3;
  border-top: 1px solid var(--border-color);
  padding: 12px 16px;
  background-color: var(--card-bg);
  z-index: 10;
  /* 绝对固定高度，防止挤压 */
  height: var(--input-area-height);
  min-height: var(--input-area-height);
  /* 确保底部固定 */
  position: sticky;
  bottom: 0;
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
  font-size: var(--font-size-base);
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

send-icon {
  stroke-width: 2;
}

chat-messages .mjx-chtml {
  margin: 0.5em 0;
  font-size: var(--font-size-lg);
}

chat-messages .mjx-math {
  max-width: 100%;
  overflow-x: auto;
  overflow-y: hidden;
}

chat-messages .mjx-chtml.MJXc-display {
  margin: 1em 0;
  padding: 0.5em 0;
  overflow-x: auto;
  overflow-y: hidden;
  text-align: center;
}

chat-messages .MJX-TEX {
  text-align: center;
}

chat-messages .mjx-container {
  padding: 6px 0;
}

/* 暗色模式下的 MathJax 样式 */
@media (prefers-color-scheme: dark) {
  chat-messages .mjx-math {
    color: #f1f5f9;
  }
}

/* 通知样式 */
.notification {
  position: fixed;
  top: 16px;
  right: 16px;
  padding: 12px 16px;
  border-radius: var(--radius);
  background-color: var(--card-bg);
  box-shadow: var(--shadow);
  z-index: 1000;
  max-width: 400px;
  animation: slide-in 0.3s ease forwards;
  border-left: 4px solid;
  color: var(--text-color);
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

.notification.warning {
  border-left-color: #f59e0b;
}

.notification-content {
  display: flex;
  align-items: center;
  gap: 8px;
}

.notification-content svg {
  flex-shrink: 0;
  color: #10b981;
}

.notification.error svg {
  color: #ef4444;
}

.notification.info svg {
  color: #3b82f6;
}

.notification.warning svg {
  color: #f59e0b;
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

/* 暗色模式下的通知样式 */
@media (prefers-color-scheme: dark) {
  .notification {
    background-color: var(--dark-card-bg);
    color: var(--dark-text-color);
  }

  .notification.success {
    border-left-color: #10b981;
    background-color: var(--dark-card-bg);
  }

  .notification.error {
    border-left-color: #ef4444;
    background-color: var(--dark-card-bg);
  }

  .notification.info {
    border-left-color: #3b82f6;
    background-color: var(--dark-card-bg);
  }

  .notification.warning {
    border-left-color: #f59e0b;
    background-color: var(--dark-card-bg);
  }

  .notification-content svg {
    color: #34d399;
    /* 更亮的绿色，增强暗色模式下的对比度 */
  }

  .notification.error svg {
    color: #f87171;
    /* 更亮的红色 */
  }

  .notification.info svg {
    color: #60a5fa;
    /* 更亮的蓝色 */
  }

  .notification.warning svg {
    color: #fbbf24;
    /* 更亮的黄色 */
  }

  /* 为暗色模式添加更明显的阴影 */
  .notification {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }
}

chat-messages a {
  color: var(--primary-color);
  text-decoration: none;
  border-bottom: 0px dashed var(--primary-color);
  cursor: pointer;
  position: relative;
  padding-right: 16px;
}

chat-messages a::after {
  content: '📋';
  font-size: var(--font-size-sm);
  position: absolute;
  right: 0;
  top: 0;
  opacity: 0.7;
}

chat-messages a:hover {
  opacity: 0.8;
}

chat-messages a:active {
  opacity: 0.6;
}

/* 暗色模式下的链接样式 */
@media (prefers-color-scheme: dark) {
  chat-messages a {
    color: #6366f1;
    border-bottom-color: #6366f1;
  }
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
    flex: 1;
    overflow-y: auto;
    padding: 24px 16px;
    background-color: var(--bg-color);
    scrollbar-width: thin;
    min-height: 0;
    position: relative;
    /* 确保内容正确定位 */
  }
}

/* 小屏幕模式 */
@media (max-width: 767px) {
  .chat-header h1 {
    font-size: var(--font-size-lg);
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
    color: var(--text色);
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
    color: var(--primary-color);
    border-color: rgba(79, 70, 229, 0.3);
  }

  .loading-text {
    color: var(--text-secondary);
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
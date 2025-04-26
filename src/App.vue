<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { listen } from '@tauri-apps/api/event';
import hljs from 'highlight.js';
import 'highlight.js/styles/github.min.css';
import 'highlight.js/styles/github-dark.min.css'; // 暗色主题
import LoadingLogo from './components/LoadingLogo.vue';
import Setting from './components/Setting.vue';
import { refreshGlobalStyles } from './themeUtils.ts';
import mermaid from 'mermaid'; // 导入Mermaid.js库

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

// 添加新的动画状态控制变量
const fadeInMessages = ref(true);
const messageTransition = ref(true);

// 切换设置界面的显示
function toggleSettings() {
  showSettings.value = !showSettings.value;
  // 如果在小屏幕上打开了历史栏，同时关闭它
  if (showSettings.value && isHistoryOpen.value && windowWidth.value < 768) {
    isHistoryOpen.value = false;
  }
}

// 初始化Mermaid.js配置
function initMermaid() {
  mermaid.initialize({
    startOnLoad: false,
    theme: document.documentElement.getAttribute('data-theme') === 'dark' || 
          (document.documentElement.getAttribute('data-theme') === 'system' && 
           window.matchMedia('(prefers-color-scheme: dark)').matches) ? 'dark' : 'default',
    securityLevel: 'loose',
    flowchart: { 
      htmlLabels: true,
      useMaxWidth: true,
    },
    fontSize: 14
  });
}

// 渲染Mermaid图表
async function renderMermaidDiagrams() {
  const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
    (document.documentElement.getAttribute('data-theme') === 'system' &&
      window.matchMedia('(prefers-color-scheme: dark)').matches);

  // 动态更新主题
  mermaid.initialize({
    theme: isDark ? 'dark' : 'default',
    securityLevel: 'loose',
    logLevel: 'debug', // 将日志级别改为 debug 以获取更多信息
    startOnLoad: false
  });

  try {
    // 查找所有需要渲染的UML元素
    const umlElements = document.querySelectorAll('.chat-messages .mermaid-container');

    for (const element of umlElements) {
      const id = element.getAttribute('data-diagram-id');
      const encodedContent = element.getAttribute('data-diagram-content');

      if (encodedContent && id) {
        let content = ''; // 在 try 块外部定义 content
        try {
          // 清空现有内容
          element.innerHTML = '<div class="mermaid-loading">UML图表渲染中...</div>';

          // 正确解码内容
          content = decodeURIComponent(encodedContent); // 分配解码后的内容
          console.log(`渲染图表 ID: ${id}`);
          console.log("Encoded Content:", encodedContent);
          console.log("Decoded Content for Mermaid:", content); // 记录传递给 Mermaid 的确切内容

          // 使用Mermaid渲染图表
          // 确保 'content' 是一个非空字符串
          if (typeof content === 'string' && content.length > 0) {
            const { svg } = await mermaid.render(id, content);
            element.innerHTML = svg;
            // 添加图表加载完成的动画效果
            element.classList.add('loaded');
          } else {
            // 如果解码后的内容无效或为空，则抛出错误
            throw new Error("解码后的内容为空或无效。");
          }

        } catch (error) {
          // 记录更详细的错误信息和失败的内容
          console.error(`渲染图表 ID ${id} 失败:`, error);
          console.error("失败的内容 (decoded):", content); // 记录导致失败的解码后内容
          element.innerHTML = `
            <div class="mermaid-error">
              <p>UML图表渲染失败</p>
              <pre>${error}</pre>
              <div class="mermaid-source">
                <details>
                  <summary>查看原始图表代码</summary>
                  <pre>${content}</pre> <!-- 在这里显示解码后的内容 -->
                </details>
              </div>
            </div>
          `;
        }
      } else {
        // 如果容器缺少必要的属性，则发出警告
        console.warn("发现缺少必要属性（id 或 content）的 Mermaid 容器。", element);
      }
    }
  } catch (error) {
    console.error("处理Mermaid图表失败:", error);
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
  // 如果正在流式输出消息，禁止切换聊天
  if (isStreaming.value) {
    showNotification("请等待当前消息输出完成", "error");
    return;
  }
  
  // 调用后端加载特定对话
  console.log(`加载对话 ${id}`);

  // 添加淡出效果但确保立即恢复可见性
  fadeInMessages.value = false;
  
  // 短暂延迟后开始加载
  setTimeout(async () => {
    isLoading.value = true;
    try {
      // 调用 Rust 函数加载特定对话内容
      chatContent.value = await invoke("get_chat_by_id", { id });
    } catch (error) {
      console.error("加载对话失败:", error);
    } finally {
      isLoading.value = false;
      // 更新聊天内容，确保样式隔离
      updateChatContent(chatContent.value);
      
      // 强制立即显示内容
      fadeInMessages.value = true;
      
      // 在移动设备上选择后自动关闭侧边栏
      if (windowWidth.value < 768) {
        isHistoryOpen.value = false;
      }
    }
  }, 300);
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
      const href = link.getAttribute('href');
      
      // 检查是否是button://格式的链接，将其转换为按钮样式
      if (href && href.startsWith('button://')) {
        // 创建按钮元素替换链接
        const buttonElement = document.createElement('button');
        buttonElement.className = 'markdown-button';
        buttonElement.textContent = link.textContent || '点击发送';
        
        // 从URL中提取消息内容，确保正确处理中文等字符
        const message = decodeURIComponent(href.substring(9));
        
        // 设置点击事件
        buttonElement.addEventListener('click', async (e) => {
          e.preventDefault();
          
          // 如果正在流式输出消息，禁止发送新消息
          if (isStreaming.value) {
            showNotification("请等待当前消息输出完成", "error");
            return;
          }
          
          if (message.trim()) {
            // 设置输入框内容
            inputMessage.value = message;
            // 发送消息
            await sendStreamMessage();
            showNotification("已发送按钮消息", "success");
          }
        });
        
        // 替换原始链接
        link.parentNode?.replaceChild(buttonElement, link);
      } else if (href) {
        // 普通链接的处理保持不变
        link.addEventListener('click', async (e) => {
          e.preventDefault();
          try {
            await writeText(href);
            showNotification(`链接已复制: ${href}`, 'success');
          } catch (error) {
            console.error('复制链接失败:', error);
            showNotification('复制链接失败', 'error');
          }
        });
      }
    });
  });
}
// 处理UML标签
function processUmlContent(html: string): string {
  // 查找所有 <uml>...</uml> 标签并替换为 mermaid 容器
  const umlRegex = /<uml>([\s\S]*?)<\/uml>/g;
  let counter = 0;

  return html.replace(umlRegex, (match, umlContent) => {
    const diagramId = `mermaid-diagram-${Date.now()}-${counter++}`;

    // 提取```mermaid和```之间的内容
    const mermaidMatch = umlContent.match(/```mermaid\s*([\s\S]*?)```/);
    let rawContent = ""; // 使用不同的变量名以清晰起见

    if (mermaidMatch && mermaidMatch[1]) {
      // 获取原始内容并去除首尾空格
      rawContent = mermaidMatch[1].trim();
    } else {
      // 如果没有使用```mermaid格式，直接使用内容并去除首尾空格
      rawContent = umlContent.trim();
    }

    // 仅对原始、整理过的内容进行一次编码
    const encodedContent = encodeURIComponent(rawContent);
    console.log("处理UML内容 (Raw):", rawContent); // 记录编码前的原始内容
    console.log("处理UML内容 (Encoded):", encodedContent); // 记录编码后的内容

    // 返回一个容器，将在渲染后处理
    return `<div class="mermaid-container" data-diagram-id="${diagramId}" data-diagram-content="${encodedContent}">
              <div class="mermaid-loading">UML图表加载中...</div>
            </div>`;
  });
}
// 修改 updateChatContent 函数，使其处理主题更改
function updateChatContent(messages: ChatMessage[]) {
  if (!messages || messages.length === 0) {
    processedChatContent.value = '';
    return;
  }

  // 获取当前主题和字体大小
  const currentTheme = document.documentElement.getAttribute('data-theme') || 'system';
  const currentFontSize = document.documentElement.getAttribute('data-font-size') || 'medium';
  const isDark = currentTheme === 'dark' || 
                (currentTheme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);

  console.log(`更新聊天内容，主题: ${currentTheme}, 字体大小: ${currentFontSize}, 是否暗色: ${isDark}`);

  // 生成消息HTML
  let messagesHtml = '';

  for (const msg of messages) {
    const messageClass = msg.msgtype.toLowerCase();
    
    // 根据消息类型确定布局方式
    const isUserMessage = msg.msgtype === 'User';
    
    // 处理消息内容中的UML标签
    const processedContent = processUmlContent(msg.content);
    
    messagesHtml += `
    <div class="message-wrapper ${messageClass} ${isUserMessage ? 'user-message-right' : ''}">
      <div class="message-avatar">
        <div class="avatar-icon">
          ${isUserMessage ?
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path><circle cx="12" cy="7" r="4"></circle></svg>' :
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path><path d="M9 9h6"></path><path d="M9 13h6"></path></svg>'
      }
        </div>
        <div class="message-time">${msg.time}</div>
      </div>
      <div class="message-bubble ${messageClass}">
        <div class="message-content markdown-body">
          ${processedContent}
        </div>
        <div class="message-actions">
          <button class="action-button copy-button" data-content="${encodeURIComponent(msg.content)}" title="复制内容">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
          </button>
          ${!isUserMessage ? 
            `<button class="action-button regenerate-button" data-message-index="${messages.indexOf(msg)}" title="重新生成">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1-18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.2"/>
              </svg>
            </button>` : ''
          }
        </div>
      </div>
    </div>
  `;
  }

  // 根据当前主题添加适当的 CSS 变量
  const lightThemeVars = `
    --message-bg: var(--card-bg, #ffffff);
    --message-color: var(--text-color, #1f2937);
    --message-border: var(--border-color, #e5e7eb);
    --message-code-bg: rgba(0, 0, 0, 0.05);
    --message-blockquote-color: #6b7280;
    --message-blockquote-border: #e5e7eb;
    --message-table-border: #e5e7eb;
    --message-table-th-bg: #f9fafb;
  `;

  const darkThemeVars = `
    --message-bg: #2d3748;
    --message-color: #f1f5f9;
    --message-border: #4a5568;
    --message-code-bg: rgba(71, 85, 105, 0.3);
    --message-blockquote-color: #94a3b8;
    --message-blockquote-border: #475569;
    --message-table-border: #475569;
    --message-table-th-bg: #1e293b;
  `;

  // 在updateChatContent函数中更新CSS部分
  processedChatContent.value = `
  <div class="scoped-content ${fadeInMessages.value ? 'fade-in' : ''} ${messageTransition.value ? 'message-transition' : ''}" data-theme="${isDark ? 'dark' : 'light'}">
    ${messagesHtml}
    <style>
      .scoped-content {
        ${isDark ? darkThemeVars : lightThemeVars}
        opacity: 0;
        transition: opacity 0.5s ease;
      }
      
      .scoped-content.fade-in {
        opacity: 1;
      }
      
      .message-wrapper {
        display: flex;
        margin-bottom: 28px;
        position: relative;
        gap: 12px;
        /* 移除初始透明度设置，保证内容始终可见 */
        opacity: 1;
        transform: translateY(0);
        transition: transform 0.4s ease, opacity 0.4s ease;
      }
      
      /* 仅当启用消息过渡时应用进入动画 */
      .message-transition .message-wrapper {
        animation: message-appear 0.4s ease forwards;
      }
      
      @keyframes message-appear {
        from {
          transform: translateY(20px);
          opacity: 0;
        }
        to {
          transform: translateY(0);
          opacity: 1;
        }
      }
      
      /* 确保在动画完成后保持可见 */
      .message-transition .message-wrapper {
        opacity: 1;
        transform: translateY(0);
      }
      
      /* 每个消息延迟出现 */
      .message-transition .message-wrapper:nth-child(1) { animation-delay: 0.1s; }
      .message-transition .message-wrapper:nth-child(2) { animation-delay: 0.15s; }
      .message-transition .message-wrapper:nth-child(3) { animation-delay: 0.2s; }
      .message-transition .message-wrapper:nth-child(4) { animation-delay: 0.25s; }
      .message-transition .message-wrapper:nth-child(5) { animation-delay: 0.3s; }
      .message-transition .message-wrapper:nth-child(n+6) { animation-delay: 0.35s; }
      
      .message-avatar {
        display: flex;
        flex-direction: column;
        align-items: center;
        margin-top: 4px;
        flex-shrink: 0;
        width: 42px;
        /* 移除初始透明度设置，确保内容始终可见 */
        transform: scale(1);
        opacity: 1;
      }
      
      /* 仅在启用动画时应用头像弹跳效果 */
      .message-transition .message-avatar {
        animation: avatar-bounce 0.6s cubic-bezier(0.175, 0.885, 0.32, 1.275) forwards;
        animation-delay: 0.3s;
      }
      
      @keyframes avatar-bounce {
        0% {
          transform: scale(0.8);
          opacity: 0.5;
        }
        60% {
          transform: scale(1.1);
          opacity: 1;
        }
        100% {
          transform: scale(1);
          opacity: 1;
        }
      }
      
      .avatar-icon {
        width: 36px;
        height: 36px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        background-color: var(--border-color);
        color: var(--text-color);
        overflow: hidden;
        margin-bottom: 6px;
        box-shadow: 0 2px 4px ${isDark ? 'rgba(0, 0, 0, 0.3)' : 'rgba(0, 0, 0, 0.1)'};
      }
      
      .message-wrapper.user .avatar-icon {
        background-color: var(--primary-color);
        color: white;
      }
      
      .message-wrapper.assistant .avatar-icon,
      .message-wrapper.system .avatar-icon {
        background-color: ${isDark ? '#4a5568' : '#e2e8f0'};
        color: ${isDark ? '#e2e8f0' : '#475569'};
      }
      
      .avatar-icon svg {
        width: 22px;
        height: 22px;
      }
      
      .message-time {
        font-size: 11px;
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
        /* 添加气泡过渡效果 */
        transition: transform 0.3s ease, box-shadow 0.3s ease;
      }
      

      .message-content {
        padding: 14px 18px;
        border-radius: 18px;
        overflow-wrap: break-word;
        overflow: hidden;
        box-shadow: 0 2px 8px ${isDark ? 'rgba(0, 0, 0, 0.3)' : 'rgba(0, 0, 0, 0.08)'};
      }
      
      .message-wrapper.user .message-content {
        background-color: var(--primary-color);
        color: white;
        border-top-right-radius: 4px;
      }
      
      .message-wrapper.assistant .message-content,
      .message-wrapper.system .message-content {
        background-color: ${isDark ? 'var(--message-bg)' : 'var(--card-bg)'};
        border: 1px solid ${isDark ? 'var(--message-border)' : 'var(--border-color)'};
        border-top-left-radius: 4px;
        color: ${isDark ? 'var(--message-color)' : 'var(--text-color)'};
      }
      
      /* Mermaid图表容器样式 */
      .mermaid-container {
        background-color: ${isDark ? '#1e293b' : '#f6f8fa'};
        border-radius: 6px;
        margin: 16px 0;
        padding: 16px;
        overflow: hidden;
        box-shadow: 0 2px 6px ${isDark ? 'rgba(0, 0, 0, 0.4)' : 'rgba(0, 0, 0, 0.08)'};
        border: 1px solid ${isDark ? '#334155' : '#e1e4e8'};
        position: relative;
        transition: transform 0.3s ease, box-shadow 0.3s ease;
        min-height: 100px;
        display: flex;
        justify-content: center;
        align-items: center;
        text-align: center;
      }
      
      .mermaid-container.loaded {
        animation: diagram-fade-in 0.5s ease forwards;
      }
      
      @keyframes diagram-fade-in {
        from {
          opacity: 0.5;
          transform: scale(0.98);
        }
        to {
          opacity: 1;
          transform: scale(1);
        }
      }
      
      .mermaid-container:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 12px ${isDark ? 'rgba(0, 0, 0, 0.5)' : 'rgba(0, 0, 0, 0.15)'};
      }
      
      .mermaid-loading {
        color: var(--text-secondary);
        font-size: 14px;
        animation: pulse 1.5s infinite;
      }
      
      @keyframes pulse {
        0% {
          opacity: 0.5;
        }
        50% {
          opacity: 1;
        }
        100% {
          opacity: 0.5;
        }
      }
      
      .mermaid-error {
        color: #e53e3e;
        padding: 12px;
        text-align: left;
      }
      
      .mermaid-error pre {
        margin-top: 8px;
        background-color: ${isDark ? 'rgba(0, 0, 0, 0.2)' : 'rgba(0, 0, 0, 0.05)'};
        padding: 8px;
        border-radius: 4px;
        overflow-x: auto;
        font-size: 12px;
      }
      
      /* Markdown 内容样式 - GitHub风格 */
      .markdown-body {
        color: inherit;
        font-size: var(--font-size-base);
        line-height: 1.6;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji';
      }
      
      .markdown-body h1,
      .markdown-body h2,
      .markdown-body h3,
      .markdown-body h4,
      .markdown-body h5,
      .markdown-body h6 {
        margin-top: 24px;
        margin-bottom: 16px;
        font-weight: 600;
        line-height: 1.25;
        color: ${isDark ? '#f1f5f9' : '#111827'};
      }
      
      .markdown-body h1 {
        font-size: 2em;
        border-bottom: 1px solid ${isDark ? '#334155' : '#eaecef'};
        padding-bottom: 0.3em;
      }
      
      .markdown-body h2 {
        font-size: 1.5em;
        border-bottom: 1px solid ${isDark ? '#334155' : '#eaecef'};
        padding-bottom: 0.3em;
      }
      
      .markdown-body h3 {
        font-size: 1.25em;
      }
      
      .markdown-body h4 {
        font-size: 1em;
      }
 
      
      .markdown-body a {
        color: ${isDark ? '#58a6ff' : '#0366d6'};
        text-decoration: none;
        transition: color 0.2s;
      }
      
      .markdown-body a:hover {
        text-decoration: underline;
      }
      
      .markdown-body img {
        max-width: 100%;
        display: block;
        margin: 16px auto;
        border-radius: 6px;
        box-shadow: 0 4px 8px ${isDark ? 'rgba(0, 0, 0, 0.4)' : 'rgba(0, 0, 0, 0.1)'};
        transition: transform 0.3s ease, box-shadow 0.3s ease;
        animation: img-fade-in 0.5s ease forwards;
      }
      
      .markdown-body img:hover {
        transform: scale(1.01);
        box-shadow: 0 6px 16px ${isDark ? 'rgba(0, 0, 0, 0.5)' : 'rgba(0, 0, 0, 0.15)'};
      }
      
      @keyframes img-fade-in {
        from {
          opacity: 0;
          filter: blur(5px);
        }
        to {
          opacity: 1;
          filter: blur(0);
        }
      }
      
      .markdown-body pre {
        background-color: ${isDark ? '#1e293b' : '#f6f8fa'};
        border-radius: 6px;
        margin: 16px 0;
        padding: 16px;
        overflow-x: auto;
        box-shadow: 0 2px 6px ${isDark ? 'rgba(0, 0, 0, 0.4)' : 'rgba(0, 0, 0, 0.08)'};
        border: 1px solid ${isDark ? '#334155' : '#e1e4e8'};
        position: relative;
        transition: transform 0.3s ease, box-shadow 0.3s ease;
      }
      
      .markdown-body pre:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 12px ${isDark ? 'rgba(0, 0, 0, 0.5)' : 'rgba(0, 0, 0, 0.15)'};
      }
      
      .markdown-body code {
        font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
        background-color: ${isDark ? 'rgba(71, 85, 105, 0.4)' : 'rgba(27, 31, 35, 0.05)'};
        border-radius: 3px;
        padding: 0.2em 0.4em;
        font-size: 85%;
        color: ${isDark ? '#f1f5f9' : '#24292e'};
      }
      
      .markdown-body pre code {
        background-color: transparent;
        padding: 0;
        border-radius: 0;
        color: inherit;
        font-size: 85%;
        line-height: 1.45;
        display: block;
      }
      
      .markdown-body pre code .hljs-keyword {
        color: ${isDark ? '#c792ea' : '#d73a49'};
        font-weight: ${isDark ? 'normal' : 'bold'};
      }
      
      .markdown-body pre code .hljs-string {
        color: ${isDark ? '#c3e88d' : '#032f62'};
      }
      
      .markdown-body pre code .hljs-comment {
        color: ${isDark ? '#676e95' : '#6a737d'};
        font-style: italic;
      }
      
      .markdown-body pre code .hljs-function {
        color: ${isDark ? '#82AAFF' : '#6f42c1'};
      }
      
      .markdown-body pre code .hljs-number {
        color: ${isDark ? '#F78C6C' : '#005cc5'};
      }
      
      .markdown-body pre code .hljs-title {
        color: ${isDark ? '#f07178' : '#6f42c1'};
      }
      
      .markdown-body pre code .hljs-attr {
        color: ${isDark ? '#FFCB6B' : '#005cc5'};
      }
      
      .markdown-body pre code .hljs-selector-class {
        color: ${isDark ? '#FFCB6B' : '#6f42c1'};
      }
      
      .markdown-body blockquote {
        color: ${isDark ? '#9ca3af' : '#6a737d'};
        border-left: 4px solid ${isDark ? '#3b82f6' : '#dfe2e5'};
        padding: 0 16px;
        margin: 16px 0;
        background-color: ${isDark ? 'rgba(30, 41, 59, 0.5)' : 'rgba(246, 248, 250, 0.5)'};
        border-radius: 0 6px 6px 0;
      }
      
      .markdown-body blockquote p {
        margin: 0.8em 0;
      }
      
      .markdown-body ul,
      .markdown-body ol {
        padding-left: 2em;
        margin: 16px 0;
      }
      
      .markdown-body li + li {
        margin-top: 0.25em;
      }
      
      .markdown-body ul li {
        list-style-type: disc;
      }
      
      .markdown-body ol li {
        list-style-type: decimal;
      }
      
      .markdown-body ul ul,
      .markdown-body ul ol,
      .markdown-body ol ul,
      .markdown-body ol ol {
        margin: 8px 0 0;
      }
      
      .markdown-body li > p {
        margin-top: 16px;
      }
      
      .markdown-body table {
        border-collapse: separate;
        border-spacing: 0;
        width: 100%;
        margin: 16px 0;
        overflow-x: auto;
        display: block;
        border-radius: 6px;
        border: 1px solid ${isDark ? '#334155' : '#dfe2e5'};
        box-shadow: 0 2px 6px ${isDark ? 'rgba(0, 0, 0, 0.3)' : 'rgba(0, 0, 0, 0.05)'};
      }
      
      .markdown-body table th,
      .markdown-body table td {
        border: 1px solid ${isDark ? '#334155' : '#dfe2e5'};
        padding: 8px 13px;
        text-align: left;
      }
      
      .markdown-body table th {
        background-color: ${isDark ? '#1e293b' : '#f6f8fa'};
        font-weight: 600;
        color: ${isDark ? '#f1f5f9' : '#24292e'};
      }
      
      .markdown-body table tr:nth-child(2n) {
        background-color: ${isDark ? 'rgba(71, 85, 105, 0.1)' : 'rgba(246, 248, 250, 0.7)'};
      }
      
      .markdown-body hr {
        height: 1px;
        padding: 0;
        margin: 24px 0;
        background-color: ${isDark ? '#334155' : '#e1e4e8'};
        border: 0;
      }
      
      .markdown-body .task-list-item {
        list-style-type: none;
        margin-left: -1.5em;
      }
      
      .markdown-body .task-list-item input {
        margin-right: 0.5em;
      }
      
      /* 移除代码块的装饰效果 */
      .markdown-body pre:before,
      .markdown-body pre:after {
        display: none;
        content: none;
      }
      
      /* 移动端优化 */
      @media (max-width: 767px) {
        .message-bubble {
          max-width: calc(90% - 42px);
        }
        
        .message-content {
          padding: 12px 16px;
        }
        
        .avatar-icon {
          width: 32px;
          height: 32px;
        }
        
        .avatar-icon svg {
          width: 18px;
          height: 18px;
        }
        
        .markdown-body pre {
          padding: 16px 12px;
        }
        
        .markdown-body {
          font-size: calc(var(--font-size-base) - 1px);
        }
      }
      
      .message-wrapper.user-message-right {
        flex-direction: row-reverse;
      }
      
      .message-wrapper.user-message-right .message-bubble {
        max-width: calc(85% - 42px);
        margin-right: 12px;
      }
      
      .message-wrapper.user-message-right .message-content {
        border-top-left-radius: 18px;
        border-top-right-radius: 4px;
      }
      
      .message-actions {
        display: flex;
        gap: 8px;
        margin-top: 8px;
      }
      
      .action-button {
        background: none;
        border: none;
        cursor: pointer;
        padding: 4px;
        border-radius: var(--radius-sm);
        transition: var(--transition);
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--text-secondary);
      }
      
      .action-button:hover {
        background-color: rgba(0, 0, 0, 0.05);
        color: var(--text-color);
      }
      
      .action-button svg {
        width: 16px;
        height: 16px;
      }
      
      .copy-button {
        color: var(--primary-color);
      }
      
      .copy-button:hover {
        background-color: rgba(59, 130, 246, 0.1);
      }
      
      .regenerate-button {
        color: var(--text-secondary);
      }
      
      .regenerate-button:hover {
        background-color: rgba(0, 0, 0, 0.05);
        color: var(--text-color);
      }
      
      /* 自定义按钮样式 */
      .markdown-button {
        display: inline-block;
        padding: 8px 16px;
        background-color: var(--primary-color);
        color: white;
        border: none;
        border-radius: var(--radius);
        cursor: pointer;
        font-weight: 500;
        font-size: 14px;
        transition: all 0.3s ease;
        box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
        margin: 8px 0;
        text-align: center;
      }
      
      .markdown-button:hover {
        background-color: var(--primary-hover);
        transform: translateY(-2px);
        box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
      }
      
      .markdown-button:active {
        transform: translateY(0);
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
      }
      
      /* 暗色模式下的按钮样式 */
      @media (prefers-color-scheme: dark) {
        .markdown-button {
          box-shadow: 0 2px 5px rgba(0, 0, 0, 0.3);
        }
        
        .markdown-button:hover {
          box-shadow: 0 4px 8px rgba(0, 0, 0, 0.4);
        }
      }
    </style>
  </div>
`;

  // 确保内容立即可见，而不是等待动画
  nextTick(() => {
    // 强制使内容可见
    const chatMessages = document.querySelector('.chat-messages');
    if (chatMessages) {
      chatMessages.querySelector('.scoped-content')?.classList.add('fade-in');
    }
    
    // 应用代码高亮
    applyHighlight();

    // 渲染数学公式
    renderMathInElement();
    
    // 渲染Mermaid图表
    renderMermaidDiagrams();

    // 设置外部链接处理
    setupExternalLinks();

    // 设置复制按钮和重做按钮的事件监听器
    setupActionButtons();

    // 滚动到底部
    scrollToBottom();
    
    // 刷新全局样式，确保主题一致性
    refreshGlobalStyles();
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
    
    // 滚动到底部，添加平滑效果
    scrollToBottom(true);
  });

  // 监听流完成事件
  const unlistenComplete = await listen('stream-complete', async () => {
    isStreaming.value = false;
    isLoading.value = false;

    // 重新加载聊天历史
    await loadChatHistory();
    
    // 重新启用消息过渡动画
    messageTransition.value = true;
  });

  // 在组件卸载时清理事件监听
  onUnmounted(() => {
    unlistenStream();
    unlistenComplete();
  });
}

// 设置复制按钮和重做按钮的事件监听器
function setupActionButtons() {
  nextTick(() => {
    // 设置复制按钮事件监听
    document.querySelectorAll('.chat-messages .copy-button').forEach(button => {
      button.addEventListener('click', async () => {
        const encodedContent = (button as HTMLElement).dataset.content;
        if (encodedContent) {
          const content = decodeURIComponent(encodedContent);
          try {
            await writeText(content);
            showNotification("内容已复制到剪贴板", "success");
          } catch (error) {
            console.error("复制失败:", error);
            showNotification("复制失败", "error");
          }
        }
      });
    });

    // 设置重做按钮事件监听
    document.querySelectorAll('.chat-messages .regenerate-button').forEach(button => {
      button.addEventListener('click', async () => {
        // 如果正在流式传输，禁止重做操作
        if (isStreaming.value) {
          showNotification("请等待当前消息输出完成", "error");
          return;
        }
        
        const messageIndex = Number((button as HTMLElement).dataset.messageIndex);
        if (!isNaN(messageIndex)) {
          try {
            // 显示加载状态
            isLoading.value = true;
            isStreaming.value = true;
            messageTransition.value = false;
            
            // 调用后端重新生成消息
            await invoke("regenerate_message", { messageIndex });
            
            // 处理将在事件监听器中完成
          } catch (error) {
            console.error("重新生成失败:", error);
            showNotification("重新生成失败", "error");
            isStreaming.value = false;
            isLoading.value = false;
          }
        }
      });
    });
  });
}

// 流式发送消息
async function sendStreamMessage() {
  if (!inputMessage.value.trim()) return;

  // 禁用消息过渡动画，因为流式响应会自动处理
  messageTransition.value = false;
  fadeInMessages.value = true; // 确保内容可见
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
  } finally {
    // 恢复消息过渡动画
    setTimeout(() => {
      messageTransition.value = true;
      fadeInMessages.value = true; // 确保内容可见
    }, 1000);
  }
}

// 自动滚动到底部
function scrollToBottom(smooth = false) {
  nextTick(() => {
    const chatContent = document.querySelector('.chat-content');
    if (chatContent) {
      if (smooth) {
        chatContent.scrollTo({
          top: chatContent.scrollHeight,
          behavior: 'smooth'
        });
      } else {
        chatContent.scrollTop = chatContent.scrollHeight;
      }
    }
  });
}

// 创建新对话
async function createNewChat() {
  // 如果正在流式输出消息，禁止创建新聊天
  if (isStreaming.value) {
    showNotification("请等待当前消息输出完成", "error");
    return;
  }
  
  // 添加淡出效果
  fadeInMessages.value = false;
  
  setTimeout(async () => {
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
      // 短暂延迟后淡入新消息
      setTimeout(() => {
        fadeInMessages.value = true;
      }, 100);
    }
  }, 300);
}

// 监听 chatContent 变化，确保 MathJax 重新渲染
watch(chatContent, () => {
  nextTick(() => {
    console.log("聊天内容变化:", chatContent.value);
    refreshGlobalStyles();
    renderMathInElement();
    renderMermaidDiagrams(); // 添加Mermaid图表渲染
  });
});

// 监听主题变化，更新聊天内容和Mermaid配置
watch(() => document.documentElement.getAttribute('data-theme'), (newTheme) => {
  console.log("主题变化:", newTheme);
  
  // 当主题变化时，更新Mermaid配置
  const isDark = newTheme === 'dark' || 
                (newTheme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
  
  mermaid.initialize({
    theme: isDark ? 'dark' : 'default'
  });
  
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
    
    // 初始化Mermaid
    initMermaid();

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

  // 修改事件监听器以响应主题和字体大小变化，确保延迟处理
  window.addEventListener('themeChanged', (e: Event) => {
    const customEvent = e as CustomEvent;
    console.log('主题已变更:', customEvent.detail);
    // 添加延迟以确保主题变更完全应用
    setTimeout(() => {
      // 更新Mermaid主题
      const isDark = document.documentElement.getAttribute('data-theme') === 'dark' || 
                  (document.documentElement.getAttribute('data-theme') === 'system' && 
                   window.matchMedia('(prefers-color-scheme: dark)').matches);
      
      mermaid.initialize({
        theme: isDark ? 'dark' : 'default'
      });
      
      if (chatContent.value) {
        updateChatContent(chatContent.value);
      }
    }, 100);
  });

  window.addEventListener('fontSizeChanged', (e: Event) => {
    const customEvent = e as CustomEvent;
    console.log('字体大小已变更:', customEvent.detail);
    // 添加延迟以确保字体大小变更完全应用
    setTimeout(() => {
      if (chatContent.value) {
        updateChatContent(chatContent.value);
      }
    }, 100);
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
        <div class="settings-modal-content animate-in">
          <Setting @close="toggleSettings" />
        </div>
      </div>

      <!-- 通知组件 -->
      <div v-if="notification.visible" class="notification animated-notification" :class="notification.type">
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
      <aside class="history-sidebar animated-sidebar" :class="{ 'history-open': isHistoryOpen }">
        <div class="history-header">
          <h3>对话历史</h3>
          <!-- 小屏幕时在历史栏中添加关闭按钮 -->
          <button class="close-history" @click="toggleHistory">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="18" y="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
        <!-- 历史列表其余部分保持不变 -->
        <div class="history-actions">
          <button class="new-chat-button" @click="createNewChat" :class="{ 'streaming-disabled': isStreaming }">
            <svg class="icon" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="12" y1="5" x2="12" y2="19"></line>
              <line x1="5" y1="12" x2="19" y2="12"></line>
            </svg>
            新对话
          </button>
        </div>
        <div class="history-list">
          <div v-for="(item, index) in chatHistory" :key="item.id" 
               @click="isStreaming ? showNotification('请等待当前消息输出完成', 'error') : selectHistory(item.id)" 
               class="history-item" :class="{ 'streaming-disabled': isStreaming }"
               :style="{ animationDelay: index * 0.05 + 's' }">
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
            <div class="loading-spinner enhanced"></div>
            <div class="loading-text">加载中...</div>
          </div>
          <div v-else-if="!processedChatContent" class="empty-chat animated-empty">
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
            <input v-model="inputMessage" type="text" placeholder="输入消息..." class="message-input animated-input" />
            <button type="submit" class="send-button animated-button" :disabled="isStreaming" 
                    :class="{ 'streaming': isStreaming }">
              <svg v-if="!isStreaming" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"
                fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                class="send-icon">
                <line x1="22" y1="2" x2="11" y2="13"></line>
                <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
              </svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                class="loading-icon rotating">
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
  .title {
    color: var(--dark-text-color);
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
  color: var(--text-color);
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
  border-bottom: 0px solid var(--border-color);
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
  animation: item-slide-in 0.5s ease forwards;
  opacity: 0;
  transform: translateX(-10px);
}

@keyframes item-slide-in {
  from {
    opacity: 0;
    transform: translateX(-10px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.history-item:hover {
  background-color: rgba(0, 0, 0, 0.03);
  border-color: var(--border-color);
  transform: translateX(3px);
}

.history-item-content {
  display: flex;
  align-items: center;
}

.history-icon {
  color: var (--text-secondary);
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
    transform: scale(0.95) translateY(10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
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

.loading-spinner.enhanced {
  width: 32px;
  height: 32px;
  border: 3px solid rgba(79, 70, 229, 0.2);
  border-top: 3px solid var(--primary-color);
  border-radius: 50%;
  animation: enhanced-spin 1.2s cubic-bezier(0.68, -0.55, 0.27, 1.55) infinite;
  margin-bottom: 12px;
  box-shadow: 0 0 15px rgba(79, 70, 229, 0.2);
}

@keyframes enhanced-spin {
  0% {
    transform: rotate(0deg) scale(0.9);
  }
  50% {
    transform: rotate(180deg) scale(1.1);
  }
  100% {
    transform: rotate(360deg) scale(0.9);
  }
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
  transform: scale(1.01);
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
  transform: translateY(-50%) scale(1.08);
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.4);
}

.send-button.streaming {
  background-color: #2563eb;
}

send-icon {
  stroke-width: 2;
}

.loading-icon.rotating {
  animation: rotate 2s linear infinite;
}

@keyframes rotate {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

/* 代码高亮 - 暗色模式适配 */
@media (prefers-color-scheme: dark) {

  /* 隐藏亮色主题 */
  .hljs-github {
    display: none 
  }

  /* 显示暗色主题 */
  .hljs-github-dark {
    display: block 
  }
}

@media (prefers-color-scheme: light) {

  /* 显示亮色主题 */
  .hljs-github {
    display: block 
  }

  /* 隐藏暗色主题 */
  .hljs-github-dark {
    display: none 
  }
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
  border-radius: var (--radius);
  background-color: var(--card-bg);
  box-shadow: var(--shadow);
  z-index: 1000;
  max-width: 400px;
  animation: notification-slide-in 0.4s cubic-bezier(0.16, 1, 0.3, 1) forwards;
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

@keyframes notification-slide-in {
  from {
    transform: translateX(100%);
    opacity: 0;
  }
  50% {
    transform: translateX(-10px);
    opacity: 1;
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

/* 添加新的动效样式 */
.animated-notification {
  animation: notification-slide-in 0.4s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}

.animated-sidebar {
  transition: transform 0.4s cubic-bezier(0.16, 1, 0.3, 1);
}

.animated-empty {
  animation: fade-in 0.8s ease;
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.animate-in {
  animation: modal-in 0.4s cubic-bezier(0.16, 1, 0.3, 1) forwards;
}

.animated-input {
  transition: all 0.3s ease;
}

.animated-button {
  transition: all 0.3s ease;
}

.animated-button:not(:disabled):hover {
  transform: translateY(-50%) scale(1.08);
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.4);
}

.new-chat-button {
  transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}

.new-chat-button:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 15px rgba(59, 130, 246, 0.3);
}

.new-chat-button:active {
  transform: translateY(0);
}

.settings-button {
  transition: all 0.3s ease;
}

.settings-button:hover {
  transform: translateY(-2px);
}

.settings-button:active {
  transform: translateY(0);
}

/* 自定义标题栏按钮动效 */
.window-controls button {
  transition: all 0.2s ease;
}

.window-controls button:hover {
  transform: scale(1.1);
}

.window-controls button.close:hover {
  background-color: #e81123;
  color: white;
  transform: scale(1.1);
}

/* 修改 modal-in 动画更平滑 */
@keyframes modal-in {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(10px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

/* 移动端优化 */
@media (max-width: 767px) {
  .history-item:hover {
    transform: translateX(1px);
  }
}

/* 流式输出时禁用相关样式 */
.send-button:disabled {
  background-color: #93c5fd;
  cursor: not-allowed;
  transform: translateY(-50%) scale(1);
  box-shadow: none;
  opacity: 0.7;
}

/* 流式输出时禁用的按钮和链接 */
.streaming-disabled {
  opacity: 0.6;
  cursor: not-allowed;
  pointer-events: none;
}

/* 流式输出时的视觉反馈 */
.new-chat-button.streaming-disabled {
  background-color: #93c5fd;
  transform: none;
  box-shadow: none;
}

/* 在聊天列表项上添加状态指示 */
.history-item.streaming-disabled::after {
  content: "⌛";
  position: absolute;
  right: 10px;
  font-size: 12px;
  opacity: 0.7;
}

/* 自定义按钮样式 */
.markdown-button {
  display: inline-block;
  padding: 8px 16px;
  background-color: var(--primary-color);
  color: white;
  border: none;
  border-radius: var(--radius);
  cursor: pointer;
  font-weight: 500;
  font-size: 14px;
  transition: all 0.3s ease;
  box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
  margin: 8px 0;
  text-align: center;
}

.markdown-button:hover {
  background-color: var(--primary-hover);
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
}

.markdown-button:active {
  transform: translateY(0);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

/* 暗色模式下的按钮样式 */
@media (prefers-color-scheme: dark) {
  .markdown-button {
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.3);
  }
  
  .markdown-button:hover {
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.4);
  }
}
</style>
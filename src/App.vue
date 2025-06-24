<!--
NPULearn - 智能学习助手桌面应用
Copyright (c) 2025 NPULearn Contributors

This work is licensed under CC BY-NC-SA 4.0.
Commercial use requires explicit authorization.
For details, see LICENSE file or visit:
https://creativecommons.org/licenses/by-nc-sa/4.0/
-->


<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { listen } from '@tauri-apps/api/event';
import 'highlight.js/styles/github.min.css';
import 'highlight.js/styles/github-dark.min.css'; // 暗色主题
import LoadingLogo from './components/LoadingLogo.vue';
import Setting from './components/Setting.vue';
import html2canvas from 'html2canvas'; // 导入 html2canvas

import { ApiKeyType, useSettingsProvider } from './composables/useSettings';
import { Window } from '@tauri-apps/api/window';


import { loadMathJax, renderMathInElement } from "./App/mathjax.ts";
import { createNewChat, loadChatHistory, selectHistory } from "./App/chatHistory.ts";
import { initMermaid, changeMermaidTheme, setupAllMermaidInteractions } from "./App/typesetting/mermaidRenderer.ts";
import { applyHighlight, setupAllCopyButtons } from "./App/typesetting/typesetting.ts";
import { chatHistory, eventBus, isLoading, isStreaming } from "./App/eventBus.ts";
import { ChatHistory, ChatMessage } from "./App/types.ts";



// 初始化全局设置，在整个应用中提供设置
const {
  settings,
  notification,
  showNotification,
  initAppSettings,
  apiKeys,
  loadApiKeys,
  fetchGeminiModels
} = useSettingsProvider();

const isAppLoading = ref(true);
const isMobile = ref(false); // 添加移动设备状态

// 处理聊天内容，隔离样式
const processedChatContent = ref("");


// 改为空数组，将从后端加载
const windowWidth = ref(window.innerWidth);
const isHistoryOpen = ref(windowWidth.value >= 768);
const inputMessage = ref("");

const showSettings = ref(false);

// 添加对话重命名和删除功能所需的状态
const currentChatId = ref<number | null>(null); // 当前选中的对话ID
const isRenamingChat = ref(false); // 是否正在重命名对话
const newChatTitle = ref(""); // 新的对话标题
const showConfirmDelete = ref(false); // 是否显示删除确认对话框
const chatToDeleteId = ref<number | null>(null); // 要删除的对话ID
const showMessageContextMenu = ref(false); // 是否显示消息上下文菜单
const messageContextMenuPosition = ref({ x: 0, y: 0 }); // 消息上下文菜单位置
const messageContextMenuIndex = ref<number | null>(null); // 当前右键菜单对应的消息索引
const selectedTextAtContextMenu = ref<string>(""); // 保存右键时的选中文本

// 添加对话历史项右键菜单相关状态
const showChatContextMenu = ref(false);
const chatContextMenuPosition = ref({ x: 0, y: 0 });
const chatContextMenuId = ref<number | null>(null);
const selectedModel = ref<string | null>(null); // 当前选中的模型

// 悬浮滚动按钮相关状态
const showScrollToBottomButton = ref(false);
let scrollCheckTimeout: NodeJS.Timeout | null = null;

// 切换设置界面的显示
function toggleSettings() {
  showSettings.value = !showSettings.value;
  // 如果在小屏幕上打开了历史栏，同时关闭它
  if (showSettings.value) {
    autoHideHistory();
  }
}

function autoHideHistory() {
  if (windowWidth.value < 768) {
    isHistoryOpen.value = false;
  }
}

// 切换历史列表显示
function toggleHistory() {
  isHistoryOpen.value = !isHistoryOpen.value;
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


// 修改链接处理函数
function setupExternalLinks() {
  document.querySelectorAll('.chat-messages a').forEach(link => {
    const href = link.getAttribute('href');

    if (href) {
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
}

// 全局阻止所有链接跳转的函数
function preventAllNavigation() {
  // 阻止所有 a 标签的默认跳转行为
  document.addEventListener('click', (e) => {
    const target = e.target as HTMLElement;
    const link = target.closest('a');
    
    // 检查是否是白名单中的元素，如果是则不阻止
    if (isWhitelistedElement(target)) {
      console.log('允许白名单元素的点击事件:', target);
      return; // 不阻止白名单元素的事件
    }
    
    if (link && link.href) {
      e.preventDefault();
      e.stopPropagation();
      
      const href = link.href;
      console.log('阻止链接跳转:', href);
      
      // 可以选择将链接复制到剪贴板
      if (href.startsWith('http://') || href.startsWith('https://')) {
        writeText(href).then(() => {
          showNotification(`外部链接已复制到剪贴板: ${href}`, 'info');
        }).catch(error => {
          console.error('复制链接失败:', error);
          showNotification('复制链接失败', 'error');
        });
      }
    }
  }, true); // 使用捕获阶段

  // 阻止表单提交到外部
  document.addEventListener('submit', (e) => {
    const form = e.target as HTMLFormElement;
    if (form.action && (form.action.startsWith('http://') || form.action.startsWith('https://'))) {
      e.preventDefault();
      console.log('阻止表单提交到外部:', form.action);
      showNotification('已阻止表单提交到外部网站', 'warning');
    }
  }, true);

  // 阻止 window.open
  const originalOpen = window.open;
  window.open = function(url?: string | URL, target?: string, features?: string) {
    console.log('阻止 window.open:', url);
    showNotification('已阻止弹出新窗口', 'warning');
    return null;
  };
}

// 检查元素是否在白名单中
function isWhitelistedElement(element: HTMLElement): boolean {
  // 检查元素本身和其父元素是否包含白名单的类名或属性
  let currentElement: HTMLElement | null = element;
  
  while (currentElement) {
    // WolframAlpha 相关查询的白名单
    if (currentElement.closest('.wolfram-related-queries')) {
      return true;
    }
    
    // 具有 data-query 属性的元素（WolframAlpha 查询项）
    if (currentElement.hasAttribute('data-query')) {
      return true;
    }
    
    // Interactive Button 相关元素
    if (currentElement.classList.contains('interactive-command-button') ||
        currentElement.closest('.interactive-command-button')) {
      return true;
    }
    
    // 图表交互按钮（Mermaid 和 Pintora）
    if (currentElement.classList.contains('refresh-diagram-button') ||
        currentElement.classList.contains('zoom-diagram-button') ||
        currentElement.closest('.refresh-diagram-button, .zoom-diagram-button')) {
      return true;
    }
    
    // 代码复制按钮
    if (currentElement.classList.contains('code-copy-button') ||
        currentElement.closest('.code-copy-button')) {
      return true;
    }
    
    // 应用内的其他交互按钮
    if (currentElement.classList.contains('interactive-button') ||
        currentElement.classList.contains('wolfram-query-item') ||
        currentElement.classList.contains('allow-navigation') ||
        currentElement.classList.contains('markdown-button') ||
        currentElement.classList.contains('copy-button') ||
        currentElement.classList.contains('regenerate-button') ||
        currentElement.hasAttribute('data-allow-click')) {
      return true;
    }
    
    // 特定容器内的所有元素（如果容器本身允许交互）
    if (currentElement.closest('.interactive-button-container') ||
        currentElement.closest('.wolfram-result-container') ||
        currentElement.closest('.message-actions')) {
      return true;
    }
    
    // 检查父元素
    currentElement = currentElement.parentElement;
  }
  
  return false;
}


// 修改 updateChatContent 函数，移除直接DOM操作
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
    const processedContent = msg.content;

    messagesHtml += `
    <div class="message-wrapper ${messageClass} ${isUserMessage ? 'user-message-right' : ''}" @contextmenu.prevent="openMessageContextMenu($event, ${messages.indexOf(msg)})">
      <div class="message-avatar ${messageClass}">
        <div class="avatar-icon">
          ${isUserMessage ?
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path><circle cx="12" cy="7" r="4"></circle></svg>' :
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path><path d="M9 9h6"></path><path d="M9 13h6"></path></svg>'
      }
        </div>
        <div class="message-time ${messageClass}">${msg.time}</div>
      </div>
      <div class="message-bubble ${messageClass}">
        <div class="message-content markdown-body" data-message-index="${messages.indexOf(msg)}">
          ${processedContent}
        </div>
        <div class="message-actions ${messageClass}">
          <button class="action-button copy-button" data-content="${encodeURIComponent(msg.content)}" title="复制内容">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
          </button>
          <button class="action-button render-image-button" data-message-index="${messages.indexOf(msg)}" title="渲染成图片">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
              <circle cx="8.5" cy="8.5" r="1.5"></circle>
              <polyline points="21 15 16 10 5 21"></polyline>
            </svg>
          </button>
          ${!isUserMessage ?
        `<button class="action-button regenerate-button" data-message-index="${messages.indexOf(msg)}" title="重新生成">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M23 4v6h-6"></path>
                <path d="M1 20v-6h6"></path>
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10"></path>
                <path d="M20.49 15a9 9 0 0 1-14.85 3.36L1 14"></path>
              </svg>
            </button>` : ''
      }
        </div>
      </div>
    </div>
  `;
  }

  // 移除动画相关的类，保留fade-in以确保消息立即可见  
  let generatedHtml = `
  <div class="scoped-content fade-in" data-theme="${isDark ? 'dark' : 'light'}">
    ${messagesHtml}
  </div>
`;
  processedChatContent.value = generatedHtml;

  // 创建一个解析器来在内存中处理HTML
  const parser = new DOMParser();
  const doc = parser.parseFromString(`<div class="chat-messages">${generatedHtml}</div>`, 'text/html');
  const virtualElement = doc.querySelector('.chat-messages');

  // 延迟检查滚动按钮状态
  nextTick(() => {
    setTimeout(() => {
      checkScrollToBottomButton();
    }, 100);
  });

  // 延迟检查滚动按钮状态
  nextTick(() => {
    setTimeout(() => {
      checkScrollToBottomButton();
    }, 100);
  });

  if (!virtualElement) return;

  // 在虚拟DOM中应用代码高亮
  applyHighlight(virtualElement as HTMLElement).then(highlightedElement => {
    // 更新处理后的HTML
    processedChatContent.value = highlightedElement.innerHTML;
    // 重新渲染后再执行其他操作
    if (!highlightedElement) return;
    // 在下一个tick中，当DOM更新后，添加事件监听
    nextTick(setupFunctions);
  }).catch(error => {
    console.error("代码高亮失败:", error);
  });
}

function setupFunctions() {
  // 为真实DOM中的消息添加右键菜单事件
  document.querySelectorAll('.chat-messages .message-content[data-message-index]').forEach(messageElement => {
    messageElement.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      e.stopPropagation();
      const messageIndex = parseInt((messageElement as HTMLElement).dataset.messageIndex || '0', 10);
      openMessageContextMenu(e as MouseEvent, messageIndex);
    });
  });
  // 其他需要在DOM更新后执行的代码...
  renderMathInElement();
  setupExternalLinks();
  setupActionButtons(); 
  setupAllCopyButtons();
  
  // 重要：为所有 Mermaid 图表绑定交互事件（包括流式传输结束后的图表）
  const chatMessagesContainer = document.querySelector('.chat-messages') as HTMLElement;
  if (chatMessagesContainer) {
    console.log('在聊天内容更新后，重新绑定所有 Mermaid 图表的交互事件');
    setupAllMermaidInteractions(chatMessagesContainer);
  } else {
    console.log('在聊天内容更新后，使用全局容器绑定 Mermaid 图表的交互事件');
    setupAllMermaidInteractions(document.body);
  }
  
  scrollToBottom(true, false); // 强制滚动，因为这是新内容渲染

  // 内容渲染完成后重新设置滚动监听器和检查滚动按钮状态
  setTimeout(() => {
    setupScrollListener();
  }, 200);
}

// 流式消息处理相关函数
async function setupStreamListeners() {
  // 添加一个用于跟踪最新更新请求的ID
  let latestUpdateId = 0;

  const unlistenStream = await listen('stream-message', (event) => {
    // 标记正在接收流式消息
    isStreaming.value = true;
    console.log("流式消息接收中，暂停UML渲染");

    // 生成新的更新ID
    const currentUpdateId = ++latestUpdateId;

    // 将后端发送的聊天历史更新到前端
    try {
      // 检查 payload 是否为字符串（错误信息）
      if (typeof event.payload === 'string') {
        console.error("后端返回错误:", event.payload);
        showNotification(`${event.payload}`, "error");
        isStreaming.value = false;
        isLoading.value = false;
        return;
      }

      // 尝试进行类型断言，如果失败会抛出异常
      const chatData = event.payload as ChatHistory;

      // 验证数据结构
      if (!chatData || !chatData.content || !Array.isArray(chatData.content)) {
        const errorMessage = `接收到无效的聊天数据格式: ${JSON.stringify(event.payload)}`;
        console.error(errorMessage);
        showNotification(errorMessage, "error");
        isStreaming.value = false;
        isLoading.value = false;
        return;
      }

      // 使用requestAnimationFrame确保在下一帧渲染前进行检查
      requestAnimationFrame(() => {
        // 只有当当前更新ID是最新的时才执行更新
        if (currentUpdateId === latestUpdateId) {
          // 更新聊天内容显示
          updateChatContent(chatData.content);
        } else {
          console.log(`跳过过时的更新 (ID: ${currentUpdateId})`);
        }
      });
    } catch (error) {
      // 显示详细的错误信息
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error("处理流式消息失败:", errorMessage);
      showNotification(`处理消息失败: ${errorMessage}`, "error");
      isStreaming.value = false;
      isLoading.value = false;
    }
  });

  // 监听流完成事件
  const unlistenComplete = await listen('stream-complete', async () => {
    console.log("流式消息接收完成，开始处理延迟的渲染任务");
    // 标记流式消息接收完成
    isStreaming.value = false;
    isLoading.value = false;

    // 生成最终更新的ID
    const finalUpdateId = ++latestUpdateId;

    try {
      const result = await invoke("get_chat_html");

      // 检查返回值类型并记录详细信息
      if (typeof result === 'string') {
        console.error("获取聊天内容返回错误字符串:", result);
        showNotification(`获取聊天内容失败: ${result}`, "error");
        return;
      }

      const chatContent = result as ChatMessage[];

      // 验证返回的数据
      if (!Array.isArray(chatContent)) {
        const errorMessage = `获取聊天内容返回无效格式: ${JSON.stringify(result)}`;
        console.error(errorMessage);
        showNotification(errorMessage, "error");
        return;
      }

      // 同样检查是否为最新更新
      requestAnimationFrame(() => {
        if (finalUpdateId === latestUpdateId) {
          updateChatContent(chatContent);
          nextTick(() => {
            invoke("get_chat_history_items").then((historyItems: any) => {
              chatHistory.value = historyItems as ChatHistory[];
              console.log("聊天历史已更新:", chatHistory.value);
            }).catch(error => {
              console.error("获取聊天历史失败:", error);
              showNotification("获取聊天历史失败", "error");
            });
          });
        }
      });
    } catch (error) {
      // 显示详细的错误信息
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error("获取最终聊天内容失败:", errorMessage);
      showNotification(`获取聊天内容失败: ${errorMessage}`, "error");
    }
  });

  // 在组件卸载时清理事件监听
  onUnmounted(() => {
    unlistenStream();
    unlistenComplete();
  });
}

// 设置复制按钮和重做按钮的事件监听器
function setupActionButtons() {
  // 设置复制按钮事件监听
  document.querySelectorAll('.chat-messages .copy-button').forEach(button => {
    button.addEventListener('click', async () => {
      const encodedContent = (button as HTMLElement).dataset.content;
      if (encodedContent) {
        try {
          const content = decodeURIComponent(encodedContent);

          // 创建一个临时的DOM元素来解析内容
          const tempDiv = document.createElement('div');
          tempDiv.innerHTML = content;

          // 检查是否存在包含原始消息的元素
          const originalMsgElement = tempDiv.querySelector('.original-message');
          let textToWrite: string;

          if (originalMsgElement && originalMsgElement.getAttribute('data-content')) {
            // 如果找到原始消息元素，获取并解码原始消息
            try {
              // 从Base64解码原始Markdown内容，使用更可靠的方法处理UTF-8编码
              const base64Content = originalMsgElement.getAttribute('data-content') || '';

              // 使用 base64ToUint8Array 和 TextDecoder 正确处理 UTF-8 编码的文本
              const binaryString = atob(base64Content);
              const bytes = new Uint8Array(binaryString.length);
              for (let i = 0; i < binaryString.length; i++) {
                bytes[i] = binaryString.charCodeAt(i);
              }
              textToWrite = new TextDecoder('utf-8').decode(bytes);

              console.log("已找到并解码原始Markdown内容");
            } catch (decodeError) {
              console.error("Base64解码失败:", decodeError);
              // 解码失败则回退到使用HTML内容
              textToWrite = content;
            }
          } else {
            // 如果没有找到原始消息元素，就使用HTML内容
            textToWrite = content;
          }

          await writeText(textToWrite);
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
          isStreaming.value = true;          // 获取当前选择的模型名称
          const currentApiType = selectedModel.value as ApiKeyType;
          const currentModelName = getCurrentSelectedModel(currentApiType);

          // 调用后端重新生成消息
          await invoke("regenerate_message", { 
            messageIndex, 
            keyType: selectedModel.value, 
            modelName: currentModelName 
          });

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
  // 设置渲染图片按钮事件监听
  document.querySelectorAll('.chat-messages .render-image-button').forEach(button => {
    button.addEventListener('click', async () => {
      const messageIndex = Number((button as HTMLElement).dataset.messageIndex);
      if (isNaN(messageIndex)) return;

      const messageContentElement = document.querySelector(`.chat-messages .message-content[data-message-index="${messageIndex}"]`) as HTMLElement;

      if (messageContentElement) {
        showNotification("正在渲染图片...", "info");
        try {
          // 获取当前主题，以便在渲染时应用正确的背景色
          const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
            (document.documentElement.getAttribute('data-theme') === 'system' &&
              window.matchMedia('(prefers-color-scheme: dark)').matches);
          const backgroundColor = isDark ? '#1e293b' : '#ffffff'; // 根据主题设置背景色

          // 获取原始元素宽度
          // const originalWidth = messageContentElement.offsetWidth;

          // 设置默认宽度（例如 800px）或使用原始宽度的最大值
          // const targetWidth = Math.min(Math.max(originalWidth, 800), 1200); // 宽度介于800-1200px之间
          const targetWidth = 1080;

          // 创建一个包装容器以控制宽度
          const wrapperDiv = document.createElement('div');
          wrapperDiv.style.width = `${targetWidth}px`;
          wrapperDiv.style.backgroundColor = backgroundColor;

          // 克隆原始元素到包装容器中以便渲染
          const clonedElement = messageContentElement.cloneNode(true) as HTMLElement;
          wrapperDiv.appendChild(clonedElement);

          // 临时添加到文档中（不可见），以便html2canvas能正确计算尺寸
          wrapperDiv.style.position = 'absolute';
          wrapperDiv.style.left = '-9999px';
          document.body.appendChild(wrapperDiv);

          const canvas = await html2canvas(clonedElement, {
            useCORS: true, // 允许加载跨域图片
            scale: 2, // 提高分辨率
            backgroundColor: backgroundColor,
            logging: true,
            width: targetWidth, // 指定宽度
            onclone: (clonedDoc) => {
              // 移除不需要的元素
              const apiFooters = clonedDoc.querySelectorAll('.api-call-footer');
              apiFooters.forEach(footer => {
                footer.remove();
              });

              const detailsElements = clonedDoc.querySelectorAll('details');
              detailsElements.forEach(details => {
                details.removeAttribute('open'); // 关闭所有折叠框
              });

              const codeCopyButtons = clonedDoc.querySelectorAll('.code-copy-button');
              codeCopyButtons.forEach(button => {
                button.remove();
              });

              const thinkSummary = clonedDoc.querySelectorAll('summary');
              thinkSummary.forEach(summary => {
                summary.remove();
              });

              const miniDetailsContainers = clonedDoc.querySelectorAll('.mini-details-container');
              miniDetailsContainers.forEach(container => {
                container.remove();
              });

              const miniTechDetails = clonedDoc.querySelectorAll('.mini-tech-details');
              miniTechDetails.forEach(container => {
                container.remove();
              });

              // 确保克隆的文档中 Mermaid 图表已渲染为 SVG
              const originalMermaidContainers = messageContentElement.querySelectorAll('.mermaid-container');
              const clonedMermaidContainers = clonedDoc.querySelectorAll('.mermaid-container');
              originalMermaidContainers.forEach((originalContainer, index) => {
                if (clonedMermaidContainers[index]) {
                  // 尝试直接复制 SVG 内容
                  const svgElement = originalContainer.querySelector('svg');
                  if (svgElement) {
                    clonedMermaidContainers[index].innerHTML = svgElement.outerHTML;
                  } else {
                    // 如果没有 SVG，保留加载或错误状态
                    clonedMermaidContainers[index].innerHTML = originalContainer.innerHTML;
                  }
                }
              });

              // 确保克隆的文档中 Pintora 图表已渲染为 SVG
              const originalPintoraContainers = messageContentElement.querySelectorAll('.pintora-container');
              const clonedPintoraContainers = clonedDoc.querySelectorAll('.pintora-container');
              originalPintoraContainers.forEach((originalContainer, index) => {
                if (clonedPintoraContainers[index]) {
                  // 尝试直接复制 SVG 内容
                  const svgElement = originalContainer.querySelector('.pintora-diagram svg');
                  if (svgElement) {
                    const diagramElement = clonedPintoraContainers[index].querySelector('.pintora-diagram');
                    if (diagramElement) {
                      diagramElement.innerHTML = svgElement.outerHTML;
                    }
                  } else {
                    // 如果没有 SVG，保留加载或错误状态
                    clonedPintoraContainers[index].innerHTML = originalContainer.innerHTML;
                  }
                }
              });
            }
          });

          // 渲染完成后移除临时元素
          document.body.removeChild(wrapperDiv);

          // 创建下载链接
          const link = document.createElement('a');
          link.download = `NPULearn-message-${Date.now()}.png`;
          link.href = canvas.toDataURL('image/png');
          link.click();
          showNotification("图片已保存", "success");
        } catch (error) {
          console.error("渲染图片失败:", error);
          showNotification("渲染图片失败", "error");
        }
      } else {
        showNotification("找不到要渲染的消息内容", "error");
      }
    });
  });
}

// 重置 Textarea 高度到初始状态
function resetTextareaHeight() {
  nextTick(() => {
    const textarea = document.querySelector('.message-input') as HTMLTextAreaElement;
    if (textarea) {
      textarea.style.height = 'auto'; // 先重置
      textarea.style.height = '48px'; // 设置为初始的 min-height
    }
  });
}

// 流式发送消息 - 非阻塞版本
async function sendStreamMessage() {
  if (!inputMessage.value.trim()) return;

  // 保存消息内容并立即清空输入框，提升用户体验
  const message = inputMessage.value;
  inputMessage.value = "";

  // 重置文本区域高度
  resetTextareaHeight();

  // 检查当前是否有选择的对话
  if (!await invoke("check_current_chat_id")) {
    console.log("未选择对话，正在创建新对话...");

    // 显示加载状态
    isLoading.value = true;

    try {
      // 创建新对话
      await invoke("create_new_chat");
      // 刷新历史列表
      await loadChatHistory();
      console.log("已创建新对话，继续发送消息");
    } catch (error) {
      console.error("创建新对话失败:", error);
      showNotification("创建新对话失败", "error");
      isLoading.value = false;
      return; // 创建失败则不继续发送消息
    }
  }

  // 先设置状态，确保在任何渲染发生前就已标记为流传输
  isStreaming.value = true;
  isLoading.value = true;  console.log("开始流式传输消息");
  scrollToBottom(true, true); // 强制滚动到底部

  // 获取当前选择的模型名称
  const currentApiType = selectedModel.value as ApiKeyType;
  const currentModelName = getCurrentSelectedModel(currentApiType);
  
  console.log(`当前API类型: ${currentApiType}, 选择的模型: ${currentModelName}`);
  
  // 使用 Promise 包装后端调用，但不等待它完成
  invoke("process_message_stream", { 
    message, 
    keyType: selectedModel.value,
    modelName: currentModelName 
  })
    .catch(error => {
      console.error("消息发送失败:", error);
      showNotification("消息发送失败", "error");
      isStreaming.value = false;
      isLoading.value = false;
      isStreaming.value = false;
    });

  // 由于已经设置了状态并启动了异步处理，函数可以立即返回
  // 实际的响应处理将由事件监听器完成
}


// 流式发送消息 - 非阻塞版本
async function sendStreamMessageDirect(message: string) {

  // 保存消息内容并立即清空输入框，提升用户体验
  inputMessage.value = "";

  // 重置文本区域高度
  resetTextareaHeight();

  // 检查当前是否有选择的对话
  if (!await invoke("check_current_chat_id")) {
    console.log("未选择对话，正在创建新对话...");

    // 显示加载状态
    isLoading.value = true;

    try {
      // 创建新对话
      await invoke("create_new_chat");
      // 刷新历史列表
      await loadChatHistory();
      console.log("已创建新对话，继续发送消息");
    } catch (error) {
      console.error("创建新对话失败:", error);
      showNotification("创建新对话失败", "error");
      isLoading.value = false;
      return; // 创建失败则不继续发送消息
    }
  }

  // 先设置状态，确保在任何渲染发生前就已标记为流传输
  isStreaming.value = true;
  isLoading.value = true;
  console.log("开始流式传输消息，已禁用UML渲染");

  // 获取当前选择的模型名称
  const currentApiType = selectedModel.value as ApiKeyType;
  const currentModelName = getCurrentSelectedModel(currentApiType);

  // 使用 Promise 包装后端调用，但不等待它完成
  invoke("process_message_stream", { 
    message, 
    keyType: selectedModel.value,
    modelName: currentModelName 
  })
    .catch(error => {
      console.error("消息发送失败:", error);
      showNotification("消息发送失败", "error");
      isStreaming.value = false;
      isLoading.value = false;
      isStreaming.value = false;
    });

}

// 检查用户是否已经滚动到底部
function isUserAtBottom() {
  const chatContent = document.querySelector('.chat-content');
  if (!chatContent) return false;

  const scrollHeight = chatContent.scrollHeight;
  const clientHeight = chatContent.clientHeight;
  const scrollTop = chatContent.scrollTop;

  // 允许一定的误差范围（300px），因为滚动可能不完全精确
  const isAtBottom = scrollTop + clientHeight >= scrollHeight - 300;
  return isAtBottom;
}

// 检查是否显示滚动到底部按钮
function checkScrollToBottomButton() {
  const chatContent = document.querySelector('.chat-content');
  if (!chatContent) {
    showScrollToBottomButton.value = false;
    return;
  }

  const scrollHeight = chatContent.scrollHeight;
  const clientHeight = chatContent.clientHeight;
  const scrollTop = chatContent.scrollTop;

  // 如果内容高度小于等于容器高度，说明不需要滚动，隐藏按钮
  if (scrollHeight <= clientHeight) {
    showScrollToBottomButton.value = false;
    return;
  }

  // 当用户向上滚动超过一定距离时显示按钮（容差设为150px）
  const distanceFromBottom = scrollHeight - (scrollTop + clientHeight);
  const shouldShow = distanceFromBottom > 150;

  showScrollToBottomButton.value = shouldShow;

  // 调试信息
  console.log(`滚动检查: 距离底部=${distanceFromBottom}px, 显示按钮=${shouldShow}`);
}

// 监听聊天内容区域的滚动事件
function setupScrollListener() {
  const chatContent = document.querySelector('.chat-content');
  if (!chatContent) {
    // 如果当前没有找到元素，稍后重试
    setTimeout(setupScrollListener, 500);
    return;
  }

  // 实时监听滚动事件
  const handleScroll = () => {
    checkScrollToBottomButton();
  };

  // 移除可能存在的旧监听器
  chatContent.removeEventListener('scroll', handleScroll);
  // 添加新的滚动监听器
  chatContent.addEventListener('scroll', handleScroll, { passive: true });

  // 初始检查一次
  checkScrollToBottomButton();

  console.log('滚动监听器已设置');
}

// 强制滚动到底部（从悬浮按钮触发）
function forceScrollToBottom() {
  scrollToBottom(true, true);
  showScrollToBottomButton.value = false;
}

// 自动滚动到底部 - 改进版，只在用户已经在底部时滚动
function scrollToBottom(smooth = false, force = false) {
  // 如果不是强制滚动，检查用户是否在底部
  if (!force && !isUserAtBottom()) {
    console.log('用户不在底部，跳过自动滚动');
    return;
  }

  // 首次尝试滚动
  nextTick(() => {
    scrollToBottomImpl(smooth);

    // 添加额外的延迟滚动尝试，处理动态内容和渲染延迟
    setTimeout(() => {
      scrollToBottomImpl(smooth);

      // 再次尝试，确保捕获所有内容变化
      setTimeout(() => {
        scrollToBottomImpl(smooth);
      }, 100);
    }, 50);
  });
}

// 滚动实现
function scrollToBottomImpl(smooth = false) {
  const chatContent = document.querySelector('.chat-content');
  if (!chatContent) return;

  // 获取滚动容器的总高度和可见高度
  const scrollHeight = chatContent.scrollHeight;
  const clientHeight = chatContent.clientHeight;

  // 计算需要滚动到的位置（使用额外的缓冲空间确保到底部）
  const scrollPosition = scrollHeight - clientHeight + 10;

  if (smooth) {
    chatContent.scrollTo({
      top: scrollPosition,
      behavior: 'smooth'
    });
  } else {
    chatContent.scrollTop = scrollPosition;
  }

  // 确保滚动生效
  if (chatContent.scrollTop < scrollPosition - 20) {
    chatContent.scrollTop = scrollPosition;
  }
}
// 处理输入框按键事件
function handleInputKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter' && event.ctrlKey) {
    event.preventDefault(); // 阻止默认的 Enter 行为（如果 textarea 在 form 内）
    sendStreamMessage();
  }
  // 允许 Shift+Enter 换行，textarea 默认支持
}



// // 监听 chatContent 变化，确保 MathJax 重新渲染
// watch(chatContent, () => {
//   nextTick(() => {
//     console.log("聊天内容变化:", chatContent.value);
//     refreshGlobalStyles();
//     renderMathInElement();

//     // 只在非流传输状态下渲染UML图表
//     if (!isStreaming.value) {
//       renderMermaidDiagrams();
//     } else {
//       console.log("正在流式传输中，跳过UML渲染");
//     }
//   });
// });

// 监听主题变化，更新聊天内容和Mermaid配置
watch(() => document.documentElement.getAttribute('data-theme'), (newTheme, oldTheme) => {
  // 增加判断，仅在主题实际变化时执行
  if (newTheme !== oldTheme) {
    console.log("主题变化:", newTheme);

    // 当主题变化时，更新Mermaid配置
    const isDark = newTheme === 'dark' ||
      (newTheme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);

    changeMermaidTheme(isDark ? 'dark' : 'default');

    // 当主题变化时，重新应用样式
    // 延迟执行，确保全局样式已应用
    setTimeout(() => {
      loadChatHistory().catch(error => {
        console.error("加载聊天历史失败:", error);
        showNotification("加载聊天历史失败", "error");
      });
    }, 50); // 短暂延迟
  }
});


// 组件加载时初始化对话内容
onMounted(async () => {
  eventBus.on('history:autoHide', () => {
    autoHideHistory();
  }); eventBus.on('content:update', (messages) => {
    updateChatContent(messages.messages);
  });

  eventBus.on('chart:open', (data) => {
    openChartViewer(data.svgContent, data.diagramContent);
  });

  eventBus.on('message:send', (message) => {
    sendStreamMessageDirect(message);
  });
  eventBus.on('notification:show', (data) => {
    showNotification(data.message, data.type as 'info' | 'success' | 'error');
  });

  // 添加全局拖动和结束拖动事件
  window.addEventListener('mousemove', handleDrag);
  window.addEventListener('mouseup', endDrag);
  window.addEventListener('touchmove', handleDrag);
  window.addEventListener('touchend', endDrag);

  selectedModel.value = ApiKeyType.Gemini; // 默认选择Gemini模型

  // 检测是否为移动设备
  isMobile.value = /Mobi|Android|iPhone|iPad|iPod/i.test(navigator.userAgent);

  try {
    // 初始化应用设置 (这会调用 refreshGlobalStyles)
    await initAppSettings();

    // 初始化Mermaid (主题应基于 initAppSettings 后的全局设置)
    initMermaid();    // 加载 MathJax
    await loadMathJax();

    // 设置流式消息监听器
    await setupStreamListeners();

    // 初始化全局链接拦截
    preventAllNavigation();

    // 加载聊天历史和当前对话内容
    await loadChatHistory();

    // 加载API密钥并检查是否需要获取Gemini模型
    await loadApiKeys();
    const geminiKeys = apiKeys.value.filterByType(ApiKeyType.Gemini);
    if (geminiKeys.keys.length > 0) {
      console.log('应用启动时检测到Gemini API密钥，自动获取最新模型列表...');
      // 异步获取，不阻塞应用启动
      fetchGeminiModels().catch(error => {
        console.error('应用启动时获取Gemini模型失败:', error);
      });
    } else {
      console.log('应用启动时未检测到Gemini API密钥，跳过模型列表获取');
    }

    isStreaming.value = false; // 初始加载时默认没有流传输// 所有内容加载完成后，隐藏启动logo
    setTimeout(() => {
      isAppLoading.value = false;
      // 确保在应用完全加载后设置滚动监听器
      setupScrollListener();
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

      changeMermaidTheme(isDark ? 'dark' : 'default');
      loadChatHistory().catch(error => {
        console.error("加载聊天历史失败:", error);
        showNotification("加载聊天历史失败", "error");
      });
    }, 100); // 保持延迟
  });

  window.addEventListener('fontSizeChanged', (e: Event) => {
    const customEvent = e as CustomEvent;
    console.log('字体大小已变更:', customEvent.detail);
    // 添加延迟以确保字体大小变更完全应用
    setTimeout(() => {
      loadChatHistory().catch(error => {
        console.error("加载聊天历史失败:", error);
        showNotification("加载聊天历史失败", "error");
      });
    }, 100); // 保持延迟
  });
});

// 组件卸载时清理事件监听
onUnmounted(() => {
  window.removeEventListener('mousemove', handleDrag);
  window.removeEventListener('mouseup', endDrag);
  window.removeEventListener('touchmove', handleDrag);
  window.removeEventListener('touchend', endDrag);
  window.removeEventListener('resize', handleResize);
  // 清除主题和字体大小变化的事件监听
  window.removeEventListener('themeChanged', (_: Event) => { }); window.removeEventListener('fontSizeChanged', (_: Event) => { });
  // 移除菜单关闭监听器
  removeDocumentClickListener();

  // 清理滚动超时
  if (scrollCheckTimeout) {
    clearTimeout(scrollCheckTimeout);
  }

  eventBus.all.clear();
});


const minimizeWindow = () => Window.getCurrent().minimize();
const toggleMaximize = async () => {
  const currentWindow = Window.getCurrent();
  const isMaximized = await currentWindow.isMaximized();
  isMaximized ? currentWindow.unmaximize() : currentWindow.maximize();
};
const closeWindow = () => Window.getCurrent().close();

// 自动调整 textarea 高度
function autoResizeTextarea(event: Event) {
  const textarea = event.target as HTMLTextAreaElement;
  textarea.style.height = 'auto'; // 重置高度以获取正确的 scrollHeight
  // 设置最小高度为单行高度，最大高度为 5 行左右
  const minHeight = 48; // 初始高度
  const maxHeight = minHeight * 5; // 约5行
  const newHeight = Math.max(minHeight, Math.min(textarea.scrollHeight, maxHeight));
  textarea.style.height = `${newHeight}px`;
}

// 添加图表查看器相关的状态
const isChartViewerOpen = ref(false);
const currentChartContent = ref('');
const currentChartSvg = ref('');
const chartViewerScale = ref(1);
const chartViewerPosition = ref({ x: 0, y: 0 });
const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });

// 打开图表查看器
function openChartViewer(svg: string, content: string) {
  currentChartSvg.value = svg;
  currentChartContent.value = content;
  chartViewerScale.value = 1;
  chartViewerPosition.value = { x: 0, y: 0 };
  isChartViewerOpen.value = true;

  // 阻止背景滚动
  document.body.style.overflow = 'hidden';
}

// 关闭图表查看器
function closeChartViewer() {
  isChartViewerOpen.value = false;

  // 恢复背景滚动
  document.body.style.overflow = '';
}

// 重置缩放和位置
function resetChartViewer() {
  chartViewerScale.value = 1;
  chartViewerPosition.value = { x: 0, y: 0 };
}

// 处理缩放
function handleChartViewerWheel(e: WheelEvent) {
  e.preventDefault();
  const delta = e.deltaY > 0 ? -0.1 : 0.1;
  const newScale = Math.max(0.5, Math.min(10, chartViewerScale.value + delta));
  chartViewerScale.value = newScale;
}

// 开始拖动
function startDrag(e: MouseEvent | TouchEvent) {
  isDragging.value = true;

  // 处理鼠标事件
  if ('clientX' in e) {
    dragStart.value = {
      x: e.clientX - chartViewerPosition.value.x,
      y: e.clientY - chartViewerPosition.value.y
    };
  }
  // 处理触摸事件
  else if (e.touches && e.touches[0]) {
    dragStart.value = {
      x: e.touches[0].clientX - chartViewerPosition.value.x,
      y: e.touches[0].clientY - chartViewerPosition.value.y
    };
  }
}

// 拖动过程
function handleDrag(e: MouseEvent | TouchEvent) {
  if (!isDragging.value) return;

  let clientX, clientY;

  // 处理鼠标事件
  if ('clientX' in e) {
    clientX = e.clientX;
    clientY = e.clientY;
  }
  // 处理触摸事件
  else if (e.touches && e.touches[0]) {
    clientX = e.touches[0].clientX;
    clientY = e.touches[0].clientY;
  } else {
    return;
  }

  chartViewerPosition.value = {
    x: clientX - dragStart.value.x,
    y: clientY - dragStart.value.y
  };
}

// 结束拖动
function endDrag() {
  isDragging.value = false;
}

// 在 data 部分添加变量来存储事件监听器引用
const documentClickListener = ref<((e: MouseEvent) => void) | null>(null);

// 修改 openMessageContextMenu 函数，添加事件冒泡控制和更严格的条件检查
function openMessageContextMenu(event: MouseEvent, messageIndex: number) {
  // 保存当前选中的文本（在任何操作之前立即获取）
  const selection = window.getSelection();
  const selectedText = selection ? selection.toString().trim() : "";
  selectedTextAtContextMenu.value = selectedText;

  console.log('捕获到的选中文本:', selectedText); // 调试日志

  // 防止事件冒泡和默认行为
  event.preventDefault();
  event.stopPropagation();

  // 确保清理可能已存在的菜单（关闭其他菜单）
  closeAllContextMenus();

  // 获取菜单将要放置的位置
  const x = event.clientX;
  const y = event.clientY;

  // 设置菜单索引
  messageContextMenuIndex.value = messageIndex;
  // 设置菜单位置
  messageContextMenuPosition.value = { x, y };
  // 显示菜单
  showMessageContextMenu.value = true;

  // 下一个渲染周期调整菜单位置
  nextTick(() => {
    adjustMenuPosition('.context-menu');
  });

  // 添加全局点击事件来关闭菜单
  setupDocumentClickListener();
}

// 修改对话历史右键菜单函数
function openChatContextMenu(event: MouseEvent, chatId: number) {
  // 防止事件冒泡和默认行为
  event.preventDefault();
  event.stopPropagation();

  // 确保清理可能已存在的菜单
  closeAllContextMenus();

  // 获取菜单将要放置的位置
  const x = event.clientX;
  const y = event.clientY;

  // 设置菜单ID和位置
  chatContextMenuPosition.value = { x, y };
  chatContextMenuId.value = chatId;
  // 显示菜单
  showChatContextMenu.value = true;

  // 下一个渲染周期调整菜单位置
  nextTick(() => {
    adjustMenuPosition('.context-menu');
  });

  // 添加全局点击事件来关闭菜单
  setupDocumentClickListener();
}

// 新增函数：统一调整菜单位置，避免超出视口
function adjustMenuPosition(menuSelector: string) {
  const menu = document.querySelector(menuSelector) as HTMLElement;
  if (!menu) return;

  // 获取视口大小
  const viewportWidth = window.innerWidth;
  const viewportHeight = window.innerHeight;

  // 获取菜单尺寸
  const menuWidth = menu.offsetWidth;
  const menuHeight = menu.offsetHeight;

  // 获取当前位置
  let currentX = parseInt(menu.style.left || '0', 10);
  let currentY = parseInt(menu.style.top || '0', 10);

  // 水平方向调整
  if (currentX + menuWidth > viewportWidth) {
    currentX = viewportWidth - menuWidth - 10; // 留出10px边距
  }
  // 确保不超出左边界
  currentX = Math.max(10, currentX);

  // 垂直方向调整
  if (currentY + menuHeight > viewportHeight) {
    currentY = viewportHeight - menuHeight - 10; // 留出10px边距
  }
  // 确保不超出上边界
  currentY = Math.max(10, currentY);

  // 应用调整后的位置
  menu.style.left = `${currentX}px`;
  menu.style.top = `${currentY}px`;
}

// 新增函数：关闭所有上下文菜单
function closeAllContextMenus() {
  showMessageContextMenu.value = false;
  showChatContextMenu.value = false;
  // 移除可能存在的文档事件监听器
  removeDocumentClickListener();
}

// 新增函数：设置文档点击事件监听器（避免重复）
function setupDocumentClickListener() {
  // 先移除可能已存在的监听器
  removeDocumentClickListener();

  // 创建新的监听器
  documentClickListener.value = (e: MouseEvent) => {
    const messageMenu = document.querySelector('.context-menu');
    // 如果点击的不是菜单内部元素，则关闭所有菜单
    if (messageMenu && !messageMenu.contains(e.target as Node)) {
      closeAllContextMenus();
    }
  };

  // 使用 setTimeout 确保监听器在当前点击事件处理完后才添加
  setTimeout(() => {
    document.addEventListener('click', documentClickListener.value!);
  }, 0);
}

// 新增函数：移除文档点击事件监听器
function removeDocumentClickListener() {
  if (documentClickListener.value) {
    document.removeEventListener('click', documentClickListener.value);
    documentClickListener.value = null;
  }
}

// 修改关闭对话右键菜单函数
function closeChatContextMenu() {
  showChatContextMenu.value = false;
  // 移除文档点击事件监听器
  removeDocumentClickListener();
}

// 修改关闭消息右键菜单函数
function closeMessageContextMenu() {
  showMessageContextMenu.value = false;
  // 移除文档点击事件监听器
  removeDocumentClickListener();
}

// 复制消息内容
async function copyMessageContent() {
  if (messageContextMenuIndex.value !== null && messageContextMenuIndex.value >= 0) {
    const chatContent = await invoke("get_chat_html") as ChatMessage[];
    const message = chatContent[messageContextMenuIndex.value];
    if (message) {
      try {
        await writeText(message.content);
        showNotification("消息内容已复制到剪贴板", "success");
      } catch (error) {
        console.error("复制消息内容失败:", error);
        showNotification("复制消息内容失败", "error");
      }
    }
  }
  closeMessageContextMenu();
}

// 复制选中文本
async function copySelectedText() {
  try {
    // 首先尝试使用保存的选中文本
    let textToCopy = selectedTextAtContextMenu.value;

    // 如果没有保存的文本，再尝试获取当前选中的文本
    if (!textToCopy || !textToCopy.trim()) {
      const selection = window.getSelection();
      textToCopy = selection ? selection.toString() : "";
    }

    if (textToCopy && textToCopy.trim()) {
      await writeText(textToCopy);
      showNotification("选中文本已复制到剪贴板", "success");
    } else {
      showNotification("没有选中任何文本", "info");
    }
  } catch (error) {
    console.error("复制选中文本失败:", error);
    showNotification("复制选中文本失败", "error");
  }
  closeMessageContextMenu();
}

// 删除消息
async function deleteMessage() {
  if (messageContextMenuIndex.value !== null && messageContextMenuIndex.value >= 0) {
    try {
      // 先获取当前聊天ID
      const chatId = await invoke("get_current_chat_id");

      // 使用chatId和messageIndex调用后端API
      const updatedContent = await invoke("delete_chat_message", {
        chatId: chatId,
        messageIndex: messageContextMenuIndex.value
      });

      // 刷新聊天界面
      updateChatContent(updatedContent as ChatMessage[]);
      showNotification("消息已删除", "success");
    } catch (error) {
      console.error("删除消息失败:", error);
      showNotification("删除消息失败", "error");
    }
  }
  closeMessageContextMenu();
}

// 重新生成当前消息
async function regenerateCurrentMessage() {
  if (messageContextMenuIndex.value !== null && messageContextMenuIndex.value >= 0) {
    try {
      // 显示加载状态
      isLoading.value = true;
      isStreaming.value = true;      // 获取当前选择的模型名称
      const currentApiType = selectedModel.value as ApiKeyType;
      const currentModelName = getCurrentSelectedModel(currentApiType);

      // 调用后端重新生成消息
      await invoke("regenerate_message", { 
        messageIndex: messageContextMenuIndex.value, 
        keyType: selectedModel.value,
        modelName: currentModelName 
      });

      // 处理将在事件监听器中完成
    } catch (error) {
      console.error("重新生成失败:", error);
      showNotification("重新生成失败", "error");
      isStreaming.value = false;
      isLoading.value = false;
    }
  }
  closeMessageContextMenu();
}
const canRegenerateMessage = computed(() => {
  let canRegenerateMessageResult = false;
  if (messageContextMenuIndex.value !== null && messageContextMenuIndex.value >= 0) {
    invoke("get_chat_html").then((chatContent) => {
      const messages = chatContent as ChatMessage[];
      const index = messageContextMenuIndex.value;
      // Only proceed if index is not null and within bounds
      if (messages && index !== null && messages.length > index) {
        const message = messages[index];
        canRegenerateMessageResult = message && message.msgtype === 'Assistant';
      } else {
        canRegenerateMessageResult = false;
      }
    }).catch(() => {
      canRegenerateMessageResult = false;
    });
  } else {
    canRegenerateMessageResult = false;
  }
  return canRegenerateMessageResult;
});
// 修改对 chat-messages 的点击处理，确保点击消息区域但非消息内容时关闭所有菜单
function handleChatMessagesClick(event: MouseEvent) {
  // 检查是否点击在消息本身
  const isMessageContent = (event.target as HTMLElement).closest('.message-content');
  // 如果点击在消息区域但不是消息内容本身，关闭所有右键菜单
  if (!isMessageContent) {
    closeAllContextMenus();
  }
}

// 取消重命名操作
function cancelRename() {
  isRenamingChat.value = false;
  newChatTitle.value = "";
  chatContextMenuId.value = null;
  removeDocumentClickListener();
}

// 提交重命名操作
async function submitRename() {
  if (!newChatTitle.value.trim() || !chatContextMenuId.value) {
    showNotification("标题不能为空", "error");
    return;
  }

  try {
    // 调用后端重命名API
    await invoke("rename_chat", {
      id: chatContextMenuId.value,
      newTitle: newChatTitle.value.trim()
    });

    // 更新本地聊天历史
    await loadChatHistory();
    showNotification("重命名成功", "success");

    // 关闭对话框
    isRenamingChat.value = false;
    newChatTitle.value = "";
    chatContextMenuId.value = null;
  } catch (error) {
    console.error("重命名失败:", error);
    showNotification("重命名失败", "error");
  }
}

// 取消删除操作
function cancelDelete() {
  showConfirmDelete.value = false;
  chatToDeleteId.value = null;
}

// 执行删除操作
async function submitDelete() {
  if (!chatToDeleteId.value) {
    showNotification("无效的对话ID", "error");
    return;
  }

  try {
    // 调用后端删除API
    await invoke("delete_chat", { id: chatToDeleteId.value });

    // 更新本地聊天历史

    // 如果当前显示的就是被删除的对话，则清空显示内容
    // 检查当前活跃的对话ID是否与被删除的ID相同
    await loadChatHistory();
    const currentId = chatHistory.value.find(item => item.id === currentChatId.value)?.id;
    if (currentId === chatToDeleteId.value) {
      updateChatContent([]);
    }

    showNotification("删除成功", "success");
  } catch (error) {
    console.error("删除失败:", error);
    showNotification("删除失败", "error");
  } finally {
    // 关闭对话框
    showConfirmDelete.value = false;
    chatToDeleteId.value = null;
  }
}

// 打开重命名对话框
function renameChatDialog() {
  if (!chatContextMenuId.value) {
    closeChatContextMenu();
    return;
  }

  // 获取当前对话标题作为默认值
  const currentChat = chatHistory.value.find(item => item.id === chatContextMenuId.value);
  if (currentChat) {
    newChatTitle.value = currentChat.title;
  }

  // 显示重命名对话框
  isRenamingChat.value = true;

  // 关闭右键菜单
  closeChatContextMenu();

  // 聚焦输入框
  nextTick(() => {
    const inputElement = document.querySelector('.modal-input') as HTMLInputElement;
    if (inputElement) {
      inputElement.focus();
      inputElement.select();
    }
  });
}

// 显示删除确认对话框
function confirmDeleteChat() {
  if (!chatContextMenuId.value) {
    showNotification("无效的对话ID", "error");
    return;
  }

  chatToDeleteId.value = chatContextMenuId.value;
  showConfirmDelete.value = true;
  closeChatContextMenu();
}

// 获取当前选择的模型名称
function getCurrentSelectedModel(apiType: ApiKeyType): string {
  const modelName = settings.value.model_selection[apiType];
  console.log(`获取模型名称 - API类型: ${apiType}, 模型名称: ${modelName}, 完整设置:`, settings.value.model_selection);
  return modelName || 'default-model';
}

function updateModel(event: Event) {
  const newModel = (event.target as HTMLSelectElement).value;
  if (newModel) {
    selectedModel.value = newModel;
    showNotification(`模型已切换到 ${newModel}`, 'success');
  } else {
    showNotification('请选择一个有效的模型', 'error');
  }
}

// 文件上传功能
async function uploadFile() {
  if (isStreaming.value) {
    showNotification("请等待当前消息输出完成", "error");
    return;
  }

  try {
    isLoading.value = true;
    await invoke("upload_file_from_local");
    showNotification("文件上传成功", "success");
    // 自动滚动到底部显示新添加的内容
    nextTick(() => {
      scrollToBottom(true, true); // 强制滚动，因为有新内容
    });
  } catch (error) {
    console.error("文件上传失败:", error);
    const errorMessage = error instanceof Error ? error.message : String(error);
    showNotification(`文件上传失败: ${errorMessage}`, "error");
  } finally {
    isLoading.value = false;
  }
}
</script>

<template>
  <div class="app-container">
    <!-- 自定义标题栏 - 仅在非移动设备上显示 -->
    <div v-if="!isMobile" class="custom-titlebar" data-tauri-drag-region>
      <div class="custom-titlebar" data-tauri-drag-region>
        <div class="app-icon">
          <img src="./assets/npulearn.png" alt="NPULearn" />
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
              <line x1="18" y1="6" x2="6" y2="18"></line>
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
            @contextmenu.prevent="openChatContextMenu($event, item.id)" class="history-item"
            :class="{ 'streaming-disabled': isStreaming }" :style="{ animationDelay: index * 0.05 + 's' }">
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

        <!-- 对话记录右键菜单 -->
        <div v-if="showChatContextMenu" class="context-menu"
          :style="{ top: chatContextMenuPosition.y + 'px', left: chatContextMenuPosition.x + 'px' }">
          <div class="context-menu-item" @click="renameChatDialog">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 20h9"></path>
              <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path>
            </svg>
            重命名
          </div>
          <div class="context-menu-item delete-item" @click="confirmDeleteChat">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 6h18"></path>
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
            </svg>
            删除
          </div>
        </div>

        <div class="history-footer"> <button @click="toggleSettings" class="settings-button">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <!-- 现代化的设置图标：简洁的滑块式设计 -->
              <rect x="4" y="8" width="2" height="12" rx="1"></rect>
              <rect x="11" y="3" width="2" height="14" rx="1"></rect>
              <rect x="18" y="8" width="2" height="12" rx="1"></rect>
              <circle cx="5" cy="4" r="2"></circle>
              <circle cx="12" cy="19" r="2"></circle>
              <circle cx="19" cy="4" r="2"></circle>
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
          <!-- 右侧模型选择区 -->
          <div class="model-selector">
            <select v-model="selectedModel" @change="updateModel">
              <option v-for="model in ApiKeyType" :key="model" :value="model">{{ model }}</option>
            </select>
          </div>
        </header>

        <!-- 聊天内容区域 - 添加点击事件处理函数 -->
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
          <div v-html="processedChatContent" class="chat-messages" @click="handleChatMessagesClick"></div>

          <!-- 悬浮滚动到底部按钮 -->
          <transition name="scroll-button">
            <button v-if="showScrollToBottomButton" class="scroll-to-bottom-button" @click="forceScrollToBottom"
              title="滚动到底部">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="7 13 12 18 17 13"></polyline>
                <polyline points="7 6 12 11 17 6"></polyline>
              </svg>
            </button>
          </transition>

          <!-- 消息右键菜单 - 添加固定的位置样式 -->
          <div v-if="showMessageContextMenu" class="context-menu"
            :style="{ top: messageContextMenuPosition.y + 'px', left: messageContextMenuPosition.x + 'px' }">
            <div class="context-menu-item" @click="copyMessageContent">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
              复制内容
            </div>
            <div class="context-menu-item" @click="copySelectedText"
              v-if="selectedTextAtContextMenu && selectedTextAtContextMenu.trim()">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M7 2h10a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z"></path>
                <path d="M9 2v20"></path>
                <path d="M15 2v20"></path>
              </svg>
              复制选中文本
            </div>
            <div class="context-menu-item delete-item" @click="deleteMessage">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M3 6h18"></path>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              </svg>
              删除消息
            </div>
            <div class="context-menu-item" v-if="canRegenerateMessage" @click="regenerateCurrentMessage">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M23 4v6h-6"></path>
                <path d="M1 20v-6h6"></path>
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10"></path>
                <path d="M20.49 15a9 9 0 0 1-14.85 3.36L1 14"></path>
              </svg>
              重新生成
            </div>
          </div>
        </div> <!-- 底部输入区 -->
        <div class="chat-input-area">
          <form @submit.prevent="sendStreamMessage" class="input-form">
            <div class="input-container">
              <button type="button" class="upload-button" @click="uploadFile" :disabled="isStreaming" title="上传文件">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none"
                  stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                  <polyline points="14,2 14,8 20,8"></polyline>
                  <line x1="16" y1="13" x2="8" y2="13"></line>
                  <line x1="16" y1="17" x2="8" y2="17"></line>
                  <polyline points="10,9 9,9 8,9"></polyline>
                </svg>
              </button>
              <textarea v-model="inputMessage" placeholder="输入消息... (Ctrl+Enter 发送)"
                class="message-input animated-input" rows="1" @keydown="handleInputKeydown"
                @input="autoResizeTextarea"></textarea>
            </div>
            <!-- 将按钮移到 textarea 外部 -->
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

    <!-- 图表查看器模态框 -->
    <div v-if="isChartViewerOpen" class="chart-viewer-modal" @wheel.prevent="handleChartViewerWheel">
      <div class="chart-viewer-overlay" @click="closeChartViewer"></div>
      <div class="chart-viewer-content">
        <div class="chart-viewer-header">
          <h3>图表查看器</h3>
          <div class="chart-viewer-controls">
            <button class="chart-control-button" @click="resetChartViewer" title="重置缩放">
              <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M3 12a9 9 0 1 0 18 0a9 9 0 0 0-18 0z"></path>
                <path d="M14 8H8v6h6"></path>
              </svg>
            </button>
            <button class="chart-control-button" @click="closeChartViewer" title="关闭">
              <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          </div>
        </div>
        <div class="chart-viewer-body">
          <div class="chart-viewer-diagram" @mousedown="startDrag" @touchstart="startDrag" :style="{
            transform: `scale(${chartViewerScale}) translate(${chartViewerPosition.x / chartViewerScale}px, ${chartViewerPosition.y / chartViewerScale}px)`,
            cursor: isDragging ? 'grabbing' : 'grab'
          }" v-html="currentChartSvg"></div>
        </div>
        <div class="chart-viewer-footer">
          <div class="chart-viewer-info">
            <span>缩放: {{ Math.round(chartViewerScale * 100) }}%</span>
            <span>提示: 滚轮缩放, 拖动移动</span>
          </div>
          <div class="chart-viewer-code-toggle">
            <details>
              <summary>查看图表代码</summary>
              <pre class="chart-viewer-code">{{ currentChartContent }}</pre>
            </details>
          </div>
        </div>
      </div>
    </div>

    <!-- 对话重命名对话框 - 移到根容器层级 -->
    <div v-if="isRenamingChat" class="modal-overlay" @click.self="cancelRename">
      <div class="modal-content">
        <div class="modal-header">
          <h3>重命名对话</h3>
          <button class="modal-close" @click="cancelRename">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <input type="text" v-model="newChatTitle" placeholder="输入新标题" class="modal-input" @keyup.enter="submitRename">
        </div>
        <div class="modal-footer">
          <button class="modal-button cancel" @click="cancelRename">取消</button>
          <button class="modal-button confirm" @click="submitRename">确认</button>
        </div>
      </div>
    </div>

    <!-- 对话删除确认对话框 - 移到根容器层级 -->
    <div v-if="showConfirmDelete" class="modal-overlay" @click.self="cancelDelete">
      <div class="modal-content">
        <div class="modal-header">
          <h3>删除对话</h3>
          <button class="modal-close" @click="cancelDelete">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <p>确定要删除这个对话吗？此操作不可撤销。</p>
        </div>
        <div class="modal-footer">
          <button class="modal-button cancel" @click="cancelDelete">取消</button>
          <button class="modal-button delete" @click="submitDelete">删除</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&display=swap');
</style>
<style>
.history-sidebar {
  width: var(--sidebar-width);
  background-color: var(--card-bg);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  position: fixed;
  /* 检测是否有标题栏， 有就设置top为32px*/
  --titlebar-height: v-bind("isMobile ? '0px' : '32px'");
  height: calc(100vh - var(--titlebar-height));
  /* 默认留出标题栏高度 */
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
  /* 检测是否有标题栏， 有就设置top为32px*/
  --titlebar-height: v-bind("isMobile ? '0px' : '32px'");
  height: calc(100vh - var(--titlebar-height));
  /* 默认减去标题栏高度 */
  overflow: hidden;
}

/* 响应式设计调整 */
@media (min-width: 768px) {
  .history-sidebar {
    transform: translateX(0);
    position: relative;
    box-shadow: none;
    top: 0;
    /* 在大屏幕上始终从顶部开始 */
  }

  .chat-container {
    margin-left: 0;
    width: calc(100% - var(--sidebar-width));
    /* 大屏幕非移动设备下，如果标题栏存在，需要减去其高度 */
    --titlebar-height: v-bind("isMobile ? '0px' : '32px'");
    height: calc(100vh - var(--titlebar-height));
  }

}
</style>
<style src="./style.css"></style>

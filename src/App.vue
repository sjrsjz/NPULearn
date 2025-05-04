<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch, computed } from "vue";
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
import html2canvas from 'html2canvas'; // 导入 html2canvas

import { useSettingsProvider } from './composables/useSettings';
import { Window } from '@tauri-apps/api/window';

// 初始化全局设置，在整个应用中提供设置
const {
  notification,
  showNotification,
  initAppSettings
} = useSettingsProvider();

const isAppLoading = ref(true);
const isMobile = ref(false); // 添加移动设备状态

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

// 移除动画状态控制变量，改为固定显示
const fadeInMessages = ref(true); // 保留但始终为true，以确保内容始终可见
const messageTransition = ref(false); // 禁用消息过渡动画

// 添加用于防抖渲染的变量
const pendingMermaidRender = ref(false);
const mermaidRenderTimer = ref<number | null>(null);

// 添加变量标记是否正在接收流式消息
const isReceivingStream = ref(false);

// 添加对话重命名和删除功能所需的状态
const currentChatId = ref<number | null>(null); // 当前选中的对话ID
const isRenamingChat = ref(false); // 是否正在重命名对话
const newChatTitle = ref(""); // 新的对话标题
const showConfirmDelete = ref(false); // 是否显示删除确认对话框
const chatToDeleteId = ref<number | null>(null); // 要删除的对话ID
const showMessageContextMenu = ref(false); // 是否显示消息上下文菜单
const messageContextMenuPosition = ref({ x: 0, y: 0 }); // 消息上下文菜单位置
const messageContextMenuIndex = ref<number | null>(null); // 当前右键菜单对应的消息索引

// 添加对话历史项右键菜单相关状态
const showChatContextMenu = ref(false);
const chatContextMenuPosition = ref({ x: 0, y: 0 });
const chatContextMenuId = ref<number | null>(null);

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

// 修改渲染Mermaid图表函数，添加防抖机制
async function renderMermaidDiagrams(retryCount = 0, maxRetries = 3) {
  // 如果已有渲染计划，不重复触发
  if (pendingMermaidRender.value) {
    console.log('已有渲染计划，忽略当前渲染请求');
    return;
  }

  pendingMermaidRender.value = true;

  // 清除之前的计时器（如果有）
  if (mermaidRenderTimer.value !== null) {
    clearTimeout(mermaidRenderTimer.value);
  }

  // 设置防抖延迟，避免频繁渲染
  mermaidRenderTimer.value = setTimeout(async () => {
    pendingMermaidRender.value = false;
    mermaidRenderTimer.value = null;

    const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
      (document.documentElement.getAttribute('data-theme') === 'system' &&
        window.matchMedia('(prefers-color-scheme: dark)').matches);

    // 动态更新主题
    mermaid.initialize({
      theme: isDark ? 'dark' : 'default',
      securityLevel: 'loose',
      logLevel: 'debug',
      startOnLoad: false
    });

    try {
      // 查找所有需要渲染的UML元素
      const umlElements = document.querySelectorAll('.chat-messages .mermaid-container:not(.loaded)');
      console.log(`尝试渲染 ${umlElements.length} 个UML图表，当前重试次数: ${retryCount}`);

      if (umlElements.length === 0 && retryCount === 0) {
        // 第一次调用且没有找到未加载的图表，检查是否需要全局重新渲染
        const allUmlElements = document.querySelectorAll('.chat-messages .mermaid-container');
        if (allUmlElements.length > 0) {
          // 不再自动重新渲染所有图表，避免性能问题
          console.log(`未找到未加载的图表，存在 ${allUmlElements.length} 个已加载图表`);
        }
        return;
      }

      let renderPromises = [];

      for (const element of umlElements) {
        const id = element.getAttribute('data-diagram-id');
        const encodedContent = element.getAttribute('data-diagram-content');
        const lastRenderedContent = element.getAttribute('data-last-rendered');

        // 跳过内容未变化的图表渲染，避免重复工作
        if (encodedContent && lastRenderedContent && encodedContent === lastRenderedContent) {
          console.log(`跳过图表 ID: ${id} 的渲染，内容未变化`);
          continue;
        }

        if (encodedContent && id) {
          let content = '';
          try {
            // 清空现有内容
            element.innerHTML = '<div class="mermaid-loading">UML图表渲染中...</div>';

            // 正确解码内容
            content = decodeURIComponent(encodedContent);

            // 使用Promise.resolve()包装渲染过程，以便收集所有渲染任务
            renderPromises.push(
              Promise.resolve().then(async () => {
                if (typeof content === 'string' && content.length > 0) {
                  try {
                    const { svg } = await mermaid.render(id, content);
                    element.innerHTML = svg;
                    // 添加图表加载完成的标记
                    element.classList.add('loaded');
                    // 记录已渲染的内容，用于后续比较避免重复渲染
                    element.setAttribute('data-last-rendered', encodedContent);
                    return true;
                  } catch (renderError) {
                    console.error(`单个图表渲染失败 ID ${id}:`, renderError);
                    element.innerHTML = `
                      <div class="mermaid-error">
                        <p>UML图表渲染失败</p>
                        <pre>${renderError}</pre>
                        <div class="mermaid-source">
                          <details>
                            <summary>查看原始图表代码</summary>
                            <pre>${content}</pre>
                          </details>
                        </div>
                        <button class="retry-render-button" data-diagram-id="${id}">
                          重试渲染
                        </button>
                      </div>
                    `;
                    return false;
                  }
                } else {
                  throw new Error("解码后的内容为空或无效。");
                }
              })
            );
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
                    <pre>${content}</pre>
                  </details>
                </div>
                <button class="retry-render-button" data-diagram-id="${id}">
                  重试渲染
                </button>
              </div>
            `;
          }
        } else {
          // 如果容器缺少必要的属性，则发出警告
          console.warn("发现缺少必要属性（id 或 content）的 Mermaid 容器。", element);
        }
      }

      // 等待所有渲染完成
      if (renderPromises.length > 0) {
        const results = await Promise.all(renderPromises);
        const failedCount = results.filter(success => !success).length;

        // 如果有失败的图表，且未超过最大重试次数，则重试
        if (failedCount > 0 && retryCount < maxRetries) {
          console.log(`${failedCount}个图表渲染失败，将在1.5秒后重试 (${retryCount + 1}/${maxRetries})`);
          setTimeout(() => renderMermaidDiagrams(retryCount + 1, maxRetries), 1500);
        } else if (failedCount > 0) {
          console.log(`渲染完成，但有${failedCount}个图表渲染失败，已达到最大重试次数`);
          // 为失败的图表添加重试按钮事件监听
          setupRetryButtons();

          // 添加这一行来设置图表的可点击功能
          setupMermaidRefresh();
        } else {
          console.log('所有图表渲染成功');

          // 添加这一行来设置图表的可点击功能
          setupMermaidRefresh();
        }
      } else {
        // 如果没有需要渲染的图表，也需要调用setupMermaidRefresh来处理已渲染的图表
        setupMermaidRefresh();
      }
    } catch (error) {
      console.error("处理Mermaid图表失败:", error);
      if (retryCount < maxRetries) {
        console.log(`整体处理失败，将在1.5秒后重试 (${retryCount + 1}/${maxRetries})`);
        setTimeout(() => renderMermaidDiagrams(retryCount + 1, maxRetries), 1500);
      } else {
        // 即使出错，也尝试为已渲染的图表添加交互功能
        setupMermaidRefresh();
      }
    }
  }, 500); // 500ms防抖延迟
}

// 设置图表渲染失败后的重试按钮事件
function setupRetryButtons() {
  nextTick(() => {
    document.querySelectorAll('.chat-messages .retry-render-button').forEach(button => {
      button.addEventListener('click', async (e) => {
        e.preventDefault();
        const targetButton = e.target as HTMLElement;
        const diagramId = targetButton.getAttribute('data-diagram-id');
        const container = document.querySelector(`.mermaid-container[data-diagram-id="${diagramId}"]`);

        if (container) {
          // 移除loaded类以便重新渲染
          container.classList.remove('loaded');
          showNotification("正在重新渲染图表...", "info");

          // 特别处理这个容器
          await renderMermaidDiagrams(0, 3);
        }
      });
    });
  });
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
      chatContent.value = await invoke("select_chat_by_id", { id });
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
async function applyHighlight() {
  await nextTick(); // 确保 DOM 更新完成
  // 查找所有代码块并应用高亮
  const codeElements = document.querySelectorAll('.chat-messages pre code');

  for (const el of codeElements) {
    const preElement = el.parentElement as HTMLPreElement | null;
    if (!preElement) continue;
    hljs.highlightElement(el as HTMLElement);
    const codeContent = el.textContent?.trim() || '';
    if (!codeContent) continue;

    // 检测是否为 mermaid 代码块
    if (el.classList.contains('language-mermaid')) {
      const preElement = el.parentElement;
      if (!preElement) return;

      // 获取 mermaid 代码内容
      const mermaidContent = el.textContent?.trim() || '';

      if (!mermaidContent) return;

      // 创建唯一的图表ID
      const diagramId = `mermaid-diagram-${Date.now()}-${Math.floor(Math.random() * 10000)}`;

      // 编码内容，以便在属性中安全存储
      const encodedContent = encodeURIComponent(mermaidContent);

      // 创建 mermaid 容器 - 根据流式传输状态显示不同内容
      const mermaidContainer = document.createElement('div');
      mermaidContainer.className = 'mermaid-container';
      mermaidContainer.setAttribute('data-diagram-id', diagramId);
      mermaidContainer.setAttribute('data-diagram-content', encodedContent);

      // 根据是否在流式传输中显示不同的内容
      if (isReceivingStream.value) {
        mermaidContainer.innerHTML = `
            <div class="mermaid-loading">
              <div class="placeholder-box">
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M12 20.94c1.5 0 2.75 1.06 4 1.06 3 0 6-8 6-12.22A4.91 4.91 0 0 0 17 5c-2.22 0-4 1.44-5 2-1-.56-2.78-2-5-2a4.9 4.9 0 0 0-5 4.78C2 14 5 22 8 22c1.25 0 2.5-1.06 4-1.06Z"></path>
                  <path d="M10 2c1 .5 2 2 2 5"></path>
                </svg>
                <div>UML图表将在消息完整接收后渲染</div>
              </div>
              <div class="mermaid-preview">
                <div class="preview-header">代码预览：</div>
                <pre class="preview-code">${mermaidContent}</pre>
              </div>
            </div>`;
        console.log(`流式传输中: 为 mermaid 代码块创建带预览的占位符容器 ${diagramId}`);
      } else {
        mermaidContainer.innerHTML = `<div class="mermaid-loading">UML图表加载中...</div>`;
        console.log(`已将 language-mermaid 代码块转换为 mermaid 渲染容器: ${diagramId}`);
      }

      // 替换原始的 pre 元素
      preElement.parentNode?.replaceChild(mermaidContainer, preElement);
    }
    // 新增：检测是否为 tool_code 代码块并进行处理
    else if (el.classList.contains('language-tool_code')) {
      console.log("检测到 tool_code 代码块:", codeContent);

      // 创建工具代码容器
      const toolCodeContainer = document.createElement('div');
      toolCodeContainer.className = 'tool-code-container';

      // 设置加载状态并保留原始代码
      toolCodeContainer.innerHTML = `
        <div class="tool-code-loading">正在解析工具代码...</div>
        <pre class="tool-code-original"><code>${codeContent}</code></pre>
      `;

      // 替换原始的 pre 元素
      preElement.parentNode?.replaceChild(toolCodeContainer, preElement);

      try {
        // 调用后端解析代码
        const astResult = await invoke<string>("parse_code", { code: codeContent });
        console.log("AST 解析结果:", astResult);

        // 解析AST JSON并处理
        const astJson = JSON.parse(astResult);
        const apiInfo = parseApiCall(astJson);

        if (apiInfo) {
          // 如果成功解析出API调用信息，显示结构化结果
          toolCodeContainer.innerHTML = `
            <div class="tool-code-header">工具代码解析结果:</div>
            <div class="tool-code-result">
              <div class="tool-api-info">
                <div class="tool-api-row"><span class="tool-api-label">API:</span> <span class="tool-api-value">${apiInfo.api_name}</span></div>
                <div class="tool-api-row"><span class="tool-api-label">函数:</span> <span class="tool-api-value">${apiInfo.function_name}</span></div>
                ${Object.entries(apiInfo.arguments).map(([key, value]) =>
            `<div class="tool-api-row"><span class="tool-api-label">参数 ${key}:</span> <span class="tool-api-value tool-api-param">${value}</span></div>`
          ).join('')}
              </div>
            </div>
            <details class="tool-code-details">
              <summary>查看AST详情</summary>
              <pre class="tool-code-ast"><code>${JSON.stringify(astJson, null, 2)}</code></pre>
            </details>
            <div class="tool-code-header original-header">原始代码:</div>
            <pre class="tool-code-original"><code>${codeContent}</code></pre>
          `;
        } else {
          // 如果解析失败，显示原始AST结果
          toolCodeContainer.innerHTML = `
            <div class="tool-code-header">工具代码 AST:</div>
            <pre class="tool-code-ast"><code>${JSON.stringify(JSON.parse(astResult), null, 2)}</code></pre>
            <div class="tool-code-header original-header">原始代码:</div>
            <pre class="tool-code-original"><code>${codeContent}</code></pre>
          `;
        }

        // 对AST结果应用高亮
        const astCodeElement = toolCodeContainer.querySelector('.tool-code-ast code');
        if (astCodeElement) {
          hljs.highlightElement(astCodeElement as HTMLElement);
        }

        // 对原始代码应用高亮
        const originalCodeElement = toolCodeContainer.querySelector('.tool-code-original code');
        if (originalCodeElement) {
          hljs.highlightElement(originalCodeElement as HTMLElement);
        }
      } catch (error) {
        console.error("解析 tool_code 失败:", error);
        // 显示错误信息
        toolCodeContainer.innerHTML = `
          <div class="tool-code-error">解析工具代码失败:</div>
          <pre class="tool-code-error-message">${error instanceof Error ? error.message : String(error)}</pre>
          <div class="tool-code-header original-header">原始代码:</div>
          <pre class="tool-code-original"><code>${codeContent}</code></pre>
        `;

        // 确保原始代码也被高亮
        const originalCodeElement = toolCodeContainer.querySelector('.tool-code-original code');
        if (originalCodeElement) {
          hljs.highlightElement(originalCodeElement as HTMLElement);
        }
      }
    }
    else {
      // 为非 mermaid 的代码块添加复制按钮
      const preElement = el.parentElement;
      if (!preElement || preElement.querySelector('.code-copy-button')) return;

      // 获取代码内容
      const codeContent = el.textContent || '';

      // 创建复制按钮
      const copyButton = document.createElement('button');
      copyButton.className = 'code-copy-button';
      copyButton.innerHTML = `
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        `;
      copyButton.title = "复制代码";

      // 添加点击事件
      copyButton.addEventListener('click', async (e) => {
        e.preventDefault();
        try {
          await writeText(codeContent);
          // 临时更改按钮状态
          copyButton.innerHTML = `
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M20 6L9 17l-5-5"></path>
              </svg>
            `;
          copyButton.classList.add('success');

          // 2秒后恢复原样
          setTimeout(() => {
            copyButton.innerHTML = `
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                </svg>
              `;
            copyButton.classList.remove('success');
          }, 2000);

          showNotification("代码已复制到剪贴板", "success");
        } catch (error) {
          console.error("复制代码失败:", error);
          showNotification("复制代码失败", "error");
        }
      });

      // 添加复制按钮到 pre 元素
      preElement.classList.add('code-block-with-copy');
      preElement.appendChild(copyButton);
    }
  }

  // 代码高亮和mermaid处理完成后，触发图表渲染
  // 但仅在非流式传输状态下进行
  if (!isReceivingStream.value) {
    setTimeout(() => {
      renderMermaidDiagrams();
    }, 300);
  } else {
    console.log("流式传输中，跳过Mermaid图表渲染");
  }
}

/**
 * 解析工具代码AST，提取API调用的关键信息
 * @param ast 解析后的AST JSON对象
 * @returns 包含API调用信息的对象，如果解析失败则返回null
 */
function parseApiCall(ast: any) {
  try {
    // 检查根节点是否为LambdaCall类型
    if (ast.node_type !== "LambdaCall") {
      console.warn("根节点不是LambdaCall类型");
      return null;
    }

    // 提取基本信息
    let result: {
      type: string;
      print_call: boolean;
      api_name: string | null;
      function_name: string | null;
      arguments: Record<string, string>;
    } = {
      type: "api_call",
      print_call: false,
      api_name: null,
      function_name: null,
      arguments: {}
    };

    // 检查是否是print调用
    if (ast.children && ast.children.length >= 1 &&
      ast.children[0].node_type === "Variable(\"print\")") {
      result.print_call = true;
    }

    // 查找API调用部分(GetAttr节点)
    let apiCallNode = null;

    // 如果是print调用，API调用节点在第二个子节点的子节点里
    if (result.print_call && ast.children.length >= 2 && ast.children[1].children) {
      const tupleNode = ast.children[1];
      if (tupleNode.children.length > 0) {
        const firstChild = tupleNode.children[0];
        if (firstChild.node_type === "LambdaCall" && firstChild.children.length > 0) {
          apiCallNode = firstChild.children[0]; // 应该是GetAttr节点
        }
      }
    } else if (!result.print_call) {
      // 直接API调用情况(没有print)
      // 根据实际结构调整查找逻辑
      apiCallNode = ast.children[0]; // 可能直接是GetAttr
    }

    // 提取API名称和函数名
    if (apiCallNode && apiCallNode.node_type === "GetAttr") {
      // 提取API名称(第一个子节点)
      if (apiCallNode.children.length > 0 &&
        apiCallNode.children[0].node_type &&
        apiCallNode.children[0].node_type.startsWith("Variable(")) {
        // 从 Variable("default_api") 中提取 default_api
        const apiNameMatch = apiCallNode.children[0].node_type.match(/Variable\("(.+)"\)/);
        if (apiNameMatch) {
          result.api_name = apiNameMatch[1];
        }
      }

      // 提取函数名(第二个子节点)
      if (apiCallNode.children.length > 1 &&
        apiCallNode.children[1].node_type &&
        apiCallNode.children[1].node_type.startsWith("String(")) {
        // 从 String("image_gen") 中提取 image_gen
        const funcNameMatch = apiCallNode.children[1].node_type.match(/String\("(.+)"\)/);
        if (funcNameMatch) {
          result.function_name = funcNameMatch[1];
        }
      }
    }

    // 查找参数节点 - 通常在LambdaCall的第二个子节点
    let argsNode = null;
    if (result.print_call && ast.children.length >= 2 &&
      ast.children[1].children && ast.children[1].children.length > 0) {
      const lambdaCallNode = ast.children[1].children[0];
      if (lambdaCallNode.children && lambdaCallNode.children.length > 1) {
        argsNode = lambdaCallNode.children[1];
      }
    } else if (!result.print_call && ast.children.length > 1) {
      argsNode = ast.children[1];
    }

    // 处理参数 - 在Tuple节点中查找Assign节点
    if (argsNode && argsNode.node_type === "Tuple") {
      for (const child of argsNode.children) {
        if (child.node_type === "Assign" && child.children.length >= 2) {
          const paramName = child.children[0].node_type.match(/Variable\("(.+)"\)/)?.[1];

          // 参数值可能是字符串或其他类型
          let paramValue = null;
          if (child.children[1].node_type.startsWith("String(")) {
            // 从 String("value") 中提取 value
            const valueMatch = child.children[1].node_type.match(/String\("(.+)"\)/);
            if (valueMatch) {
              paramValue = valueMatch[1];
            } else {
              // 如果无法提取，则使用原始token值
              paramValue = child.children[1].start_token.token;
            }
          } else if (child.children[1].start_token) {
            paramValue = child.children[1].start_token.token;
          }

          if (paramName && paramValue !== null) {
            result.arguments[paramName] = paramValue;
          }
        }
      }
    }

    // 检查是否解析到足够的信息
    if (!result.api_name || !result.function_name) {
      console.warn("未能提取完整的API调用信息");
      return null;
    }

    return result;
  } catch (error) {
    console.error("解析API调用失败:", error);
    return null;
  }
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
// 修改 updateChatContent 函数，移除动画效果
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
  processedChatContent.value = `
  <div class="scoped-content fade-in" data-theme="${isDark ? 'dark' : 'light'}">
    ${messagesHtml}
    <style>
      .scoped-content {
        opacity: 1; /* 直接设置为可见，移除过渡效果 */
      }
      
      .scoped-content.fade-in {
        opacity: 1;
      }
      
      .message-wrapper {
        display: flex;
        margin-bottom: 28px;
        position: relative;
        gap: 12px;
        opacity: 1;
        transform: translateY(0);
      }
      
      /* 移除所有消息出现的动画效果 */
      
      .message-avatar {
        display: flex;
        flex-direction: column;
        align-items: center;
        margin-top: 4px;
        flex-shrink: 0;
        width: 42px;
        transform: scale(1);
        opacity: 1;
      }
      
      /* 移除头像的动画效果 */
      
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
        transform: translateY(16px);
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
      
      /* Mermaid图表容器样式 - 移除动画效果 */
      .mermaid-container {
        background-color: ${isDark ? '#1e293b' : '#f6f8fa'};
        border-radius: 6px;
        margin: 16px 0;
        padding: 16px;
        overflow: hidden;
        overflow-x: auto;
        box-shadow: 0 2px 6px ${isDark ? 'rgba(0, 0, 0, 0.4)' : 'rgba(0, 0, 0, 0.08)'};
        border: 1px solid ${isDark ? '#334155' : '#e1e4e8'};
        position: relative;
        min-height: 100px;
        display: flex;
        justify-content: center;
        align-items: center;
        text-align: center;
      }
      
      /* 移除加载动画，使图表立即显示 */
      .mermaid-container.loaded {
        opacity: 1;
        transform: scale(1);
      }
      
      .mermaid-loading {
        color: var(--text-secondary);
        font-size: 14px;
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
      }
      
      .markdown-body pre {
        background-color: ${isDark ? '#1e293b' : '#f6f8fa'};
        border-radius: 6px;
        margin: 16px 0;
        padding: 16px;
        overflow-x: auto; /* 确保代码块水平滚动 */
        box-shadow: 0 2px 6px ${isDark ? 'rgba(0, 0, 0, 0.4)' : 'rgba(0, 0, 0, 0.08)'};
        border: 1px solid ${isDark ? '#334155' : '#e1e4e8'};
        position: relative;
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
        overflow-x: auto; /* 确保表格水平滚动 */
        display: block; /* 确保 overflow-x 生效 */
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
      

      
      .message-wrapper.user-message-right {
        flex-direction: row-reverse;
      }
      
      .message-wrapper.user-message-right.message-bubble {
        max-width: calc(85% - 42px);
      }
      
      .message-wrapper.user-message-right.message-content {
        border-top-left-radius: 18px;
        border-top-right-radius: 4px;
      }
      
      .message-actions {
        display: flex;
        gap: 8px;
        margin-top: 8px;
        justify-content: flex-start;
      }

      .message-actions.user {
        justify-content: flex-end;
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
      
      .render-image-button {
        color: #8b5cf6; /* 紫色图标 */
      }
      
      .render-image-button:hover {
        background-color: rgba(139, 92, 246, 0.1);
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
      
      
      .render-image-button {
        color: #8b5cf6; /* 紫色图标 */
      }
      
      .render-image-button:hover {
        background-color: rgba(139, 92, 246, 0.1);
      }

      .thinking-details {
        margin-bottom: 16px;
        border-radius: var(--radius);
        overflow: hidden;
      }
      .thinking-summary {
        cursor: pointer;
        font-weight: bold;
        padding: 8px 12px;
        background-color: var(--card-bg, #f0f0f0);
        color: var(--text-color);
        border-radius: var(--radius-sm);
        border: 1px solid var(--border-color);
        transition: background-color 0.2s;
      }
      .thinking-summary:hover {
        background-color: var(--hover-bg, #e0e0e0);
      }
      .thinking-content {
        padding: 12px 16px;
        border: 1px solid var(--border-color, #ddd);
        border-radius: var(--radius-sm);
        margin-top: 8px;
        background-color: var(--card-bg, #ffffff);
        color: var(--text-color);
      }
      [data-theme="dark"] .thinking-summary {
        background-color: var(--card-bg, #1e293b);
        border-color: var(--border-color, #334155);
      }
      [data-theme="dark"] .thinking-summary:hover {
        background-color: var(--hover-bg, #2d3748);
      }
      [data-theme="dark"] .thinking-content {
        border-color: var(--border-color, #334155);
        background-color: var(--card-bg, #1e293b);
      }

      /* 移动端优化 */
      @media (max-width: 767px) {
        .message-bubble {
          max-width: calc(90% - 42px);
          transform: translateY(0);
        }
        
        .message-content {
          padding: 12px 16px;
        }
        
        .avatar-icon {
          width: 32px;
          height: 32px;
          display: none;
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
        
        /* 在移动端调整消息和头像的布局 */
        .message-wrapper {
          flex-direction: column;
          margin-bottom: 32px;
        }
        
        .message-wrapper.user-message-right {
          flex-direction: column;
          align-items: flex-end;
        }
        
        .message-avatar {
          margin-top: 0;
          margin-bottom: 8px;
          flex-direction: row;
          width: auto;
          align-self: flex-start;
        }
        
        .message-avatar.user {
          align-self: flex-end;
        }
        
        .avatar-icon {
          margin-bottom: 0;
          margin-right: 8px;
        }
        
        .message-bubble {
          max-width: 100%;
        }
        
        /* 修复用户消息在移动端的布局 */
        .message-wrapper.user-message-right {
          flex-direction: column;
          align-items: flex-end;
        }
        
        /* 修复时间显示位置 - 对用户消息特殊处理 */
        .message-wrapper.user-message-right .message-avatar {
          align-self: flex-end;
          flex-direction: row; /* 不使用反向排列，让时间保持在右侧 */
        }
        
        .message-wrapper.user-message-right .message-time {
          order: -1; /* 使时间元素显示在最左侧 */
          margin-right: 8px; /* 给时间和头像之间添加间距 */
          margin-left: 0;
        }
        
        /* 确保头像和时间垂直对齐 */
        .message-avatar {
          display: flex;
          align-items: center;
        }
        
        /* 为确保用户头像和助手头像的样式一致 */
        .message-wrapper.user-message-right .message-avatar .avatar-icon {
          margin-right: 0;
          margin-left: 0; /* 移除左侧间距 */
        }
        
        /* 所有消息的气泡宽度保持一致 */
        .message-bubble {
          max-width: 100%;
        }
      }
    </style>
  </div>
`;

  // 确保内容立即可见，不依赖动画
  nextTick(() => {
    // 强制使内容可见
    const chatMessages = document.querySelector('.chat-messages');
    if (chatMessages) {
      chatMessages.querySelector('.scoped-content')?.classList.add('fade-in');

      // 为每个消息容器添加右键菜单事件
      document.querySelectorAll('.message-content[data-message-index]').forEach(messageElement => {
        messageElement.addEventListener('contextmenu', (e) => {
          e.preventDefault();
          e.stopPropagation();
          const messageIndex = parseInt((messageElement as HTMLElement).dataset.messageIndex || '0', 10);
          openMessageContextMenu(e as MouseEvent, messageIndex);
        });
      });
    }

    // 应用代码高亮
    applyHighlight();

    // 渲染数学公式
    renderMathInElement();

    // 设置外部链接处理
    setupExternalLinks();

    // 设置复制按钮和重做按钮的事件监听器
    setupActionButtons();

    // 滚动到底部
    scrollToBottom();

    // 刷新全局样式，确保主题一致性
    refreshGlobalStyles();

    // 只在非流传输状态下渲染UML图表，添加明确的日志
    if (!isReceivingStream.value) {
      console.log("消息更新完成，准备渲染UML图表");
      setTimeout(() => {
        renderMermaidDiagrams();

        // 即使没有新渲染的图表，也应该设置已有图表的交互功能
        // 为确保DOM已更新，使用短暂延迟
        setTimeout(() => {
          setupMermaidRefresh();
        }, 300);
      }, 500);
    } else {
      console.log("正在流式传输中，跳过UML渲染");
    }
  });
}

// 流式消息处理相关函数
async function setupStreamListeners() {
  // 监听流式消息事件
  const unlistenStream = await listen('stream-message', (event) => {
    // 标记正在接收流式消息
    isReceivingStream.value = true;
    console.log("流式消息接收中，暂停UML渲染");

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
    console.log("流式消息接收完成，开始渲染UML图表");

    // 重新加载聊天历史
    await loadChatHistory();

    // 保持禁用消息过渡动画
    messageTransition.value = false;

    // 先更新UI显示
    updateChatContent(chatContent.value);

    // 标记流式消息接收完成
    isReceivingStream.value = false;
    isStreaming.value = false;
    isLoading.value = false;

    // 流式传输完成后，启动图表渲染
    setTimeout(() => {
      console.log("开始执行延迟的UML渲染");
      renderMermaidDiagrams();

      // 添加这行以确保渲染完毕后设置交互功能
      setTimeout(() => {
        setupMermaidRefresh();
      }, 400);
    }, 800); // 给更长的延迟以确保DOM完全更新
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

            const canvas = await html2canvas(messageContentElement, {
              useCORS: true, // 允许加载跨域图片（如果需要）
              scale: 2, // 提高分辨率
              backgroundColor: backgroundColor, // 设置背景色
              logging: true, // 开启日志以便调试
              onclone: (clonedDoc) => {
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
                // 确保克隆的文档中 MathJax 公式已渲染
                // MathJax 的渲染可能比较复杂，html2canvas 可能无法完美捕获动态生成的 SVG
                // 这里可以尝试强制重新渲染，但这可能不可靠
                // if (window.MathJax && window.MathJax.typesetPromise) {
                //   window.MathJax.typesetPromise([clonedDoc.body]);
                // }
              }
            });

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

  // 禁用消息过渡动画
  messageTransition.value = false;
  fadeInMessages.value = true; // 确保内容可见

  // 保存消息内容并立即清空输入框，提升用户体验
  const message = inputMessage.value;
  inputMessage.value = "";

  // 重置文本区域高度
  resetTextareaHeight();

  // 检查当前是否有选择的对话
  if (!chatContent.value || chatContent.value.length === 0) {
    console.log("未选择对话，正在创建新对话...");

    // 显示加载状态
    isLoading.value = true;

    try {
      // 创建新对话
      chatContent.value = await invoke("create_new_chat");
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
  isReceivingStream.value = true;
  isStreaming.value = true;
  isLoading.value = true;

  console.log("开始流式传输消息，已禁用UML渲染");

  // 使用 Promise 包装后端调用，但不等待它完成
  invoke("process_message_stream", { message })
    .catch(error => {
      console.error("消息发送失败:", error);
      showNotification("消息发送失败", "error");
      isStreaming.value = false;
      isLoading.value = false;
      isReceivingStream.value = false;
    });

  // 由于已经设置了状态并启动了异步处理，函数可以立即返回
  // 实际的响应处理将由事件监听器完成
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

// 处理输入框按键事件
function handleInputKeydown(event: KeyboardEvent) {
  if (event.key === 'Enter' && event.ctrlKey) {
    event.preventDefault(); // 阻止默认的 Enter 行为（如果 textarea 在 form 内）
    sendStreamMessage();
  }
  // 允许 Shift+Enter 换行，textarea 默认支持
}

// 创建新对话
async function createNewChat() {
  // 如果正在流式输出消息，禁止创建新聊天
  if (isStreaming.value) {
    showNotification("请等待当前消息输出完成", "error");
    return;
  }

  // 直接显示内容，不使用淡出淡入效果
  fadeInMessages.value = true;

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
      // 确保内容可见
      fadeInMessages.value = true;
    }
  }, 100); // 减少延迟时间
}

// 监听 chatContent 变化，确保 MathJax 重新渲染
watch(chatContent, () => {
  nextTick(() => {
    console.log("聊天内容变化:", chatContent.value);
    refreshGlobalStyles();
    renderMathInElement();

    // 只在非流传输状态下渲染UML图表
    if (!isReceivingStream.value) {
      renderMermaidDiagrams();
    } else {
      console.log("正在流式传输中，跳过UML渲染");
    }
  });
});

// 监听主题变化，更新聊天内容和Mermaid配置
watch(() => document.documentElement.getAttribute('data-theme'), (newTheme, oldTheme) => {
  // 增加判断，仅在主题实际变化时执行
  if (newTheme !== oldTheme) {
    console.log("主题变化:", newTheme);

    // 当主题变化时，更新Mermaid配置
    const isDark = newTheme === 'dark' ||
      (newTheme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);

    mermaid.initialize({
      theme: isDark ? 'dark' : 'default'
    });

    // 当主题变化时，重新应用样式
    // 延迟执行，确保全局样式已应用
    setTimeout(() => {
      if (chatContent.value) {
        updateChatContent(chatContent.value);
      }
    }, 50); // 短暂延迟
  }
});


// 组件加载时初始化对话内容
onMounted(async () => {
  // 检测是否为移动设备
  isMobile.value = /Mobi|Android|iPhone|iPad|iPod/i.test(navigator.userAgent);

  try {
    // 初始化应用设置 (这会调用 refreshGlobalStyles)
    await initAppSettings();

    // 初始化Mermaid (主题应基于 initAppSettings 后的全局设置)
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
    // 确保初始加载时使用正确的全局主题，但不强制渲染UML
    isReceivingStream.value = false; // 初始加载时默认没有流传输
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
    }, 100); // 保持延迟
  });

  window.addEventListener('fontSizeChanged', (e: Event) => {
    const customEvent = e as CustomEvent;
    console.log('字体大小已变更:', customEvent.detail);
    // 添加延迟以确保字体大小变更完全应用
    setTimeout(() => {
      if (chatContent.value) {
        updateChatContent(chatContent.value);
      }
    }, 100); // 保持延迟
  });
});

// 组件卸载时清理事件监听
onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  // 清除主题和字体大小变化的事件监听
  window.removeEventListener('themeChanged', (_: Event) => { });
  window.removeEventListener('fontSizeChanged', (_: Event) => { });
  // 移除菜单关闭监听器
  removeDocumentClickListener();
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

// 添加鼠标和触摸事件监听
onMounted(() => {
  // 添加全局拖动和结束拖动事件
  window.addEventListener('mousemove', handleDrag);
  window.addEventListener('mouseup', endDrag);
  window.addEventListener('touchmove', handleDrag);
  window.addEventListener('touchend', endDrag);
});

onUnmounted(() => {
  // 移除全局拖动和结束拖动事件
  window.removeEventListener('mousemove', handleDrag);
  window.removeEventListener('mouseup', endDrag);
  window.removeEventListener('touchmove', handleDrag);
  window.removeEventListener('touchend', endDrag);
});

// 修改 setupMermaidRefresh 函数，添加点击事件以打开图表查看器
function setupMermaidRefresh() {
  nextTick(() => {
    // 为所有图表容器添加刷新按钮
    document.querySelectorAll('.chat-messages .mermaid-container').forEach(container => {
      // 检查容器是否已经有刷新按钮
      if (!container.querySelector('.refresh-diagram-button')) {
        const refreshButton = document.createElement('button');
        refreshButton.className = 'refresh-diagram-button';
        refreshButton.innerHTML = `
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M23 4v6h-6"></path>
            <path d="M1 20v-6h6"></path>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10"></path>
            <path d="M20.49 15a9 9 0 0 1-14.85 3.36L1 14"></path>
          </svg>
        `;
        refreshButton.title = "刷新图表";

        refreshButton.addEventListener('click', async (e) => {
          e.preventDefault();
          e.stopPropagation();
          const targetButton = e.currentTarget as HTMLElement;
          const container = targetButton.closest('.mermaid-container');

          if (container) {
            // 移除loaded类以便重新渲染
            container.classList.remove('loaded');
            // 清除上次渲染的内容记录，强制重新渲染
            container.removeAttribute('data-last-rendered');
            targetButton.classList.add('refreshing');
            showNotification("正在刷新图表...", "info");

            // 延迟后渲染以确保UI更新
            setTimeout(async () => {
              await renderMermaidDiagrams(0, 3);
              targetButton.classList.remove('refreshing');
            }, 100);
          }
        });

        // 将按钮添加到容器中
        container.appendChild(refreshButton);
      }

      // 添加放大按钮
      if (!container.querySelector('.zoom-diagram-button')) {
        const zoomButton = document.createElement('button');
        zoomButton.className = 'zoom-diagram-button';
        zoomButton.innerHTML = `
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            <line x1="11" y1="8" x2="11" y2="14"></line>
            <line x1="8" y1="11" x2="14" y2="11"></line>
          </svg>
        `;
        zoomButton.title = "放大查看";

        zoomButton.addEventListener('click', (e) => {
          e.preventDefault();
          e.stopPropagation();

          const container = (e.currentTarget as HTMLElement).closest('.mermaid-container') as HTMLElement;
          if (container) {
            const svgElement = container.querySelector('svg');
            const contentElement = container.getAttribute('data-diagram-content');

            if (svgElement && contentElement) {
              const svgContent = svgElement.outerHTML;
              const diagramContent = decodeURIComponent(contentElement);
              openChartViewer(svgContent, diagramContent);
            }
          }
        });

        container.appendChild(zoomButton);
      }

      // 为整个容器添加点击事件以打开查看器
      if (!container.hasAttribute('data-has-click-listener')) {
        container.setAttribute('data-has-click-listener', 'true');

        container.addEventListener('click', (e) => {
          // 点击按钮时不触发
          if ((e.target as HTMLElement).closest('.refresh-diagram-button, .zoom-diagram-button')) {
            return;
          }

          const svgElement = container.querySelector('svg');
          const contentElement = container.getAttribute('data-diagram-content');

          if (svgElement && contentElement) {
            const svgContent = svgElement.outerHTML;
            const diagramContent = decodeURIComponent(contentElement);
            openChartViewer(svgContent, diagramContent);
          }
        });

        // 添加视觉提示，表明容器可点击
        container.classList.add('clickable-container');
      }
    });
  });
}

// 在 data 部分添加变量来存储事件监听器引用
const documentClickListener = ref<((e: MouseEvent) => void) | null>(null);

// 修改 openMessageContextMenu 函数，添加事件冒泡控制和更严格的条件检查
function openMessageContextMenu(event: MouseEvent, messageIndex: number) {
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
    const message = chatContent.value[messageContextMenuIndex.value];
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

      // 更新本地聊天内容
      chatContent.value = updatedContent as ChatMessage[];
      showNotification("消息已删除", "success");

      // 刷新聊天界面
      updateChatContent(chatContent.value);
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
      isStreaming.value = true;
      messageTransition.value = false;

      // 调用后端重新生成消息
      await invoke("regenerate_message", { messageIndex: messageContextMenuIndex.value });

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

// 判断是否可以重新生成消息
const canRegenerateMessage = computed(() => {
  if (messageContextMenuIndex.value !== null && messageContextMenuIndex.value >= 0) {
    const message = chatContent.value[messageContextMenuIndex.value];
    return message && message.msgtype === 'Assistant';
  }
  return false;
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
    await loadChatHistory();

    // 如果当前显示的就是被删除的对话，则清空显示内容
    // 检查当前活跃的对话ID是否与被删除的ID相同
    const currentId = chatHistory.value.find(item => item.id === currentChatId.value)?.id;
    if (currentId === chatToDeleteId.value) {
      chatContent.value = [];
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

</script>

<template>
  <div class="app-container">
    <!-- 自定义标题栏 - 仅在非移动设备上显示 -->
    <div v-if="!isMobile" class="custom-titlebar" data-tauri-drag-region>
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
        </div>
        <!-- 底部输入区 -->
        <div class="chat-input-area">
          <form @submit.prevent="sendStreamMessage" class="input-form">
            <textarea v-model="inputMessage" placeholder="输入消息... (Ctrl+Enter 发送)" class="message-input animated-input"
              rows="1" @keydown="handleInputKeydown" @input="autoResizeTextarea"></textarea>
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
          <h3>Mermaid 图表查看器</h3>
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
              <summary>查看 Mermaid 代码</summary>
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
  /* 检测是否有标题栏， 有就设置top为32px*/
  --titlebar-height: v-bind("isMobile ? '0px' : '32px'");
  top: var(--titlebar-height);
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
  --titlebar-height: v-bind("isMobile ? '0px' : '32px'");
  /* Define CSS variable */
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
    height: calc(100vh - 32px);
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
  color: var (--text-secondary);
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
  box-shadow: var(--shadow);
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
  /* 内边距决定了容器和内容的间距 */
  background-color: var(--card-bg);
  z-index: 10;
  position: sticky;
  bottom: 0;
}

.input-form {
  display: flex;
  align-items: flex-end;
  /* 底部对齐 textarea 和 button */
  gap: 8px;
  /* 设置 textarea 和 button 之间的间距 */
  width: 100%;
  /* 确保表单宽度正确 */
  max-width: 900px;
  margin: 0 auto;
}

.message-input {
  flex: 1;
  padding: 12px 16px;
  /* 恢复标准 padding */
  border: 1px solid var(--border-color);
  border-radius: var(--radius);
  font-size: var(--font-size-base);
  outline: none;
  transition: var(--transition);
  font-family: inherit;
  box-shadow: var(--shadow-sm);
  background-color: var(--card-bg);
  color: var (--text-color);
  resize: none;
  /* 禁止用户手动调整大小 */
  overflow-y: auto;
  /* 内容超出时显示滚动条 */
  line-height: 1.5;
  /* 确保行高一致 */
  min-height: 48px;
  /* 保证至少有输入框的高度 */
  max-height: 150px;
  /* 限制最大高度，例如约5行 */
  height: 48px;
  /* 初始高度设为 min-height */
}

.message-input:focus {
  border-color: var(--primary-color);
  box-shadow: 0 0 0 2px rgba(79, 70, 229, 0.1);
}

.send-button {
  width: 40px;
  height: 40px;
  transform: translateY(-4px);
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
  flex-shrink: 0;
  /* 防止按钮被压缩 */
}

.send-button:hover:not(:disabled) {
  /* 仅在非禁用状态下应用 hover 效果 */
  background-color: var(--primary-hover);
  transform: scale(1.08);
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

chat-messages .mjx-chtml {
  margin: 0.5em 0;
  font-size: var(--font-size-lg);
}

chat-messages .mjx-math {
  max-width: 100%;
  /* 确保不超过容器宽度 */
  overflow-x: auto;
  /* 水平溢出时显示滚动条 */
  overflow-y: hidden;
  /* 隐藏垂直滚动条 */
  display: block;
  /* 确保 max-width 生效 */
  padding-bottom: 4px;
  /* 为滚动条留出一点空间 */
}

chat-messages .mjx-chtml.MJXc-display {
  margin: 1em 0;
  padding: 0.5em 0;
  overflow-x: auto;
  /* 水平溢出时显示滚动条 */
  overflow-y: hidden;
  /* 隐藏垂直滚动条 */
  text-align: center;
  display: block;
  /* 确保 overflow 生效 */
  max-width: 100%;
  /* 确保不超过容器宽度 */
  padding-bottom: 4px;
  /* 为滚动条留出一点空间 */
}

chat-messages .MJX-TEX {
  text-align: center;
}

chat-messages .mjx-container {
  padding: 6px 0;
  max-width: 100%;
  /* 确保容器不超过父元素 */
  overflow-x: auto;
  /* 为容器本身也添加滚动条 */
  overflow-y: hidden;
  display: block;
  /* 确保 overflow 生效 */
  padding-bottom: 4px;
  /* 为滚动条留出一点空间 */
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
  transform: scale(1.08);
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
  transform: scale(1);
  transform: translateY(-4px);
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

/* 添加占位符样式 */
.placeholder-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--text-secondary);
  padding: 20px;
  text-align: center;
  font-size: 14px;
}

.placeholder-box svg {
  opacity: 0.7;
}

.render-image-button {
  color: #8b5cf6;
  /* 紫色图标 */
}

.render-image-button:hover {
  background-color: rgba(139, 92, 246, 0.1);
}


/* 自定义滚动条样式 - 改进版本 */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
  border-radius: 6px;
}

::-webkit-scrollbar-thumb {
  background-color: var(--scrollbar-thumb, rgba(100, 116, 139, 0.5));
  border-radius: 6px;
  transition: background-color 0.3s ease;
}

::-webkit-scrollbar-thumb:hover {
  background-color: var(--scrollbar-thumb-hover, rgba(100, 116, 139, 0.7));
}

/* 根据主题设置滚动条变量 */
:root {
  --scrollbar-thumb: rgba(100, 116, 139, 0.4);
  --scrollbar-thumb-hover: rgba(100, 116, 139, 0.7);
}

:root[data-theme="dark"],
:root[data-theme="system"] {
  --scrollbar-thumb: rgba(148, 163, 184, 0.4);
  --scrollbar-thumb-hover: rgba(148, 163, 184, 0.7);
}

/* 特定区域滚动条定制 */
.chat-content::-webkit-scrollbar {
  width: 6px;
}

.chat-content::-webkit-scrollbar-thumb {
  background-color: var(--scrollbar-thumb);
  border-radius: 6px;
}

.chat-content::-webkit-scrollbar-thumb:hover {
  background-color: var(--scrollbar-thumb-hover);
}

.history-list::-webkit-scrollbar {
  width: 5px;
}

.history-list::-webkit-scrollbar-thumb {
  background-color: var(--scrollbar-thumb);
  border-radius: 4px;
}

.history-list::-webkit-scrollbar-thumb:hover {
  background-color: var(--scrollbar-thumb-hover);
}

/* 支持Firefox的滚动条样式 */
* {
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb) transparent;
}

.chat-content,
.history-list {
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb) transparent;
}


.mermaid-preview {
  margin-top: 12px;
  border-top: 1px solid var(--border-color);
  padding-top: 12px;
  text-align: left;
}

.preview-header {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  margin-bottom: 8px;
  font-weight: 500;
  text-align: left;
}

.preview-code {
  background-color: rgba(0, 0, 0, 0.03);
  border-radius: 4px;
  padding: 8px;
  overflow-x: auto;
  font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  font-size: var(--font-size-sm);
  line-height: 1.4;
  max-height: 200px;
  overflow-y: auto;
  white-space: pre;
  color: var(--text-color);
  border: 1px solid var(--border-color);
  text-align: left;
}

/* 暗色主题支持 */
:root[data-theme="dark"] .preview-code,
:root[data-theme="system"] .preview-code {
  background-color: rgba(255, 255, 255, 0.05);
}

/* Mermaid 容器可点击样式 */
.mermaid-container.clickable-container {
  cursor: zoom-in;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.mermaid-container.clickable-container:hover {
  transform: scale(1.01);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

/* 图表容器操作按钮 */
.mermaid-container {
  position: relative;
}

.refresh-diagram-button,
.zoom-diagram-button {
  position: absolute;
  background-color: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: 50%;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  opacity: 0.7;
  transition: opacity 0.2s ease, transform 0.2s ease;
  z-index: 5;
  box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
}

.refresh-diagram-button {
  top: 8px;
  right: 8px;
}

.zoom-diagram-button {
  top: 8px;
  right: 44px;
  /* 位于刷新按钮旁边 */
}

.refresh-diagram-button:hover,
.zoom-diagram-button:hover {
  opacity: 1;
  transform: scale(1.1);
}

.refresh-diagram-button svg,
.zoom-diagram-button svg {
  color: var(--text-color);
}

/* 图表查看器样式 */
.chart-viewer-modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1100;
  display: flex;
  align-items: center;
  justify-content: center;
}

.chart-viewer-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(3px);
}

.chart-viewer-content {
  position: relative;
  width: 90%;
  height: 90%;
  max-width: 1200px;
  max-height: 90vh;
  background-color: var(--card-bg);
  border-radius: var(--radius-lg);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: modal-in 0.3s ease forwards;
  z-index: 1101;
}

.chart-viewer-header {
  padding: 16px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.chart-viewer-header h3 {
  margin: 0;
  font-size: var(--font-size-lg);
  color: var(--text-color);
}

.chart-viewer-controls {
  display: flex;
  gap: 8px;
}

.chart-control-button {
  background: none;
  border: none;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border-radius: var(--radius-sm);
  color: var(--text-color);
  transition: all 0.2s ease;
}

.chart-control-button:hover {
  background-color: rgba(0, 0, 0, 0.05);
  transform: scale(1.1);
}

.chart-viewer-body {
  flex: 1;
  padding: 24px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  background-color: var(--bg-color);
}

.chart-viewer-diagram {
  transition: transform 0.05s linear;
  transform-origin: center center;
  max-width: 100%;
  max-height: 100%;
  touch-action: none;
  will-change: transform;
}

.chart-viewer-diagram svg {
  max-width: 100%;
  max-height: 100%;
  display: block;
}

.chart-viewer-footer {
  padding: 12px 16px;
  border-top: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.chart-viewer-info {
  display: flex;
  justify-content: space-between;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}

.chart-viewer-code-toggle {
  width: 100%;
}

.chart-viewer-code-toggle summary {
  cursor: pointer;
  color: var(--primary-color);
  font-size: var(--font-size-sm);
  font-weight: 500;
  padding: 4px 0;
  transition: color 0.2s ease;
}

.chart-viewer-code-toggle summary:hover {
  color: var(--primary-hover);
}

.chart-viewer-code {
  margin-top: 8px;
  padding: 12px;
  background-color: rgba(0, 0, 0, 0.03);
  border-radius: var(--radius-sm);
  overflow-x: auto;
  font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  font-size: 12px;
  line-height: 1.4;
  max-height: 150px;
  overflow-y: auto;
  white-space: pre;
  color: var(--text-color);
  border: 1px solid var(--border-color);
}

/* 移动端适配 */
@media (max-width: 768px) {
  .chart-viewer-content {
    width: 95%;
    height: 95%;
    max-width: none;
  }

  .chart-viewer-header {
    padding: 12px;
  }

  .chart-viewer-body {
    padding: 16px;
  }

  .refresh-diagram-button,
  .zoom-diagram-button {
    width: 24px;
    height: 24px;
  }

  .zoom-diagram-button {
    right: 40px;
  }

  .refresh-diagram-button svg,
  .zoom-diagram-button svg {
    width: 14px;
    height: 14px;
  }
}

/* 暗色主题支持 */
:root[data-theme="dark"] .chart-viewer-code,
:root[data-theme="system"] .chart-viewer-code {
  background-color: rgba(255, 255, 255, 0.05);
}

/* 代码块复制按钮样式 */
.code-block-with-copy {
  position: relative;
  padding-right: 30px;
  /* 为按钮留出空间 */
}

.code-copy-button {
  position: absolute;
  top: 8px;
  right: 8px;
  background-color: var(--bg-color);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  opacity: 0.7;
  transition: opacity 0.2s ease, transform 0.2s ease, background-color 0.2s ease;
  z-index: 5;
  color: var(--text-secondary);
}

.code-copy-button:hover {
  opacity: 1;
  transform: scale(1.1);
  background-color: var(--card-bg);
}

.code-copy-button.success {
  background-color: #10b981;
  color: white;
  border-color: #10b981;
  opacity: 1;
}

/* 暗色主题支持 */
:root[data-theme="dark"] .code-copy-button,
:root[data-theme="system"] .code-copy-button {
  background-color: rgba(255, 255, 255, 0.1);
  color: var(--dark-text-secondary);
}

:root[data-theme="dark"] .code-copy-button:hover,
:root[data-theme="system"] .code-copy-button:hover {
  background-color: rgba(255, 255, 255, 0.2);
  color: var(--dark-text-color);
}

/* 对话右键菜单样式 */
.context-menu {
  position: fixed; /* 固定定位，相对于视口 */
  background-color: var(--card-bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15); /* 增强阴影效果 */
  z-index: 1000;
  padding: 8px 0;
  width: 160px;
  animation: fade-in-zoom 0.2s ease; /* 添加缩放动画 */
  max-height: 300px;
  overflow-y: auto;
  pointer-events: auto; /* 确保菜单可点击 */
}

/* 添加菜单显示动画 */
@keyframes fade-in-zoom {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

.context-menu-item {
  padding: 8px 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-color);
  transition: background-color 0.2s ease;
}

.context-menu-item:hover {
  background-color: rgba(0, 0, 0, 0.05);
}

/* 深色模式下菜单hover效果 */
:root[data-theme="dark"] .context-menu-item:hover,
:root[data-theme="system"] .context-menu-item:hover {
  background-color: rgba(255, 255, 255, 0.1);
}

/* 对话重命名和删除确认对话框样式 */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background-color: var(--card-bg);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow);
  width: 90%;
  max-width: 400px;
  animation: modal-in 0.3s ease forwards;
}

.modal-header {
  padding: 16px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.modal-header h3 {
  margin: 0;
  font-size: var(--font-size-lg);
  color: var(--text-color);
}

.modal-close {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  transition: background-color 0.2s ease;
}

.modal-close:hover {
  background-color: rgba(0, 0, 0, 0.05);
}

.modal-body {
  padding: 16px;
}

.modal-input {
  width: 100%;
  padding: 12px 16px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius);
  font-size: var(--font-size-base);
  outline: none;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
  font-family: inherit;
  background-color: var(--card-bg);
  color: var(--text-color);
}

.modal-input:focus {
  border-color: var(--primary-color);
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
}

.modal-footer {
  padding: 16px;
  border-top: 1px solid var(--border-color);
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.modal-button {
  padding: 8px 16px;
  border: none;
  border-radius: var(--radius);
  cursor: pointer;
  font-size: var(--font-size-base);
  font-weight: 500;
  transition: background-color 0.2s ease, transform 0.2s ease;
}

.modal-button.cancel {
  background-color: rgba(0, 0, 0, 0.05);
  color: var(--text-color);
}

.modal-button.cancel:hover {
  background-color: rgba(0, 0, 0, 0.1);
}

.modal-button.confirm {
  background-color: var(--primary-color);
  color: white;
}

.modal-button.confirm:hover {
  background-color: var(--primary-hover);
}

.modal-button.delete {
  background-color: #ef4444;
  color: white;
}

.modal-button.delete:hover {
  background-color: #dc2626;
}
</style>
// hljs
import hljs from 'highlight.js/lib/core';
import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { handleMermaidRender } from './mermaidRenderer';
import { isStreaming, AppEvents } from '../eventBus';
import { handleTypstRender } from './typstRenderer';
import { handleHTMLRender } from './htmlRenderer';
import { handleInteractiveButton } from './interactiveButton';
import { handleKaTeXRender } from './katexRenderer';
import { handleWolframRender } from './wolframRenderer';
import { handlePintoraRender } from './pintoraRenderer';
import { nextTick } from 'vue';
/**
 * HTML转义函数，防止XSS攻击
 * @param str 需要转义的字符串
 * @returns 转义后的安全字符串
 */
function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

// 主函数：应用代码高亮和处理各种特殊代码块
async function applyHighlight(container: HTMLElement): Promise<HTMLElement> {
  const timestamp = Date.now();
  const resultContainer = container.cloneNode(true) as HTMLElement;

  // 查找所有代码块，排除thinking-details中的代码块
  const codeElements = resultContainer.querySelectorAll('.chat-messages pre:not(.thinking-details pre):not(:has(.thinking-details)) code');
  if (!codeElements || codeElements.length === 0) return resultContainer;

  console.log(`处理 ${codeElements.length} 个代码块`);

  // 创建一批次操作对象，用于收集所有修改
  const batch = {
    highlightElements: [] as HTMLElement[],
    toolCodeReplacements: [] as { original: Element, replacement: HTMLElement, content: string }[],
    actionElements: [] as { element: Element, content: string }[]
  };

  // 收集所有需要高亮的代码块
  await collectCodeHighlight(codeElements, batch);

  // 收集所有需要处理的工具代码块
  await collectToolCodeBlocks(codeElements, batch);

  // 收集所有需要添加复制按钮的代码块
  await collectCodeBlockActions(codeElements, batch);

  // 应用所有批次操作到HTML (一次性更新以减少抖动)
  await applyBatchOperations(batch);

  // 标记处理完成
  resultContainer.setAttribute('data-highlight-applied', 'true');

  console.log(`高亮处理完成，耗时: ${Date.now() - timestamp}ms`);
  return resultContainer;
}

// 收集需要高亮的代码元素
async function collectCodeHighlight(codeElements: NodeListOf<Element>, batch: any): Promise<void> {
  for (const el of codeElements) {
    const preElement = el.parentElement as HTMLPreElement | null;
    if (!preElement) continue;

    // 收集需要高亮的元素
    batch.highlightElements.push(el as HTMLElement);
  }
}

// 收集需要处理的工具代码块
async function collectToolCodeBlocks(codeElements: NodeListOf<Element>, batch: any): Promise<void> {
  for (const el of codeElements) {
    // 跳过非tool_code代码块
    if (!el.classList.contains('language-tool_code')) continue;

    const codeContent = el.textContent?.trim() || '';
    console.warn("tool_code 内容:", codeContent);
    if (!codeContent) continue;

    const preElement = el.parentElement;
    if (!preElement) continue;

    console.log("检测到 tool_code 代码块:", codeContent);

    // Base64编码代码内容，防止特殊字符导致存储异常
    const encodedContent = btoa(encodeURIComponent(codeContent));

    // 创建工具代码容器
    const toolCodeContainer = document.createElement('div');
    toolCodeContainer.className = 'tool-code-container';

    // 设置初始加载状态，使用Base64编码存储原始内容
    toolCodeContainer.innerHTML = `
          <div class="tool-code-loading">正在解析工具代码...</div>
          <pre class="tool-code-original" style="display: none;" data-encoded="true"><code>${encodedContent}</code></pre>
        `;

    // 收集替换信息，但不立即执行
    batch.toolCodeReplacements.push({
      original: preElement,
      replacement: toolCodeContainer,
      content: encodedContent
    });
  }
}

// 收集需要添加复制按钮的代码块
async function collectCodeBlockActions(codeElements: NodeListOf<Element>, batch: any): Promise<void> {
  for (const el of codeElements) {
    const preElement = el.parentElement;
    // 跳过已经处理过的或特殊类型的代码块
    if (!preElement ||
      preElement.querySelector('.code-copy-button') ||
      el.classList.contains('language-tool_code')) continue;

    // 获取代码内容
    const codeContent = el.textContent || '';

    // 收集需要添加按钮的元素
    batch.actionElements.push({ element: preElement, content: codeContent });
  }
}

// 一次性应用所有批处理操作
async function applyBatchOperations(batch: any): Promise<void> {
  // 1. 首先执行代码高亮 (这不会改变DOM结构)
  for (const el of batch.highlightElements) {
    hljs.highlightElement(el);
  }

  // 2. 执行所有工具代码块替换
  for (const item of batch.toolCodeReplacements) {
    item.original.parentNode?.replaceChild(item.replacement, item.original);
  }

  // 3. 为常规代码块添加复制按钮
  for (const item of batch.actionElements) {
    await addCopyButtonToCodeBlock(item.element, item.content);
  }

  // 4. 现在处理所有工具代码块内容
  await Promise.all(batch.toolCodeReplacements.map(async (item: any) => {
    try {
      await processToolCode(item.replacement, item.content);
    } catch (error) {
      const codeContent = decodeURIComponent(atob(item.content));
      await handleToolCodeError(item.replacement, error, escapeHtml(codeContent));
    }
  }));
}


// 处理工具代码的逻辑
async function processToolCode(toolCodeContainer: HTMLDivElement, encodedContent: string): Promise<void> {
  // 解码Base64编码的内容
  const codeContent = decodeURIComponent(atob(encodedContent));
  // 将原始代码包裹在可折叠的 details 元素中
  const originalCodeHtml = `
      <div class="mini-details-container">
        <details class="mini-tech-details">
          <summary-all aria-label="查看原始代码"></summary-all>
          <p>原始代码:</p>
          <pre class="tool-code-original"><code>${escapeHtml(codeContent)}</code></pre>
        </details>
      </div>
    `;

  if (isStreaming.value) {
    // 直接显示流式传输提示和折叠的原始代码
    toolCodeContainer.innerHTML = `
        <div class="tool-code-header">工具代码解析结果:</div>
        <div class="tool-code-result">
          <div class="tool-api-info">
            <div class="tool-api-row"><span class="tool-api-label">API:</span> <span class="tool-api-value">default_api</span></div>
            <div class="tool-api-row"><span class="tool-api-label">函数:</span> <span class="tool-api-value">流式传输中...</span></div>
          </div>
        </div>
        <div class="api-call-notice">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
          <span>API功能将在消息完整接收后处理</span>
        </div>
        ${originalCodeHtml}
        `; // 使用包含 details 的 originalCodeHtml

    // 高亮显示原始代码 (在 details 内部)
    await highlightToolCodeElements(toolCodeContainer);
    return;
  }

  try { // 添加 try-catch 以处理解析和调用错误
    // 调用后端解析代码
    const astResult = await invoke<string>("parse_code", { code: codeContent });
    console.log("AST 解析结果:", astResult);

    // 解析AST JSON
    const astJson = JSON.parse(astResult);

    let finalHtml = '';

    // 检查根节点是否为 "Expressions" 类型
    if (astJson.node_type === "Expressions" && Array.isArray(astJson.children)) {
      // 处理多个表达式
      let combinedResultsHtml = '';
      for (const childAst of astJson.children) {
        // 对每个子 AST 进行处理
        const apiInfo = await parseApiCall(childAst);
        if (apiInfo) {
          // 成功解析为 API 调用，传入子 AST 和原始代码内容
          // processApiCallResult 现在不包含原始代码块
          combinedResultsHtml += await processApiCallResult(apiInfo, childAst, codeContent);
        } else {
          // 子 AST 解析失败，显示该子 AST 的备用视图
          // createToolCodeFallbackView 现在不包含原始代码块
          combinedResultsHtml += await createToolCodeFallbackView(JSON.stringify(childAst, null, 2));
        }
        // 在每个处理结果之间添加分隔符
        combinedResultsHtml += '<hr class="expression-separator">';
      }
      // 移除最后一个分隔符
      if (combinedResultsHtml.endsWith('<hr class="expression-separator">')) {
        combinedResultsHtml = combinedResultsHtml.slice(0, -'<hr class="expression-separator">'.length);
      }

      // 组合结果和唯一的、可折叠的原始代码块
      finalHtml = combinedResultsHtml + originalCodeHtml;

    } else {
      // 处理单个表达式或非 "Expressions" 根节点 (原始逻辑)
      const apiInfo = await parseApiCall(astJson);
      let resultHtml = '';
      if (apiInfo) {
        // processApiCallResult 现在不包含原始代码块
        resultHtml = await processApiCallResult(apiInfo, astJson, codeContent);
      } else {
        // 解析失败时显示原始AST结果
        // createToolCodeFallbackView 现在不包含原始代码块
        resultHtml = await createToolCodeFallbackView(astResult);
      }
      // 组合结果和可折叠的原始代码块
      finalHtml = resultHtml + originalCodeHtml;
    }

    // 添加Wolfram相关查询的全局事件监听器
    if (!document.querySelector('#wolfram-query-handler')) {
      registerWolframQueryEventHandler();
    }

    toolCodeContainer.innerHTML = finalHtml;
    // 对容器内所有新添加的代码块进行高亮 (包括 details 内部的)
    await highlightToolCodeElements(toolCodeContainer);

  } catch (error) {
    // 处理 invoke 或 JSON.parse 可能出现的错误
    await handleToolCodeError(toolCodeContainer, error, codeContent); // 传递原始 codeContent
  }
}

// 注册Wolfram相关查询的事件处理器
function registerWolframQueryEventHandler() {
  // 创建一个标记元素，表示已经注册了事件处理器
  const marker = document.createElement('div');
  marker.id = 'wolfram-query-handler';
  marker.style.display = 'none';
  document.body.appendChild(marker);

  // 监听自定义事件
  window.addEventListener('send-wolfram-query', async (e: any) => {
    if (e.detail && e.detail.query) {
      // 如果正在流式输出消息，禁止发送新消息
      if (isStreaming.value) {
        AppEvents.showNotification("请等待当前消息输出完成", "error");
        return;
      }

      const query = e.detail.query;
      if (query.trim()) {
        // 发送消息，在查询前添加"wolfram:"前缀以便可以识别这是Wolfram查询
        await AppEvents.sendStreamMessageDirect("> wolfram: " + query);
        AppEvents.showNotification("已发送Wolfram Alpha查询", "success");
      }
    }
  });

  console.log("Wolfram相关查询事件处理器已注册");
}

// 将 JSON 对象转换为交互式树形 HTML
function convertJsonToTreeView(json: any, isRoot: boolean = true): string {
  if (json === null) return '<span class="json-null">null</span>';
  if (json === undefined) return '<span class="json-undefined">undefined</span>';

  // 处理基本类型
  if (typeof json !== 'object') {
    if (typeof json === 'string') {
      return `<span class="json-string">"${escapeHtml(json)}"</span>`;
    }
    if (typeof json === 'number') {
      return `<span class="json-number">${json}</span>`;
    }
    if (typeof json === 'boolean') {
      return `<span class="json-boolean">${json}</span>`;
    }
    return escapeHtml(String(json));
  }

  // 处理数组
  if (Array.isArray(json)) {
    if (json.length === 0) return '<span class="json-array">[]</span>';

    const items = json.map((item, index) => {
      return `
          <li class="json-array-item">
            <span class="json-array-index">[${index}]</span>
            <div class="json-array-value">${convertJsonToTreeView(item, false)}</div>
          </li>
        `;
    }).join('');

    return `
        <details class="json-details" ${isRoot ? 'open' : ''}>
          <summary class="json-array-summary">Array[${json.length}]</summary>
          <ul class="json-array-list">${items}</ul>
        </details>
      `;
  }

  // 处理对象
  const keys = Object.keys(json);
  if (keys.length === 0) return '<span class="json-object">{}</span>';

  const items = keys.map(key => {
    // 特殊处理一些常见的AST属性，使树形视图更易读
    let keyDisplay = key;
    let collapsedByDefault = false;

    // 对一些较大的属性默认折叠
    if (['children', 'body', 'expressions', 'statements'].includes(key)) {
      collapsedByDefault = true;
    }

    // 对于节点类型，使用不同颜色
    if (key === 'node_type') {
      return `
          <li class="json-property">
            <span class="json-property-key">${keyDisplay}:</span>
            <span class="json-property-value json-node-type">${escapeHtml(String(json[key]))}</span>
          </li>
        `;
    }

    return `
        <li class="json-property">
          <span class="json-property-key">${keyDisplay}:</span>
          <div class="json-property-value">${convertJsonToTreeView(json[key], false && !collapsedByDefault)}</div>
        </li>
      `;
  }).join('');

  return `
      <details class="json-details" ${isRoot ? 'open' : ''}>
        <summary class="json-object-summary">Object{${keys.length}}</summary>
        <ul class="json-object-list">${items}</ul>
      </details>
    `;
}

// 创建工具代码的备用视图（解析失败时） - 移除原始代码部分
async function createToolCodeFallbackView(astResult: string): Promise<string> {
  try {
    const parsedAst = JSON.parse(astResult);

    return `
      <div class="tool-code-header">工具代码 AST:</div>
      <div class="ast-tree-view">${convertJsonToTreeView(parsedAst)}</div>
      <div class="mini-details-container">
        <details class="mini-tech-details">
          <summary aria-label="查看原始JSON"><span class="detail-chevron">▲</span></summary>
          <pre class="tool-code-ast"><code>${escapeHtml(JSON.stringify(parsedAst, null, 2))}</code></pre>
        </details>
      </div>
      `;
  } catch (e) {
    // 解析失败时回退到原始文本显示
    return `
      <div class="tool-code-header">工具代码 AST:</div>
      <pre class="tool-code-ast"><code>${escapeHtml(astResult)}</code></pre>
      `;
  }
}

async function processApiCallResult(apiInfo: any, astJson: any, codeContent: string): Promise<string> {
  // 尝试处理特殊API调用
  const specialApiHtml = await handleSpecialApiCall(apiInfo);

  if (specialApiHtml) {
    // 如果成功生成了特殊API的HTML，返回带有原始代码和AST的完整结构 (在details内部)
    return `
        ${specialApiHtml}
        <div class="mini-details-container">
          <details class="mini-tech-details">
            <summary aria-label="查看技术详情"><span class="detail-chevron">▲</span></summary>
            <div class="api-details">
              <h4>API调用信息</h4>
              <div class="tool-api-info">
                <div class="tool-api-row"><span class="tool-api-label">API:</span> <span class="tool-api-value">${escapeHtml(apiInfo.api_name)}</span></div>
                <div class="tool-api-row"><span class="tool-api-label">函数:</span> <span class="tool-api-value">${escapeHtml(apiInfo.function_name)}</span></div>
                ${Object.entries(apiInfo.arguments).map(([key, value]) =>
      `<div class="tool-api-row"><span class="tool-api-label">参数 ${key}:</span> <span class="tool-api-value tool-api-param">${escapeHtml(String(value))}</span></div>`
    ).join('')}
              </div>
  
              <h4>AST详情</h4>
              <div class="ast-tree-view">${convertJsonToTreeView(astJson)}</div>
              <details class="mini-tech-details">
                <summary aria-label="查看原始JSON"><span class="detail-chevron">▲</span></summary>
                <pre class="tool-code-ast"><code>${escapeHtml(JSON.stringify(astJson, null, 2))}</code></pre>
              </details>
  
              <h4>原始代码</h4>
              <pre class="tool-code-original"><code>${escapeHtml(codeContent)}</code></pre>
            </div>
          </details>
        </div>
      `;
  } else {
    // 如果不是特殊API或处理失败，返回默认的结构化结果 (不包含原始代码块)
    return `
        <div class="tool-code-header">工具代码解析结果:</div>
        <div class="tool-code-result">
          <div class="tool-api-info">
            <div class="tool-api-row"><span class="tool-api-label">API:</span> <span class="tool-api-value">${escapeHtml(apiInfo.api_name)}</span></div>
            <div class="tool-api-row"><span class="tool-api-label">函数:</span> <span class="tool-api-value">${escapeHtml(apiInfo.function_name)}</span></div>
            ${Object.entries(apiInfo.arguments).map(([key, value]) =>
      `<div class="tool-api-row"><span class="tool-api-label">参数 ${key}:</span> <span class="tool-api-value tool-api-param">${escapeHtml(String(value))}</span></div>`
    ).join('')}
          </div>
        </div>
        <div class="mini-details-container">
          <details class="mini-tech-details">
            <summary aria-label="查看AST详情"><span class="detail-chevron">▲</span></summary>
            <div class="ast-tree-view">${convertJsonToTreeView(astJson)}</div>
            <details class="mini-tech-details ast-raw-json">
              <summary aria-label="查看原始JSON"><span class="detail-chevron">▲</span></summary>
              <pre class="tool-code-ast"><code>${escapeHtml(JSON.stringify(astJson, null, 2))}</code></pre>
            </details>
          </details>
        </div>
      `;
  }
}

// 处理工具代码错误 - 确保仍然显示原始代码
async function handleToolCodeError(toolCodeContainer: HTMLDivElement, error: unknown, codeContent: string): Promise<void> {
  console.error("解析 tool_code 失败:", error);

  // 显示错误信息，并包含原始代码
  toolCodeContainer.innerHTML = `
    <div class="tool-code-error">解析工具代码失败:</div>
    <pre class="tool-code-error-message">${escapeHtml(error instanceof Error ? error.message : String(error))}</pre>
    <div class="tool-code-header original-header">原始代码:</div>
    <pre class="tool-code-original"><code>${escapeHtml(codeContent)}</code></pre>
  `;

  // 确保原始代码也被高亮
  const originalCodeElement = toolCodeContainer.querySelector('.tool-code-original code');
  if (originalCodeElement) {
    hljs.highlightElement(originalCodeElement as HTMLElement);
  }
  // 高亮错误消息中的代码（如果需要）
  const errorMessageElement = toolCodeContainer.querySelector('.tool-code-error-message');
  if (errorMessageElement) {
    // 尝试高亮，如果不是代码则忽略
    try { hljs.highlightElement(errorMessageElement as HTMLElement); } catch (e) { }
  }
}

// 高亮工具代码元素 - 修改为同时支持树形视图
async function highlightToolCodeElements(container: HTMLDivElement): Promise<void> {
  // 对AST结果应用高亮
  const astCodeElement = container.querySelector('.tool-code-ast code');
  if (astCodeElement) {
    hljs.highlightElement(astCodeElement as HTMLElement);
  }

  // 对原始代码应用高亮
  const originalCodeElement = container.querySelector('.tool-code-original code');
  if (originalCodeElement) {
    hljs.highlightElement(originalCodeElement as HTMLElement);
  }

  // 添加树形视图的交互功能
  const jsonDetails = container.querySelectorAll('.json-details');
  jsonDetails.forEach(detail => {
    const summary = detail.querySelector('summary');
    if (summary) {
      summary.addEventListener('click', (e) => {
        // 防止默认的 details 展开/折叠行为冒泡
        e.stopPropagation();
      });
    }
  });
}

// 为代码块添加复制按钮
async function addCopyButtonToCodeBlock(preElement: Element, codeContent: string): Promise<void> {
  // 生成唯一ID
  const uniqueId = `copy-btn-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

  // 创建复制按钮
  const copyButton = document.createElement('button');
  copyButton.className = 'code-copy-button';
  copyButton.id = uniqueId; // 添加唯一ID
  copyButton.dataset.content = btoa(encodeURIComponent(codeContent)); // 将代码内容存储为base64编码的数据属性
  copyButton.innerHTML = `
    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
    </svg>
  `;
  copyButton.title = "复制代码";

  // 添加复制按钮到 pre 元素
  preElement.classList.add('code-block-with-copy');
  preElement.appendChild(copyButton);
}

async function setupAllCopyButtons(): Promise<void> {
  // 查找所有复制按钮
  const copyButtons = document.querySelectorAll('.code-copy-button');
  console.log(`设置 ${copyButtons.length} 个复制按钮事件`);

  copyButtons.forEach(button => {
    // 避免重复添加事件监听器
    if (button.hasAttribute('data-event-bound')) return;

    button.setAttribute('data-event-bound', 'true');
    button.addEventListener('click', async (e) => {
      e.preventDefault();
      e.stopPropagation();

      const btn = e.currentTarget as HTMLElement;
      try {
        const content = btn.dataset.content
          ? decodeURIComponent(atob(btn.dataset.content))
          : '';

        if (!content) {
          console.error("没有找到要复制的内容");
          AppEvents.showNotification("复制失败：没有内容", "error");
          return;
        }

        await writeText(content);

        // 临时更改按钮状态
        btn.innerHTML = `
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M20 6L9 17l-5-5"></path>
                  </svg>
              `;
        btn.classList.add('success');

        // 2秒后恢复原样
        setTimeout(() => {
          if (document.body.contains(btn)) {
            btn.innerHTML = `
                          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                          </svg>
                      `;
            btn.classList.remove('success');
          }
        }, 2000);

        AppEvents.showNotification("代码已复制到剪贴板", "success");
      } catch (error) {
        console.error("复制代码失败:", error);
        AppEvents.showNotification("复制代码失败", "error");
      }
    });
  });
}

/**
 * 解析工具代码AST，提取API调用的关键信息
 * @param ast 解析后的AST JSON对象
 * @returns 包含API调用信息的对象，如果解析失败则返回null
 */
async function parseApiCall(ast: any) {
  try {
    // 检查根节点类型
    if (!ast.node_type) {
      console.warn("AST 节点缺少 node_type 属性");
      return null;
    }

    // 处理 Expressions 类型的根节点（表示多个表达式）
    if (ast.node_type === "Expressions") {
      if (ast.children && ast.children.length > 0) {
        // 尝试解析第一个表达式（通常是一个 LambdaCall）
        return await parseApiCall(ast.children[0]);
      }
      return null;
    }

    // 检查是否为 LambdaCall 类型
    if (ast.node_type !== "LambdaCall" && ast.node_type !== "Tuple") {
      console.warn(`根节点不是支持的类型: ${ast.node_type}`);
      return null;
    }

    // 提取基本信息
    let result: {
      type: string;
      print_call: boolean;
      api_name: string | null;
      function_name: string | null;
      arguments: Record<string, any>;
    } = {
      type: "api_call",
      print_call: false,
      api_name: null,
      function_name: null,
      arguments: {}
    };

    // 检查是否是 print 调用
    if (ast.children && ast.children.length >= 1 &&
      ast.children[0].node_type === "Variable(\"print\")") {
      result.print_call = true;
    }

    // 查找 API 调用部分 (GetAttr 节点)
    let apiCallNode = null;
    let argsNode = null;

    if (result.print_call) {
      // 处理 print(default_api.function_name(...)) 格式
      if (ast.children.length >= 2) {
        // 从 Tuple 节点中提取 LambdaCall
        const tupleNode = ast.children[1];
        if (tupleNode.node_type === "Tuple" && tupleNode.children && tupleNode.children.length > 0) {
          const firstChild = tupleNode.children[0];
          if (firstChild.node_type === "LambdaCall" && firstChild.children && firstChild.children.length > 0) {
            apiCallNode = firstChild.children[0]; // GetAttr 节点
            if (firstChild.children.length > 1) {
              argsNode = firstChild.children[1]; // 参数节点
            }
          }
        }
      }
    } else {
      // 处理直接 API 调用: default_api.function_name(...)
      if (ast.children && ast.children.length > 0) {
        apiCallNode = ast.children[0]; // 第一个子节点应该是 GetAttr
        if (ast.children.length > 1) {
          argsNode = ast.children[1]; // 第二个子节点是参数节点
        }
      }
    }

    // 提取 API 名称和函数名
    if (apiCallNode && apiCallNode.node_type === "GetAttr") {
      // 提取 API 名称 (第一个子节点)
      if (apiCallNode.children && apiCallNode.children.length > 0 &&
        apiCallNode.children[0].node_type &&
        apiCallNode.children[0].node_type.startsWith("Variable(")) {
        // 从 Variable("default_api") 中提取 default_api
        const apiNameMatch = apiCallNode.children[0].node_type.match(/Variable\("(.+)"\)/);
        if (apiNameMatch) {
          result.api_name = apiNameMatch[1];
        }
      }

      // 提取函数名 (第二个子节点)
      if (apiCallNode.children && apiCallNode.children.length > 1 &&
        apiCallNode.children[1].node_type) {
        // 处理函数名称，可能以 String("xxx") 或其他方式表示
        if (apiCallNode.children[1].node_type.startsWith("String(")) {
          const funcNameMatch = apiCallNode.children[1].node_type.match(/String\("(.+)"\)/);
          if (funcNameMatch) {
            result.function_name = funcNameMatch[1];
          }
        } else {
          // 直接从 token 中提取函数名
          result.function_name = apiCallNode.children[1].start_token?.token || null;
        }
      }
    }

    // 处理参数
    if (argsNode && argsNode.node_type === "Tuple" && argsNode.children) {
      for (const child of argsNode.children) {
        if (child.node_type === "Assign" && child.children && child.children.length >= 2) {
          // 提取参数名
          let paramName = null;
          if (child.children[0].node_type.startsWith("Variable(")) {
            const paramNameMatch = child.children[0].node_type.match(/Variable\("(.+)"\)/);
            if (paramNameMatch) {
              paramName = paramNameMatch[1];
            }
          }

          // 提取参数值
          let paramValue = null;
          const valueNode = child.children[1];

          if (!valueNode.node_type) {
            console.warn("参数值节点缺少 node_type");
            continue;
          }

          if (valueNode.node_type.startsWith("String(")) {
            // 字符串类型
            paramValue = valueNode.start_token?.token || "";
          } else if (valueNode.node_type.startsWith("Number(")) {
            // 数字类型
            const numValue = valueNode.start_token?.token;
            paramValue = !isNaN(Number(numValue)) ? Number(numValue) : numValue;
          } else if (valueNode.node_type.startsWith("Boolean(")) {
            // 布尔类型
            paramValue = valueNode.start_token?.token === "True";
          } else if (valueNode.node_type === "Variable(\"None\")") {
            // None 类型
            paramValue = null;
          } else if (valueNode.node_type === "Variable(\"True\")") {
            // True 类型
            paramValue = true;
          } else if (valueNode.node_type === "Variable(\"False\")") {
            // False 类型
            paramValue = false;
          } else {
            // 其他类型或复杂表达式
            console.warn("未知参数类型:", valueNode.node_type);
            paramValue = valueNode.start_token?.token || null;
          }

          // 添加到参数列表
          if (paramName !== null && paramValue !== undefined) {
            result.arguments[paramName] = paramValue;
          }
        }
      }
    }

    // 检查是否解析到必要的API信息
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

/**
 * 处理特殊API调用，生成对应的HTML内容
 * @param apiInfo 解析出的API调用信息
 * @returns 处理后的HTML内容，如果无法处理则返回null
 */
async function handleSpecialApiCall(apiInfo: any): Promise<string | null> {
  // 只处理default_api的调用
  if (apiInfo.api_name !== 'default_api') {
    return null;
  }

  // 根据函数名分发到不同的处理函数
  switch (apiInfo.function_name) {
    case 'mermaid_render':
      return await handleMermaidRender(apiInfo);
    case 'interactive_button':
      return await handleInteractiveButton(apiInfo);
    case 'typst_render':
      return await handleTypstRender(apiInfo);
    case 'html_render':
      return await handleHTMLRender(apiInfo);
    case 'katex_render':
      return await handleKaTeXRender(apiInfo);
    case 'wolfram_alpha_compute':
      return await handleWolframRender(apiInfo);
    case 'pintora_render':
      return await handlePintoraRender(apiInfo);
    // 在这里可以方便地添加新的函数处理
    default:
      return null; // 不认识的函数调用，返回null使用默认显示
  }
}

export {
  applyHighlight,
  handleSpecialApiCall,
  processApiCallResult,
  setupAllCopyButtons
};
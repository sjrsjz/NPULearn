
import { nextTick } from 'vue';
// hljs
import hljs from 'highlight.js/lib/core';
import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { renderMermaidDiagrams, setupMermaidRefresh } from '../mermaidRenderer';
import { isStreaming, AppEvents } from '../eventBus';


// 主函数：应用代码高亮和处理各种特殊代码块
async function applyHighlight(): Promise<void> {
    await nextTick(); // 确保 DOM 更新完成
    const timestamp = Date.now();

    // 查找所有代码块
    const codeElements = document.querySelectorAll('.chat-messages pre code');
    if (!codeElements || codeElements.length === 0) return;

    console.log(`处理 ${codeElements.length} 个代码块`);

    // 创建一批次操作对象，用于收集所有DOM修改
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

    // 应用所有批次操作到DOM (一次性更新以减少抖动)
    await applyBatchOperations(batch);

    // 根据是否在流式传输中决定如何处理图表渲染
    if (!isStreaming.value) {
        schedulePostHighlightTasks();
    } else {
        console.log("流式传输中，推迟渲染任务");
    }

    console.log(`高亮处理完成，耗时: ${Date.now() - timestamp}ms`);
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
        if (!codeContent) continue;

        const preElement = el.parentElement;
        if (!preElement) continue;

        console.log("检测到 tool_code 代码块:", codeContent);

        // 创建工具代码容器
        const toolCodeContainer = document.createElement('div');
        toolCodeContainer.className = 'tool-code-container';

        // 设置初始加载状态
        toolCodeContainer.innerHTML = `
          <div class="tool-code-loading">正在解析工具代码...</div>
          <pre class="tool-code-original" style="display: none;"><code>${codeContent}</code></pre>
        `;

        // 收集替换信息，但不立即执行
        batch.toolCodeReplacements.push({
            original: preElement,
            replacement: toolCodeContainer,
            content: codeContent
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
            el.classList.contains('language-mermaid') ||
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
        addCopyButtonToCodeBlock(item.element, item.content);
    }

    // 4. 现在处理所有工具代码块内容
    // 这部分需要异步处理，但因为容器已经创建，
    // 所以即使这些操作会更改内部内容，也不会导致页面布局大幅变化
    await Promise.all(batch.toolCodeReplacements.map(async (item: any) => {
        try {
            await processToolCode(item.replacement, item.content);
        } catch (error) {
            handleToolCodeError(item.replacement, error, item.content);
        }
    }));
}

// 处理工具代码的逻辑
async function processToolCode(toolCodeContainer: HTMLDivElement, codeContent: string): Promise<void> {
    if (isStreaming.value) {
        // 直接显示原始代码和流式传输提示
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
        <pre class="tool-code-original"><code>${codeContent}</code></pre>
        `;

        // 高亮显示AST和原始代码
        highlightToolCodeElements(toolCodeContainer);
        return;
    }


    // 调用后端解析代码
    const astResult = await invoke<string>("parse_code", { code: codeContent });
    console.log("AST 解析结果:", astResult);

    // 解析AST JSON并处理
    const astJson = JSON.parse(astResult);
    const apiInfo = parseApiCall(astJson);

    if (apiInfo) {
        toolCodeContainer.innerHTML = processApiCallResult(apiInfo, astJson, codeContent);
    } else {
        // 解析失败时显示原始AST结果
        toolCodeContainer.innerHTML = createToolCodeFallbackView(astResult, codeContent);
        // 高亮显示AST和原始代码
        highlightToolCodeElements(toolCodeContainer);
    }
}

// // 创建工具代码流式视图
// function createToolCodeStreamingView(apiInfo: any, codeContent: string): string {
//     return `
//     <div class="tool-code-header">工具代码解析结果:</div>
//     <div class="tool-code-result">
//       <div class="tool-api-info">
//         <div class="tool-api-row"><span class="tool-api-label">API:</span> <span class="tool-api-value">${apiInfo.api_name}</span></div>
//         <div class="tool-api-row"><span class="tool-api-label">函数:</span> <span class="tool-api-value">${apiInfo.function_name}</span></div>
//         ${Object.entries(apiInfo.arguments).map(([key, value]) =>
//         `<div class="tool-api-row"><span class="tool-api-label">参数 ${key}:</span> <span class="tool-api-value tool-api-param">${value}</span></div>`
//     ).join('')}
//       </div>
//     </div>
//     <div class="api-call-notice">
//       <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
//         <circle cx="12" cy="12" r="10"></circle>
//         <line x1="12" y1="8" x2="12" y2="12"></line>
//         <line x1="12" y1="16" x2="12.01" y2="16"></line>
//       </svg>
//       <span>API功能将在消息完整接收后处理</span>
//     </div>
//     <pre class="tool-code-original"><code>${codeContent}</code></pre>
//   `;
// }

// 创建工具代码的备用视图（解析失败时）
function createToolCodeFallbackView(astResult: string, codeContent: string): string {
    return `
    <div class="tool-code-header">工具代码 AST:</div>
    <pre class="tool-code-ast"><code>${JSON.stringify(JSON.parse(astResult), null, 2)}</code></pre>
    <div class="tool-code-header original-header">原始代码:</div>
    <pre class="tool-code-original"><code>${codeContent}</code></pre>
  `;
}

// 处理工具代码错误
function handleToolCodeError(toolCodeContainer: HTMLDivElement, error: unknown, codeContent: string): void {
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

// 高亮工具代码元素
function highlightToolCodeElements(container: HTMLDivElement): void {
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
}


// 为代码块添加复制按钮
function addCopyButtonToCodeBlock(preElement: Element, codeContent: string): void {
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

            AppEvents.showNotification("代码已复制到剪贴板", "success");
        } catch (error) {
            console.error("复制代码失败:", error);
            AppEvents.showNotification("复制代码失败", "error");
        }
    });

    // 添加复制按钮到 pre 元素
    preElement.classList.add('code-block-with-copy');
    preElement.appendChild(copyButton);
}

// 处理高亮后需要执行的任务
function schedulePostHighlightTasks(): void {
    // 使用 Promise.resolve() 和 setTimeout 来确保这些任务在当前事件循环之后执行
    Promise.resolve().then(() => {
        setTimeout(() => {
            renderMermaidDiagrams();
            setupInteractiveButtons(); // 设置交互按钮
        }, 300);
    });
}

function completeStreamRendering(): void {
    console.log("流式传输完成，执行延迟的渲染任务");

    // 标记需要重新渲染的Mermaid容器
    document.querySelectorAll('.mermaid-container:not(.loaded)').forEach(container => {
        container.removeAttribute('data-last-rendered');
        // 添加一个标志，防止重复处理
        container.setAttribute('data-pending-render', 'true');
    });

    // 处理工具代码块
    document.querySelectorAll('.tool-code-container').forEach(async container => {
        const originalCodeElem = container.querySelector('.tool-code-original code');
        if (originalCodeElem && originalCodeElem.textContent) {
            const codeContent = originalCodeElem.textContent.trim();

            // 重新处理工具代码
            try {
                await processToolCode(container as HTMLDivElement, codeContent);
            } catch (error) {
                console.error("重新处理工具代码失败:", error);
                handleToolCodeError(container as HTMLDivElement, error, codeContent);
            }
        }
    });

    // 使用更稳健的渲染策略，增加延迟确保DOM已准备好
    setTimeout(() => {
        console.log("开始第一轮Mermaid图表渲染");
        // 检查渲染前的图表数量
        const beforeRenderCount = document.querySelectorAll('.mermaid-container').length;
        console.log(`渲染前找到 ${beforeRenderCount} 个Mermaid容器`);

        // 第一轮渲染
        renderMermaidDiagrams(0, 5).then(() => {
            // 检查渲染结果
            const unrenderedCount = document.querySelectorAll('.mermaid-container:not(.loaded)').length;
            const renderedCount = document.querySelectorAll('.mermaid-container.loaded').length;
            console.log(`第一轮渲染后: 已渲染 ${renderedCount} 个图表, 未渲染 ${unrenderedCount} 个图表`);

            // 如果仍有未渲染的图表，进行第二轮渲染
            if (unrenderedCount > 0) {
                console.log(`进行第二轮渲染`);
                // 增加延迟时间，确保DOM更新完成
                setTimeout(() => {
                    renderMermaidDiagrams(0, 5).then(() => {
                        // 处理按钮和刷新逻辑
                        console.log("渲染完成，设置刷新和交互按钮");
                        setupMermaidRefresh();
                        setupInteractiveButtons();

                        // 再增加一个保险措施，确保图表可见
                        document.querySelectorAll('.mermaid-container').forEach(container => {
                            if (container.querySelector('svg')) {
                                container.classList.add('loaded');
                                container.querySelector('.mermaid-loading')?.remove();
                            }
                        });
                    });
                }, 800); // 增加延迟
            } else {
                console.log("所有图表已在第一轮渲染完成");
                setupMermaidRefresh();
                setupInteractiveButtons();
            }
        });
    }, 700); // 增加初始延迟
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
                        paramValue = child.children[1].start_token.token;
                    } else if (child.children[1].node_type.startsWith("Number(")) {
                        paramValue = child.children[1].start_token.token as number;
                    } else if (child.children[1].node_type.startsWith("Boolean(")) {
                        paramValue = child.children[1].start_token.token === "True" ? true : false;
                    } else {
                        // 其他类型的处理逻辑可以在这里添加
                        console.warn("未知参数类型:", child.children[1].node_type);
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

/**
 * 处理特殊API调用，生成对应的HTML内容
 * @param apiInfo 解析出的API调用信息
 * @returns 处理后的HTML内容，如果无法处理则返回null
 */
function handleSpecialApiCall(apiInfo: any): string | null {
    // 只处理default_api的调用
    if (apiInfo.api_name !== 'default_api') {
        return null;
    }

    // 根据函数名分发到不同的处理函数
    switch (apiInfo.function_name) {
        case 'mermaid_render':
            return handleMermaidRender(apiInfo);
        case 'interactive_button':
            return handleInteractiveButton(apiInfo);
        // 在这里可以方便地添加新的函数处理
        default:
            return null; // 不认识的函数调用，返回null使用默认显示
    }
}

/**
 * 处理mermaid_render API调用
 * @param apiInfo API调用信息
 * @returns 生成的HTML内容
 */
function handleMermaidRender(apiInfo: any): string {
    // 获取mermaid代码参数
    const mermaidCode = apiInfo.arguments.mermaid_code || '';
    console.log("处理mermaid_render:", mermaidCode);
    // 创建唯一的图表ID
    const diagramId = `mermaid-diagram-${Date.now()}-${Math.floor(Math.random() * 10000)}`;

    // 编码内容，以便在属性中安全存储
    const encodedContent = encodeURIComponent(mermaidCode);

    // 构建HTML
    return `
    <div class="special-api-call mermaid-api-call">
      <div class="api-call-header">
        <span class="api-call-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 20.94c1.5 0 2.75 1.06 4 1.06 3 0 6-8 6-12.22A4.91 4.91 0 0 0 17 5c-2.22 0-4 1.44-5 2-1-.56-2.78-2-5-2a4.9 4.9 0 0 0-5 4.78C2 14 5 22 8 22c1.25 0 2.5-1.06 4-1.06Z"></path>
            <path d="M10 2c1 .5 2 2 2 5"></path>
          </svg>
        </span>
        <span class="api-call-title">Mermaid 图表</span>
      </div>
      <div class="mermaid-container" data-diagram-id="${diagramId}" data-diagram-content="${encodedContent}">
        <div class="mermaid-loading">UML图表加载中...</div>
      </div>
      <div class="api-call-footer">
        <details>
          <summary>查看图表代码</summary>
          <pre class="api-call-code"><code class="language-mermaid">${mermaidCode.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
        </details>
      </div>
    </div>
  `;
}

/**
 * 处理interactive_button API调用
 * @param apiInfo API调用信息
 * @returns 生成的HTML内容
 */
function handleInteractiveButton(apiInfo: any): string {
    // 获取参数
    const message = apiInfo.arguments.message || '点击发送';
    const command = apiInfo.arguments.command || '';

    // 编码命令，用于button属性
    const encodedCommand = encodeURIComponent(command);

    // 构建HTML - 使用与button://链接处理相同的类和属性
    return `
    <div class="special-api-call interactive-button-api-call">
      <div class="api-call-header">
        <span class="api-call-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
            <path d="M7 11v2"></path>
            <path d="M11 7h2"></path>
            <path d="M11 15h2"></path>
            <path d="M15 11v2"></path>
          </svg>
        </span>
        <span class="api-call-title">交互按钮</span>
      </div>
      <div class="interactive-button-container">
        <button class="markdown-button interactive-command-button" data-command="${encodedCommand}">${message}</button>
      </div>
      <div class="api-call-footer">
        <details>
          <summary>查看按钮配置</summary>
          <pre class="api-call-code"><code>消息: ${message}
命令: ${command}</code></pre>
        </details>
      </div>
    </div>
  `;
}

// 添加此函数用于设置交互按钮的点击事件
function setupInteractiveButtons() {
    nextTick(() => {
        document.querySelectorAll('.chat-messages .interactive-command-button').forEach(button => {
            if (button.hasAttribute('data-event-attached')) return; // 避免重复添加事件

            button.setAttribute('data-event-attached', 'true');
            button.addEventListener('click', async (e) => {
                e.preventDefault();

                // 如果正在流式输出消息，禁止发送新消息
                if (isStreaming.value) {
                    AppEvents.showNotification("请等待当前消息输出完成", "error");
                    return;
                }

                const encodedCommand = (button as HTMLElement).getAttribute('data-command');
                if (encodedCommand) {
                    const command = decodeURIComponent(encodedCommand);
                    if (command.trim()) {
                        // 发送消息
                        await AppEvents.sendStreamMessageDirect("> " + command);
                        AppEvents.showNotification("已发送命令", "success");
                    }
                }
            });
        });
    });
}

// 修改原有的解析逻辑，整合特殊API处理
function processApiCallResult(apiInfo: any, astJson: any, codeContent: string): string {
    // 尝试处理特殊API调用
    const specialApiHtml = handleSpecialApiCall(apiInfo);

    if (specialApiHtml) {
        // 如果成功生成了特殊API的HTML，返回带有原始代码和AST的完整结构
        return `
      ${specialApiHtml}
      <details class="tool-code-details">
        <summary>查看技术详情</summary>
        <div class="api-details">
          <h4>API调用信息</h4>
          <div class="tool-api-info">
            <div class="tool-api-row"><span class="tool-api-label">API:</span> <span class="tool-api-value">${apiInfo.api_name}</span></div>
            <div class="tool-api-row"><span class="tool-api-label">函数:</span> <span class="tool-api-value">${apiInfo.function_name}</span></div>
            ${Object.entries(apiInfo.arguments).map(([key, value]) =>
            `<div class="tool-api-row"><span class="tool-api-label">参数 ${key}:</span> <span class="tool-api-value tool-api-param">${value}</span></div>`
        ).join('')}
          </div>
          
          <h4>AST详情</h4>
          <pre class="tool-code-ast"><code>${JSON.stringify(astJson, null, 2)}</code></pre>
          
          <h4>原始代码</h4>
          <pre class="tool-code-original"><code>${codeContent}</code></pre>
        </div>
      </details>
    `;
    } else {
        // 如果不是特殊API或处理失败，返回默认的结构化结果
        return `
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
    }
}


export {
    applyHighlight,
    completeStreamRendering,
    schedulePostHighlightTasks,
    handleSpecialApiCall,
    processApiCallResult,
};
import { nextTick } from "vue";
import { AppEvents, isStreaming } from '../eventBus';

// 动态导入 Mermaid 以避免构建时的模块加载问题
let mermaid: any = null;

// 加载 Mermaid 模块
async function loadMermaidModule() {
    if (mermaid) {
        return mermaid;
    }
    
    try {
        const module = await import('mermaid');
        mermaid = module.default;
        return mermaid;
    } catch (error) {
        console.error('Failed to load Mermaid module:', error);
        throw error;
    }
}

// 初始化Mermaid.js配置
async function initMermaid() {
    try {
        const mermaidInstance = await loadMermaidModule();
        mermaidInstance.initialize({
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
    } catch (error) {
        console.error('Mermaid 初始化失败:', error);
    }
}

// 修改渲染Mermaid图表函数，接受容器参数
async function renderMermaidDiagrams(retryCount = 0, maxRetries = 3, container: HTMLElement = document.body) {
    if (!mermaid) {
        console.error('Mermaid 模块未加载');
        return;
    }

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
        // 查找所有需要渲染的UML元素，使用传入的容器
        const umlElements = container.querySelectorAll('.mermaid-container:not(.loaded)');
        console.log(`尝试渲染 ${umlElements.length} 个UML图表，当前重试次数: ${retryCount}`);

        if (umlElements.length === 0 && retryCount === 0) {
            // 第一次调用且没有找到未加载的图表，检查是否需要全局重新渲染
            const allUmlElements = container.querySelectorAll('.mermaid-container');
            if (allUmlElements.length > 0) {
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
                                    return true;                                } catch (renderError) {
                                    console.error(`单个图表渲染失败 ID ${id}:`, renderError);
                                    element.innerHTML = `
                                        <div class="mermaid-error">
                                            <p>UML图表渲染失败</p>
                                            <pre class="error-message">${renderError}</pre>
                                            <div class="mermaid-source">
                                            <details>
                                                <summary>查看原始图表代码</summary>
                                                <div class="code-container">
                                                <pre class="code-content">${content}</pre>
                                                </div>
                                            </details>
                                            </div>
                                        </div>
                                        `;
                                    return false;
                                }
                            } else {
                                throw new Error("解码后的内容为空或无效。");
                            }
                        })
                    );                } catch (error) {
                    // 记录更详细的错误信息和失败的内容
                    console.error(`渲染图表 ID ${id} 失败:`, error);
                    console.error("失败的内容 (decoded):", content); // 记录导致失败的解码后内容
                    element.innerHTML = `
                        <div class="mermaid-error">
                            <p>UML图表渲染失败</p>
                            <pre class="error-message">${error}</pre>
                            <div class="mermaid-source">
                            <details>
                                <summary>查看原始图表代码</summary>
                                <div class="code-container">
                                <pre class="code-content">${content}</pre>
                                </div>
                            </details>
                            </div>
                        </div>
                        `;
                }
            } else {
                // 如果容器缺少必要的属性，则发出警告
                console.warn("发现缺少必要属性（id 或 content）的 Mermaid 容器。", element);
            }
        }        // 等待所有渲染完成
        if (renderPromises.length > 0) {
            const results = await Promise.all(renderPromises);
            const failedCount = results.filter(success => !success).length;

            // 如果有失败的图表，且未超过最大重试次数，则重试
            if (failedCount > 0 && retryCount < maxRetries) {
                console.log(`${failedCount}个图表渲染失败，将在1.5秒后重试 (${retryCount + 1}/${maxRetries})`);
                setTimeout(() => renderMermaidDiagrams(retryCount + 1, maxRetries, container), 1500);
            } else if (failedCount > 0) {
                console.log(`渲染完成，但有${failedCount}个图表渲染失败，已达到最大重试次数`);
                // 为失败的图表添加重试按钮事件监听
                setupAllMermaidInteractions(container);
            } else {
                console.log('所有图表渲染成功');
                // 设置图表的可点击功能
                setupAllMermaidInteractions(container);
            }

            // 无论成功失败，都延迟再次调用一次以确保所有图表都得到正确处理
            setTimeout(() => {
                console.log('延迟检查，确保所有图表事件绑定正确');
                setupAllMermaidInteractions(container);
            }, 500);
        } else {
            // 如果没有需要渲染的图表，也处理已渲染的图表
            setupAllMermaidInteractions(container);
        }
    } catch (error) {
        console.error("处理Mermaid图表失败:", error);
        if (retryCount < maxRetries) {
            console.log(`整体处理失败，将在1.5秒后重试 (${retryCount + 1}/${maxRetries})`);
            setTimeout(() => renderMermaidDiagrams(retryCount + 1, maxRetries, container), 1500);
        } else {
            // 即使出错，也尝试为已渲染的图表添加交互功能
            setupAllMermaidInteractions(container);
        }
    }
}

// 设置图表渲染失败后的重试按钮事件（已移除重试按钮，此函数暂时保留以避免调用错误）
function setupRetryButtons(_container: HTMLElement = document.body) {
    // 重试按钮已被移除，此函数不再执行任何操作
    console.log('重试按钮已被移除，setupRetryButtons 函数不再执行操作');
}

// 修改设置刷新按钮函数，接受容器参数
function setupMermaidRefresh(container: HTMLElement = document.body) {
    nextTick(() => {
        // 为所有图表容器添加刷新按钮
        container.querySelectorAll('.mermaid-container').forEach(diagramContainer => {
            // 强制类型转换以便后续操作
            const mermaidContainer = diagramContainer as HTMLElement;

            // 检查容器是否已经有刷新按钮
            if (!mermaidContainer.querySelector('.refresh-diagram-button')) {
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
                    const clickedContainer = targetButton.closest('.mermaid-container');

                    if (clickedContainer) {
                        // 移除loaded类以便重新渲染
                        clickedContainer.classList.remove('loaded');
                        // 清除上次渲染的内容记录，强制重新渲染
                        clickedContainer.removeAttribute('data-last-rendered');
                        targetButton.classList.add('refreshing');
                        AppEvents.showNotification("正在刷新图表...", "info");

                        // 延迟后渲染以确保UI更新
                        setTimeout(async () => {
                            await renderMermaidDiagrams(0, 3, container);
                            targetButton.classList.remove('refreshing');
                        }, 100);
                    }
                });

                // 将按钮添加到容器中
                mermaidContainer.appendChild(refreshButton);
            }

            // 改进的渲染成功检查逻辑
            const hasLoadedClass = mermaidContainer.classList.contains('loaded');
            const hasErrorElement = mermaidContainer.querySelector('.mermaid-error');
            const hasSvgElement = mermaidContainer.querySelector('svg');
            const isRenderedSuccessfully = hasLoadedClass && !hasErrorElement && hasSvgElement;

            console.log(`图表容器检查 - ID: ${mermaidContainer.getAttribute('data-diagram-id')}, loaded: ${hasLoadedClass}, hasError: ${!!hasErrorElement}, hasSvg: ${!!hasSvgElement}, success: ${isRenderedSuccessfully}`);

            // 只有成功渲染的图表才添加放大按钮
            if (isRenderedSuccessfully && !mermaidContainer.querySelector('.zoom-diagram-button')) {
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

                    const clickedContainer = (e.currentTarget as HTMLElement).closest('.mermaid-container') as HTMLElement;
                    if (clickedContainer) {
                        const svgElement = clickedContainer.querySelector('svg');
                        const contentElement = clickedContainer.getAttribute('data-diagram-content');

                        if (svgElement && contentElement) {
                            const svgContent = svgElement.outerHTML;
                            const diagramContent = decodeURIComponent(contentElement);
                            AppEvents.openChartViewer(svgContent, diagramContent);
                        }
                    }
                });

                mermaidContainer.appendChild(zoomButton);
                console.log(`已为图表添加放大按钮 - ID: ${mermaidContainer.getAttribute('data-diagram-id')}`);
            } else if (!isRenderedSuccessfully && mermaidContainer.querySelector('.zoom-diagram-button')) {
                // 如果图表渲染失败，但之前添加了放大按钮，则移除它
                const zoomButton = mermaidContainer.querySelector('.zoom-diagram-button');
                if (zoomButton) {
                    zoomButton.remove();
                    console.log(`已移除失败图表的放大按钮 - ID: ${mermaidContainer.getAttribute('data-diagram-id')}`);
                }
            }

            // 只为成功渲染的图表添加点击事件
            if (isRenderedSuccessfully) {
                // 为整个容器添加点击事件以打开查看器
                if (!mermaidContainer.hasAttribute('data-has-click-listener')) {
                    mermaidContainer.setAttribute('data-has-click-listener', 'true');                    mermaidContainer.addEventListener('click', (e) => {
                        // 点击按钮时不触发
                        if ((e.target as HTMLElement).closest('.refresh-diagram-button, .zoom-diagram-button')) {
                            return;
                        }

                        const svgElement = mermaidContainer.querySelector('svg');
                        const contentElement = mermaidContainer.getAttribute('data-diagram-content');

                        if (svgElement && contentElement) {
                            const svgContent = svgElement.outerHTML;
                            const diagramContent = decodeURIComponent(contentElement);
                            AppEvents.openChartViewer(svgContent, diagramContent);
                        }
                    });

                    // 添加视觉提示，表明容器可点击
                    mermaidContainer.classList.add('clickable-container');
                    console.log(`已为图表添加点击事件 - ID: ${mermaidContainer.getAttribute('data-diagram-id')}`);
                }
            } else {
                // 如果图表渲染失败，移除点击相关的类和属性
                mermaidContainer.classList.remove('clickable-container');
                mermaidContainer.removeAttribute('data-has-click-listener');
                console.log(`已移除失败图表的点击功能 - ID: ${mermaidContainer.getAttribute('data-diagram-id')}`);
            }
        });
    });
}

function changeMermaidTheme(theme: string) {
    if (theme === 'dark') {
        mermaid.initialize({ theme: 'dark' });
    } else if (theme === 'default') {
        mermaid.initialize({ theme: 'default' });
    } else {
        console.warn(`未知主题: ${theme}`);
    }
}

/**
 * 处理mermaid_render API调用
 * @param apiInfo API调用信息
 * @param ifStreaming 是否正在流式传输，如果为false则立即渲染图表
 * @returns 生成的HTML内容
 */
async function handleMermaidRender(apiInfo: any) {
    // 获取mermaid代码参数
    const mermaidCode = apiInfo.arguments.mermaid_code || '';
    console.log("处理mermaid_render:", mermaidCode);
    // 创建唯一的图表ID
    const diagramId = `mermaid-diagram-${Date.now()}-${Math.floor(Math.random() * 10000)}`;

    // 编码内容，以便在属性中安全存储
    const encodedContent = encodeURIComponent(mermaidCode);

    // 准备初始内容
    let initialContent = '<div class="mermaid-loading">UML图表加载中...</div>';
    let isLoaded = false;

    // 如果不是流式传输，则立即渲染图表
    if (!isStreaming.value && mermaidCode) {
        try {
            // 初始化mermaid确保渲染环境正确
            const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                (document.documentElement.getAttribute('data-theme') === 'system' &&
                    window.matchMedia('(prefers-color-scheme: dark)').matches);

            mermaid.initialize({
                theme: isDark ? 'dark' : 'default',
                securityLevel: 'loose',
                startOnLoad: false
            });

            // 立即渲染图表
            const renderResult = await mermaid.render(diagramId, mermaidCode);
            const svgContent = renderResult.svg;
            initialContent = svgContent;
            isLoaded = true;        } catch (error) {
            console.error("立即渲染图表失败:", error);
            initialContent = `
                <div class="mermaid-error">
                    <p>UML图表渲染失败</p>
                    <pre class="error-message">${error}</pre>
                    <div class="mermaid-source">
                    <details>
                        <summary>查看原始图表代码</summary>
                        <div class="code-container">
                        <pre class="code-content">${mermaidCode}</pre>
                        </div>
                    </details>
                    </div>
                </div>
            `;
        }
    }

    // 构建HTML
    const html = `
    <div class="special-api-call mermaid-api-call" id="${diagramId}-container">
      <div class="api-call-header">
        <span class="api-call-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 20.94c1.5 0 2.75 1.06 4 1.06 3 0 6-8 6-12.22A4.91 4.91 0 0 0 17 5c-2.22 0-4 1.44-5 2-1-.56-2.78-2-5-2a4.9 4.9 0 0 0-5 4.78C2 14 5 22 8 22c1.25 0 2.5-1.06 4-1.06Z"></path>
            <path d="M10 2c1 .5 2 2 2 5"></path>
          </svg>
        </span>
        <span class="api-call-title">Mermaid 图表</span>
      </div>
      <div class="mermaid-container ${isLoaded ? 'loaded' : ''}" data-diagram-id="${diagramId}" data-diagram-content="${encodedContent}" ${isLoaded ? `data-last-rendered="${encodedContent}"` : ''}>
        ${initialContent}
      </div>
      <div class="api-call-footer">
        <details>
          <summary>查看图表代码</summary>
          <pre class="api-call-code"><code class="language-mermaid">${mermaidCode.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
        </details>
      </div>
    </div>
  `;    // 非流式传输下，为刚渲染的图表绑定事件
    if (!isStreaming.value) {
        setTimeout(() => {
            const container = document.getElementById(`${diagramId}-container`);
            if (container) {
                console.log(`立即为非流式图表绑定事件 - ID: ${diagramId}`);
                // 统一处理图表按钮和事件绑定
                setupAllMermaidInteractions(container);

                // 再次延迟确保绑定成功
                setTimeout(() => {
                    console.log(`延迟再次检查图表事件绑定 - ID: ${diagramId}`);
                    setupAllMermaidInteractions(container);
                }, 200);
            }
        }, 0);  // 使用setTimeout确保HTML先被添加到DOM
    }

    return html;
}

// 统一处理所有mermaid图表的按钮和事件绑定
function setupAllMermaidInteractions(container: HTMLElement = document.body) {
    console.log(`开始设置Mermaid图表交互功能，容器:`, container);

    // 使用更长的延迟确保DOM完全更新
    nextTick(() => {
        setTimeout(() => {
            console.log(`执行Mermaid图表交互设置`);

            // 检查容器内的所有图表
            const mermaidContainers = container.querySelectorAll('.mermaid-container');
            console.log(`找到 ${mermaidContainers.length} 个图表容器`);

            // 设置刷新按钮和交互功能
            setupMermaidRefresh(container);
            // 设置重试按钮事件
            setupRetryButtons(container);

            // 再次验证设置结果
            setTimeout(() => {
                const updatedContainers = container.querySelectorAll('.mermaid-container');
                updatedContainers.forEach((mermaidContainer, index) => {
                    const hasRefreshBtn = mermaidContainer.querySelector('.refresh-diagram-button');
                    const hasZoomBtn = mermaidContainer.querySelector('.zoom-diagram-button');
                    const hasClickListener = mermaidContainer.hasAttribute('data-has-click-listener');
                    const id = mermaidContainer.getAttribute('data-diagram-id');

                    console.log(`图表 ${index + 1} (ID: ${id}) - 刷新按钮: ${!!hasRefreshBtn}, 放大按钮: ${!!hasZoomBtn}, 点击监听: ${hasClickListener}`);
                });
            }, 100);
        }, 50);
    });
}

export { initMermaid, renderMermaidDiagrams, setupMermaidRefresh, setupRetryButtons, setupAllMermaidInteractions, changeMermaidTheme, handleMermaidRender };
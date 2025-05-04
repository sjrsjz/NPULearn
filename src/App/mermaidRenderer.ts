import mermaid from 'mermaid'; // 导入Mermaid.js库
import { nextTick } from "vue";

import { AppEvents } from './eventBus';

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

// 修改渲染Mermaid图表函数，移除防抖机制
async function renderMermaidDiagrams(retryCount = 0, maxRetries = 3) {
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
                    );
                } catch (error) {
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
                    AppEvents.showNotification("正在重新渲染图表...", "info");

                    // 特别处理这个容器
                    await renderMermaidDiagrams(0, 3);
                }
            });
        });
    });
}

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
                        AppEvents.showNotification("正在刷新图表...", "info");

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

            // 检查图表是否渲染成功（有 loaded 类且没有 mermaid-error 元素）
            const isRenderedSuccessfully = container.classList.contains('loaded') &&
                !container.querySelector('.mermaid-error');

            // 只有成功渲染的图表才添加放大按钮
            if (isRenderedSuccessfully && !container.querySelector('.zoom-diagram-button')) {
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
                            AppEvents.openChartViewer(svgContent, diagramContent);
                        }
                    }
                });

                container.appendChild(zoomButton);
            } else if (!isRenderedSuccessfully && container.querySelector('.zoom-diagram-button')) {
                // 如果图表渲染失败，但之前添加了放大按钮，则移除它
                const zoomButton = container.querySelector('.zoom-diagram-button');
                if (zoomButton) zoomButton.remove();
            }

            // 只为成功渲染的图表添加点击事件
            if (isRenderedSuccessfully) {
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
                            AppEvents.openChartViewer(svgContent, diagramContent);
                        }
                    });

                    // 添加视觉提示，表明容器可点击
                    container.classList.add('clickable-container');
                }
            } else {
                // 如果图表渲染失败，移除点击相关的类和属性
                container.classList.remove('clickable-container');
                // 不删除事件监听器，因为这可能导致内存泄漏问题，而是让它不执行实际操作
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

export { initMermaid, renderMermaidDiagrams, setupMermaidRefresh, setupRetryButtons, changeMermaidTheme };
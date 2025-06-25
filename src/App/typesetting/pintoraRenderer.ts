import { nextTick } from "vue";
import { AppEvents, isStreaming } from '../eventBus';

// 动态导入 Pintora 以避免构建时的模块加载问题
let pintoraStandalone: any = null;

// 防止重复初始化的标志
let isPintoraInitialized = false;
let pintoraInitPromise: Promise<void> | null = null;

// 加载 Pintora 模块
async function loadPintoraModule() {
    if (pintoraStandalone) {
        return pintoraStandalone;
    }
    
    try {
        const module = await import('@pintora/standalone');
        pintoraStandalone = module.pintoraStandalone;
        return pintoraStandalone;
    } catch (error) {
        console.error('Failed to load Pintora module:', error);
        throw error;
    }
}

// 初始化Pintora配置
async function initPintora() {
    // 如果已经初始化或正在初始化，返回现有的 Promise
    if (isPintoraInitialized) {
        return Promise.resolve();
    }
    
    if (pintoraInitPromise) {
        return pintoraInitPromise;
    }

    pintoraInitPromise = new Promise<void>(async (resolve, reject) => {
        try {
            // 延迟初始化以确保所有依赖已加载
            setTimeout(async () => {
                try {
                    // 动态加载 Pintora 模块
                    const standalone = await loadPintoraModule();
                    
                    // 检查 pintoraStandalone 是否正确加载
                    if (!standalone || typeof standalone.initBrowser !== 'function') {
                        throw new Error('Pintora Standalone 未正确加载');
                    }

                    // 修复：初始化函数不接受 defaultConfig 参数
                    standalone.initBrowser();
                    
                    // 初始化后单独设置主题
                    const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                        (document.documentElement.getAttribute('data-theme') === 'system' &&
                            window.matchMedia('(prefers-color-scheme: dark)').matches);
                    
                    // 使用setConfig而不是传入defaultConfig
                    standalone.setConfig({
                        themeConfig: {
                            theme: isDark ? 'dark' : 'default'
                        }
                    });

                    isPintoraInitialized = true;
                    console.log('Pintora 初始化成功');
                    resolve();
                } catch (error) {
                    console.error('Pintora 初始化失败:', error);
                    reject(error);
                }
            }, 200); // 增加延迟到 200ms
        } catch (error) {
            console.error('Pintora 初始化过程中出错:', error);
            reject(error);
        }
    });

    return pintoraInitPromise;
}

// 渲染Pintora图表函数
async function renderPintoraDiagrams(retryCount = 0, maxRetries = 3, container: HTMLElement = document.body) {
    // 确保 Pintora 已初始化
    if (!isPintoraInitialized) {
        try {
            await initPintora();
        } catch (error) {
            console.error('无法初始化 Pintora，跳过图表渲染:', error);
            return;
        }
    }

    const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
        (document.documentElement.getAttribute('data-theme') === 'system' &&
            window.matchMedia('(prefers-color-scheme: dark)').matches);

    try {
        // 查找所有需要渲染的Pintora元素
        const pintoraElements = container.querySelectorAll('.pintora-container:not(.loaded)');
        console.log(`尝试渲染 ${pintoraElements.length} 个Pintora图表，当前重试次数: ${retryCount}`);

        if (pintoraElements.length === 0 && retryCount === 0) {
            const allPintoraElements = container.querySelectorAll('.pintora-container');
            if (allPintoraElements.length > 0) {
                console.log(`未找到未加载的图表，存在 ${allPintoraElements.length} 个已加载图表`);
            }
            return;
        }

        let renderPromises = [];

        for (const element of pintoraElements) {
            const id = element.getAttribute('data-render-id');
            const encodedContent = element.getAttribute('data-render-content');
            const lastRenderedContent = element.getAttribute('data-last-rendered');

            // 跳过内容未变化的图表渲染
            if (encodedContent && lastRenderedContent && encodedContent === lastRenderedContent) {
                console.log(`跳过图表 ID: ${id} 的渲染，内容未变化`);
                continue;
            }

            if (encodedContent && id) {
                let content = '';
                try {
                    // 清空现有内容
                    element.innerHTML = '<div class="pintora-loading">Pintora图表渲染中...</div>';

                    // 解码内容
                    content = decodeURIComponent(encodedContent);

                    renderPromises.push(
                        Promise.resolve().then(async () => {
                            if (typeof content === 'string' && content.length > 0) {
                                return new Promise<boolean>((resolve) => {
                                    try {
                                        let renderContainer = document.createElement('div');
                                        // 修改: 使用onError回调处理渲染错误
                                        pintoraStandalone.renderTo(content, {
                                            container: renderContainer,
                                            config: {
                                                themeConfig: {
                                                    theme: isDark ? 'dark' : 'default'
                                                }
                                            },
                                            onError: (renderError: any) => {
                                                console.error(`单个图表渲染失败 ID ${id}:`, renderError);
                                                element.innerHTML = `
                                                    <div class="pintora-error">
                                                        <p>Pintora图表渲染失败</p>
                                                        <pre class="error-message">${renderError.message}</pre>
                                                        <div class="pintora-source">
                                                        <details>
                                                            <summary>查看原始图表代码</summary>
                                                            <div class="code-container">
                                                            <pre class="code-content">${content}</pre>
                                                            </div>
                                                        </details>
                                                        </div>
                                                    </div>
                                                `;
                                                // 仍然显示错误界面，但不将其标记为已加载
                                                resolve(false);
                                            }
                                        });

                                        // 渲染成功
                                        const renderContent = document.createElement('div');
                                        renderContent.className = 'pintora-rendered-content';
                                        renderContent.style.overflowX = 'auto';
                                        renderContent.style.maxWidth = '100%';
                                        renderContent.style.textAlign = 'center';
                                        
                                        const pintoraDiv = document.createElement('div');
                                        pintoraDiv.className = 'pintora';
                                        pintoraDiv.innerHTML = renderContainer.innerHTML;
                                        
                                        renderContent.appendChild(pintoraDiv);
                                        element.innerHTML = '';
                                        element.appendChild(renderContent);
                                        
                                        // 添加图表加载完成的标记
                                        element.classList.add('loaded');
                                        // 记录已渲染的内容
                                        element.setAttribute('data-last-rendered', encodedContent);
                                        resolve(true);
                                    } catch (error) {
                                        console.error(`渲染图表过程发生意外错误 ID ${id}:`, error);
                                        element.innerHTML = `
                                            <div class="pintora-error">
                                                <p>Pintora图表渲染失败</p>
                                                <pre class="error-message">${error instanceof Error ? error.message : String(error)}</pre>
                                                <div class="pintora-source">
                                                <details>
                                                    <summary>查看原始图表代码</summary>
                                                    <div class="code-container">
                                                    <pre class="code-content">${content}</pre>
                                                    </div>
                                                </details>
                                                </div>
                                            </div>
                                        `;
                                        resolve(false);
                                    }
                                });
                            } else {
                                throw new Error("解码后的内容为空或无效。");
                            }
                        })
                    );
                } catch (error) {
                    console.error(`渲染图表 ID ${id} 失败:`, error);
                    console.error("失败的内容 (decoded):", content);
                    element.innerHTML = `
                        <div class="pintora-error">
                            <p>Pintora图表渲染失败</p>
                            <pre class="error-message">${error}</pre>
                            <div class="pintora-source">
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
                console.warn("发现缺少必要属性（id 或 content）的 Pintora 容器。", element);
            }
        }

        // 等待所有渲染完成
        if (renderPromises.length > 0) {
            const results = await Promise.all(renderPromises);
            const failedCount = results.filter(success => !success).length;

            // 如果有失败的图表，且未超过最大重试次数，则重试
            if (failedCount > 0 && retryCount < maxRetries) {
                console.log(`${failedCount}个图表渲染失败，将在1.5秒后重试 (${retryCount + 1}/${maxRetries})`);
                setTimeout(() => renderPintoraDiagrams(retryCount + 1, maxRetries, container), 1500);
            } else if (failedCount > 0) {
                console.log(`渲染完成，但有${failedCount}个图表渲染失败，已达到最大重试次数`);
                // 为失败的图表添加重试按钮事件监听
                setupAllPintoraInteractions(container);
            } else {
                console.log('所有图表渲染成功');
                // 设置图表的可点击功能
                setupAllPintoraInteractions(container);
            }

            // 无论成功失败，都延迟再次调用一次以确保所有图表都得到正确处理
            setTimeout(() => {
                console.log('延迟检查，确保所有图表事件绑定正确');
                setupAllPintoraInteractions(container);
            }, 500);
        } else {
            // 如果没有需要渲染的图表，也处理已渲染的图表
            setupAllPintoraInteractions(container);
        }
    } catch (error) {
        console.error("处理Pintora图表失败:", error);
        if (retryCount < maxRetries) {
            console.log(`整体处理失败，将在1.5秒后重试 (${retryCount + 1}/${maxRetries})`);
            setTimeout(() => renderPintoraDiagrams(retryCount + 1, maxRetries, container), 1500);
        } else {
            // 即使出错，也尝试为已渲染的图表添加交互功能
            setupAllPintoraInteractions(container);
        }
    }
}

// 统一处理所有Pintora图表的按钮和事件绑定
function setupAllPintoraInteractions(container: HTMLElement = document.body) {
    console.log(`开始设置Pintora图表交互功能，容器:`, container);

    // 使用更长的延迟确保DOM完全更新
    nextTick(() => {
        setTimeout(() => {
            console.log(`执行Pintora图表交互设置`);

            // 检查容器内的所有Pintora图表
            const pintoraContainers = container.querySelectorAll('.pintora-container');
            console.log(`找到 ${pintoraContainers.length} 个Pintora图表容器`);

            // 设置刷新按钮和交互功能
            setupPintoraRefresh(container);
            // 设置重试按钮事件
            setupRetryButtons(container);

            // 再次验证设置结果
            setTimeout(() => {
                const updatedContainers = container.querySelectorAll('.pintora-container');
                updatedContainers.forEach((pintoraContainer, index) => {
                    const hasRefreshBtn = pintoraContainer.querySelector('.refresh-diagram-button');
                    const hasZoomBtn = pintoraContainer.querySelector('.zoom-diagram-button');
                    const hasClickListener = pintoraContainer.hasAttribute('data-has-click-listener');
                    const id = pintoraContainer.getAttribute('data-render-id');

                    console.log(`Pintora图表 ${index + 1} (ID: ${id}) - 刷新按钮: ${!!hasRefreshBtn}, 放大按钮: ${!!hasZoomBtn}, 点击监听: ${hasClickListener}`);
                });
            }, 100);
        }, 50);
    });
}

// 设置图表渲染失败后的重试按钮事件（已移除重试按钮，此函数暂时保留以避免调用错误）
function setupRetryButtons(_container: HTMLElement = document.body) {
    // 重试按钮已被移除，此函数不再执行任何操作
    console.log('重试按钮已被移除，setupRetryButtons 函数不再执行操作');
}

// 设置刷新按钮函数
function setupPintoraRefresh(container: HTMLElement = document.body) {
    nextTick(() => {
        // 为所有图表容器添加刷新按钮
        const pintoraContainers = container.querySelectorAll('.pintora-container');
        console.log(`setupPintoraRefresh: 找到 ${pintoraContainers.length} 个 Pintora 容器`);
        
        pintoraContainers.forEach((diagramContainer, index) => {
            const pintoraContainer = diagramContainer as HTMLElement;
            const containerId = pintoraContainer.getAttribute('data-render-id');
            
            console.log(`处理容器 ${index + 1}/${pintoraContainers.length}, ID: ${containerId}`);

            // 检查容器是否已经有刷新按钮
            if (!pintoraContainer.querySelector('.refresh-diagram-button')) {
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
                    const clickedContainer = targetButton.closest('.pintora-container');

                    if (clickedContainer) {
                        // 移除loaded类以便重新渲染
                        clickedContainer.classList.remove('loaded');
                        // 清除上次渲染的内容记录，强制重新渲染
                        clickedContainer.removeAttribute('data-last-rendered');
                        targetButton.classList.add('refreshing');
                        AppEvents.showNotification("正在刷新图表...", "info");

                        // 延迟后渲染以确保UI更新
                        setTimeout(async () => {
                            await renderPintoraDiagrams(0, 3, container);
                            targetButton.classList.remove('refreshing');
                        }, 100);
                    }
                });

                // 将按钮添加到容器中
                pintoraContainer.appendChild(refreshButton);
                console.log(`已为容器 ${containerId} 添加刷新按钮`);
            }

            // 检查图表是否渲染成功
            const hasLoadedClass = pintoraContainer.classList.contains('loaded');
            const hasErrorElement = pintoraContainer.querySelector('.pintora-error');
            const hasSvgElement = pintoraContainer.querySelector('svg');
            const isRenderedSuccessfully = hasLoadedClass && !hasErrorElement && hasSvgElement;

            console.log(`容器 ${containerId} 状态检查 - loaded: ${hasLoadedClass}, hasError: ${!!hasErrorElement}, hasSvg: ${!!hasSvgElement}, success: ${isRenderedSuccessfully}`);

            // 只有成功渲染的图表才添加放大按钮
            if (isRenderedSuccessfully && !pintoraContainer.querySelector('.zoom-diagram-button')) {
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

                    const clickedContainer = (e.currentTarget as HTMLElement).closest('.pintora-container') as HTMLElement;
                    if (clickedContainer) {
                        const svgElement = clickedContainer.querySelector('svg');
                        const contentElement = clickedContainer.getAttribute('data-render-content');

                        if (svgElement && contentElement) {
                            const svgContent = svgElement.outerHTML;
                            const diagramContent = decodeURIComponent(contentElement);
                            AppEvents.openChartViewer(svgContent, diagramContent);
                        }
                    }
                });

                pintoraContainer.appendChild(zoomButton);
                console.log(`已为容器 ${containerId} 添加放大按钮`);
            } else if (!isRenderedSuccessfully && pintoraContainer.querySelector('.zoom-diagram-button')) {
                // 如果图表渲染失败，但之前添加了放大按钮，则移除它
                const zoomButton = pintoraContainer.querySelector('.zoom-diagram-button');
                if (zoomButton) {
                    zoomButton.remove();
                    console.log(`已移除失败图表的放大按钮 - ID: ${containerId}`);
                }
            }

            // 只为成功渲染的图表添加点击事件
            if (isRenderedSuccessfully) {
                // 为整个容器添加点击事件以打开查看器
                if (!pintoraContainer.hasAttribute('data-has-click-listener')) {
                    pintoraContainer.setAttribute('data-has-click-listener', 'true');

                    pintoraContainer.addEventListener('click', (e) => {
                        // 点击按钮时不触发
                        if ((e.target as HTMLElement).closest('.refresh-diagram-button, .zoom-diagram-button')) {
                            return;
                        }

                        const svgElement = pintoraContainer.querySelector('svg');
                        const contentElement = pintoraContainer.getAttribute('data-render-content');

                        if (svgElement && contentElement) {
                            const svgContent = svgElement.outerHTML;
                            const diagramContent = decodeURIComponent(contentElement);
                            AppEvents.openChartViewer(svgContent, diagramContent);
                        }
                    });

                    // 添加视觉提示，表明容器可点击
                    pintoraContainer.classList.add('clickable-container');
                    console.log(`已为容器 ${containerId} 添加点击事件`);
                }
            } else {
                // 如果图表渲染失败，移除点击相关的类和属性
                pintoraContainer.classList.remove('clickable-container');
                pintoraContainer.removeAttribute('data-has-click-listener');
                console.log(`已移除失败图表的点击功能 - ID: ${containerId}`);
            }
        });
    });
}

// 更改Pintora主题
function changePintoraTheme(theme: string) {
    if (pintoraStandalone) {
        // 修复：使用 setConfig 而不是不存在的 updateConfig
        pintoraStandalone.setConfig({
            themeConfig: {
                theme: theme === 'dark' ? 'dark' : 'default'
            }
        });
    }
}

/**
 * 处理 Pintora 渲染 API 调用
 * @param apiInfo API 调用信息
 * @returns 生成的 HTML 内容
 */
async function handlePintoraRender(apiInfo: any): Promise<string> {
    // 获取 Pintora 代码参数
    const pintoraCode = apiInfo.arguments.diagram || '';
    console.log("处理 pintora_render:", pintoraCode);

    // 创建唯一的渲染 ID
    const renderId = `pintora-render-${Date.now()}-${Math.floor(Math.random() * 10000)}`;

    // 编码内容，以便在属性中安全存储
    const encodedContent = encodeURIComponent(pintoraCode);

    // 准备初始内容
    let initialContent = '<div class="pintora-loading">Pintora图表加载中...</div>';
    let isLoaded = false;

    // 如果不是流式传输，则立即渲染图表
    if (!isStreaming.value && pintoraCode) {
        try {
            // 确保Pintora已初始化
            const isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
                (document.documentElement.getAttribute('data-theme') === 'system' &&
                    window.matchMedia('(prefers-color-scheme: dark)').matches);

            // 立即渲染图表
            let container: HTMLElement = document.createElement('div');
            let renderSuccess = true;
            
            // 修改: 使用onError回调处理渲染错误
            pintoraStandalone.renderTo(pintoraCode, {
                container,
                config: {
                    themeConfig: {
                        theme: isDark ? 'dark' : 'default'
                    }
                },
                onError: (error: any) => {
                    console.error("Pintora 渲染失败:", error);
                    initialContent = `
                        <div class="pintora-error">
                            <p>Pintora图表渲染失败</p>
                            <pre class="error-message">${error.message}</pre>
                            <div class="pintora-source">
                            <details>
                                <summary>查看原始图表代码</summary>
                                <div class="code-container">
                                <pre class="code-content">${pintoraCode}</pre>
                                </div>
                            </details>
                            </div>
                        </div>
                    `;
                    renderSuccess = false;
                }
            });
            
            if (renderSuccess) {
                initialContent = `<div class="pintora-rendered-content" style="overflow-x: auto; max-width: 100%; text-align: center;">
                    <div class="pintora">${container.innerHTML}</div>
                </div>`;
                isLoaded = true;
            }
        } catch (error) {
            console.error("Pintora 渲染过程发生意外错误:", error);
            initialContent = `
                <div class="pintora-error">
                    <p>Pintora图表渲染失败</p>
                    <pre class="error-message">${error instanceof Error ? error.message : String(error)}</pre>
                    <div class="pintora-source">
                    <details>
                        <summary>查看原始图表代码</summary>
                        <div class="code-container">
                        <pre class="code-content">${pintoraCode}</pre>
                        </div>
                    </details>
                    </div>
                </div>
            `;
        }
    }

    // 构建HTML
    const html = `
    <div class="special-api-call pintora-api-call" id="${renderId}-container">
        <div class="api-call-header">
            <span class="api-call-icon">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                    <line x1="3" y1="9" x2="21" y2="9"></line>
                    <line x1="9" y1="21" x2="9" y2="9"></line>
                </svg>
            </span>
            <span class="api-call-title">Pintora 图表</span>
        </div>
        <div class="pintora-container ${isLoaded ? 'loaded' : ''}" data-render-id="${renderId}" data-render-content="${encodedContent}" ${isLoaded ? `data-last-rendered="${encodedContent}"` : ''}>
            ${initialContent}
        </div>
        <div class="api-call-footer">
            <details>
                <summary>查看源代码</summary>
                <pre class="api-call-code"><code class="language-pintora">${pintoraCode.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
            </details>
        </div>
    </div>
    `;

    // 非流式传输下，为刚渲染的图表绑定事件
    if (!isStreaming.value) {
        setTimeout(() => {
            const container = document.getElementById(`${renderId}-container`);
            if (container) {
                console.log(`立即为非流式图表绑定事件 - ID: ${renderId}, 渲染成功: ${isLoaded}`);
                // 统一处理图表按钮和事件绑定
                setupAllPintoraInteractions(container);

                // 再次延迟确保绑定成功
                setTimeout(() => {
                    console.log(`延迟再次检查图表事件绑定 - ID: ${renderId}`);
                    setupAllPintoraInteractions(container);
                }, 200);
            }
        }, 0);  // 使用setTimeout确保HTML先被添加到DOM
    }

    return html;
}

// 导出函数
export { initPintora, renderPintoraDiagrams, setupPintoraRefresh, setupRetryButtons, setupAllPintoraInteractions, changePintoraTheme, handlePintoraRender };

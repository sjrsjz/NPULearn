/**
 * HTML 内容渲染器
 * 参考 mermaidRenderer.ts 的实现方式，确保 HTML 渲染与其他渲染器保持一致
 */
import { nextTick } from "vue";
import { AppEvents } from '../eventBus';

/**
 * 渲染所有 HTML 容器
 * 无条件强制渲染所有HTML容器，不使用缓存机制
 * @param container 容器元素，默认为document.body
 */
async function renderHTMLContainers(container: HTMLElement = document.body): Promise<void> {
    try {
        // 查找所有HTML元素，无论是否已加载
        const htmlElements = container.querySelectorAll('.html-container');
        console.log(`强制渲染 ${htmlElements.length} 个HTML内容`);

        if (htmlElements.length === 0) {
            console.log('未找到HTML容器');
            return;
        }

        let renderPromises = [];

        for (const element of htmlElements) {
            // 重置加载状态，确保强制重新渲染
            element.classList.remove('loaded');
            element.removeAttribute('data-last-rendered');

            const encodedContent = element.getAttribute('data-html-content');

            if (encodedContent) {
                let content = '';
                try {
                    // 清空现有内容或显示加载状态
                    const contentElement = element.querySelector('.html-content');
                    if (contentElement) {
                        contentElement.innerHTML = '<div class="html-loading">HTML内容加载中...</div>';
                    }

                    // 解码内容
                    content = decodeURIComponent(encodedContent);

                    renderPromises.push(
                        Promise.resolve().then(() => {
                            if (typeof content === 'string' && content.length > 0) {
                                try {
                                    // 找到内容容器
                                    const contentElement = element.querySelector('.html-content');
                                    if (contentElement) {
                                        // 清除现有内容
                                        contentElement.innerHTML = '';
                                        
                                        // 创建iframe进行隔离渲染
                                        const iframe = document.createElement('iframe');
                                        iframe.className = 'html-iframe';
                                        
                                        // 只允许脚本，不允许同源，提高安全性
                                        iframe.setAttribute('sandbox', 'allow-scripts');
                                        
                                        iframe.style.width = '100%';
                                        iframe.style.border = 'none';
                                        iframe.style.overflow = 'hidden'; // 防止出现滚动条
                                        iframe.title = '隔离的HTML内容';
                                        
                                        // 添加iframe到容器
                                        contentElement.appendChild(iframe);

                                        // 检测输入内容是否为HTML
                                        const isHTML = /<[a-z][\s\S]*>/i.test(content);
                                        
                                        // 为非HTML内容构建基本HTML结构
                                        if (!isHTML) {
                                            content = `
                                                <div style="white-space: pre-wrap; font-family: system-ui, -apple-system, sans-serif;">
                                                    ${content}
                                                </div>
                                            `;
                                        }
                                        
                                        // 构建完整的HTML文档
                                        const htmlDoc = `
                                            <!DOCTYPE html>
                                            <html>
                                            <head>
                                                <meta charset="UTF-8">
                                                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                                                <style>
                                                    body {
                                                        margin: 0;
                                                        padding: 10px;
                                                        font-family: system-ui, -apple-system, sans-serif;
                                                        color: #333;
                                                        background-color: transparent;
                                                    }
                                                    
                                                    img, video {
                                                        max-width: 100%;
                                                        height: auto;
                                                    }
                                                    
                                                    pre {
                                                        overflow-x: auto;
                                                        background-color: #f5f5f5;
                                                        padding: 10px;
                                                        border-radius: 5px;
                                                    }
                                                </style>
                                            </head>
                                            <body>${content}</body>
                                            </html>
                                        `;
                                        
                                        // 使用srcdoc属性
                                        iframe.srcdoc = htmlDoc;
                                        
                                        // 调整iframe高度以适应内容
                                        iframe.onload = () => {
                                            try {
                                                const resizeObserver = new ResizeObserver(() => {
                                                    if (iframe.contentWindow && iframe.contentDocument?.body) {
                                                        const height = iframe.contentDocument.body.scrollHeight;
                                                        iframe.style.height = (height + 20) + 'px'; // 添加一些额外空间
                                                    }
                                                });
                                                
                                                if (iframe.contentDocument?.body) {
                                                    // 初始设置高度
                                                    iframe.style.height = (iframe.contentDocument.body.scrollHeight + 20) + 'px';
                                                    // 观察高度变化
                                                    resizeObserver.observe(iframe.contentDocument.body);
                                                    
                                                    // 添加事件监听器检测内容变化
                                                    const mutationObserver = new MutationObserver(() => {
                                                        if (iframe.contentDocument?.body) {
                                                            iframe.style.height = (iframe.contentDocument.body.scrollHeight + 20) + 'px';
                                                        }
                                                    });
                                                    
                                                    mutationObserver.observe(iframe.contentDocument.body, {
                                                        childList: true,
                                                        subtree: true
                                                    });
                                                }
                                            } catch (resizeError) {
                                                console.error("设置iframe高度失败:", resizeError);
                                            }
                                        };
                                    }

                                    // 添加加载完成标记
                                    element.classList.add('loaded');
                                    
                                    return true;
                                } catch (renderError) {
                                    console.error('HTML内容渲染失败:', renderError);
                                    const contentElement = element.querySelector('.html-content');
                                    if (contentElement) {
                                        contentElement.innerHTML = `
                                            <div class="html-error">
                                                <p>HTML内容渲染失败</p>
                                                <pre class="error-message">${renderError}</pre>
                                            </div>
                                        `;
                                    }
                                    return false;
                                }
                            } else {
                                throw new Error("解码后的HTML内容为空或无效");
                            }
                        })
                    );
                } catch (error) {
                    console.error('解码HTML内容失败:', error);
                    const contentElement = element.querySelector('.html-content');
                    if (contentElement) {
                        contentElement.innerHTML = `
                            <div class="html-error">
                                <p>HTML内容解析失败</p>
                                <pre class="error-message">${error}</pre>
                            </div>
                        `;
                    }
                }
            } else {
                console.warn("发现缺少必要属性的HTML容器", element);
            }
        }

        // 等待所有渲染完成
        if (renderPromises.length > 0) {
            await Promise.all(renderPromises);
            console.log('所有HTML内容渲染完成');
        }
        
        // 设置刷新按钮
        setupHTMLRefresh(container);
        
    } catch (error) {
        console.error("处理HTML内容失败:", error);
        setupHTMLRefresh(container);
    }
}

/**
 * 设置HTML容器的刷新按钮
 * @param container 容器元素，默认为document.body
 */
function setupHTMLRefresh(container: HTMLElement = document.body): void {
    nextTick(() => {
        container.querySelectorAll('.html-container').forEach(htmlContainer => {
            // 检查容器是否已经有刷新按钮
            if (!htmlContainer.querySelector('.refresh-html-button')) {
                const refreshButton = document.createElement('button');
                refreshButton.className = 'refresh-html-button';
                refreshButton.innerHTML = `
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M23 4v6h-6"></path>
                    <path d="M1 20v-6h6"></path>
                    <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10"></path>
                    <path d="M20.49 15a9 9 0 0 1-14.85 3.36L1 14"></path>
                </svg>
                `;
                refreshButton.title = "刷新HTML内容";

                refreshButton.addEventListener('click', async (e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    const targetButton = e.currentTarget as HTMLElement;
                    targetButton.classList.add('refreshing');
                    AppEvents.showNotification("正在刷新HTML内容...", "info");

                    // 延迟后渲染以确保UI更新
                    setTimeout(async () => {
                        await renderHTMLContainers(container);
                        targetButton.classList.remove('refreshing');
                    }, 100);
                });

                // 将按钮添加到容器中
                htmlContainer.appendChild(refreshButton);
            }
        });
    });
}

/**
 * 处理 html_render API 调用，生成初始 HTML 容器结构
 * @param apiInfo API 调用信息
 * @returns 生成的 HTML 容器结构
 */
function handleHTMLRender(apiInfo: any): string {
    // 获取参数（可能是 html 或 html_content）
    const htmlContent = apiInfo.arguments.html || '<p>无内容</p>';
    const title = apiInfo.arguments.title || 'HTML 内容';
    const width = apiInfo.arguments.width || '100%';
    const height = apiInfo.arguments.height || 'auto';

    // 处理混合内容，检查是否包含样式或代码片段
    // 这段代码允许用户输入不完整的HTML或混合内容，如纯文本和CSS
    let processedContent = htmlContent;
    
    processedContent = htmlContent

    console.log("处理 html_render:", processedContent.substring(0, 50) + (processedContent.length > 50 ? '...' : ''));

    // 对 HTML 内容进行编码，以便安全地存储在 data-* 属性中
    const encodedContent = encodeURIComponent(processedContent);

    // 构建 HTML 容器结构
    return `
    <div class="special-api-call html-container" data-html-content="${encodedContent}" data-width="${width}" data-height="${height}">
      <div class="api-call-header">
        <span class="api-call-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="16 18 22 12 16 6"></polyline>
            <polyline points="8 6 2 12 8 18"></polyline>
          </svg>
        </span>
        <span class="api-call-title">${title}</span>
      </div>
      <div class="html-content-wrapper">
        <div class="html-content"></div>
      </div>
      <div class="api-call-footer">
        <details>
          <summary>查看原始HTML</summary>
          <pre class="api-call-code"><code>${htmlContent.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
        </details>
      </div>
    </div>
  `;
}

export { handleHTMLRender, renderHTMLContainers, setupHTMLRefresh };
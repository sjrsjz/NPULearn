/**
 * HTML 内容渲染器
 * 参考 mermaidRenderer.ts 的实现方式，确保 HTML 渲染与其他渲染器保持一致
 */
import { nextTick } from "vue";
import { AppEvents, isStreaming } from '../eventBus';

/**
 * 渲染所有 HTML 容器
 * 无条件强制渲染所有HTML容器，不使用缓存机制
 * @param container 容器元素，默认为document.body
 */
async function renderHTMLContainers(container: HTMLElement): Promise<void> {
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
                    iframe.setAttribute('sandbox', 'allow-scripts');                    iframe.style.width = '100%';
                    iframe.style.border = 'none';
                    iframe.style.overflow = 'auto'; // 允许滚动条（垂直和水平）
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

                    // 构建完整的HTML文档，增强样式隔离
                    const htmlDoc = `
                                            <!DOCTYPE html>
                                            <html>
                                            <head>
                                                <meta charset="UTF-8">
                                                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                                                <base target="_blank">                                                <style>
                                                    /* 重置所有样式，智能处理溢出 */
                                                    html, body {
                                                        margin: 0;
                                                        padding: 10px;
                                                        font-family: system-ui, -apple-system, sans-serif;
                                                        color: #333;
                                                        background-color: transparent;
                                                        width: 100%;
                                                        height: auto;
                                                        overflow: visible; /* 允许内容正常溢出，由iframe处理滚动 */
                                                    }
                                                    
                                                    /* 元素基础样式 */
                                                    * {
                                                        box-sizing: border-box;
                                                    }
                                                    
                                                    /* 图片和视频响应式 */
                                                    img, video {
                                                        max-width: 100%;
                                                        height: auto;
                                                    }
                                                    
                                                    /* 代码块样式 */
                                                    pre {
                                                        overflow-x: auto;
                                                        background-color: #f5f5f5;
                                                        padding: 10px;
                                                        border-radius: 5px;
                                                        white-space: pre;
                                                        word-wrap: normal;
                                                    }
                                                    
                                                    /* 表格样式 */
                                                    table {
                                                        overflow-x: auto;
                                                        border-collapse: collapse;
                                                        width: auto;
                                                        min-width: 100%;
                                                    }
                                                    
                                                    /* 宽内容处理 */
                                                    .wide-content {
                                                        overflow-x: auto;
                                                    }
                                                </style>
                                            </head>
                                            <body>${content}</body>
                                            </html>
                                        `;

                    // 使用srcdoc属性
                    iframe.srcdoc = htmlDoc;                    // 调整iframe高度以适应内容，支持水平滚动
                    iframe.onload = () => {
                      try {                        // 设置初始高度和滚动处理
                        const updateIframeSize = () => {
                          if (iframe.contentWindow && iframe.contentDocument?.body) {
                            // 获取内容的实际高度和宽度
                            const bodyHeight = iframe.contentDocument.body.scrollHeight;
                            const bodyWidth = iframe.contentDocument.body.scrollWidth;
                            
                            // 设置最大高度限制（比如500px），超过则显示滚动条
                            const maxHeight = 1024;
                            const minHeight = 200;
                            
                            if (bodyHeight <= maxHeight) {
                              // 内容不高，设置为实际高度加padding
                              iframe.style.height = Math.max(bodyHeight + 50, minHeight) + 'px';
                              iframe.style.overflowY = 'hidden';
                            } else {
                              // 内容太高，设置固定高度并显示滚动条
                              iframe.style.height = maxHeight + 'px';
                              iframe.style.overflowY = 'auto';
                            }
                            
                            // 水平滚动处理
                            if (bodyWidth > iframe.offsetWidth) {
                              iframe.style.overflowX = 'auto';
                            } else {
                              iframe.style.overflowX = 'hidden';
                            }
                          }
                        };

                        const resizeObserver = new ResizeObserver(() => {
                          updateIframeSize();
                        });

                        if (iframe.contentDocument?.body) {
                          // 初始设置
                          updateIframeSize();
                          
                          // 观察高度变化
                          resizeObserver.observe(iframe.contentDocument.body);

                          // 添加事件监听器检测内容变化
                          const mutationObserver = new MutationObserver(() => {
                            updateIframeSize();
                          });

                          mutationObserver.observe(iframe.contentDocument.body, {
                            childList: true,
                            subtree: true,
                            attributes: true,
                            attributeOldValue: true
                          });
                          
                          // 监听图片和其他资源加载完成
                          iframe.contentDocument.addEventListener('load', updateIframeSize, true);
                          
                          // 延迟确保所有内容都已渲染
                          setTimeout(updateIframeSize, 500);
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
async function handleHTMLRender(apiInfo: any) {
  // 获取参数（可能是 html 或 html_content）
  const htmlContent = apiInfo.arguments.html || '<p>无内容</p>';
  const title = apiInfo.arguments.title || 'HTML 内容';
  const width = apiInfo.arguments.width || '100%';
  const height = apiInfo.arguments.height || 'auto';

  // 处理混合内容，检查是否包含样式或代码片段
  let processedContent = htmlContent;

  console.log("处理 html_render:", processedContent.substring(0, 50) + (processedContent.length > 50 ? '...' : ''));

  // 对 HTML 内容进行编码，以便安全地存储在 data-* 属性中
  const encodedContent = encodeURIComponent(processedContent);

  // 基本HTML容器结构开始
  let htmlStructure = `
    <div class="special-api-call html-container ${!isStreaming.value ? 'loaded' : ''}" data-html-content="${encodedContent}" data-width="${width}" data-height="${height}">
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
        <div class="html-content">`;

  if (isStreaming.value) {
    // 流式传输模式：添加加载占位符
    htmlStructure += `<div class="html-loading">HTML内容加载中...（流式传输完成后将自动渲染）</div>`;
  } else {
    // 非流式传输模式：直接渲染内容
    // 检测输入内容是否为HTML
    const isHTML = /<[a-z][\s\S]*>/i.test(processedContent);

    // 为非HTML内容构建基本HTML结构
    if (!isHTML) {
      processedContent = `
                <div style="white-space: pre-wrap; font-family: system-ui, -apple-system, sans-serif;">
                    ${processedContent}
                </div>
            `;
    }

    // 构建完整的HTML文档，增强样式隔离
    const htmlDoc = `
            <!DOCTYPE html>
            <html>
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <base target="_blank">                <style>
                    /* 重置所有样式，智能处理溢出 */
                    html, body {
                        margin: 0;
                        padding: 10px;
                        font-family: system-ui, -apple-system, sans-serif;
                        color: #333;
                        background-color: white;
                        width: 100%;
                        height: auto;
                        overflow: visible; /* 允许内容正常溢出，由iframe处理滚动 */
                    }
                    
                    /* 元素基础样式 */
                    * {
                        box-sizing: border-box;
                    }
                    
                    /* 图片和视频响应式 */
                    img, video {
                        max-width: 100%;
                        height: auto;
                    }
                    
                    /* 代码块样式 */
                    pre {
                        overflow-x: auto;
                        background-color: #f5f5f5;
                        padding: 10px;
                        border-radius: 5px;
                        white-space: pre;
                        word-wrap: normal;
                    }
                    
                    /* 表格样式 */
                    table {
                        overflow-x: auto;
                        border-collapse: collapse;
                        width: auto;
                        min-width: 100%;
                    }
                    
                    /* 宽内容处理 */
                    .wide-content {
                        overflow-x: auto;
                    }
                </style>
            </head>
            <body>${processedContent}</body>
            </html>
        `;    // 添加iframe以渲染HTML内容
    htmlStructure += `<iframe class="html-iframe" sandbox="allow-scripts" style="width: 100%; border: none; overflow: auto;" title="隔离的HTML内容" srcdoc="${htmlDoc.replace(/"/g, '&quot;')}" onload="setTimeout(() => {
    try {
        if (this.contentDocument && this.contentDocument.body) {            // 设置高度和滚动处理的函数
            const updateSize = () => {
                const bodyHeight = this.contentDocument.body.scrollHeight;
                const bodyWidth = this.contentDocument.body.scrollWidth;
                
                // 设置最大高度限制，超过则显示滚动条
                const maxHeight = 1000;
                const minHeight = 200;
                
                if (bodyHeight <= maxHeight) {
                    // 内容不高，设置为实际高度加padding
                    this.style.height = Math.max(bodyHeight + 50, minHeight) + 'px';
                    this.style.overflowY = 'hidden';
                } else {
                    // 内容太高，设置固定高度并显示滚动条
                    this.style.height = maxHeight + 'px';
                    this.style.overflowY = 'auto';
                }
                
                // 水平滚动处理
                if (bodyWidth > this.offsetWidth) {
                    this.style.overflowX = 'auto';
                } else {
                    this.style.overflowX = 'hidden';
                }
            };
            
            // 初始设置
            updateSize();
            
            const resizeObserver = new ResizeObserver(updateSize);
            resizeObserver.observe(this.contentDocument.body);
            
            const mutationObserver = new MutationObserver(updateSize);
            mutationObserver.observe(this.contentDocument.body, {
                childList: true,
                subtree: true,
                attributes: true,
                attributeOldValue: true
            });
            
            // 监听图片加载
            this.contentDocument.addEventListener('load', function(e) {
                if (e.target instanceof HTMLImageElement) {
                    setTimeout(updateSize, 100);
                }
            }, true);
            
            // 延迟再次设置，确保所有内容都已渲染
            setTimeout(updateSize, 500);
        }
    } catch (error) {
        console.error('设置iframe高度失败:', error);
    }
}, 100);"></iframe>`;
  }

  // 完成HTML结构
  htmlStructure += `
        </div>
      </div>
      <div class="api-call-footer">
        <details>
          <summary>查看原始HTML</summary>
          <pre class="api-call-code"><code>${htmlContent.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
        </details>
      </div>
    </div>
    `;

  return htmlStructure;
}

export { handleHTMLRender, renderHTMLContainers, setupHTMLRefresh };
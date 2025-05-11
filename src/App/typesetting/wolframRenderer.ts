import { invoke } from '@tauri-apps/api/core';
import { AppEvents } from '../eventBus';

// 缓存机制，避免重复渲染时多次调用API
const wolframCache = new Map<string, any>();

/**
 * 处理wolfram_alpha_compute API调用
 * @param apiInfo API调用信息
 * @returns 生成的HTML内容
 */
export async function handleWolframRender(apiInfo: any): Promise<string | null> {
    try {
        // 获取参数
        const query = apiInfo.arguments.query || '';
        const imageOnly = apiInfo.arguments.image_only === 'True' || 
                         apiInfo.arguments.image_only === true || 
                         false;
        const format = apiInfo.arguments.format || 'html'; // 默认使用HTML格式
        
        // 早期返回，避免无意义的API调用
        if (!query.trim()) {
            return createWolframErrorView('查询内容不能为空');
        }

        // 创建缓存键
        const cacheKey = `${query}-${imageOnly}-${format}`;
        
        // 检查缓存
        if (wolframCache.has(cacheKey)) {
            console.log('使用缓存的Wolfram Alpha结果');
            const cachedResult = wolframCache.get(cacheKey);
            return createWolframResultView(cachedResult, query, format);
        }

        // 创建唯一的容器ID
        const containerId = `wolfram-container-${Date.now()}-${Math.floor(Math.random() * 10000)}`;
        
        // 创建加载视图
        const loadingViewHtml = createWolframLoadingView(query, containerId);
        
        // 异步调用后端进行Wolfram Alpha计算
        setTimeout(async () => {
            try {
                // 直接使用containerId查找容器元素
                const container = document.getElementById(containerId);
                
                if (!container) {
                    console.error('找不到Wolfram Alpha加载容器:', containerId);
                    return;
                }
                
                // 调用后端API
                const result = await invoke<any>('wolfram_alpha_compute', {
                    query,
                    imageOnly,
                    format
                });
                
                // 缓存结果
                wolframCache.set(cacheKey, result);
                
                // 更新UI
                const resultHtml = createWolframResultView(result, query, format);
                container.innerHTML = resultHtml;
            } catch (error) {
                console.error('Wolfram Alpha API调用失败:', error);
                
                // 更新UI显示错误 - 直接使用containerId
                const errorHtml = createWolframErrorView(
                    error instanceof Error ? error.message : '调用Wolfram Alpha API失败'
                );
                
                const container = document.getElementById(containerId);
                if (container) {
                    container.innerHTML = errorHtml;
                }
                
                AppEvents.showNotification('Wolfram Alpha计算失败', 'error');
            }
        }, 100);
        
        // 立即返回加载视图
        return loadingViewHtml;
        
    } catch (error) {
        console.error('处理Wolfram Alpha渲染失败:', error);
        return createWolframErrorView(
            error instanceof Error ? error.message : '处理Wolfram Alpha渲染失败'
        );
    }
}

// 创建Wolfram Alpha加载视图
function createWolframLoadingView(query: string, containerId: string): string {
    return `
    <div class="special-api-call wolfram-api-call">
      <div class="api-call-header">
        <span class="api-call-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
        </span>
        <span class="api-call-title">Wolfram Alpha 计算</span>
      </div>
      <div id="${containerId}" class="wolfram-loading-container">
        <div class="wolfram-loading-indicator">
          <div class="wolfram-spinner"></div>
          <p>计算中: ${escapeHtml(query)}</p>
        </div>
      </div>
    </div>
  `;
}

// 创建Wolfram Alpha结果视图
function createWolframResultView(result: any, query: string, format: string): string {
    // 如果结果为空或出错
    if (!result || result.error) {
        return createWolframErrorView(result?.error || '没有找到结果');
    }
    
    // 获取结果内容
    let contentHtml = '';
    
    if (format === 'html' && result[0]?.plaintext) {
        // 处理HTML格式的结果
        const htmlContent = result[0].plaintext;
        
        // 创建一个临时容器来解析HTML
        const tempContainer = document.createElement('div');
        tempContainer.innerHTML = htmlContent;
        
        // 查找并处理所有相关查询区域
        const relatedQueriesElements = tempContainer.querySelectorAll('.wolfram-related-queries');
        relatedQueriesElements.forEach((relatedQueryElement, index) => {
            const listElement = relatedQueryElement.querySelector('ul');
            if (listElement) {
                // 为这个列表分配唯一ID
                const queryListId = `wolfram-queries-html-${Date.now()}-${index}-${Math.floor(Math.random() * 10000)}`;
                listElement.id = queryListId;
                
                // 处理所有列表项
                const items = listElement.querySelectorAll('li');
                items.forEach(item => {
                    // 提取文本内容作为查询
                    const queryText = item.textContent || '';
                    // 添加数据属性和标题
                    item.setAttribute('data-query', encodeURIComponent(queryText));
                    item.setAttribute('title', '点击发送此查询');
                });
                
                // 添加加载后的处理脚本
                setTimeout(() => {
                    const renderedList = document.getElementById(queryListId);
                    if (!renderedList) return;
                    
                    const renderedItems = renderedList.querySelectorAll('li');
                    renderedItems.forEach(item => {
                        if (!item.hasAttribute('data-event-added')) {
                            item.setAttribute('data-event-added', 'true');
                            item.addEventListener('click', function() {
                                const query = decodeURIComponent(this.getAttribute('data-query') || '');
                                if (!query) return;
                                
                                // 检查是否正在流式输出
                                if (document.body.classList.contains('streaming')) {
                                    AppEvents.showNotification("请等待当前消息输出完成", "error");
                                    return;
                                }
                                
                                // 发送消息
                                AppEvents.sendStreamMessageDirect("> wolfram alpha: " + query);
                                AppEvents.showNotification("已发送Wolfram Alpha查询", "success");
                            });
                        }
                    });
                }, 0);
            }
        });
        
        // 将处理后的HTML转回字符串
        contentHtml = tempContainer.innerHTML;
    } else if (Array.isArray(result)) {
        // 处理结果数组
        contentHtml = '<div class="wolfram-results">';
        
        for (const item of result) {
            if (typeof item === 'object') {
                contentHtml += '<div class="wolfram-result-item">';
                
                // 标题
                if (item.title) {
                    contentHtml += `<h3 class="wolfram-item-title">${escapeHtml(item.title)}</h3>`;
                }
                
                // 文本内容
                if (item.plaintext) {
                    contentHtml += `<p class="wolfram-item-text">${escapeHtml(item.plaintext)}</p>`;
                }
                
                // 图片
                if (item.img_base64) {
                    const contentType = item.img_contenttype || 'image/png';
                    contentHtml += `
                        <div class="wolfram-item-image">
                            <img src="data:${contentType};base64,${item.img_base64}" alt="Wolfram Alpha result" />
                        </div>
                    `;
                }
                
                // Mathematica 输入
                if (item.minput) {
                    contentHtml += `
                        <div class="wolfram-item-code">
                            <strong>Mathematica 输入:</strong> <code>${escapeHtml(item.minput)}</code>
                        </div>
                    `;
                }
                
                // Mathematica 输出
                if (item.moutput) {
                    contentHtml += `
                        <div class="wolfram-item-code">
                            <strong>Mathematica 输出:</strong> <code>${escapeHtml(item.moutput)}</code>
                        </div>
                    `;
                }
                
                // 相关查询 - 使用唯一ID来标识每个列表
                if (item.relatedQueries && Array.isArray(item.relatedQueries)) {
                    const queryListId = `wolfram-queries-${Date.now()}-${Math.floor(Math.random() * 10000)}`;
                    contentHtml += `
                        <div class="wolfram-related-queries">
                            <strong>相关查询:</strong>
                            <ul id="${queryListId}">
                                ${item.relatedQueries.map((q: string) => 
                                    `<li data-query="${encodeURIComponent(q)}" title="点击发送此查询">${escapeHtml(q)}</li>`
                                ).join('')}
                            </ul>
                        </div>
                    `;
                    
                    // 为当前列表添加加载完成后的事件处理
                    setTimeout(() => {
                        const queryList = document.getElementById(queryListId);
                        if (!queryList) return;
                        
                        const items = queryList.querySelectorAll('li');
                        items.forEach(item => {
                            item.addEventListener('click', function() {
                                const query = decodeURIComponent(this.getAttribute('data-query') || '');
                                if (!query) return;
                                
                                // 检查是否正在流式输出
                                if (document.body.classList.contains('streaming')) {
                                    AppEvents.showNotification("请等待当前消息输出完成", "error");
                                    return;
                                }
                                
                                // 发送消息
                                AppEvents.sendStreamMessageDirect("> wolfram alpha: " + query);
                                AppEvents.showNotification("已发送Wolfram Alpha查询", "success");
                            });
                        });
                    }, 0);
                }
                
                contentHtml += '</div><hr>';
            }
        }
        
        // 移除最后一个分隔线
        contentHtml = contentHtml.replace(/<hr>$/, '');
        contentHtml += '</div>';
    } else {
        // 其他格式或不支持的结果类型
        contentHtml = `<pre>${escapeHtml(JSON.stringify(result, null, 2))}</pre>`;
    }

    // 添加CSS样式
    addWolframQueryStyles();
    
    return `
    <div class="special-api-call wolfram-api-call">
      <div class="api-call-header">
        <span class="api-call-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"></path>
          </svg>
        </span>
        <span class="api-call-title">Wolfram Alpha 计算结果</span>
      </div>
      <div class="wolfram-result-container">
        <div class="wolfram-query">
          <strong>查询:</strong> ${escapeHtml(query)}
        </div>
        <div class="wolfram-content">
          ${contentHtml}
        </div>
      </div>
    </div>
  `;
}

// 创建Wolfram Alpha错误视图
function createWolframErrorView(errorMessage: string): string {
    return `
    <div class="special-api-call wolfram-api-call error">
      <div class="api-call-header">
        <span class="api-call-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
        </span>
        <span class="api-call-title">Wolfram Alpha 错误</span>
      </div>
      <div class="wolfram-error-container">
        <p class="wolfram-error-message">${escapeHtml(errorMessage)}</p>
      </div>
    </div>
  `;
}

// 添加Wolfram查询样式
function addWolframQueryStyles() {
    // 检查是否已添加样式
    if (document.getElementById('wolfram-query-styles')) return;
    
    const styleElement = document.createElement('style');
    styleElement.id = 'wolfram-query-styles';
    styleElement.textContent = `
        .wolfram-related-queries ul li {
            cursor: pointer;
            padding: 4px 8px;
            margin: 2px 0;
            border-radius: 4px;
            transition: background-color 0.2s;
        }
        
        .wolfram-related-queries ul li:hover {
            background-color: rgba(0, 0, 0, 0.05);
            color: #0066cc;
        }
        
        .wolfram-related-queries ul li::after {
            content: " →";
            opacity: 0;
            transition: opacity 0.2s;
        }
        
        .wolfram-related-queries ul li:hover::after {
            opacity: 0.8;
        }
    `;
    
    document.head.appendChild(styleElement);
}

// 声明全局类型
declare global {
    interface Window {
        hasWolframQueryHandler?: boolean;
        isStreamingMessage?: boolean;
        sendWolframQuery?: (query: string) => void;
    }
}

// HTML转义函数
function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
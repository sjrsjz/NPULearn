// 根据官方文档，导入正确的 API
import { $typst, preloadRemoteFonts } from '@myriaddreamin/typst.ts';
// 导入 WASM 模块
import compilerWasm from '@myriaddreamin/typst-ts-web-compiler/pkg/typst_ts_web_compiler_bg.wasm?url';
import rendererWasm from '@myriaddreamin/typst-ts-renderer/pkg/typst_ts_renderer_bg.wasm?url';

import { isStreaming, AppEvents } from '../eventBus';
import { nextTick } from "vue";

// 在文件顶部添加接口定义
interface FontConfig {
    fontPaths?: string[];
    fontBlobs?: ArrayBuffer[];
}

interface WebFontConfig {
    url: string;
    name: string;
}

const webFonts = [] as WebFontConfig[]; // 这里可以添加网络字体配置，例如：

// 添加网络字体加载函数
async function loadWebFont(fontConfig: WebFontConfig): Promise<ArrayBuffer> {
    try {
        const response = await fetch(fontConfig.url);
        if (!response.ok) {
            throw new Error(`Failed to load font ${fontConfig.name}: ${response.statusText}`);
        }
        return await response.arrayBuffer();
    } catch (error) {
        console.error(`加载字体 ${fontConfig.name} 失败:`, error);
        throw error;
    }
}

// 添加批量加载网络字体的函数
async function loadWebFonts(webFonts: WebFontConfig[]): Promise<FontConfig> {
    const fontBlobs: ArrayBuffer[] = [];

    try {
        const loadPromises = webFonts.map(font => loadWebFont(font));
        const loadedFonts = await Promise.all(loadPromises);
        fontBlobs.push(...loadedFonts);

        return {
            fontBlobs
        };
    } catch (error) {
        console.error("加载网络字体失败:", error);
        throw error;
    }
}

// 初始化 typst 渲染器
let initialized = false;

async function initializeTypst() {
    if (initialized) return;

    try {
        // 尝试加载本地字体文件
        const localFontBlobs = await loadLocalFontBlobs();
        console.log('成功加载字体文件数量:', localFontBlobs.length);

        // 准备字体配置
        const beforeBuild = [];

        if (localFontBlobs.length > 0) {
            // 将 ArrayBuffer 转换为 Uint8Array 并添加到字体配置
            const fontData = localFontBlobs.map(blob => new Uint8Array(blob));
            beforeBuild.push(preloadRemoteFonts(fontData));
            console.log('添加字体到 beforeBuild 配置');
        }

        // 使用字体配置初始化
        $typst.setCompilerInitOptions({
            getModule: () => compilerWasm,
            beforeBuild: beforeBuild
        });

        $typst.setRendererInitOptions({
            getModule: () => rendererWasm,
        });

        initialized = true;
        console.log("Typst 渲染器初始化成功，加载了字体文件");
    } catch (error) {
        console.error("Typst 渲染器初始化失败:", error);
        throw error;
    }
}

async function loadLocalFontBlobs(): Promise<ArrayBuffer[]> {
    const fontBlobs: ArrayBuffer[] = [];

    // 尝试多种字体加载方式
    const fontSources = [
        // 尝试通过 Vite 动态导入
        async () => {
            try {
                const fontModule = await import('../../assets/fonts/NotoSansSC-VF.ttf?url');
                const fontUrl = fontModule.default;
                console.log('通过 Vite 动态导入获得字体 URL:', fontUrl);
                return fontUrl;
            } catch (e) {
                console.warn('Vite 动态导入字体失败:', e);
                return null;
            }
        },
        // 直接尝试各种路径
        () => Promise.resolve('/src/assets/fonts/NotoSansSC-VF.ttf'),
        () => Promise.resolve('./src/assets/fonts/NotoSansSC-VF.ttf'),
        () => Promise.resolve('src/assets/fonts/NotoSansSC-VF.ttf'),
        () => Promise.resolve('/assets/fonts/NotoSansSC-VF.ttf'),
        () => Promise.resolve('./assets/fonts/NotoSansSC-VF.ttf'),
        () => Promise.resolve('assets/fonts/NotoSansSC-VF.ttf'),
        () => Promise.resolve('/fonts/NotoSansSC-VF.ttf'), // public 目录
        () => Promise.resolve('./fonts/NotoSansSC-VF.ttf'),
        () => Promise.resolve('fonts/NotoSansSC-VF.ttf')
    ];

    for (const fontSource of fontSources) {
        try {
            const fontUrl = await fontSource();
            if (!fontUrl) continue;

            console.log('尝试加载字体文件:', fontUrl);
            const response = await fetch(fontUrl);

            if (response.ok) {
                const fontBlob = await response.arrayBuffer();
                fontBlobs.push(fontBlob);
                console.log('成功加载字体文件:', fontUrl, '大小:', fontBlob.byteLength);
                return fontBlobs; // 成功加载一个字体文件后返回
            } else {
                console.warn('字体文件响应错误:', fontUrl, response.status);
            }
        } catch (error) {
            console.warn('尝试加载字体文件失败:', error);
        }
    }

    if (fontBlobs.length === 0) {
        console.warn('所有字体路径都加载失败，将使用系统默认字体');
    }

    return fontBlobs;
}

/**
 * 检测当前是否为暗色模式
 */
function isDarkMode(): boolean {
    // 检查 document.documentElement 的 data-theme 属性
    const theme = document.documentElement.getAttribute('data-theme');
    if (theme) {
        return theme === 'dark';
    }

    // 检查 CSS 类名
    if (document.documentElement.classList.contains('dark')) {
        return true;
    }

    // 检查系统偏好
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
        return true;
    }

    return false;
}

/**
 * 渲染 Typst 代码为 SVG
 * @param content Typst 代码内容
 * @param fontConfig 字体配置
 * @param webFonts 网络字体配置
 * @returns 渲染后的 SVG 字符串
 */
export async function renderTypstToSVG(
    content: string,
    fontConfig?: FontConfig,
    webFonts?: WebFontConfig[]
): Promise<string> {
    try {
        // 如果提供了网络字体配置，先加载网络字体
        if (webFonts && webFonts.length > 0) {
            const webFontConfig = await loadWebFonts(webFonts);
            // 合并字体配置
            fontConfig = {
                fontPaths: [...(fontConfig?.fontPaths || [])],
                fontBlobs: [
                    ...(webFontConfig.fontBlobs || []),
                    ...(fontConfig?.fontBlobs || [])
                ]
            };
        }

        await initializeTypst();

        // 检测暗色模式并设置相应的颜色
        const isDark = isDarkMode();
        const backgroundColor = isDark ? '#1a1a1a' : '#ffffff';
        const textColor = isDark ? '#ffffff' : '#000000';        // 添加默认字体设置和暗色模式适配到content前
        const contentWithFontFallback = `
#set page(
  width: auto, 
  height: auto,
  fill: rgb("${backgroundColor}")
)
#set text(
  font: (
    "Noto Sans SC",
    "NotoSansSC-VF", 
    "思源黑体",
    "Source Han Sans SC",
    "PingFang SC",
    "Microsoft YaHei",
    "微软雅黑",
    "SimSun", 
    "宋体",
    "sans-serif"
  ),
  size: 16pt,
  fill: rgb("${textColor}")
)

${content}`;

        // 使用 $typst.svg API 进行渲染
        let svg = await $typst.svg({ mainContent: contentWithFontFallback });

        // 后处理 SVG 以确保暗色模式适配
        if (isDark) {
            svg = processSvgForDarkMode(svg, backgroundColor, textColor);
        }

        return svg;
    } catch (error) {
        console.error("Typst 渲染失败:", error);
        throw error;
    }
}

/**
 * 处理 SVG 以适配暗色模式
 */
function processSvgForDarkMode(svg: string, backgroundColor: string, textColor: string): string {
    // 确保 SVG 有正确的背景色
    if (svg.includes('<svg')) {
        // 添加样式来处理暗色模式
        const style = `
        <style>
        .typst-svg-dark {
            background-color: ${backgroundColor};
        }
        .typst-svg-dark text {
            fill: ${textColor} !important;
        }
        .typst-svg-dark path[fill="#000000"],
        .typst-svg-dark path[fill="#000"] {
            fill: ${textColor} !important;
        }
        .typst-svg-dark rect[fill="#ffffff"],
        .typst-svg-dark rect[fill="#fff"] {
            fill: ${backgroundColor} !important;
        }
        </style>`;

        // 在 SVG 开始标签后插入样式
        svg = svg.replace('<svg', `<svg class="typst-svg-dark"`);
        svg = svg.replace('>', '>' + style);
    }

    return svg;
}
/**
 * 处理typst_render API调用
 * @param apiInfo API调用信息
 * @param isStream 是否是流式输出
 * @returns 生成的HTML内容
 */
export async function handleTypstRender(apiInfo: any): Promise<string> {
    // 获取typst代码参数
    const typstCode = apiInfo.arguments.typst_code || '';
    console.log("处理typst_render:", typstCode.substring(0, 50) + "...");

    // 创建唯一ID
    const typstId = `typst-doc-${Date.now()}-${Math.floor(Math.random() * 10000)}`;

    // 编码内容，以便在属性中安全存储
    const encodedContent = encodeURIComponent(typstCode);

    // 准备初始内容
    let initialContent = '<div class="typst-loading">Typst 文档加载中...</div>';
    let isLoaded = false;    // 如果不是流式输出，直接渲染Typst代码
    if (!isStreaming.value) {
        try {
            const svg = await renderTypstToSVG(typstCode, undefined, webFonts);
            initialContent = `<div class="typst-document-container">${svg}</div>`;
            isLoaded = true;
        } catch (error) {
            console.error("立即渲染 Typst 文档失败:", error);
            initialContent = `
                <div class="typst-error">
                    <p>Typst 文档渲染失败</p>
                    <pre class="error-message">${error}</pre>
                    <div class="typst-source">
                    <details>
                        <summary>查看源代码</summary>
                        <div class="code-container">
                        <pre class="code-content">${typstCode}</pre>
                        </div>
                    </details>
                    </div>
                </div>
            `;
        }
    }    // 构建HTML
    const html = `
    <div class="special-api-call typst-api-call" id="${typstId}-container">
      <div class="api-call-header">
        <span class="api-call-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
            <path d="M14 2v6h6"></path>
            <path d="M16 13H8"></path>
            <path d="M16 17H8"></path>
            <path d="M10 9H8"></path>
          </svg>
        </span>
        <span class="api-call-title">Typst 文档</span>
      </div>
      <div class="typst-container ${isLoaded ? 'loaded' : ''}" data-typst-id="${typstId}" data-typst-content="${encodedContent}" ${isLoaded ? `data-last-rendered="${encodedContent}"` : ''}>
        ${initialContent}
      </div>
      <div class="api-call-footer">
        <details>
          <summary>查看源代码</summary>
          <pre class="api-call-code"><code class="language-typst">${typstCode.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
        </details>
      </div>
    </div>
  `;

    // 非流式传输下，为刚渲染的文档绑定事件
    if (!isStreaming.value) {
        setTimeout(() => {
            const container = document.getElementById(`${typstId}-container`);
            if (container) {
                console.log(`立即为非流式 Typst 文档绑定事件 - ID: ${typstId}`);
                // 统一处理文档按钮和事件绑定
                setupAllTypstInteractions(container);

                // 再次延迟确保绑定成功
                setTimeout(() => {
                    console.log(`延迟再次检查 Typst 文档事件绑定 - ID: ${typstId}`);
                    setupAllTypstInteractions(container);
                }, 200);
            }
        }, 0);  // 使用setTimeout确保HTML先被添加到DOM
    }

    return html;
}

/**
 * 渲染Typst文档函数，接受容器参数
 */
async function renderTypstDocuments(retryCount = 0, maxRetries = 3, container: HTMLElement = document.body) {
    try {
        // 查找所有需要渲染的Typst元素，使用传入的容器
        const typstElements = container.querySelectorAll('.typst-container:not(.loaded)');
        console.log(`尝试渲染 ${typstElements.length} 个Typst文档，当前重试次数: ${retryCount}`);

        if (typstElements.length === 0 && retryCount === 0) {
            // 第一次调用且没有找到未加载的文档，检查是否需要全局重新渲染
            const allTypstElements = container.querySelectorAll('.typst-container');
            if (allTypstElements.length > 0) {
                console.log(`未找到未加载的文档，存在 ${allTypstElements.length} 个已加载文档`);
            }
            return;
        }

        let renderPromises = [];

        for (const element of typstElements) {
            const id = element.getAttribute('data-typst-id');
            const encodedContent = element.getAttribute('data-typst-content');
            const lastRenderedContent = element.getAttribute('data-last-rendered');

            // 跳过内容未变化的文档渲染，避免重复工作
            if (encodedContent && lastRenderedContent && encodedContent === lastRenderedContent) {
                console.log(`跳过文档 ID: ${id} 的渲染，内容未变化`);
                continue;
            }

            if (encodedContent && id) {
                let content = '';
                try {
                    // 清空现有内容
                    element.innerHTML = '<div class="typst-loading">Typst 文档渲染中...</div>';

                    // 正确解码内容
                    content = decodeURIComponent(encodedContent);

                    // 使用Promise.resolve()包装渲染过程，以便收集所有渲染任务
                    renderPromises.push(
                        Promise.resolve().then(async () => {
                            if (typeof content === 'string' && content.length > 0) {
                                try {
                                    const svg = await renderTypstToSVG(content, undefined, webFonts);
                                    element.innerHTML = `<div class="typst-document-container">${svg}</div>`;
                                    // 添加文档加载完成的标记
                                    element.classList.add('loaded');
                                    // 记录已渲染的内容，用于后续比较避免重复渲染
                                    element.setAttribute('data-last-rendered', encodedContent);
                                    return true;
                                } catch (renderError) {
                                    console.error(`单个文档渲染失败 ID ${id}:`, renderError);
                                    element.innerHTML = `
                                        <div class="typst-error">
                                            <p>Typst 文档渲染失败</p>
                                            <pre class="error-message">${renderError}</pre>
                                            <div class="typst-source">
                                            <details>
                                                <summary>查看源代码</summary>
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
                    console.error(`渲染文档 ID ${id} 失败:`, error);
                    console.error("失败的内容 (decoded):", content); // 记录导致失败的解码后内容
                    element.innerHTML = `
                        <div class="typst-error">
                            <p>Typst 文档渲染失败</p>
                            <pre class="error-message">${error}</pre>
                            <div class="typst-source">
                            <details>
                                <summary>查看源代码</summary>
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
                console.warn("发现缺少必要属性（id 或 content）的 Typst 容器。", element);
            }
        }

        // 等待所有渲染完成
        if (renderPromises.length > 0) {
            const results = await Promise.all(renderPromises);
            const failedCount = results.filter(success => !success).length;

            // 如果有失败的文档，且未超过最大重试次数，则重试
            if (failedCount > 0 && retryCount < maxRetries) {
                console.log(`${failedCount}个文档渲染失败，将在1.5秒后重试 (${retryCount + 1}/${maxRetries})`);
                setTimeout(() => renderTypstDocuments(retryCount + 1, maxRetries, container), 1500);
            } else if (failedCount > 0) {
                console.log(`渲染完成，但有${failedCount}个文档渲染失败，已达到最大重试次数`);
                // 为失败的文档添加重试按钮事件监听
                setupAllTypstInteractions(container);
            } else {
                console.log('所有文档渲染成功');
                // 设置文档的可点击功能
                setupAllTypstInteractions(container);
            }

            // 无论成功失败，都延迟再次调用一次以确保所有文档都得到正确处理
            setTimeout(() => {
                console.log('延迟检查，确保所有 Typst 文档事件绑定正确');
                setupAllTypstInteractions(container);
            }, 500);
        } else {
            // 如果没有需要渲染的文档，也处理已渲染的文档
            setupAllTypstInteractions(container);
        }
    } catch (error) {
        console.error("处理Typst文档失败:", error);
        if (retryCount < maxRetries) {
            console.log(`整体处理失败，将在1.5秒后重试 (${retryCount + 1}/${maxRetries})`);
            setTimeout(() => renderTypstDocuments(retryCount + 1, maxRetries, container), 1500);
        } else {
            // 即使出错，也尝试为已渲染的文档添加交互功能
            setupAllTypstInteractions(container);
        }
    }
}

/**
 * 设置 Typst 文档的所有交互功能，包括刷新、缩放、点击查看等。
 * @param container 包含 Typst 文档的父元素
 */
function setupAllTypstInteractions(container: HTMLElement = document.body) {
    nextTick(() => {
        // 为所有文档容器添加刷新按钮
        container.querySelectorAll('.typst-container').forEach(documentContainer => {
            // 强制类型转换以便后续操作
            const typstContainer = documentContainer as HTMLElement;

            // 检查容器是否已经有刷新按钮
            if (!typstContainer.querySelector('.refresh-typst-button')) {
                const refreshButton = document.createElement('button');
                refreshButton.className = 'refresh-typst-button';
                refreshButton.innerHTML = `
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M23 4v6h-6"></path>
            <path d="M1 20v-6h6"></path>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10"></path>
            <path d="M20.49 15a9 9 0 0 1-14.85 3.36L1 14"></path>
          </svg>
        `;
                refreshButton.title = "刷新文档";

                refreshButton.addEventListener('click', async (e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    const targetButton = e.currentTarget as HTMLElement;
                    const clickedContainer = targetButton.closest('.typst-container');

                    if (clickedContainer) {
                        // 移除loaded类以便重新渲染
                        clickedContainer.classList.remove('loaded');
                        // 清除上次渲染的内容记录，强制重新渲染
                        clickedContainer.removeAttribute('data-last-rendered');
                        targetButton.classList.add('refreshing');
                        AppEvents.showNotification("正在刷新文档...", "info");

                        // 延迟后渲染以确保UI更新
                        setTimeout(async () => {
                            await renderTypstDocuments(0, 3, container);
                            targetButton.classList.remove('refreshing');
                        }, 100);
                    }
                });

                // 将按钮添加到容器中
                typstContainer.appendChild(refreshButton);
            }

            // 改进的渲染成功检查逻辑
            const hasLoadedClass = typstContainer.classList.contains('loaded');
            const hasErrorElement = typstContainer.querySelector('.typst-error');
            const hasDocumentElement = typstContainer.querySelector('.typst-document-container');
            const isRenderedSuccessfully = hasLoadedClass && !hasErrorElement && hasDocumentElement;

            console.log(`文档容器检查 - ID: ${typstContainer.getAttribute('data-typst-id')}, loaded: ${hasLoadedClass}, hasError: ${!!hasErrorElement}, hasDocument: ${!!hasDocumentElement}, success: ${isRenderedSuccessfully}`);

            // 只有成功渲染的文档才添加放大按钮
            if (isRenderedSuccessfully && !typstContainer.querySelector('.zoom-typst-button')) {
                const zoomButton = document.createElement('button');
                zoomButton.className = 'zoom-typst-button';
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

                    const clickedContainer = (e.currentTarget as HTMLElement).closest('.typst-container') as HTMLElement;
                    if (clickedContainer) {
                        const documentElement = clickedContainer.querySelector('.typst-document-container');
                        const contentElement = clickedContainer.getAttribute('data-typst-content');

                        if (documentElement && contentElement) {
                            const documentContent = documentElement.innerHTML;
                            const sourceContent = decodeURIComponent(contentElement);
                            AppEvents.openChartViewer(documentContent, sourceContent);
                        }
                    }
                });

                typstContainer.appendChild(zoomButton);
                console.log(`已为文档添加放大按钮 - ID: ${typstContainer.getAttribute('data-typst-id')}`);
            } else if (!isRenderedSuccessfully && typstContainer.querySelector('.zoom-typst-button')) {
                // 如果文档渲染失败，但之前添加了放大按钮，则移除它
                const zoomButton = typstContainer.querySelector('.zoom-typst-button');
                if (zoomButton) {
                    zoomButton.remove();
                    console.log(`已移除失败文档的放大按钮 - ID: ${typstContainer.getAttribute('data-typst-id')}`);
                }
            }

            // 只为成功渲染的文档添加点击事件
            if (isRenderedSuccessfully) {
                // 为整个容器添加点击事件以打开查看器
                if (!typstContainer.hasAttribute('data-has-click-listener')) {
                    typstContainer.setAttribute('data-has-click-listener', 'true');

                    typstContainer.addEventListener('click', (e) => {
                        // 点击按钮时不触发
                        if ((e.target as HTMLElement).closest('.refresh-typst-button, .zoom-typst-button')) {
                            return;
                        }

                        const documentElement = typstContainer.querySelector('.typst-document-container');
                        const contentElement = typstContainer.getAttribute('data-typst-content');

                        if (documentElement && contentElement) {
                            const documentContent = documentElement.innerHTML;
                            const sourceContent = decodeURIComponent(contentElement);
                            AppEvents.openChartViewer(documentContent, sourceContent);
                        }
                    });

                    // 添加视觉提示，表明容器可点击
                    typstContainer.classList.add('clickable-container');
                    console.log(`已为文档添加点击事件 - ID: ${typstContainer.getAttribute('data-typst-id')}`);
                }
            } else {
                // 如果文档渲染失败，移除点击相关的类和属性
                typstContainer.classList.remove('clickable-container');
                typstContainer.removeAttribute('data-has-click-listener');
                console.log(`已移除失败文档的点击功能 - ID: ${typstContainer.getAttribute('data-typst-id')}`);
            }
        });
    });
}

// 导出新增的函数
export { renderTypstDocuments, setupAllTypstInteractions };

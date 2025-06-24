// 根据官方文档，导入正确的 API
import { $typst } from '@myriaddreamin/typst.ts';
// 导入 WASM 模块
import compilerWasm from '@myriaddreamin/typst-ts-web-compiler/pkg/typst_ts_web_compiler_bg.wasm?url';
import rendererWasm from '@myriaddreamin/typst-ts-renderer/pkg/typst_ts_renderer_bg.wasm?url';

import { isStreaming } from '../eventBus';

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
const fontConfig = {
    fontPaths: [
        "assets/fonts",
    ]
} as FontConfig; // 这里可以添加本地字体配置，例如：

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

async function initializeTypst(fontConfig?: FontConfig) {
    if (initialized) return;

    try {
        // 使用正确的初始化 API 设置编译器选项
        $typst.setCompilerInitOptions({
            getModule: () => compilerWasm,
            ...(fontConfig && { fontArgs: buildFontArgs(fontConfig) })
        });

        // 使用正确的初始化 API 设置渲染器选项
        $typst.setRendererInitOptions({
            getModule: () => rendererWasm,
            ...(fontConfig && { fontArgs: buildFontArgs(fontConfig) })
        });

        initialized = true;
        console.log("Typst 渲染器初始化成功");
    } catch (error) {
        console.error("Typst 渲染器初始化失败:", error);
        throw error;
    }
}

// 辅助函数：构建字体参数
function buildFontArgs(fontConfig: FontConfig): any[] {
    const fontArgs = [];
    
    if (fontConfig.fontPaths && fontConfig.fontPaths.length > 0) {
        fontArgs.push({ fontPaths: fontConfig.fontPaths });
    }
    
    if (fontConfig.fontBlobs && fontConfig.fontBlobs.length > 0) {
        fontArgs.push({ fontBlobs: fontConfig.fontBlobs });
    }
    
    return fontArgs;
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

        await initializeTypst(fontConfig);

        // 检测暗色模式并设置相应的颜色
        const isDark = isDarkMode();
        const backgroundColor = isDark ? '#1a1a1a' : '#ffffff';
        const textColor = isDark ? '#ffffff' : '#000000';

        // 添加默认字体设置和暗色模式适配到content前
        const contentWithFontFallback = `
#set page(
  width: auto, 
  height: auto,
  fill: rgb("${backgroundColor}")
)
#set text(
  font: (
    "Noto Sans SC",
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
    const encodedContent = encodeURIComponent(typstCode);    // 如果不是流式输出，直接渲染Typst代码
    if (!isStreaming.value) {
        try {
            const svg = await renderTypstToSVG(typstCode, fontConfig, webFonts);
            const isDark = isDarkMode();

            // 构建包含已渲染SVG的HTML
            return `
            <div class="special-api-call typst-api-call ${isDark ? 'dark-mode' : ''}">
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
                <div class="typst-container loaded ${isDark ? 'dark-mode' : ''}" data-typst-id="${typstId}" data-typst-content="${encodedContent}">
                    <div class="typst-document-container">${svg}</div>
                </div>
                <div class="api-call-footer">
                    <details>
                        <summary>查看源代码</summary>
                        <pre class="api-call-code"><code class="language-typst">${typstCode.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
                    </details>
                </div>
            </div>
            `;
        } catch (error) {
            const isDark = isDarkMode();
            return `
            <div class="special-api-call typst-api-call ${isDark ? 'dark-mode' : ''}">
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
                <div class="typst-container ${isDark ? 'dark-mode' : ''}" data-typst-id="${typstId}">
                    <div class="typst-error">渲染 Typst 文档失败: ${error instanceof Error ? error.message : String(error)}</div>
                    <pre class="typst-source-error"><code>${typstCode.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
                </div>
            </div>
            `;
        }
    }

    // 如果是流式输出，返回包含占位符的HTML（与原来类似）
    const isDark = isDarkMode();
    return `
    <div class="special-api-call typst-api-call ${isDark ? 'dark-mode' : ''}">
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
      <div class="typst-container ${isDark ? 'dark-mode' : ''}" data-typst-id="${typstId}" data-typst-content="${encodedContent}">
        <div class="typst-loading">Typst 文档渲染中...</div>
        <div class="typst-document-container"></div>
      </div>
      <div class="api-call-footer">
        <details>
          <summary>查看源代码</summary>
          <pre class="api-call-code"><code class="language-typst">${typstCode.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
        </details>
      </div>
    </div>
  `;
}

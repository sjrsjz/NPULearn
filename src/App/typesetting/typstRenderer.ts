import { $typst } from '@myriaddreamin/typst.ts/dist/esm/contrib/snippet.mjs';

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
        const compilerOptions: any = {
            getModule: () =>
                'https://cdn.jsdelivr.net/npm/@myriaddreamin/typst-ts-web-compiler/pkg/typst_ts_web_compiler_bg.wasm',
        };

        // 添加字体配置
        if (fontConfig) {
            compilerOptions.fontArgs = [];
            if (fontConfig.fontPaths) {
                compilerOptions.fontArgs.push({ fontPaths: fontConfig.fontPaths });
            }
            if (fontConfig.fontBlobs) {
                compilerOptions.fontArgs.push({ fontBlobs: fontConfig.fontBlobs });
            }
        }

        $typst.setCompilerInitOptions(compilerOptions);

        const rendererOptions: any = {
            getModule: () =>
                'https://cdn.jsdelivr.net/npm/@myriaddreamin/typst-ts-renderer/pkg/typst_ts_renderer_bg.wasm'
        };

        // 添加字体配置
        if (fontConfig) {
            rendererOptions.fontArgs = [];
            if (fontConfig.fontPaths) {
                rendererOptions.fontArgs.push({ fontPaths: fontConfig.fontPaths });
            }
            if (fontConfig.fontBlobs) {
                rendererOptions.fontArgs.push({ fontBlobs: fontConfig.fontBlobs });
            }
        }

        $typst.setRendererInitOptions(rendererOptions);

        initialized = true;
        console.log("Typst 渲染器初始化成功");
    } catch (error) {
        console.error("Typst 渲染器初始化失败:", error);
        throw error;
    }
}

/**
 * 渲染 Typst 代码为 SVG
 * @param content Typst 代码内容
 * @returns 渲染后的 SVG 字符串
 */
export async function renderTypstToSVG(
    content: string,
    fontConfig?: FontConfig,
    webFonts?: WebFontConfig[]
): Promise<string> {
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
    try {
        // 添加默认字体设置到content前
        const contentWithFontFallback = `
#set text(
  font: (
    "Noto Sans SC",
  )
)
#set page(width: auto, height: auto)

${content}`;

        let svg = await $typst.svg({ mainContent: contentWithFontFallback });

        return svg;
    } catch (error) {
        console.error("Typst 渲染失败:", error);
        throw error;
    }
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

    // 如果不是流式输出，直接渲染Typst代码
    if (!isStreaming.value) {
        try {
            const svg = await renderTypstToSVG(typstCode, fontConfig, webFonts);



            // 构建包含已渲染SVG的HTML
            return `
            <div class="special-api-call typst-api-call">
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
                <div class="typst-container loaded" data-typst-id="${typstId}" data-typst-content="${encodedContent}">
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
            return `
            <div class="special-api-call typst-api-call">
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
                <div class="typst-container" data-typst-id="${typstId}">
                    <div class="typst-error">渲染 Typst 文档失败: ${error instanceof Error ? error.message : String(error)}</div>
                    <pre class="typst-source-error"><code>${typstCode.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
                </div>
            </div>
            `;
        }
    }

    // 如果是流式输出，返回包含占位符的HTML（与原来类似）
    return `
    <div class="special-api-call typst-api-call">
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
      <div class="typst-container" data-typst-id="${typstId}" data-typst-content="${encodedContent}">
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

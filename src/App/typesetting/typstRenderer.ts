import { $typst } from '@myriaddreamin/typst.ts/dist/esm/contrib/snippet.mjs';

// 初始化 typst 渲染器
let initialized = false;

async function initializeTypst() {
    if (initialized) return;

    try {
        // 设置编译器和渲染器初始化选项
        const wasmBaseURL = './node_modules/@myriaddreamin/typst.ts/dist/wasm/';

        await $typst.setCompilerInitOptions({
            getModule: async () => {
                return await fetch(`${wasmBaseURL}compiler-$typst-compiler.wasm`).then(r => r.arrayBuffer());
            }
        });

        await $typst.setRendererInitOptions({
            getModule: async () => {
                return await fetch(`${wasmBaseURL}renderer-$typst-svg.wasm`).then(r => r.arrayBuffer());
            }
        });

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
export async function renderTypstToSVG(content: string): Promise<string> {
    await initializeTypst();
    try {
        const svg = await $typst.svg({ mainContent: content });
        return svg;
    } catch (error) {
        console.error("Typst 渲染失败:", error);
        throw error;
    }
}

/**
 * 处理 Typst 容器的渲染
 */
export async function renderTypstContainer(container: HTMLElement): Promise<void> {
    const encodedContent = container.getAttribute('data-typst-content');
    if (!encodedContent) return;

    try {
        const content = decodeURIComponent(encodedContent);
        const svg = await renderTypstToSVG(content);

        const docContainer = container.querySelector('.typst-document-container');
        if (docContainer) {
            docContainer.innerHTML = svg;
            container.classList.add('loaded');
            container.querySelector('.typst-loading')?.remove();
        }
    } catch (error) {
        container.innerHTML = `
      <div class="typst-error">渲染 Typst 文档失败: ${error instanceof Error ? error.message : String(error)}</div>
      <pre class="typst-source-error"><code>${decodeURIComponent(encodedContent).replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
    `;
    }
}


/**
 * 处理typst_render API调用
 * @param apiInfo API调用信息
 * @returns 生成的HTML内容
 */
export function handleTypstRender(apiInfo: any): string {
    // 获取typst代码参数
    const typstCode = apiInfo.arguments.typst_code || '';
    console.log("处理typst_render:", typstCode.substring(0, 50) + "...");

    // 创建唯一ID
    const typstId = `typst-doc-${Date.now()}-${Math.floor(Math.random() * 10000)}`;

    // 编码内容，以便在属性中安全存储
    const encodedContent = encodeURIComponent(typstCode);

    // 构建HTML
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

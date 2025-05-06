import katex from 'katex';

/**
 * 处理 KaTeX 渲染 API 调用
 * @param apiInfo API 调用信息
 * @returns 生成的 HTML 内容
 */
export async function handleKaTeXRender(apiInfo: any): Promise<string> {
    // 获取 KaTeX 代码参数
    const katexCode = apiInfo.arguments.katex_code || '';
    console.log("处理 katex_render:", katexCode);

    // 创建唯一的渲染 ID
    const renderId = `katex-render-${Date.now()}-${Math.floor(Math.random() * 10000)}`;

    // 编码内容，以便在属性中安全存储
    const encodedContent = encodeURIComponent(katexCode);

    try {
        // 渲染 KaTeX 代码为 HTML，使用一个包装div来控制显示
        const renderedHTML = `<div class="katex-wrapper">${katex.renderToString(katexCode, {
            throwOnError: false,
            displayMode: true,
            output: 'mathml'  // 使用 MathML 输出
        })}</div>`;

        // 返回包含渲染结果的 HTML
        return `
        <div class="special-api-call katex-api-call">
            <div class="api-call-header">
                <span class="api-call-icon">
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M12 20.94c1.5 0 2.75 1.06 4 1.06 3 0 6-8 6-12.22A4.91 4.91 0 0 0 17 5c-2.22 0-4 1.44-5 2-1-.56-2.78-2-5-2a4.9 4.9 0 0 0-5 4.78C2 14 5 22 8 22c1.25 0 2.5-1.06 4-1.06Z"></path>
                        <path d="M10 2c1 .5 2 2 2 5"></path>
                    </svg>
                </span>
                <span class="api-call-title">KaTeX 渲染</span>
            </div>
            <div class="katex-container loaded" data-render-id="${renderId}" data-render-content="${encodedContent}">
                <div class="katex-rendered-content">${renderedHTML}</div>
            </div>
            <div class="api-call-footer">
                <details>
                    <summary>查看源代码</summary>
                    <pre class="api-call-code"><code class="language-katex">${katexCode.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
                </details>
            </div>
        </div>
        `;
    } catch (error) {
        // 错误处理部分保持不变
        console.error("KaTeX 渲染失败:", error);
        return `
        <div class="special-api-call katex-api-call">
            <div class="api-call-header">
                <span class="api-call-icon">
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M12 20.94c1.5 0 2.75 1.06 4 1.06 3 0 6-8 6-12.22A4.91 4.91 0 0 0 17 5c-2.22 0-4 1.44-5 2-1-.56-2.78-2-5-2a4.9 4.9 0 0 0-5 4.78C2 14 5 22 8 22c1.25 0 2.5-1.06 4-1.06Z"></path>
                        <path d="M10 2c1 .5 2 2 2 5"></path>
                    </svg>
                </span>
                <span class="api-call-title">KaTeX 渲染</span>
            </div>
            <div class="katex-container" data-render-id="${renderId}">
                <div class="katex-error">渲染 KaTeX 失败: ${error instanceof Error ? error.message : String(error)}</div>
                <pre class="katex-source-error"><code>${katexCode.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</code></pre>
            </div>
        </div>
        `;
    }
}
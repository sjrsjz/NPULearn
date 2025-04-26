/**
 * 全局主题和字体大小的工具函数
 */

export interface ThemeSettings {
    theme: 'system' | 'light' | 'dark';
    fontSize: 'small' | 'medium' | 'large';
}

/**
 * 应用主题设置到文档根元素
 * @param theme 主题：'system', 'light', 或 'dark'
 */
export function applyTheme(theme: 'system' | 'light' | 'dark'): void {
    if (theme === 'system') {
        document.documentElement.removeAttribute('data-theme');
    } else {
        document.documentElement.setAttribute('data-theme', theme);
    }
    console.log(`应用主题: ${theme}`);

    // 触发自定义事件，通知应用程序主题已更改
    window.dispatchEvent(new CustomEvent('themeChanged', { detail: theme }));
    
    // 不要直接调用refreshGlobalStyles，而是稍后再调用，
    // 以确保themeChanged事件的处理程序有机会先运行
    setTimeout(() => {
        console.log("延迟刷新全局样式...");
        // 修改：调用完整的refreshGlobalStyles函数来确保markdown内容也被正确处理
        refreshGlobalStyles();
    }, 50);
}

/**
 * 应用字体大小设置到文档根元素
 * @param fontSize 字体大小：'small', 'medium', 或 'large'
 */
export function applyFontSize(fontSize: 'small' | 'medium' | 'large'): void {
    document.documentElement.setAttribute('data-font-size', fontSize);
    console.log(`应用字体大小: ${fontSize}`);

    // 触发自定义事件，通知应用程序字体大小已更改
    window.dispatchEvent(new CustomEvent('fontSizeChanged', { detail: fontSize }));
    
    // 添加：直接刷新全局样式，确保字体大小变化也会应用到markdown内容
    setTimeout(() => {
        refreshGlobalStyles();
    }, 50);
}

/**
 * 检查当前环境是否为暗色模式
 * @returns 是否为暗色模式
 */
export function isDarkMode(): boolean {
    const theme = document.documentElement.getAttribute('data-theme');
    return theme === 'dark' || (theme !== 'light' && window.matchMedia('(prefers-color-scheme: dark)').matches);
}

/**
 * 获取当前的字体大小设置
 * @returns 当前字体大小设置
 */
export function getCurrentFontSize(): 'small' | 'medium' | 'large' {
    return (document.documentElement.getAttribute('data-font-size') as 'small' | 'medium' | 'large') || 'medium';
}

/**
 * 获取字体大小值（像素）
 * @param size 字体大小类别
 * @returns 对应的像素值
 */
export function getFontSizeValues(size: 'small' | 'medium' | 'large'): { base: string, sm: string, lg: string } {
    switch (size) {
        case 'small':
            return { base: '14px', sm: '12px', lg: '16px' };
        case 'large':
            return { base: '18px', sm: '16px', lg: '20px' };
        default: // medium
            return { base: '16px', sm: '14px', lg: '18px' };
    }
}

/**
 * 全局样式刷新函数
 * 更新整个应用的主题和字体大小
 */
export function refreshGlobalStyles(): void {
    const currentTheme = document.documentElement.getAttribute('data-theme') as 'light' | 'dark' | null || 'system';
    const currentFontSize = getCurrentFontSize();

    // 对文档整体应用主题属性，但不触发applyTheme以避免循环调用
    if (currentTheme === 'system') {
        document.documentElement.removeAttribute('data-theme');
    } else {
        document.documentElement.setAttribute('data-theme', currentTheme);
    }
    document.documentElement.setAttribute('data-font-size', currentFontSize);

    // 2. 应用样式到动态内容（如聊天内容）
    const chatMessages = document.querySelectorAll('.chat-messages style');
    const fontSizes = getFontSizeValues(currentFontSize);
    const isDark = isDarkMode();

    chatMessages.forEach(style => {
        const styleContent = style.textContent || '';
        let newStyleContent = styleContent.replace(/html|body/g, '.scoped-content');

        // 添加暗色主题覆盖样式
        if (isDark) {
            newStyleContent += `
        .scoped-content {
          color: #f1f5f9 ;
          background-color: transparent ;
        }
        .scoped-content a { color: #58a6ff ; text-decoration: none ; }
        .scoped-content a:hover { text-decoration: underline ; }
        .scoped-content code { 
          background-color: rgba(71, 85, 105, 0.4) ; 
          padding: 0.2em 0.4em ;
          border-radius: 3px ;
          font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace ;
        }
        .scoped-content pre { 
          background-color: #1e293b ; 
          border-radius: 6px ;
          box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4) ;
          margin: 16px 0 ;
          padding: 16px ;
          position: relative ;
        }
        .scoped-content pre code { 
          background-color: transparent ; 
          padding: 0 ;
        }
        .scoped-content blockquote { 
          color: #9ca3af ; 
          border-left: 4px solid #3b82f6 ; 
          padding: 0 16px ;
          margin: 16px 0 ;
          background-color: rgba(30, 41, 59, 0.5) ;
          border-radius: 0 6px 6px 0 ;
        }
        .scoped-content table { 
          border-spacing: 0 ;
          border-collapse: separate ;
          border-radius: 6px ;
          margin: 16px 0 ;
          border: 1px solid #334155 ;
          box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3) ;
        }
        .scoped-content table th { 
          background-color: #1e293b ; 
          padding: 8px 13px ;
          border: 1px solid #334155 ;
          font-weight: 600 ;
        }
        .scoped-content table td { 
          border: 1px solid #334155 ; 
          padding: 8px 13px ;
        }
        .scoped-content table tr:nth-child(2n) {
          background-color: rgba(71, 85, 105, 0.1) ;
        }
        .scoped-content hr { 
          height: 1px ;
          background-color: #334155 ;
          border: 0 ;
          margin: 24px 0 ;
        }
        .scoped-content h1, .scoped-content h2 { 
          border-bottom: 1px solid #334155 ;
          padding-bottom: 0.3em ;
          margin-top: 24px ;
          margin-bottom: 16px ;
        }
        .scoped-content h3, .scoped-content h4, .scoped-content h5, .scoped-content h6 {
          margin-top: 24px ;
          margin-bottom: 16px ;
        }

        .scoped-content ul, .scoped-content ol {
          padding-left: 2em ;
          margin: 16px 0 ;
        }
        .scoped-content li + li {
          margin-top: 0.25em ;
        }
        .scoped-content .message-content {
          padding: 14px 18px ;
          border-radius: 18px ;
          box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3) ;
        }
        .scoped-content .message-time { color: #94a3b8 ; }
        /* 移除了pre:before和pre:after相关样式 */
        .scoped-content pre {
          position: relative ;
        }
      `;
        } else {
            // 添加亮色主题的样式覆盖
            newStyleContent += `
        .scoped-content a { 
          color: #0366d6 ; 
          text-decoration: none ;
        }
        .scoped-content a:hover { 
          text-decoration: underline ; 
        }
        .scoped-content code { 
          background-color: rgba(27, 31, 35, 0.05) ; 
          padding: 0.2em 0.4em ;
          border-radius: 3px ;
          font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace ;
        }
        .scoped-content pre { 
          background-color: #f6f8fa ; 
          border-radius: 6px ;
          box-shadow: 0 2px 6px rgba(0, 0, 0, 0.08) ;
          border: 1px solid #e1e4e8 ;
          margin: 16px 0 ;
          padding: 16px ;
          position: relative ;
        }
        .scoped-content pre code { 
          background-color: transparent ; 
          padding: 0 ;
        }
        .scoped-content blockquote { 
          color: #6a737d ; 
          border-left: 4px solid #dfe2e5 ; 
          padding: 0 16px ;
          margin: 16px 0 ;
          background-color: rgba(246, 248, 250, 0.5) ;
          border-radius: 0 6px 6px 0 ;
        }
        .scoped-content table { 
          border-spacing: 0 ;
          border-collapse: separate ;
          border-radius: 6px ;
          margin: 16px 0 ;
          border: 1px solid #dfe2e5 ;
          box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05) ;
        }
        .scoped-content table th { 
          background-color: #f6f8fa ; 
          padding: 8px 13px ;
          border: 1px solid #dfe2e5 ;
          font-weight: 600 ;
        }
        .scoped-content table td { 
          border: 1px solid #dfe2e5 ; 
          padding: 8px 13px ;
        }
        .scoped-content table tr:nth-child(2n) {
          background-color: rgba(246, 248, 250, 0.7) ;
        }
        .scoped-content hr { 
          height: 1px ;
          background-color: #e1e4e8 ;
          border: 0 ;
          margin: 24px 0 ;
        }
        .scoped-content h1, .scoped-content h2 { 
          border-bottom: 1px solid #eaecef ;
          padding-bottom: 0.3em ;
          margin-top: 24px ;
          margin-bottom: 16px ;
        }
        .scoped-content h3, .scoped-content h4, .scoped-content h5, .scoped-content h6 {
          margin-top: 24px ;
          margin-bottom: 16px ;
        }

        .scoped-content ul, .scoped-content ol {
          padding-left: 2em ;
          margin: 16px 0 ;
        }
        .scoped-content li + li {
          margin-top: 0.25em ;
        }
        .scoped-content .message-content {
          padding: 14px 18px ;
          border-radius: 18px ;
          box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08) ;
        }
        .scoped-content .message-time { color: #94a3b8 ; }
        /* 移除了pre:before和pre:after相关样式 */
        .scoped-content pre {
          position: relative ;
        }
      `;
        }

        // 添加字体大小覆盖样式
        newStyleContent += `
      .scoped-content { font-size: ${fontSizes.base} ; }
      .scoped-content code, .scoped-content pre { font-size: calc(${fontSizes.base} * 0.85) ; }
      .scoped-content h1 { font-size: calc(${fontSizes.base} * 2) ; }
      .scoped-content h2 { font-size: calc(${fontSizes.base} * 1.5) ; }
      .scoped-content h3 { font-size: calc(${fontSizes.base} * 1.25) ; }
      .scoped-content h4 { font-size: ${fontSizes.base} ; }
      .scoped-content h5 { font-size: ${fontSizes.sm} ; }
      .scoped-content h6 { font-size: calc(${fontSizes.sm} * 0.95) ; }
      .scoped-content .message-time { font-size: ${fontSizes.sm} ; }
    `;

        style.textContent = newStyleContent;
    });

    // 3. 刷新代码高亮和数学公式渲染
    // 如果你的应用使用了代码高亮或数学公式库
    if (window.hljs) {
        document.querySelectorAll('pre code').forEach((el) => {
            window.hljs.highlightElement(el as HTMLElement);
        });
    }

    if (window.MathJax && window.MathJax.typesetPromise) {
        const chatMessagesElement = document.querySelector('.chat-messages');
        if (chatMessagesElement) {
            window.MathJax.typesetPromise([chatMessagesElement as HTMLElement]).catch((err: Error) => {
                console.error('MathJax 渲染错误:', err);
            });
        }
    }
    
    // 添加：强制重新应用markdown样式到内容区
    applyMarkdownStyles();
}

/**
 * 强制重新应用Markdown样式到内容区
 * 确保在主题切换后markdown内容正确显示
 */
function applyMarkdownStyles(): void {
    // 查找所有markdown内容容器
    const markdownContainers = document.querySelectorAll('.markdown-content, .chat-messages .message-content');
    
    if (markdownContainers.length === 0) {
        return; // 没有找到markdown内容，无需处理
    }
    
    // 对每个容器触发重新渲染
    markdownContainers.forEach(container => {
        // 触发一个自定义事件，让markdown渲染器知道需要重新渲染
        container.dispatchEvent(new CustomEvent('markdown-refresh', { bubbles: true }));
        
        // 如果容器有特定的刷新方法，尝试调用
        if ((container as any).refreshStyles) {
            (container as any).refreshStyles();
        }
        
        // 如果没有自定义事件处理程序，可以尝试通过添加/移除类来触发样式重计算
        container.classList.add('refresh-styles');
        setTimeout(() => {
            container.classList.remove('refresh-styles');
        }, 10);
    });
}

// 监听系统颜色方案变化
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (_) => {
    const theme = document.documentElement.getAttribute('data-theme');
    if (theme === 'system') {
        refreshGlobalStyles();
    }
});


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

    // 1. 应用基础主题属性
    applyTheme(currentTheme);
    applyFontSize(currentFontSize);

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
          color: #f1f5f9 !important;
          background-color: transparent !important;
        }
        .scoped-content a { color: #6366f1 !important; }
        .scoped-content code { background-color: rgba(71, 85, 105, 0.3) !important; }
        .scoped-content pre { background-color: #1e293b !important; }
        .scoped-content blockquote { color: #94a3b8 !important; border-left-color: #475569 !important; }
        .scoped-content table th { background-color: #1e293b !important; }
        .scoped-content table td, .scoped-content table th { border-color: #475569 !important; }
        .scoped-content hr { background-color: #475569 !important; }
        .scoped-content h1, .scoped-content h2 { border-bottom-color: #475569 !important; }
        .scoped-content .system { background-color: #2d333b !important; }
        .scoped-content .user { background-color: #254254 !important; }
        .scoped-content .message-time { color: #aaa !important; }
      `;
        }

        // 添加字体大小覆盖样式
        newStyleContent += `
      .scoped-content { font-size: ${fontSizes.base} !important; }
      .scoped-content code, .scoped-content pre { font-size: calc(${fontSizes.base} * 0.85) !important; }
      .scoped-content h1 { font-size: calc(${fontSizes.base} * 2) !important; }
      .scoped-content h2 { font-size: calc(${fontSizes.base} * 1.5) !important; }
      .scoped-content h3 { font-size: calc(${fontSizes.base} * 1.25) !important; }
      .scoped-content h4 { font-size: ${fontSizes.base} !important; }
      .scoped-content h5 { font-size: ${fontSizes.sm} !important; }
      .scoped-content h6 { font-size: calc(${fontSizes.sm} * 0.95) !important; }
      .scoped-content .message-time { font-size: ${fontSizes.sm} !important; }
    `;

        style.textContent = newStyleContent;
    });

    // 3. 刷新代码高亮和数学公式渲染
    // 如果你的应用使用了代码高亮或数学公式库
    if (window.hljs) {
        document.querySelectorAll('.chat-messages pre code').forEach((el) => {
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
}

// 监听系统颜色方案变化
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
    const theme = document.documentElement.getAttribute('data-theme');
    if (theme === 'system') {
        refreshGlobalStyles();
    }
});
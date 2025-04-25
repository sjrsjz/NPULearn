/**
 * Markdown 样式定义
 * 支持深色和浅色主题
 */
import { isDarkMode, getCurrentFontSize, getFontSizeValues } from './themeUtils';

export interface MarkdownStyleOptions {
    theme?: 'light' | 'dark' | 'auto';
    fontSize?: 'small' | 'medium' | 'large';
}

/**
 * 生成 Markdown 样式
 * @param options 样式选项
 * @returns CSS 样式字符串
 */
export function getMarkdownStyles(options?: MarkdownStyleOptions): string {
    const theme = options?.theme || 'auto';
    const fontSize = options?.fontSize || getCurrentFontSize();
    const fontSizes = getFontSizeValues(fontSize);
    console.log(`当前字体大小: ${fontSize}`);

    // 判断是否使用深色主题
    const useDarkTheme = theme === 'dark' || (theme === 'auto' && isDarkMode());

    // 基础样式 - 使用 CSS 变量以适配全局主题
    const baseStyles = `
    body {
      font-family: var(--font-family, 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans", sans-serif);
      line-height: 1.6;
      color: var(--text-color, #1f2937);
      background-color: var(--bg-color, #f9fafb);
      margin: 16px;
      font-size: ${fontSizes.base};
    }
    
    h1, h2, h3, h4, h5, h6 {
      margin-top: 28px;
      margin-bottom: 16px;
      font-weight: 600;
      color: var(--text-color, #1f2937);
      line-height: 1.3;
    }
    
    h1 { font-size: calc(${fontSizes.base} * 1.8); padding-bottom: .3em; border-bottom: 0px solid var(--border-color, #e5e7eb); }
    h2 { font-size: calc(${fontSizes.base} * 1.5); padding-bottom: .3em; border-bottom: 0px solid var(--border-color, #e5e7eb); }
    h3 { font-size: calc(${fontSizes.base} * 1.25); }
    h4 { font-size: ${fontSizes.base}; }
    h5 { font-size: ${fontSizes.sm}; }
    h6 { font-size: calc(${fontSizes.sm} * 0.95); color: var(--text-secondary, #64748b); }
    
    a {
      color: var(--primary-color, #3b82f6);
      text-decoration: none;
      transition: all 0.2s;
      border-bottom: 1px dashed var(--primary-color, #3b82f6);
    }
    
    a:hover {
      opacity: 0.85;
    }
    
    table {
      border-spacing: 0;
      border-collapse: collapse;
      margin: 16px 0;
      width: 100%;
      overflow-x: auto;
      display: block;
    }
    
    th, td {
      padding: 8px 16px;
      border: 1px solid var(--border-color, #e5e7eb);
      text-align: left;
    }
    
    th {
      font-weight: 600;
      background-color: var(--card-bg, #ffffff);
    }
    
    tr:nth-child(2n) {
      background-color: var(--bg-color, #f9fafb);
    }
    
    pre {
      padding: 16px;
      overflow: auto;
      font-size: calc(${fontSizes.base} * 0.9);
      line-height: 1.45;
      background-color: var(--card-bg, #ffffff);
      border-radius: var(--radius, 8px);
      margin: 16px 0;
      font-family: 'Fira Code', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      border: 1px solid var(--border-color, #e5e7eb);
      box-shadow: var(--shadow-sm, 0 2px 4px rgba(0, 0, 0, 0.05));
    }
    
    code {
      padding: .2em .4em;
      margin: 0;
      font-size: calc(${fontSizes.base} * 0.9);
      background-color: rgba(0, 0, 0, 0.04);
      border-radius: var(--radius-sm, 6px);
      font-family: 'Fira Code', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    }
    
    pre code {
      padding: 0;
      margin: 0;
      background-color: transparent;
      border: none;
    }
    
    blockquote {
      padding: 0 1em;
      color: var(--text-secondary, #64748b);
      border-left: .25em solid var(--border-color, #e5e7eb);
      margin: 16px 0;
      font-style: italic;
    }
    
    ul, ol {
      padding-left: 2em;
      margin: 16px 0;
    }
    
    img {
      max-width: 100%;
      height: auto;
      border-style: none;
      margin: 16px 0;
      border-radius: var(--radius-sm, 6px);
      box-shadow: var(--shadow-sm, 0 2px 4px rgba(0, 0, 0, 0.05));
    }
    
    hr {
      height: .25em;
      padding: 0;
      margin: 24px 0;
      background-color: var(--border-color, #e5e7eb);
      border: 0;
    }
    
    input[type="checkbox"] {
      margin: 0 .2em .25em -1.4em;
    }
    
    del {
      color: #dc2626;
    }
    
    ins {
      color: #059669;
      text-decoration: none;
      background-color: rgba(5, 150, 105, 0.1);
    }
    
    mark {
      background-color: rgba(252, 211, 77, 0.4);
      color: var(--text-color, #1f2937);
      padding: 0.1em 0.2em;
      border-radius: 2px;
    }
    
    .alert {
      padding: 16px;
      margin: 16px 0;
      border-radius: var(--radius, 8px);
      border: 1px solid;
      box-shadow: var(--shadow-sm, 0 2px 4px rgba(0, 0, 0, 0.05));
    }
    
    .alert-info {
      color: var(--primary-color, #3b82f6);
      background-color: rgba(59, 130, 246, 0.08);
      border-color: var(--primary-color, #60a5fa);
    }
    
    .alert-warning {
      color: #d97706;
      background-color: rgba(217, 119, 6, 0.08);
      border-color: #fbbf24;
    }
    
    .alert-danger {
      color: #dc2626;
      background-color: rgba(220, 38, 38, 0.08);
      border-color: #f87171;
    }
    
    .alert-success {
      color: #059669;
      background-color: rgba(5, 150, 105, 0.08);
      border-color: #34d399;
    }

    .chat-message {
      margin-bottom: 20px;
      animation: fadeIn 0.3s ease;
      border-radius: var(--radius, 8px);
      overflow: hidden;
      box-shadow: var(--shadow-sm, 0 2px 4px rgba(0, 0, 0, 0.05));
    }
    
    @keyframes fadeIn {
      from { opacity: 0; transform: translateY(10px); }
      to { opacity: 1; transform: translateY(0); }
    }

    

    /* 数学公式相关样式优化 */
    .mjx-chtml {
      margin: 0.5em 0;
      font-size: var(--font-size-lg, ${fontSizes.base});
    }
    
    .mjx-math {
      max-width: 100%;
      overflow-x: auto;
      overflow-y: hidden;
    }
    
    .mjx-chtml.MJXc-display {
      margin: 1em 0;
      padding: 0.5em 0;
      overflow-x: auto;
      overflow-y: hidden;
      text-align: center;
    }
    
    .MJX-TEX {
      text-align: center;
    }
    
    .mjx-container {
      padding: 6px 0;
    }

    /* 代码高亮样式调整 */
    .hljs {
      background: var(--card-bg, #ffffff);
      color: var(--text-color, #1f2937);
      border-radius: var(--radius, 8px);
    }
  `;

    // 深色主题样式覆盖 - 使用 CSS 变量以更好地融入全局主题
    const darkStyles = useDarkTheme ? `
    /* 深色主题样式覆盖 */
    body {
      color: var(--dark-text-color, #f3f4f6);
      background-color: var(--dark-bg-color, #0f172a);
    }
    
    h1, h2, h3, h4, h5 {
      color: var(--dark-text-color, #f3f4f6);
    }
    
    h1, h2 {
      color: var(--dark-text-color, #f3f4f6);
      border-bottom-color: var(--dark-border-color, #334155); 
    }
    
    h6 { 
      color: var(--dark-text-secondary, #9ca3af); 
    }
    
    a {
      color: var(--light-primary-color, #60a5fa);
    }
    
    th, td {
      color: var(--dark-text-color, #f3f4f6);
      background-color: var(--dark-card-bg, #1e293b);
      border-color: var(--dark-border-color, #334155);
    }
    
    th {
      color: var(--dark-text-color, #f3f4f6);
      background-color: var(--dark-card-bg, #1e293b);
    }
    
    tr:nth-child(2n) {
      background-color: var(--dark-card-bg, #1e293b);
      background-color: rgba(30, 41, 59, 0.5);
    }
    
    pre {
      background-color: var(--dark-card-bg, #1e293b);
      border-color: var(--dark-border-color, #334155);
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    }
    
    code {
      background-color: rgba(255, 255, 255, 0.1);
    }
    
    blockquote {
      color: var(--dark-text-secondary, #9ca3af);
      border-left-color: var(--dark-border-color, #4b5563);
    }
    
    hr {
      color: var(--dark-border-color, #334155);
      background-color: var(--dark-border-color, #334155);
    }
    
    del {
      color: #f87171;
    }
    
    ins {
      color: #34d399;
      background-color: rgba(52, 211, 153, 0.15);
    }
    
    mark {
      background-color: rgba(252, 211, 77, 0.2);
      color: #fcd34d;
    }
    
    .alert-info {
      color: var(--light-primary-color, #60a5fa);
      background-color: rgba(59, 130, 246, 0.15);
      border-color: rgba(59, 130, 246, 0.4);
    }
    
    .alert-warning {
      color: #fbbf24;
      background-color: rgba(251, 191, 36, 0.15);
      border-color: rgba(251, 191, 36, 0.4);
    }
    
    .alert-danger {
      color: #f87171;
      background-color: rgba(248, 113, 113, 0.15);
      border-color: rgba(248, 113, 113, 0.4);
    }
    
    .alert-success {
      color: #34d399;
      background-color: rgba(52, 211, 153, 0.15);
      border-color: rgba(52, 211, 153, 0.4);
    }
    
    
    .message-time {
      color: var(--dark-text-secondary, #9ca3af);
    }
    
    .message-header {
      color: var(--dark-text-secondary, #9ca3af);
      border-bottom-color: var(--dark-border-color, #334155);
    }

    /* 暗黑模式下的数学公式 */
    .mjx-math {
      color: var(--dark-text-color, #f3f4f6);
    }

    /* 暗黑模式下的代码高亮 */
    .hljs {
      background: var(--dark-card-bg, #1e293b);
      color: var(--dark-text-color, #f3f4f6);
    }
    
    img {
      opacity: 0.9;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    }
  ` : '';

    // 添加全局 CSS 变量融合代码
    const cssVariableIntegration = `
    /* CSS 变量融合代码 - 确保与全局主题一致 */
    :root {
      --font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans", sans-serif;
    }
    
    @media (prefers-color-scheme: dark) {
      :root:not([data-theme='light']) {
        /* 暗色模式下的默认值 */
        --card-bg: var(--dark-card-bg, #1e293b);
        --bg-color: var(--dark-bg-color, #0f172a);
        --text-color: var(--dark-text-color, #f3f4f6);
        --text-secondary: var(--dark-text-secondary, #9ca3af);
        --border-color: var(--dark-border-color, #334155);
      }
    }
    
    /* 链接样式改进 */
    a {
      color: var(--primary-color, #3b82f6);
      text-decoration: none;
      border-bottom: 1px dashed var(--primary-color, #3b82f6);
      transition: opacity 0.2s ease;
      padding: 0 0.1em;
    }
    
    a:hover {
      opacity: 0.8;
      background-color: rgba(59, 130, 246, 0.08);
      border-radius: 2px;
    }
    
    a::after {
      content: '📋';
      font-size: 0.8em;
      margin-left: 3px;
      opacity: 0.7;
    }
    `;

    return `<style>${baseStyles}${darkStyles}${cssVariableIntegration}</style>`;
}

/**
 * 获取用于插入到HTML中的样式标签
 * @param options 样式选项
 * @returns 包含样式的HTML字符串
 */
export function getStyleTag(options?: MarkdownStyleOptions): string {
    return getMarkdownStyles(options);
}
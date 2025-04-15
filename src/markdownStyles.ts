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

    // 基础样式 - 适用于浅色主题
    const baseStyles = `
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans", Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji";
      line-height: 1.5;
      color: #24292f;
      background-color: #ffffff;
      margin: 16px;
      font-size: ${fontSizes.base};
    }
    
    h1, h2, h3, h4, h5, h6 {
      margin-top: 24px;
      margin-bottom: 16px;
      font-weight: 600;
    }
    
    h1 { font-size: calc(${fontSizes.base} * 2); padding-bottom: .3em; border-bottom: 1px solid #eaecef; }
    h2 { font-size: calc(${fontSizes.base} * 1.5); padding-bottom: .3em; border-bottom: 1px solid #eaecef; }
    h3 { font-size: calc(${fontSizes.base} * 1.25); }
    h4 { font-size: ${fontSizes.base}; }
    h5 { font-size: ${fontSizes.sm}; }
    h6 { font-size: calc(${fontSizes.sm} * 0.95); color: #57606a; }
    
    a {
      color: #0969da;
      text-decoration: none;
    }
    
    a:hover {
      text-decoration: underline;
    }
    
    table {
      border-spacing: 0;
      border-collapse: collapse;
      margin: 16px 0;
      width: 100%;
    }
    
    th, td {
      padding: 6px 13px;
      border: 1px solid #d0d7de;
    }
    
    th {
      font-weight: 600;
      background-color: #f6f8fa;
    }
    
    tr:nth-child(2n) {
      background-color: #f6f8fa;
    }
    
    pre {
      padding: 16px;
      overflow: auto;
      font-size: calc(${fontSizes.base} * 0.85);
      line-height: 1.45;
      background-color: #f6f8fa;
      border-radius: 6px;
      margin: 16px 0;
      font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace;
    }
    
    code {
      padding: .2em .4em;
      margin: 0;
      font-size: calc(${fontSizes.base} * 0.85);
      background-color: rgba(175, 184, 193, 0.2);
      border-radius: 6px;
      font-family: ui-monospace, SFMono-Regular, SF Mono, Menlo, Consolas, Liberation Mono, monospace;
    }
    
    pre code {
      padding: 0;
      margin: 0;
      background-color: transparent;
    }
    
    blockquote {
      padding: 0 1em;
      color: #57606a;
      border-left: .25em solid #d0d7de;
      margin: 16px 0;
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
    }
    
    hr {
      height: .25em;
      padding: 0;
      margin: 24px 0;
      background-color: #d0d7de;
      border: 0;
    }
    
    input[type="checkbox"] {
      margin: 0 .2em .25em -1.4em;
    }
    
    del {
      color: #cf222e;
    }
    
    ins {
      color: #116329;
      text-decoration: none;
      background-color: #dafbe1;
    }
    
    mark {
      background-color: #fff8c5;
      color: #24292f;
    }
    
    .alert {
      padding: 16px;
      margin: 16px 0;
      border-radius: 6px;
      border: 1px solid;
    }
    
    .alert-info {
      color: #0969da;
      background-color: #ddf4ff;
      border-color: #54aeff;
    }
    
    .alert-warning {
      color: #9a6700;
      background-color: #fff8c5;
      border-color: #f3c666;
    }
    
    .alert-danger {
      color: #cf222e;
      background-color: #ffebe9;
      border-color: #ff8182;
    }
    
    .alert-success {
      color: #116329;
      background-color: #dafbe1;
      border-color: #4ac26b;
    }

    .chat-message {
      margin-bottom: 20px;
      animation: fadeIn 0.3s ease;
    }
    
    @keyframes fadeIn {
      from { opacity: 0; transform: translateY(10px); }
      to { opacity: 1; transform: translateY(0); }
    }
    
    .system {
      background-color: #f2f2f2;
      border-radius: 12px;
      padding: 12px 16px;
      max-width: 85%;
    }
    
    .user {
      background-color: #e1f5fe;
      border-radius: 12px;
      padding: 12px 16px;
      max-width: 85%;
      margin-left: auto;
    }
    
    .message-content {
      margin-bottom: 5px;
        font-size: ${fontSizes.base};
    }
    
    .message-time {
      font-size: ${fontSizes.sm};
      color: #666;
      text-align: right;
    }
  `;

    // 深色主题样式覆盖
    const darkStyles = useDarkTheme ? `
    /* 深色主题样式覆盖 */
    body {
      color: #f1f5f9;
      background-color: #111827;
    }
    
    h1, h2 { 
      border-bottom-color: #30363d; 
    }
    
    h6 { 
      color: #8b949e; 
    }
    
    a {
      color: #58a6ff;
    }
    
    th, td {
      border-color: #30363d;
    }
    
    th {
      background-color: #161b22;
    }
    
    tr:nth-child(2n) {
      background-color: #161b22;
    }
    
    pre {
      background-color: #1e293b;
    }
    
    code {
      background-color: rgba(110, 118, 129, 0.4);
    }
    
    blockquote {
      color: #8b949e;
      border-left-color: #3b3b3b;
    }
    
    hr {
      background-color: #30363d;
    }
    
    del {
      color: #f85149;
    }
    
    ins {
      color: #3fb950;
      background-color: rgba(46, 160, 67, 0.15);
    }
    
    mark {
      background-color: rgba(187, 128, 9, 0.15);
      color: #e3b341;
    }
    
    .alert-info {
      color: #58a6ff;
      background-color: rgba(56, 139, 253, 0.15);
      border-color: rgba(56, 139, 253, 0.4);
    }
    
    .alert-warning {
      color: #e3b341;
      background-color: rgba(187, 128, 9, 0.15);
      border-color: rgba(187, 128, 9, 0.4);
    }
    
    .alert-danger {
      color: #f85149;
      background-color: rgba(248, 81, 73, 0.15);
      border-color: rgba(248, 81, 73, 0.4);
    }
    
    .alert-success {
      color: #3fb950;
      background-color: rgba(46, 160, 67, 0.15);
      border-color: rgba(46, 160, 67, 0.4);
    }
    
    .system {
      background-color: #2d333b;
    }
    
    .user {
      background-color: #254254;
    }
    
    .message-time {
      color: #aaa;
    }
  ` : '';

    return `<style>${baseStyles}${darkStyles}</style>`;
}

/**
 * 获取用于插入到HTML中的样式标签
 * @param options 样式选项
 * @returns 包含样式的HTML字符串
 */
export function getStyleTag(options?: MarkdownStyleOptions): string {
    return getMarkdownStyles(options);
}
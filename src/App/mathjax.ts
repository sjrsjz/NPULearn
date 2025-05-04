

// 加载 MathJax
function loadMathJax() {
    return new Promise<void>((resolve) => {
        // 如果已经加载过，直接返回
        if (window.MathJax) {
            resolve();
            return;
        }

        // 配置 MathJax
        window.MathJax = {
            tex: {
                inlineMath: [['$', '$'], ['\\(', '\\)']],
                displayMath: [['$$', '$$'], ['\\[', '\\]']]
            },
            svg: {
                fontCache: 'global'
            },
            startup: {
                pageReady: () => {
                    return window.MathJax.startup.defaultPageReady().then(() => {
                        resolve();
                    });
                },
                defaultPageReady: () => {
                    // 这里可以添加其他初始化代码
                    return Promise.resolve();
                }
            }
        };

        // 创建脚本元素
        const script = document.createElement('script');
        script.src = 'https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-svg.js';
        script.async = true;
        script.id = 'mathjax-script';
        document.head.appendChild(script);
    });
}

// 在需要时渲染数学公式
function renderMathInElement() {
    if (window.MathJax && window.MathJax.typesetPromise) {
        window.MathJax.typesetPromise([document.querySelector('.chat-messages') as HTMLElement]).catch((err: Error) => {
            console.error('MathJax 渲染错误:', err);
        });
    }
}

export { loadMathJax, renderMathInElement };
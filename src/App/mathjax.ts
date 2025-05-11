import 'mathjax/es5/tex-svg'; // 引入 MathJax 的核心库


// 加载并初始化 MathJax
function loadMathJax(timeout = 5000) {
    return new Promise<void>((resolve, reject) => {
        // 如果已经加载过并初始化，直接返回
        if (window.MathJax && window.MathJax.typesetPromise) {
            resolve();
            return;
        }

        // 设置超时处理
        const timeoutId = setTimeout(() => {
            reject(new Error('MathJax 初始化超时'));
        }, timeout);

        // 配置 MathJax (如果还未配置)
        if (!window.MathJax) {
            window.MathJax = {
                tex: {
                    inlineMath: [['$', '$'], ['\\(', '\\)']],
                    displayMath: [['$$', '$$'], ['\\[', '\\]']],
                    processEscapes: true,
                },
                svg: {
                    fontCache: 'global',
                }
            };
        }

        // 检查 MathJax 是否已准备好
        const checkMathJax = () => {
            if (window.MathJax && window.MathJax.typesetPromise) {
                clearTimeout(timeoutId);
                resolve();
            } else {
                // 继续等待直到超时
                setTimeout(checkMathJax, 100);
            }
        };

        // 开始检查
        checkMathJax();
    });
}

// 在需要时渲染数学公式
function renderMathInElement() {
    console.log('开始渲染 MathJax 公式...');
    try {
        if (window.MathJax && window.MathJax.typesetPromise) {
            const element = document.querySelectorAll('.chat-messages');
            if (element) {
                window.MathJax.typesetPromise([element])
                    .catch((err: Error) => {
                        console.error('MathJax 渲染错误:', err);
                    });
            }
        } else {
            console.warn('MathJax 尚未完成初始化，无法渲染');
        }
    } catch (error) {
        console.error('MathJax 渲染意外错误:', error);
    }
}

export { loadMathJax, renderMathInElement };
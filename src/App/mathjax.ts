// import 'mathjax/es5/tex-svg'; // 引入 MathJax 的核心库 - 将其改为动态导入


// 加载并初始化 MathJax
function loadMathJax(timeout = 5000) {
    return new Promise<void>((resolve, reject) => {
        // 如果已经加载过并初始化，直接返回
        if (window.MathJax && (window.MathJax as any).typesetPromise) {
            resolve();
            return;
        }

        // 设置超时处理
        const timeoutId = setTimeout(() => {
            reject(new Error('MathJax 初始化超时'));
        }, timeout);

        // 配置 MathJax (必须在加载 MathJax 之前完成)
        if (!window.MathJax) {
            (window as any).MathJax = {
                tex: {
                    inlineMath: [['$', '$'], ['\(', '\)']],
                    displayMath: [['$$', '$$'], ['\[', '\]']],
                    processEscapes: true,
                },
                svg: {
                    fontCache: 'global',
                },
                options: {
                    // 忽略特定的元素和类，确保 MathJax 不会处理工具代码容器、用户消息及其子元素
                    ignoreHtmlClass: 'tool-code-container|ast-tree-view|json-details|tool-code-original|tool-code-ast|katex-container|mathjax-ignore|user|user-message-right|message-wrapper.user',
                    processHtmlClass: 'assistant|message-wrapper.assistant'
                }
            };
        }

        // 动态导入 MathJax 以确保配置先生效
        import('mathjax/es5/tex-svg' as any).then(() => {
            // 检查 MathJax 是否已准备好
            const checkMathJax = () => {
                if (window.MathJax && (window.MathJax as any).typesetPromise) {
                    clearTimeout(timeoutId);
                    resolve();
                } else {
                    // 继续等待直到超时
                    setTimeout(checkMathJax, 100);
                }
            };
            // 开始检查
            checkMathJax();
        }).catch(err => {
            clearTimeout(timeoutId);
            reject(new Error(`MathJax 动态加载失败: ${err}`));
        });
    });
}

// 在需要时渲染数学公式
function renderMathInElement() {
    console.log('开始渲染 MathJax 公式...');
    try {
        if (window.MathJax && (window.MathJax as any).typesetPromise) {
            // 只选择助手消息容器，排除用户消息
            const assistantMessages = document.querySelectorAll('.chat-messages .message-wrapper.assistant');
            if (assistantMessages && assistantMessages.length > 0) {
                // 为每个助手消息容器创建一个处理函数
                const elementsToProcess: HTMLElement[] = [];
                
                assistantMessages.forEach(messageWrapper => {
                    // 从每个助手消息中排除 tool-code-container 及其子元素
                    const toolCodeContainers = messageWrapper.querySelectorAll('.tool-code-container');
                    
                    // 临时标记 tool-code-container 为忽略类
                    toolCodeContainers.forEach(container => {
                        container.classList.add('mathjax-ignore');
                        // 也为其所有子元素添加忽略标记
                        const allChildren = container.querySelectorAll('*');
                        allChildren.forEach(child => {
                            child.classList.add('mathjax-ignore');
                        });
                    });
                    
                    // 类型断言：确保 messageWrapper 是 HTMLElement
                    if (messageWrapper instanceof HTMLElement) {
                        elementsToProcess.push(messageWrapper);
                    }
                });
                
                (window.MathJax as any).typesetPromise(elementsToProcess)
                    .then(() => {
                        // 处理完成后移除临时标记
                        const ignoredElements = document.querySelectorAll('.mathjax-ignore');
                        ignoredElements.forEach(element => {
                            element.classList.remove('mathjax-ignore');
                        });
                        console.log('MathJax 渲染完成，只处理了助手消息');
                    })
                    .catch((err: Error) => {
                        console.error('MathJax 渲染错误:', err);
                        // 即使出错也要清理临时标记
                        const ignoredElements = document.querySelectorAll('.mathjax-ignore');
                        ignoredElements.forEach(element => {
                            element.classList.remove('mathjax-ignore');
                        });
                    });
            } else {
                console.log('没有找到助手消息，跳过 MathJax 渲染');
            }
        } else {
            console.warn('MathJax 尚未完成初始化，无法渲染');
        }
    } catch (error) {
        console.error('MathJax 渲染意外错误:', error);
    }
}

export { loadMathJax, renderMathInElement };
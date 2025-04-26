// 为Web Worker创建一个单独文件
// d:\VS code Projects\tauri\NPULearn\src\workers\renderWorker.ts

const ctx: Worker = self as any;

// 处理来自主线程的消息
ctx.addEventListener('message', async (event) => {
  const { type, data } = event.data;
  
  switch (type) {
    case 'render_mermaid':
      try {
        const { id, content } = data;
        // 非DOM相关的预处理逻辑
        // 例如可以在这里进行图表语法验证或其他计算密集型任务
        const processedContent = processMermaidContent(content);
        
        ctx.postMessage({
          type: 'render_result',
          success: true,
          id,
          data: { content: processedContent }
        });
      } catch (error: any) {
        ctx.postMessage({
          type: 'render_result',
          success: false,
          error: error.message
        });
      }
      break;
      
    case 'process_content':
      try {
        // 这里处理内容分析，不涉及DOM操作
        const processedContent = processContent(data.content);
        ctx.postMessage({
          type: 'process_result',
          success: true,
          data: processedContent
        });
      } catch (error : any) {
        ctx.postMessage({
          type: 'process_result',
          success: false,
          error: error.message
        });
      }
      break;
  }
});

// 处理Mermaid内容但不涉及DOM
function processMermaidContent(content: string): string {
  // 简单的语法检查和预处理
  if (!content || content.trim() === '') {
    throw new Error('空的图表内容');
  }
  
  // 这里可以添加更多的图表预处理逻辑
  // 例如：验证图表语法、优化图表性能等
  return content.trim();
}

// 处理其他内容但不涉及DOM
function processContent(content: string): string {
  // 内容预处理逻辑
  return content;
}

export {};
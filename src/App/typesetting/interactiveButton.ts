import { AppEvents, isStreaming } from "../eventBus";

function escapeHtml(str: string): string {
    return str
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
}
/**
 * 处理interactive_button API调用
 * @param apiInfo API调用信息
 * @returns 生成的HTML内容
 */
export async function handleInteractiveButton(apiInfo: any): Promise<string> {
    // 获取参数
    const message = apiInfo.arguments.message || '点击发送';
    const command = apiInfo.arguments.command || '';

    // 编码命令，用于button属性
    const encodedCommand = encodeURIComponent(command);

    // 创建唯一ID用于按钮定位
    const buttonId = `interactive-button-${Date.now()}-${Math.floor(Math.random() * 10000)}`;

    // 构建HTML - 使用与button://链接处理相同的类和属性
    const html = `
    <div class="special-api-call interactive-button-api-call">
      <div class="api-call-header">
        <span class="api-call-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
            <path d="M7 11v2"></path>
            <path d="M11 7h2"></path>
            <path d="M11 15h2"></path>
            <path d="M15 11v2"></path>
          </svg>
        </span>
        <span class="api-call-title">交互按钮</span>
      </div>
      <div class="interactive-button-container">
        <button id="${buttonId}" class="markdown-button interactive-command-button" data-command="${encodedCommand}">${escapeHtml(message)}</button>
      </div>
      <div class="api-call-footer">
        <details>
          <summary>查看按钮配置</summary>
          <pre class="api-call-code"><code>消息: ${escapeHtml(message)}
命令: ${escapeHtml(command)}</code></pre>
        </details>
      </div>
    </div>
  `;

    // 使用事件委托，不是直接在nextTick中尝试绑定
    if (!isStreaming.value && !document.querySelector('#interactive-button-handler')) {
        registerButtonEventHandler();
    }

    return html;
}

// 添加这个新函数来注册全局事件处理器
function registerButtonEventHandler() {
    // 创建一个标记元素，表示已经注册了事件处理器
    const marker = document.createElement('div');
    marker.id = 'interactive-button-handler';
    marker.style.display = 'none';
    document.body.appendChild(marker);

    // 使用事件委托来处理所有交互按钮的点击
    document.body.addEventListener('click', async (e) => {
        const target = e.target as HTMLElement;

        // 检查点击的元素是否是交互按钮
        if (target && (
            target.classList.contains('interactive-command-button') ||
            target.closest('.interactive-command-button')
        )) {
            const button = target.classList.contains('interactive-command-button') ?
                target : target.closest('.interactive-command-button') as HTMLElement;

            e.preventDefault();

            // 如果正在流式输出消息，禁止发送新消息
            if (isStreaming.value) {
                AppEvents.showNotification("请等待当前消息输出完成", "error");
                return;
            }

            const encodedCommand = button.getAttribute('data-command');
            if (encodedCommand) {
                const cmd = decodeURIComponent(encodedCommand);
                if (cmd.trim()) {
                    // 发送消息
                    await AppEvents.sendStreamMessageDirect("> " + cmd);
                    AppEvents.showNotification("已发送命令", "success");
                }
            }
        }
    });

    console.log("交互按钮全局事件处理器已注册");
}
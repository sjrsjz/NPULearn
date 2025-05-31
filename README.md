# NPULearn

NPULearn 是一个功能强大的智能学习助手桌面应用，基于 Tauri + Vue + TypeScript 构建，集成了多种AI模型和学习工具，为用户提供全面的学习支持。

## ✨ 主要功能

### 🤖 AI 聊天助手
- 支持多种 AI 模型：DeepSeek、Gemini
- 流式响应，实时对话体验
- 聊天历史管理
- 支持消息重新生成
- 内置COT（思维链）功能，增强问题解决能力

### 📚 文档处理
- ✅ **Word 文档处理**：支持 DOCX 文件解析
- ✅ **文本文件支持**：纯文本文件内容提取
- 🚧 **PDF 文档阅读**：支持 PDF 文件内容提取和分析

### 📊 数学与科学计算
- ✅ **Wolfram Alpha 搜索显示**：在前端直接显示 Wolfram Alpha 查询结果
- 🚧 **Wolfram Alpha 集成**：强大的数学计算和科学查询
- 🚧 **在线 Python 执行**：支持代码运行和结果展示
- 🚧 **MathJax 数学公式渲染**：完美显示 LaTeX 数学表达式
- 🚧 **数学世界搜索**：快速查找数学概念和定义

### 🎨 富文本渲染
- ✅ **Markdown 支持**：完整的 Markdown 语法渲染
- ✅ **代码高亮**：多语言代码语法高亮显示
- ✅ **Mermaid 图表**：流程图、时序图等图表渲染
- ✅ **Pintora 图表**：多种图表类型支持
- ✅ **LaTeX 支持**：数学公式和符号渲染
- ✅ **交互式按钮**：支持按钮交互和操作
- 🚧 **Typst 排版**：现代化的文档排版系统

### 🛠️ 工具集成
- **剪贴板管理**：智能复制粘贴功能
- **文件对话框**：便捷的文件选择和保存
- **截图功能**：支持聊天内容截图保存
- **主题切换**：支持亮色/暗色主题

## 🚀 技术栈

### 前端
- **Vue 3** + **TypeScript**：现代化的前端框架
- **Vite**：快速的构建工具
- **Tauri**：跨平台桌面应用框架

### 后端 (Rust)
- **Tauri 核心**：应用主体框架
- **Reqwest**：HTTP 客户端
- **Tokio**：异步运行时
- **Serde**：序列化/反序列化
- **Comrak**：Markdown 解析

### AI 集成
- DeepSeek API
- Google Gemini API

## 📦 项目结构

```
NPULearn/
├── src/                    # Vue 前端源码
│   ├── components/         # Vue 组件
│   ├── composables/        # Vue 组合式函数
│   ├── App/               # 应用核心逻辑
│   │   ├── typesetting/   # 排版渲染模块
│   │   └── chatHistory.ts # 聊天历史管理
│   └── workers/           # Web Workers
├── src-tauri/             # Tauri Rust 后端
│   ├── src/
│   │   ├── aibackend/     # AI 后端集成
│   │   ├── ai_utils/      # AI 工具模块
│   │   ├── document_reader/ # 文档读取
│   │   ├── document_renderer/ # 文档渲染
│   │   └── history_msg/   # 历史消息管理
│   └── gen/               # 生成的代码
└── README.md
```

## 🛠️ 开发环境配置

### 推荐 IDE 设置
- [VS Code](https://code.visualstudio.com/)
- [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar) - Vue 支持
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) - Tauri 开发支持
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) - Rust 语言支持

### 环境要求
- Node.js 18+
- Rust 1.70+
- Bun

## 🚀 快速开始

### 1. 克隆项目
```bash
git clone <repository-url>
cd NPULearn
```

### 2. 安装依赖
```bash
bun install
```

### 3. 开发模式运行
```bash
bun run tauri dev
```

### 4. 构建应用
```bash
bun run tauri build
```

## ⚙️ 配置说明

### API 密钥配置
在应用的设置界面中配置以下 API 密钥：
- **DeepSeek API Key**：用于 DeepSeek AI 模型
- **Gemini API Key**：用于 Google Gemini 模型

### 主题设置
- 支持自动跟随系统主题
- 手动切换亮色/暗色主题
- 自定义字体设置

## 📱 平台支持

- ✅ Windows
- ✅ Linux
- ✅ Android
- 🚧 macOS

## 🤝 贡献

欢迎提交 Issue 和 Pull Request 来帮助改进这个项目！

## 📄 许可证

本项目采用 **CC BY-NC-SA 4.0** (Creative Commons Attribution-NonCommercial-ShareAlike 4.0) 许可证。

### 许可证要点：
- ✅ **自由使用**：允许个人和非商业用途
- ✅ **修改分发**：允许修改并分享，但需保持相同许可证
- ✅ **署名要求**：使用时需要适当署名
- ❌ **商业限制**：不允许商业用途，除非获得明确授权
- 📧 **商业授权**：如需商业使用，请联系项目维护者获取授权

查看 [LICENSE](LICENSE) 文件了解详情，或访问 [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) 查看完整许可证条款。

## 🔗 相关链接

- [Tauri 官方文档](https://tauri.app/)
- [Vue 3 文档](https://vuejs.org/)
- [TypeScript 文档](https://www.typescriptlang.org/)


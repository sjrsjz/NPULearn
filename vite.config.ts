import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [vue(), wasm(), topLevelAwait()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  // 添加优化配置来解决依赖问题
  optimizeDeps: {
    include: [
      '@myriaddreamin/typst.ts',
      '@myriadddreamin/typst-ts-web-compiler',
      '@myriadddreamin/typst-ts-renderer',
      'entities',
      'entities/dist/decode.js',
      'entities/dist/escape.js'
    ],
    exclude: ['@myriaddreamin/typst-ts-node-compiler']
  },
  
  // 添加解析别名
  resolve: {
    alias: {
      'entities/decode': 'entities/dist/decode.js',
      'entities/escape': 'entities/dist/escape.js'
    }
  },
    // 添加构建配置
  build: {
    commonjsOptions: {
      include: [/entities/, /node_modules/]
    },
    // 确保字体文件被正确复制
    assetsInclude: ['**/*.ttf', '**/*.woff', '**/*.woff2', '**/*.otf']
  },
  
  // 添加静态资源处理
  assetsInclude: ['**/*.ttf', '**/*.woff', '**/*.woff2', '**/*.otf'],
  
  // 定义全局变量
  define: {
    global: 'globalThis',
  }
}));

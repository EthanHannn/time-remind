import vue from '@vitejs/plugin-vue'
import UnoCSS from 'unocss/vite'
import { defineConfig } from 'vite'

declare const process: { env: Record<string, string | undefined> }

const host = process.env.TAURI_DEV_HOST

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    vue(),
    UnoCSS(),
  ],

  // 防止 Vite 掩盖 Rust 错误
  clearScreen: false,
  // 确保 Tauri 多窗口入口都被构建
  build: {
    rollupOptions: {
      input: {
        main: 'index.html',
        notification: 'notification.html',
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
}))

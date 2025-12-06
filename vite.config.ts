import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const host = process.env.TAURI_DEV_HOST

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Set base path for Tauri
  base: './',

  // CSS 优化配置
  css: {
    postcss: {
      plugins: [
        (await import('@tailwindcss/postcss')).default,
        (await import('autoprefixer')).default,
      ],
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 55323,
    strictPort: false,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 55324,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
  build: {
    // Ensure assets are properly referenced
    assetsDir: 'assets',
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          // React core libraries
          if (id.includes('react') || id.includes('react-dom')) {
            return 'vendor'
          }
          // Tauri API
          if (id.includes('@tauri-apps/api')) {
            return 'tauri'
          }
          // UI components and icons
          if (id.includes('lucide-react') || id.includes('antd')) {
            return 'ui'
          }
          // Markdown processing
          if (id.includes('react-markdown') ||
              id.includes('rehype-raw') ||
              id.includes('rehype-sanitize') ||
              id.includes('remark-gfm')) {
            return 'markdown'
          }
          // Utility libraries
          if (id.includes('uuid')) {
            return 'vendor' // 合并到 vendor 中，避免空 chunk
          }
          // i18n
          if (id.includes('react-i18next') || id.includes('i18next')) {
            return 'i18n'
          }
        },
      },
    },
    // Optimize chunks for better caching - 提高限制避免警告
    chunkSizeWarningLimit: 1000,
    // CSS 代码分割
    cssCodeSplit: true,
  },
}))

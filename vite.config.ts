import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const host = process.env.TAURI_DEV_HOST

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Set base path for Tauri
  base: './',

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
        manualChunks: {
          // React core libraries
          vendor: ['react', 'react-dom'],
          // Tauri API
          tauri: ['@tauri-apps/api'],
          // UI components and icons
          ui: ['lucide-react'],
          // Markdown processing
          markdown: [
            'react-markdown',
            'rehype-raw',
            'rehype-sanitize',
            'remark-gfm',
          ],
          // Utility libraries
          utils: ['uuid'],
        },
      },
    },
    // Optimize chunks for better caching
    chunkSizeWarningLimit: 600,
  },
}))

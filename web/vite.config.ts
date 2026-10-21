import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// En développement, l'API Rust tourne sur le port 47899 (voir README).
const api = process.env.BOOND_API ?? 'http://127.0.0.1:47899'

export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5173,
    proxy: { '/api': { target: api, changeOrigin: true } },
  },
  build: { outDir: 'dist', emptyOutDir: true },
})

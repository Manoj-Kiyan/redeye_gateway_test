import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) {
            return undefined
          }

          if (id.includes('recharts') || id.includes('d3-')) {
            return 'charts'
          }

          if (id.includes('react-router') || id.includes('@remix-run/router')) {
            return 'router'
          }

          if (id.includes('lucide-react')) {
            return 'icons'
          }

          if (
            id.includes('/react/') || id.includes('\\react\\')
            || id.includes('/react-dom/') || id.includes('\\react-dom\\')
            || id.includes('/scheduler/') || id.includes('\\scheduler\\')
          ) {
            return 'react-core'
          }

          return 'vendor'
        },
      },
    },
  },
})

import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  plugins: [vue()],
  define: {
    'process.env': {},
    'process.env.NODE_ENV': JSON.stringify('production')
  },
  build: {
    outDir: 'dist',
    lib: {
      entry: 'src/main.js',
      name: 'IncrementalApp',
      formats: ['iife'],
      fileName: () => 'incremental_update.js'
    },
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
        assetFileNames: (assetInfo) => {
          if (assetInfo.name === 'style.css') return 'incremental_update.css';
          return assetInfo.name;
        },
        globals: {
          echarts: 'echarts'
        }
      },
      external: ['echarts']
    },
    cssCodeSplit: false,
    minify: 'terser',
    terserOptions: {
      compress: {
        drop_console: true,
        drop_debugger: true
      }
    }
  },
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true
      },
      '/ws': {
        target: 'ws://localhost:8080',
        ws: true,
        changeOrigin: true
      }
    }
  }
});

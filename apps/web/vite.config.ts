import { fileURLToPath, URL } from 'node:url'
import vue from '@vitejs/plugin-vue'
import { VitePWA } from 'vite-plugin-pwa'
import { defineConfig } from 'vitest/config'

const backend = 'http://127.0.0.1:8080'

export default defineConfig({
  plugins: [
    vue(),
    VitePWA({
      registerType: 'autoUpdate',
      manifest: {
        name: 'Family Assistant',
        short_name: 'Family',
        description: 'Self-hosted family organiser',
        start_url: '/',
        display: 'standalone',
        background_color: '#ffffff',
        theme_color: '#2f6fed',
        icons: [
          { src: 'icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: 'icon-512.png', sizes: '512x512', type: 'image/png' },
          {
            src: 'icon-maskable-512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'maskable',
          },
        ],
      },
      workbox: {
        navigateFallbackDenylist: [/^\/api\//, /^\/healthz/, /^\/readyz/],
        runtimeCaching: [
          {
            // Avatars carry a ?v=<hash> cache-buster, so cache-first is safe.
            urlPattern: ({ url }) => /^\/api\/v1\/members\/[^/]+\/avatar$/.test(url.pathname),
            handler: 'CacheFirst',
            options: {
              cacheName: 'fa-avatars',
              expiration: { maxEntries: 64, maxAgeSeconds: 30 * 24 * 3600 },
              cacheableResponse: { statuses: [200] },
            },
          },
        ],
      },
    }),
  ],
  resolve: { alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) } },
  server: {
    proxy: {
      '/api': backend,
      '/healthz': backend,
      '/readyz': backend,
    },
  },
  test: { environment: 'jsdom', include: ['src/**/*.test.ts'] },
})

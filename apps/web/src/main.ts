import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'
import { registerServiceWorker } from './lib/pwa'
import { router } from './router'
import './styles/base.css'

createApp(App).use(createPinia()).use(router).mount('#app')
// No service worker under the Vite dev server: its cache would defeat HMR.
if (!import.meta.env.DEV) void registerServiceWorker()

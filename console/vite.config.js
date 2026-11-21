import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import path from 'node:path';
// https://vite.dev/config/
export default defineConfig({
    plugins: [vue()],
    resolve: {
        alias: {
            '@': path.resolve(__dirname, './src'),
        },
    },
    server: {
        port: 3000,
        host: '127.0.0.1',
        proxy: {
            '/api': {
                target: 'http://127.0.0.1:9876',
                changeOrigin: true,
            },
        },
    },
});

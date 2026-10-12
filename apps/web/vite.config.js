import preact from '@preact/preset-vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [preact()],
  server: { port: 1421, strictPort: true },
  optimizeDeps: {
    exclude: ['@rvaultpass/ui'],
  },
});

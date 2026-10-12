import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import preact from '@preact/preset-vite';
import { defineConfig } from 'vite';

// Le service worker et le script injecté par `scripting.executeScript` ne chargent pas
// d'`import` : chacun est construit à part, en un seul fichier autonome.
const SCRIPTS = {
  background: 'src/background.ts',
  content: 'src/content.ts',
};

export default defineConfig(({ mode }) => {
  const script = SCRIPTS[mode];
  if (script) {
    return {
      build: {
        outDir: 'dist',
        emptyOutDir: false,
        modulePreload: false,
        rollupOptions: {
          input: resolve(script),
          output: {
            format: 'iife',
            entryFileNames: `${mode}.js`,
            inlineDynamicImports: true,
          },
        },
      },
    };
  }

  return {
    base: './',
    plugins: [
      preact(),
      {
        name: 'copy-manifest',
        generateBundle() {
          this.emitFile({
            type: 'asset',
            fileName: 'manifest.json',
            source: readFileSync(resolve('manifest.json'), 'utf8'),
          });
        },
        transformIndexHtml(html) {
          return html.replaceAll(' crossorigin', '');
        },
      },
    ],
    build: {
      outDir: 'dist',
      emptyOutDir: true,
      modulePreload: false,
      rollupOptions: {
        input: {
          popup: resolve('popup.html'),
          options: resolve('options.html'),
        },
        output: {
          entryFileNames: '[name].js',
          chunkFileNames: 'chunks/[name]-[hash].js',
        },
      },
    },
    optimizeDeps: {
      exclude: ['@rvaultpass/ui'],
    },
  };
});

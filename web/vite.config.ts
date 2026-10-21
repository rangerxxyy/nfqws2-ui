import { defineConfig, type Plugin } from 'vite';

const devHtml = `<!doctype html>
<html lang="ru" class="dark">
  <head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>nfqws2-UI — Панель управления</title></head>
  <body><div id="root"></div><script type="module" src="/@vite/client"></script><script type="module" src="/src/main.tsx"></script></body>
</html>`;

const previewHtml = `<!doctype html>
<html lang="ru" class="dark">
  <head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>nfqws2-UI — Панель управления</title><link rel="stylesheet" href="/nfqws2-ui.css"></head>
  <body><div id="root"></div><script type="module" src="/nfqws2-ui.js"></script></body>
</html>`;

const serveDevShell: Plugin = {
  name: 'nfqws2-ui-typescript-shell',
  configureServer(server) {
    server.middlewares.use((request, response, next) => {
      if ((request.url ?? '').split('?')[0] !== '/') return next();
      response.statusCode = 200;
      response.setHeader('Content-Type', 'text/html; charset=utf-8');
      response.end(devHtml);
    });
  },
  configurePreviewServer(server) {
    server.middlewares.use((request, response, next) => {
      if ((request.url ?? '').split('?')[0] !== '/') return next();
      response.statusCode = 200;
      response.setHeader('Content-Type', 'text/html; charset=utf-8');
      response.end(previewHtml);
    });
  },
};

export default defineConfig({
  plugins: [serveDevShell],
  server: { proxy: { '/api': 'http://127.0.0.1:1002' } },
  build: {
    lib: {
      entry: 'src/main.tsx',
      formats: ['es'],
      fileName: () => 'nfqws2-ui.js',
      cssFileName: 'nfqws2-ui',
    },
    rollupOptions: {
      output: {
        assetFileNames: (asset) => asset.name?.endsWith('.css')
          ? 'nfqws2-ui.css'
          : 'assets/[name]-[hash][extname]',
      },
    },
  },
});

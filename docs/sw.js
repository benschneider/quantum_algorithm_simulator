const release = "7e02e263f20e89f32aad";
const assets = {
  "assets/icon-1024.png": "sha256-WBeTuPxnoJf9N2ehTH46yLkJi6x3b7C6OSeGUuJtCYw=",
  "assets/icon-256.png": "sha256-yLJhxxdRkOZhIjIBx58yDAxs5YvG0jiGt1ADdqtvI84=",
  "assets/icon_ios_touch_192.png": "sha256-mlxZweOXmEpGx0vRh5G+2K9hM0rhGTAp+SIbXLGQHk8=",
  "assets/maskable_icon_x512.png": "sha256-lGLrobb68qx9uH7844jE0/rkR6r+b0OlAWOttveI0Fs=",
  "favicon-cb90114adb4e01ff.ico": "sha256-/rsfNouAIJL350SASJushWrZTH7rf102Rv5EW5ZPDv4=",
  "index.html": "sha256-LIzgm2pfwqg9iUwJlTw05TMIXQreSsPXdIGTlk4PeO4=",
  "manifest.json": "sha256-3r9D/QXU8yIfnGNMUWmcyd3/vCDfto0j1lcDfOVG15U=",
  "quantsim_ui-f44bcb5a43adf9c3.js": "sha256-xleYEOBlsdmCi5+WqlNlJe8tn4kpclK4ytEV2+kePDE=",
  "quantsim_ui-f44bcb5a43adf9c3_bg.wasm": "sha256-G8d4ocrfrP6f/4i6NLNcOzjy+2jbAvLfjL9EThp4iWc="
};

// The Trunk post-build hook supplies the release fingerprint and asset integrity hashes.
const scope = self.registration.scope;
const cachePrefix = `quantsim-ui:${scope}:`;
const cacheName = cachePrefix + release;
const indexURL = new URL('index.html', scope).href;
const assetURLs = new Map(Object.entries(assets).map(([path, integrity]) =>
  [new URL(path, scope).href, integrity]));

self.addEventListener('install', event => {
  event.waitUntil((async () => {
    const cache = await caches.open(cacheName);
    const requests = Array.from(assetURLs, ([url, integrity]) =>
      new Request(url, { integrity, cache: 'no-cache' }));
    await cache.addAll(requests);
    await self.skipWaiting();
  })());
});

self.addEventListener('activate', event => {
  event.waitUntil((async () => {
    const old = (await caches.keys()).filter(key => key.startsWith(cachePrefix) && key !== cacheName);
    await Promise.all(old.map(key => caches.delete(key)));
    await self.clients.claim();
  })());
});

self.addEventListener('fetch', event => {
  const request = event.request;
  if (request.method !== 'GET') return;

  const url = new URL(request.url);
  if (request.mode === 'navigate' && [scope, indexURL].includes(url.origin + url.pathname)) {
    // Online navigation picks up the latest HTML and its matching asset URLs.
    event.respondWith(fetch(request, { cache: 'no-cache' })
      .catch(() => caches.match(indexURL, { cacheName })));
  } else if (assetURLs.has(url.href)) {
    event.respondWith(caches.match(request, { cacheName })
      .then(cached => cached || fetch(request)));
  }
});

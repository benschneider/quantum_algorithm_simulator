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

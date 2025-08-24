
// https://jakearchibald.com/2014/offline-cookbook/#stale-while-revalidate

self.addEventListener('install', (event) => {
  event.waitUntil(
    (async function () {
      const cache = await caches.open(version);
      await cache.addAll(offlineFundamentals);
    })(),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async function () {
      const cacheNames = await caches.keys();
      await Promise.all(
        cacheNames
          .filter((cacheName) => {
            // Return true if you want to remove this cache,
            // but remember that caches are shared across
            // the whole origin
            version !== cacheName
          })
          .map((cacheName) => caches.delete(cacheName)),
      );
    })(),
  );
});


self.addEventListener('fetch', (event) => {
  event.respondWith(
    (async function () {
      const cache = await caches.open('mysite-dynamic');
      const cachedResponse = await cache.match(event.request);
      const networkResponsePromise = fetch(event.request);

      event.waitUntil(
        (async function () {
          const networkResponse = await networkResponsePromise;
          await cache.put(event.request, networkResponse.clone());
        })(),
      );

      // Returned the cached response if we have one, otherwise return the network response.
      return cachedResponse || networkResponsePromise;
    })(),
  );
});
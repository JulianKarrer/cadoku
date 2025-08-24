const version = "2025-08-24T17:06:41Z";
const offlineFundamentals = [
  "cadoku/assets/cadoku-bda12f260fd53d56.js.br",
  "cadoku/assets/favicon-1850237cac6c2879.ico",
  "cadoku/assets/3-358e63de570efccb.avif.br",
  "cadoku/assets/0-68a74fce259332a1.avif.br",
  "cadoku/assets/main-0d40693495b7bae6.css.br",
  "cadoku/assets/9-35e905dea6c50703.avif",
  "cadoku/assets/sparkle-607887cd7f01919f.avif.br",
  "cadoku/assets/8-a542e42ec631dd4d.avif",
  "cadoku/assets/mascot-25764a074b3d2e80.avif.br",
  "cadoku/assets/happy-67ece4d43ab76fdb.avif.br",
  "cadoku/assets/Mooli-cd77459f196e5305.ttf.br",
  "cadoku/assets/medium-90e9017b98af726b.avif",
  "cadoku/assets/hard-3577eb93fd4d7d53.avif.br",
  "cadoku/assets/mascot-25764a074b3d2e80.avif",
  "cadoku/assets/0-68a74fce259332a1.avif",
  "cadoku/assets/cadoku_bg-2aeba8e971b4ab21.wasm",
  "cadoku/assets/5-14e942e8ed2cce37.avif",
  "cadoku/assets/hearts-d5fcecd9be6ef3a3.avif.br",
  "cadoku/assets/1-99365fe5efa41cc9.avif",
  "cadoku/assets/8-a542e42ec631dd4d.avif.br",
  "cadoku/assets/favicon-1850237cac6c2879.ico.br",
  "cadoku/assets/4-8c4ab2d29126fe2f.avif",
  "cadoku/assets/hard-3577eb93fd4d7d53.avif",
  "cadoku/assets/happy-67ece4d43ab76fdb.avif",
  "cadoku/assets/medium-90e9017b98af726b.avif.br",
  "cadoku/assets/Mooli-cd77459f196e5305.ttf",
  "cadoku/assets/easy-5952d7c9849f11c0.avif.br",
  "cadoku/assets/6-4cf0492d32234933.avif.br",
  "cadoku/assets/9-35e905dea6c50703.avif.br",
  "cadoku/assets/5-14e942e8ed2cce37.avif.br",
  "cadoku/assets/easy-5952d7c9849f11c0.avif",
  "cadoku/assets/hearts-d5fcecd9be6ef3a3.avif",
  "cadoku/assets/4-8c4ab2d29126fe2f.avif.br",
  "cadoku/assets/challenge-502fa3e987a722ce.avif.br",
  "cadoku/assets/cadoku-bda12f260fd53d56.js",
  "cadoku/assets/7-89a172ce1654dd25.avif.br",
  "cadoku/assets/7-89a172ce1654dd25.avif",
  "cadoku/assets/2-07f348f924a2a4c8.avif.br",
  "cadoku/assets/challenge-502fa3e987a722ce.avif",
  "cadoku/assets/firework-008833342ca8a869.avif.br",
  "cadoku/assets/cadoku_bg-2aeba8e971b4ab21.wasm.br",
  "cadoku/assets/3-358e63de570efccb.avif",
  "cadoku/assets/sparkle-607887cd7f01919f.avif",
  "cadoku/assets/main-0d40693495b7bae6.css",
  "cadoku/assets/firework-008833342ca8a869.avif",
  "cadoku/assets/6-4cf0492d32234933.avif",
  "cadoku/assets/2-07f348f924a2a4c8.avif",
  "cadoku/assets/1-99365fe5efa41cc9.avif.br"
];
// END OF GENERATED CACHE FILES

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

  // Skip non-GET or unsupported scheme requests
  const url = event.request.url;
  if (event.request.method !== 'GET' ||
    !(url.startsWith('http://') || url.startsWith('https://'))) {
    return;
  }

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
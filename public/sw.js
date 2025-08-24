const version = "2025-08-24T17:11:08Z";
const offlineFundamentals = [
  "assets/cadoku-bda12f260fd53d56.js.br",
  "assets/favicon-1850237cac6c2879.ico",
  "assets/3-358e63de570efccb.avif.br",
  "assets/0-68a74fce259332a1.avif.br",
  "assets/main-0d40693495b7bae6.css.br",
  "assets/9-35e905dea6c50703.avif",
  "assets/sparkle-607887cd7f01919f.avif.br",
  "assets/8-a542e42ec631dd4d.avif",
  "assets/mascot-25764a074b3d2e80.avif.br",
  "assets/happy-67ece4d43ab76fdb.avif.br",
  "assets/Mooli-cd77459f196e5305.ttf.br",
  "assets/medium-90e9017b98af726b.avif",
  "assets/hard-3577eb93fd4d7d53.avif.br",
  "assets/mascot-25764a074b3d2e80.avif",
  "assets/0-68a74fce259332a1.avif",
  "assets/cadoku_bg-2aeba8e971b4ab21.wasm",
  "assets/5-14e942e8ed2cce37.avif",
  "assets/hearts-d5fcecd9be6ef3a3.avif.br",
  "assets/1-99365fe5efa41cc9.avif",
  "assets/8-a542e42ec631dd4d.avif.br",
  "assets/favicon-1850237cac6c2879.ico.br",
  "assets/4-8c4ab2d29126fe2f.avif",
  "assets/hard-3577eb93fd4d7d53.avif",
  "assets/happy-67ece4d43ab76fdb.avif",
  "assets/medium-90e9017b98af726b.avif.br",
  "assets/Mooli-cd77459f196e5305.ttf",
  "assets/easy-5952d7c9849f11c0.avif.br",
  "assets/6-4cf0492d32234933.avif.br",
  "assets/9-35e905dea6c50703.avif.br",
  "assets/5-14e942e8ed2cce37.avif.br",
  "assets/easy-5952d7c9849f11c0.avif",
  "assets/hearts-d5fcecd9be6ef3a3.avif",
  "assets/4-8c4ab2d29126fe2f.avif.br",
  "assets/challenge-502fa3e987a722ce.avif.br",
  "assets/cadoku-bda12f260fd53d56.js",
  "assets/7-89a172ce1654dd25.avif.br",
  "assets/7-89a172ce1654dd25.avif",
  "assets/2-07f348f924a2a4c8.avif.br",
  "assets/challenge-502fa3e987a722ce.avif",
  "assets/firework-008833342ca8a869.avif.br",
  "assets/cadoku_bg-2aeba8e971b4ab21.wasm.br",
  "assets/3-358e63de570efccb.avif",
  "assets/sparkle-607887cd7f01919f.avif",
  "assets/main-0d40693495b7bae6.css",
  "assets/firework-008833342ca8a869.avif",
  "assets/6-4cf0492d32234933.avif",
  "assets/2-07f348f924a2a4c8.avif",
  "assets/1-99365fe5efa41cc9.avif.br"
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
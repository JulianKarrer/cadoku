
const always_cache = [
  "./index.html",
  "./404.html",
  "./favicon.ico",
  "./cached_asset_list.js",
]

self.addEventListener("install", (e) => {
  e.waitUntil(
    caches
      .open(version)
      .then((cache) => {
        // cache files
        cache.addAll(always_cache + offlineFundamentals);
      })
      .then(() => {
        self.skipWaiting();
      }),
  );
});

self.addEventListener("activate", (e) => {
  // remove cache entries that are outdated
  e.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames.map((cache) => {
          if (cache !== version) {
            return caches.delete(cache);
          }
        }),
      );
    }),
  );
});


self.addEventListener("fetch", (e) => {
  e.respondWith(
    fetch(e.request)
      .then((res) => {
        const responseClone = res.clone();
        caches.open(version).then((cache) => {
          cache.put(e.request, responseClone);
        });
        return res;
      })
      .catch((err) => {
        caches.match(e.request).then((res) => res);
      }),
  );
});
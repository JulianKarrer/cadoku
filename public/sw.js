"use strict";

/**
 * Single, unchanging service worker.
 *
 * This SW expects a same-origin JS file `cached_asset_list.js` in the same scope
 * as the service worker file. That file must define:
 *
 *   const version = "2025-08-24T14:56:00Z";
 *   const offlineFundamentals = [ "cadoku/...", ... ];
 *
 * Your justfile can keep generating cached_asset_list.js exactly as you already do.
 */

/* ------------------------
   Config
   ------------------------ */
const CACHE_PREFIX = "cadoku-cache:"; // prefix; caches are named like `${CACHE_PREFIX}${version}::fundamentals`
const REMOTE_ASSET_LIST = new URL("cached_asset_list.js", self.location).href; // resolved relative to sw.js

/* ------------------------
   SW lifecycle: install
   ------------------------ */
self.addEventListener("install", (event) => {
  // The install step tries to fetch the asset list and pre-cache the listed assets.
  event.waitUntil((async () => {
    try {
      const data = await fetchRemoteAssetList();
      if (!data) {
        console.warn("sw: install - no remote asset list available; continuing without pre-cache.");
        return;
      }
      const { version, assets } = data;
      const fundCacheName = `${CACHE_PREFIX}${version}::fundamentals`;
      const cache = await caches.open(fundCacheName);

      // Best-effort: fetch and store assets individually (so single 404 won't abort install).
      await Promise.all(assets.map(async (asset) => {
        try {
          const r = await fetch(asset, { cache: "no-store" });
          if (r && r.ok) await cache.put(asset, r.clone());
          else console.warn("sw: install - failed to fetch asset", asset, r && r.status);
        } catch (e) {
          console.warn("sw: install - exception fetching asset", asset, e);
        }
      }));
    } catch (e) {
      console.warn("sw: install - asset-list fetch failed:", e);
      // Don't re-throw: allow SW to install so previously cached assets still work.
    }
  })());
});

/* ------------------------
   SW lifecycle: activate
   ------------------------ */
self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    // Determine the latest cached version (if any) and remove caches that don't match it:
    const current = await getHighestCachedVersion();
    if (current) {
      // delete caches not matching the current version (keep both fundamentals and pages for current)
      const keys = await caches.keys();
      await Promise.all(keys
        .filter(k => !k.startsWith(`${CACHE_PREFIX}${current}`))
        .map(k => caches.delete(k))
      );
    }
    // Claim clients so this SW controls pages immediately (no reload required to be controlled).
    await self.clients.claim();

    // Try a remote check to see if a newer version exists and prefetch it.
    try {
      await checkRemoteAssetListAndUpdateIfNeeded();
    } catch (e) {
      // non-fatal
      console.warn("sw: activate - remote check failed", e);
    }
  })());
});

/* ------------------------
   Fetch handler - caching strategies
   ------------------------ */
self.addEventListener("fetch", (event) => {
  const req = event.request;

  // Only handle GET requests
  if (req.method !== "GET") return;

  // Determine if navigation (HTML) request
  const accept = req.headers.get("Accept") || "";
  if (req.mode === "navigate" || accept.includes("text/html")) {
    // network-first for navigations: fetch latest HTML but fall back to cache for offline
    event.respondWith(networkFirst(req));
    // non-blocking update check on navigation to pick up new builds quicker
    event.waitUntil((async () => {
      try {
        await checkRemoteAssetListAndUpdateIfNeeded();
      } catch (e) {
        // ignore
      }
    })());
    return;
  }

  // For other assets: cache-first then update cache from network in background
  event.respondWith(cacheFirstThenUpdate(req));
});

/* ------------------------
   Message handler (client <> SW)
   ------------------------ */
self.addEventListener("message", (event) => {
  const msg = event.data || {};
  if (msg && msg.type === "SKIP_WAITING") {
    self.skipWaiting();
    return;
  }
  if (msg && msg.type === "CHECK_FOR_UPDATE") {
    event.waitUntil(checkRemoteAssetListAndUpdateIfNeeded()
      .catch(e => console.warn("sw: manual check failed:", e)));
    return;
  }
});

/* ------------------------
   Strategies implementations
   ------------------------ */

async function networkFirst(request) {
  try {
    const networkResponse = await fetch(request);
    // store a copy into pages cache for offline fallback
    const current = await getHighestCachedVersion();
    const pagesCacheName = current ? `${CACHE_PREFIX}${current}::pages` : null;
    if (pagesCacheName) {
      try {
        const c = await caches.open(pagesCacheName);
        // best-effort; ignore failures for opaque responses
        await c.put(request, networkResponse.clone());
      } catch (e) {
        // ignore caching error
      }
    }
    return networkResponse;
  } catch (e) {
    // network failed -> try cache
    const cached = await caches.match(request);
    if (cached) return cached;
    // last-resort offline HTML
    return new Response("<h1>Service Unavailable</h1><p>Offline</p>", {
      status: 503,
      statusText: "Service Unavailable",
      headers: { "Content-Type": "text/html" }
    });
  }
}

async function cacheFirstThenUpdate(request) {
  // caches.match searches across all caches
  const cached = await caches.match(request);
  // Kick off a fetch to update cache in background
  const fetchPromise = fetch(request).then(async (resp) => {
    if (!resp || !resp.ok) return resp;
    const current = await getHighestCachedVersion();
    const pagesCacheName = current ? `${CACHE_PREFIX}${current}::pages` : null;
    if (pagesCacheName) {
      try {
        const c = await caches.open(pagesCacheName);
        await c.put(request, resp.clone());
      } catch (e) {
        // ignore
      }
    }
    return resp;
  }).catch(() => null);

  return cached || fetchPromise;
}

/* ------------------------
   Remote asset-list fetching & update functions
   ------------------------ */

/**
 * Fetch cached_asset_list.js and parse it.
 * Returns { version: string, assets: string[] } or throws on parse error.
 */
async function fetchRemoteAssetList() {
  const res = await fetch(REMOTE_ASSET_LIST, { cache: "no-store" });
  if (!res.ok) {
    throw new Error("failed to fetch asset list: " + res.status);
  }
  const text = await res.text();

  // Parse version
  const versionMatch = text.match(/const\s+version\s*=\s*["']([^"']+)["']/);
  if (!versionMatch) throw new Error("version not found in asset list");
  const version = versionMatch[1];

  // Parse offlineFundamentals array literal
  const arrMatch = text.match(/const\s+offlineFundamentals\s*=\s*(\[[\s\S]*?\]);/);
  if (!arrMatch) throw new Error("offlineFundamentals not found in asset list");

  // Evaluate the array literal in a constrained manner.
  // This is acceptable here because cached_asset_list.js is generated by your build and same-origin.
  let assets;
  try {
    assets = (new Function("return " + arrMatch[1]))();
    if (!Array.isArray(assets)) throw new Error("offlineFundamentals is not an array");
  } catch (e) {
    throw new Error("failed to parse offlineFundamentals array: " + e.message);
  }

  return { version, assets };
}

/**
 * Determine the highest (latest) cached version present in caches.
 * We look for cache keys that match `${CACHE_PREFIX}${version}::fundamentals` and pick the max version.
 * Returns the version string or null if none found.
 */
async function getHighestCachedVersion() {
  const keys = await caches.keys();
  const versions = keys.map(k => {
    const m = k.match(new RegExp('^' + escapeRegExp(CACHE_PREFIX) + '(.+?)::fundamentals$'));
    return m ? m[1] : null;
  }).filter(Boolean);
  if (versions.length === 0) return null;
  // version strings are ISO timestamps - lexicographic max works.
  versions.sort();
  return versions[versions.length - 1];
}

/**
 * Check remote cached_asset_list.js and update caches if remote version is newer than
 * currently cached version. This will:
 *  - fetch the remote list
 *  - if remoteVersion != current cached version:
 *      * create new fundamentals cache and attempt to fetch & put each asset
 *      * create a new pages cache (empty)
 *      * delete old caches not matching the new version
 *      * postMessage to clients { type: "NEW_VERSION_AVAILABLE", version: remoteVersion }
 */
async function checkRemoteAssetListAndUpdateIfNeeded() {
  let data;
  try {
    data = await fetchRemoteAssetList();
  } catch (e) {
    // Can't fetch remote list — abort update.
    throw e;
  }

  const remoteVersion = data.version;
  const remoteAssets = data.assets;

  const current = await getHighestCachedVersion();
  if (current === remoteVersion) {
    return { updated: false, reason: "versions match" };
  }

  // Create new caches for remoteVersion
  const newFundName = `${CACHE_PREFIX}${remoteVersion}::fundamentals`;
  const newPagesName = `${CACHE_PREFIX}${remoteVersion}::pages`;
  const fundCache = await caches.open(newFundName);

  // Best-effort: fetch each asset and cache it. Do not abort entire operation if one fails.
  await Promise.all(remoteAssets.map(async (asset) => {
    try {
      const r = await fetch(asset, { cache: "no-store" });
      if (r && r.ok) {
        await fundCache.put(asset, r.clone());
      } else {
        console.warn("sw: update - asset fetch failed:", asset, r && r.status);
      }
    } catch (e) {
      console.warn("sw: update - asset fetch exception:", asset, e);
    }
  }));

  // Create pages cache (empty) for new version
  await caches.open(newPagesName);

  // Remove old caches that don't belong to new remoteVersion
  const keys = await caches.keys();
  await Promise.all(keys
    .filter(k => !k.startsWith(`${CACHE_PREFIX}${remoteVersion}`))
    .map(k => caches.delete(k))
  );

  // Notify clients (pages)
  const allClients = await self.clients.matchAll({ includeUncontrolled: true });
  for (const client of allClients) {
    client.postMessage({ type: "NEW_VERSION_AVAILABLE", version: remoteVersion });
  }

  return { updated: true, newVersion: remoteVersion };
}

/* ------------------------
   Utilities
   ------------------------ */

function escapeRegExp(string) {
  return string.replace(/[.*+\-?^${}()|[\]\\]/g, '\\$&');
}

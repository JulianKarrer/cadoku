"use strict";

/**
 * Unchanging service worker. Expects a same-origin JS file
 * `cached_asset_list.js` in the same directory/scope as this SW that defines:
 *
 *   const version = "2025-08-24T14:56:00Z";
 *   const offlineFundamentals = [ "cadoku/...", ... ];
 *
 * The SW will:
 *  - fetch the list at runtime,
 *  - pre-cache assets into a cache named "cadoku-cache:<version>::fundamentals",
 *  - serve cached assets offline,
 *  - network-first for navigations (with fallback to cached app-shell),
 *  - detect remote updates and create a new versioned cache, then delete old caches,
 *  - notify clients with postMessage({type: "NEW_VERSION_AVAILABLE", version}).
 */

/* ------------------------
   Configuration
   ------------------------ */
const CACHE_PREFIX = "cadoku-cache:";                     // caches named like `${CACHE_PREFIX}${version}::fundamentals`
const REMOTE_ASSET_LIST = new URL("cached_asset_list.js", self.location).href;

/* ------------------------
   Lifecycle: install
   ------------------------ */
self.addEventListener("install", (event) => {
  // Try to fetch and pre-cache the remote asset list. If it fails, we still install
  // so previously-cached assets continue working.
  event.waitUntil((async () => {
    try {
      const data = await fetchRemoteAssetList();
      if (!data) return; // nothing to pre-cache
      const { version, assets } = data;
      const fundCacheName = `${CACHE_PREFIX}${version}::fundamentals`;
      const cache = await caches.open(fundCacheName);

      // Also ensure app shell entries exist (scope root, index, 404, manifest)
      const scopeBase = ensureTrailingSlash(self.registration.scope || new URL('.', self.location).href);
      const appShellCandidates = [
        scopeBase,
        new URL('index.html', scopeBase).href,
        new URL('404.html', scopeBase).href,
        new URL('manifest.json', scopeBase).href
      ];

      // Normalize all assets to absolute URLs, dedupe
      const normalized = dedupeArray(
        appShellCandidates.concat(assets.map(a => new URL(a, self.location).href))
      );

      // Best-effort fetch+put each resource (so single 404 doesn't abort install)
      await Promise.all(normalized.map(async (url) => {
        try {
          const r = await fetch(url, { cache: "no-store" });
          if (r && r.ok) await cache.put(url, r.clone());
          else console.warn("sw: install - failed to fetch asset", url, r && r.status);
        } catch (e) {
          console.warn("sw: install - exception fetching asset", url, e);
        }
      }));
    } catch (e) {
      // non-fatal: allow install to succeed
      console.warn("sw: install - could not fetch asset list:", e);
    }
  })());
});

/* ------------------------
   Lifecycle: activate
   ------------------------ */
self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    // Keep only the highest-version caches and delete others
    const current = await getHighestCachedVersion();
    if (current) {
      const keys = await caches.keys();
      await Promise.all(keys
        .filter(k => !k.startsWith(`${CACHE_PREFIX}${current}`))
        .map(k => caches.delete(k))
      );
    }
    await self.clients.claim();

    // Opportunistic remote check & prefetch of new version
    try {
      await checkRemoteAssetListAndUpdateIfNeeded();
    } catch (e) {
      // ignore
      console.warn("sw: activate - remote check failed", e);
    }
  })());
});

/* ------------------------
   Fetch handler - strategies
   ------------------------ */
self.addEventListener("fetch", (event) => {
  const req = event.request;

  // Let non-GET fall through to network
  if (req.method !== "GET") return;

  // Accept header for HTML detection
  const accept = req.headers.get("Accept") || "";

  // Navigation requests (pages) => network-first (fresh), fallback to app-shell
  if (req.mode === "navigate" || accept.includes("text/html")) {
    event.respondWith(networkFirst(req));
    // non-blocking background update-check
    event.waitUntil((async () => {
      try { await checkRemoteAssetListAndUpdateIfNeeded(); } catch (e) { /* ignore */ }
    })());
    return;
  }

  // All other GET requests => cache-first, update in background
  event.respondWith(cacheFirstThenUpdate(req));
});

/* ------------------------
   Message handler (client -> SW)
   ------------------------ */
self.addEventListener("message", (event) => {
  const msg = event.data || {};
  if (msg && msg.type === "SKIP_WAITING") {
    self.skipWaiting();
    return;
  }
  if (msg && msg.type === "CHECK_FOR_UPDATE") {
    event.waitUntil(checkRemoteAssetListAndUpdateIfNeeded().catch(e => console.warn("sw: manual check failed:", e)));
    return;
  }
});

/* ------------------------
   Strategies
   ------------------------ */

async function networkFirst(request) {
  try {
    const networkResponse = await fetch(request);
    // store a copy in pages cache for offline fallback (best-effort)
    const current = await getHighestCachedVersion();
    if (current) {
      try {
        const pages = await caches.open(`${CACHE_PREFIX}${current}::pages`);
        await pages.put(request, networkResponse.clone());
      } catch (e) { /* ignore caching errors */ }
    }
    return networkResponse;
  } catch (err) {
    // Network failed -> try cache (exact match or app-shell fallback)
    const cached = await caches.match(request);
    if (cached) return cached;
    const scopeBase = ensureTrailingSlash(self.registration.scope || new URL('.', self.location).href);
    const indexMatch = await caches.match(scopeBase) || await caches.match(new URL('index.html', scopeBase).href) || await caches.match(new URL('404.html', scopeBase).href);
    if (indexMatch) return indexMatch;

    // final fallback HTML response
    return new Response("<h1>Service Unavailable</h1><p>Offline</p>", {
      status: 503,
      statusText: "Service Unavailable",
      headers: { "Content-Type": "text/html" }
    });
  }
}

async function cacheFirstThenUpdate(request) {
  // Try cache first
  const cached = await caches.match(request);
  // Kick off network fetch to update cache in background
  const fetchPromise = fetch(request).then(async (resp) => {
    if (!resp || !resp.ok) return resp;
    const current = await getHighestCachedVersion();
    if (current) {
      try {
        const pages = await caches.open(`${CACHE_PREFIX}${current}::pages`);
        await pages.put(request, resp.clone());
      } catch (e) { /* ignore */ }
    }
    return resp;
  }).catch(() => null);

  // If we have cached response return it immediately
  if (cached) {
    // still let fetchPromise run in background
    fetchPromise.catch(() => { });
    return cached;
  }

  // else wait for network attempt
  const netResp = await fetchPromise;
  if (netResp) return netResp;

  // If nothing available, provide a safe fallback:
  // - For images (including favicon) return 204 empty response to avoid console errors
  // - For other resources return 503
  const dest = request.destination || "";
  if (dest === "image" || request.url.endsWith("/favicon.ico")) {
    return new Response('', { status: 204 });
  }
  return new Response("", { status: 503, statusText: "Service Unavailable" });
}

/* ------------------------
   Remote asset-list utilities (parsing & update)
   ------------------------ */

/**
 * Fetch-and-parse cached_asset_list.js.
 * Returns { version: string, assets: string[] } with assets as absolute URLs.
 */
async function fetchRemoteAssetList() {
  const res = await fetch(REMOTE_ASSET_LIST, { cache: "no-store" });
  if (!res.ok) throw new Error("failed to fetch asset list: " + res.status);
  const text = await res.text();

  const versionMatch = text.match(/const\s+version\s*=\s*["']([^"']+)["']/);
  if (!versionMatch) throw new Error("version not found in asset list");
  const version = versionMatch[1];

  const arrMatch = text.match(/const\s+offlineFundamentals\s*=\s*(\[[\s\S]*?\]);/);
  if (!arrMatch) throw new Error("offlineFundamentals not found in asset list");

  let assets;
  try {
    assets = (new Function("return " + arrMatch[1]))();
    if (!Array.isArray(assets)) throw new Error("offlineFundamentals is not an array");
  } catch (e) {
    throw new Error("failed to parse offlineFundamentals array: " + e.message);
  }

  // Normalize assets to absolute URLs (resolve relative to the SW's location)
  const absolute = assets.map(a => new URL(a, self.location).href);
  return { version, assets: absolute };
}

/**
 * Get the lexicographically highest cached version (ISO timestamps sort lexicographically)
 */
async function getHighestCachedVersion() {
  const keys = await caches.keys();
  const versions = keys.map(k => {
    const m = k.match(new RegExp('^' + escapeRegExp(CACHE_PREFIX) + '(.+?)::fundamentals$'));
    return m ? m[1] : null;
  }).filter(Boolean);
  if (versions.length === 0) return null;
  versions.sort();
  return versions[versions.length - 1];
}

/**
 * If remote version differs from cached, prefetch remote assets into new cache,
 * delete old caches, and notify clients.
 */
async function checkRemoteAssetListAndUpdateIfNeeded() {
  const data = await fetchRemoteAssetList();
  const remoteVersion = data.version;
  const remoteAssets = data.assets;

  const current = await getHighestCachedVersion();
  if (current === remoteVersion) return { updated: false };

  const newFund = `${CACHE_PREFIX}${remoteVersion}::fundamentals`;
  const newPages = `${CACHE_PREFIX}${remoteVersion}::pages`;
  const fundCache = await caches.open(newFund);

  // best-effort: fetch each asset and cache it
  await Promise.all(remoteAssets.map(async (url) => {
    try {
      const r = await fetch(url, { cache: "no-store" });
      if (r && r.ok) await fundCache.put(url, r.clone());
      else console.warn("sw: update - asset fetch failed:", url, r && r.status);
    } catch (e) {
      console.warn("sw: update - asset fetch exception:", url, e);
    }
  }));

  // create pages cache for new version (empty)
  await caches.open(newPages);

  // remove other caches not matching new version
  const keys = await caches.keys();
  await Promise.all(keys
    .filter(k => !k.startsWith(`${CACHE_PREFIX}${remoteVersion}`))
    .map(k => caches.delete(k))
  );

  // notify clients to prompt reload if desired
  const allClients = await self.clients.matchAll({ includeUncontrolled: true });
  for (const client of allClients) {
    client.postMessage({ type: "NEW_VERSION_AVAILABLE", version: remoteVersion });
  }

  return { updated: true, newVersion: remoteVersion };
}

/* ------------------------
   Small helpers
   ------------------------ */

function escapeRegExp(string) {
  return string.replace(/[.*+\-?^${}()|[\]\\]/g, '\\$&');
}

function ensureTrailingSlash(s) {
  return s.endsWith('/') ? s : s + '/';
}

function dedupeArray(arr) {
  return Array.from(new Set(arr));
}

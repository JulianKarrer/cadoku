"use strict";

/**
 * Single, unchanging service worker.
 *
 * Expects a same-origin file `cached_asset_list.js` in the same scope that defines:
 *   const version = "<ISO-timestamp>";
 *   const offlineFundamentals = [ "path/to/asset", ... ];
 *
 * Behavior:
 * - install: try to fetch cached_asset_list.js and pre-cache the listed assets + app-shell.
 * - activate: keep only the latest versioned caches and claim clients.
 * - fetch:
 *    * network-first for the asset list & navigations (fallback to cached app-shell)
 *    * cache-first then update for other static assets
 * - update: fetch the remote asset list, if version differs download assets into new
 *   cache and delete old caches; postMessage clients {type: "NEW_VERSION_AVAILABLE", version}.
 */

/* ------------------------
   Configuration
   ------------------------ */
const CACHE_PREFIX = "cadoku-cache:"; // final caches: `${CACHE_PREFIX}${version}::fundamentals` or `::pages`
const REMOTE_ASSET_LIST = new URL("cached_asset_list.js", self.location).href; // resolved relative to this SW
const APP_SHELL_FILENAMES = ["", "index.html", "404.html", "manifest.json"]; // appended to scope base

/* ------------------------
   Install: prefetch asset list + assets (best-effort)
   ------------------------ */
self.addEventListener("install", (event) => {
  event.waitUntil((async () => {
    try {
      const data = await fetchRemoteAssetList(); // may throw
      const { version, assets } = data;
      const fundCacheName = `${CACHE_PREFIX}${version}::fundamentals`;
      const cache = await caches.open(fundCacheName);

      // Determine app-shell absolute URLs (scope base)
      const scopeBase = ensureTrailingSlash(self.registration?.scope || new URL('.', self.location).href);
      const appShell = APP_SHELL_FILENAMES.map(f => new URL(f, scopeBase).href);

      // Normalize asset urls and dedupe
      const normalized = dedupeArray(
        appShell.concat(assets.map(a => normalizeAssetUrl(a)))
      );

      // Best-effort: fetch and put each (so single 404 won't abort install)
      await Promise.all(normalized.map(async (url) => {
        try {
          const r = await fetch(url, { cache: "no-store", credentials: "same-origin" });
          if (r && r.ok) await cache.put(url, r.clone());
          else console.warn("sw: install - asset fetch failed", url, r && r.status);
        } catch (e) {
          console.warn("sw: install - asset fetch exception", url, e);
        }
      }));
    } catch (e) {
      // Non-fatal: allow worker to install so previously-cached version continues to work.
      console.warn("sw: install - could not prefetch asset list or assets:", e);
    }
    // do NOT call skipWaiting here — let client control activation unless you want aggressive takeover
  })());
});

/* ------------------------
   Activate: clean up old caches + claim clients; try background update
   ------------------------ */
self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    try {
      const current = await getHighestCachedVersion();
      if (current) {
        const keys = await caches.keys();
        await Promise.all(keys
          .filter(k => !k.startsWith(`${CACHE_PREFIX}${current}`))
          .map(k => caches.delete(k))
        );
      }
      await self.clients.claim();

      // Opportunistic: check remote list & update in background
      try { await checkRemoteAssetListAndUpdateIfNeeded(); } catch (e) { /* ignore */ }
    } catch (e) {
      console.warn("sw: activate error", e);
    }
  })());
});

/* ------------------------
   Fetch: strategies
   ------------------------ */
self.addEventListener("fetch", (event) => {
  const req = event.request;

  // Only handle GET requests in the SW
  if (req.method !== "GET") return;

  // If browser asks for root-level favicon.ico (often requested from the site root),
  // respond with a safe empty response (204) if we cannot serve it from cache/network.
  // We special-case it to avoid the Response-body-with-204 bug and noisy logs.
  if (req.url.endsWith("/favicon.ico")) {
    event.respondWith(handleFaviconRequest(req));
    return;
  }

  // Always treat the remote asset list script specially: prefer network, fallback to cache
  if (urlsEqualSansHash(req.url, REMOTE_ASSET_LIST)) {
    event.respondWith(networkFirst(req));
    return;
  }

  // Navigation requests — network-first (so users get fresh HTML) with offline app-shell fallback
  const accept = req.headers.get("Accept") || "";
  if (req.mode === "navigate" || accept.includes("text/html")) {
    event.respondWith(networkFirst(req));
    // Also start a background update-check (non-blocking)
    event.waitUntil(checkRemoteAssetListAndUpdateIfNeeded().catch(() => { }));
    return;
  }

  // Other requests — cache-first then update in background
  event.respondWith(cacheFirstThenUpdate(req));
});

/* ------------------------
   Message handling (client -> SW)
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
   Strategies implementations
   ------------------------ */

async function networkFirst(request) {
  try {
    const networkResponse = await fetch(request);
    // best-effort cache copy for pages fallback
    try {
      const current = await getHighestCachedVersion();
      if (current) {
        const pagesCache = await caches.open(`${CACHE_PREFIX}${current}::pages`);
        await pagesCache.put(request, networkResponse.clone());
      }
    } catch (e) { /* ignore caching errors */ }
    return networkResponse;
  } catch (err) {
    // network failed => try cache
    const cached = await caches.match(request);
    if (cached) return cached;

    // fallback to app-shell
    const scopeBase = ensureTrailingSlash(self.registration?.scope || new URL('.', self.location).href);
    const appShellMatch = await caches.match(scopeBase) ||
      await caches.match(new URL('index.html', scopeBase).href) ||
      await caches.match(new URL('404.html', scopeBase).href);
    if (appShellMatch) return appShellMatch;

    // final fallback HTML response
    return new Response("<h1>Service Unavailable</h1><p>Offline</p>", {
      status: 503,
      statusText: "Service Unavailable",
      headers: { "Content-Type": "text/html" }
    });
  }
}

async function cacheFirstThenUpdate(request) {
  // try cache first
  const cached = await caches.match(request);
  // start network update in background
  const fetchPromise = fetch(request).then(async (resp) => {
    if (!resp || !resp.ok) return resp;
    try {
      const current = await getHighestCachedVersion();
      if (current) {
        const pages = await caches.open(`${CACHE_PREFIX}${current}::pages`);
        await pages.put(request, resp.clone());
      }
    } catch (e) { /* ignore */ }
    return resp;
  }).catch(() => null);

  if (cached) {
    // let fetchPromise run but ignore result
    fetchPromise.catch(() => { });
    return cached;
  }

  // wait for network result
  const netResp = await fetchPromise;
  if (netResp) return netResp;

  // nothing available -> safe fallbacks
  const dest = request.destination || "";
  if (dest === "image" || request.url.endsWith("/favicon.ico")) {
    // 204 must have null body — use null to avoid Response constructor error
    return new Response(null, { status: 204, statusText: "No Content" });
  }

  // generic failure
  return new Response("", { status: 503, statusText: "Service Unavailable" });
}

/* ------------------------
   Asset-list parsing + update logic
   ------------------------ */

/**
 * Fetch cached_asset_list.js and parse:
 *   const version = "..."
 *   const offlineFundamentals = [ ... ];
 * Returns {version, assets: [absoluteURLs...]}
 */
async function fetchRemoteAssetList() {
  const res = await fetch(REMOTE_ASSET_LIST, { cache: "no-store", credentials: "same-origin" });
  if (!res.ok) throw new Error("failed to fetch asset list: " + res.status);
  const text = await res.text();

  const versionMatch = text.match(/const\s+version\s*=\s*["']([^"']+)["']/);
  if (!versionMatch) throw new Error("version not found in asset list");
  const version = versionMatch[1];

  const arrMatch = text.match(/const\s+offlineFundamentals\s*=\s*(\[[\s\S]*?\]);/);
  if (!arrMatch) throw new Error("offlineFundamentals not found in asset list");

  let assets;
  try {
    // Evaluate the array literal into JS array. This is acceptable only because this file
    // is same-origin and generated by your build process.
    assets = (new Function("return " + arrMatch[1]))();
    if (!Array.isArray(assets)) throw new Error("offlineFundamentals is not an array");
  } catch (e) {
    throw new Error("failed to parse offlineFundamentals array: " + e.message);
  }

  // Normalize each asset into an absolute URL:
  const absolute = assets.map(a => normalizeAssetUrl(a));
  return { version, assets: absolute };
}

/**
 * If remote version differs from cached, prefetch remote assets into a new cache,
 * delete old caches, and postMessage clients.
 */
async function checkRemoteAssetListAndUpdateIfNeeded() {
  let data;
  try {
    data = await fetchRemoteAssetList();
  } catch (e) {
    // if can't fetch or parse, abort
    throw e;
  }
  const remoteVersion = data.version;
  const remoteAssets = data.assets;

  const current = await getHighestCachedVersion();
  if (current === remoteVersion) return { updated: false };

  const newFund = `${CACHE_PREFIX}${remoteVersion}::fundamentals`;
  const newPages = `${CACHE_PREFIX}${remoteVersion}::pages`;
  const fundCache = await caches.open(newFund);

  // Best-effort: fetch each asset and cache it
  await Promise.all(remoteAssets.map(async (url) => {
    try {
      const r = await fetch(url, { cache: "no-store", credentials: "same-origin" });
      if (r && r.ok) {
        await fundCache.put(url, r.clone());
      } else {
        console.warn("sw: update - asset fetch failed:", url, r && r.status);
      }
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

  // notify clients (pages)
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

function ensureTrailingSlash(s) {
  return s.endsWith('/') ? s : s + '/';
}

function dedupeArray(arr) {
  return Array.from(new Set(arr));
}

/**
 * Normalize an asset entry from cached_asset_list.js into an absolute URL.
 * Rules:
 *  - If it already looks absolute (starts with 'http' or '//') -> leave it.
 *  - If it starts with '/' -> resolve against origin (absolute path on host).
 *  - Otherwise -> resolve relative to this SW's scope (so 'assets/..' -> '/cadoku/assets/..').
 */
function normalizeAssetUrl(a) {
  if (typeof a !== "string") return a;
  const trimmed = a.trim();
  if (/^https?:\/\//i.test(trimmed) || /^\/\//.test(trimmed)) {
    return trimmed;
  }
  if (trimmed.startsWith("/")) {
    // Absolute path on host
    return new URL(trimmed, self.location.origin).href;
  }
  // Relative entry -> resolve relative to service worker location (scope)
  return new URL(trimmed, self.location).href;
}

/**
 * Return lexicographically highest cached version (ISO timestamps sort lexicographically).
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
 * Compare URLs ignoring hash (since cache keys may differ by hash)
 */
function urlsEqualSansHash(a, b) {
  try {
    const ua = new URL(a, self.location);
    const ub = new URL(b, self.location);
    ua.hash = ""; ub.hash = "";
    return ua.href === ub.href;
  } catch (e) {
    return a === b;
  }
}

/**
 * Handle root-level favicon requests robustly.
 * Try network, then cache, else return 204 (no content).
 */
async function handleFaviconRequest(request) {
  try {
    const r = await fetch(request);
    if (r && r.ok) return r;
  } catch (e) { /* network failed */ }

  // try cache
  const cached = await caches.match(request);
  if (cached) return cached;

  // safe empty 204 with null body to avoid Response constructor error
  return new Response(null, { status: 204, statusText: "No Content" });
}

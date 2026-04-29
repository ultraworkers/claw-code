// Service worker — phase 2 stub.
//
// At the moment this exists so iOS Safari treats the page as installable
// and the manifest is honored on add-to-home-screen. We intentionally do
// NOT cache /api/* or /static/app.js so updates land instantly on reload
// during development.
//
// Phase 2 work (per queued-research-pwa-frontend-on-cloudflare):
//   - cache /static/* with a stale-while-revalidate strategy
//   - background sync for queued prompts when offline
//   - push notifications for long-running turns

const VERSION = 'v0.0.1';
const SHELL = ['/', '/static/styles.css', '/static/app.js', '/manifest.json'];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(VERSION).then((c) => c.addAll(SHELL).catch(() => {}))
  );
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(keys.filter((k) => k !== VERSION).map((k) => caches.delete(k)))
    )
  );
  self.clients.claim();
});

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  // Network-first for API + WS; never cache them.
  if (url.pathname.startsWith('/api/')) return;
  // Network-first for everything else; fall back to cache offline.
  event.respondWith(
    fetch(event.request).catch(() => caches.match(event.request))
  );
});

var CACHE_VERSION = 'pp-v1';
var STATIC_CACHE = CACHE_VERSION + '-static';
var DYNAMIC_CACHE = CACHE_VERSION + '-dynamic';

var PRECACHE_URLS = [
  '/',
  '/manifest.json',
  '/favicon.png',
  '/icons/icon-192.png',
  '/icons/icon-512.png'
];

var STATIC_EXTENSIONS = [
  '.js',
  '.css',
  '.png',
  '.jpg',
  '.jpeg',
  '.svg',
  '.ico',
  '.woff',
  '.woff2',
  '.ttf',
  '.webp'
];

function isStaticAsset(url) {
  return STATIC_EXTENSIONS.some(function (ext) {
    return url.pathname.endsWith(ext);
  });
}

function isNavigationRequest(request) {
  return request.mode === 'navigate';
}

self.addEventListener('install', function (event) {
  event.waitUntil(
    caches
      .open(STATIC_CACHE)
      .then(function (cache) {
        return cache.addAll(PRECACHE_URLS);
      })
      .then(function () {
        return self.skipWaiting();
      })
  );
});

self.addEventListener('activate', function (event) {
  event.waitUntil(
    caches
      .keys()
      .then(function (keys) {
        return Promise.all(
          keys
            .filter(function (key) {
              return (
                key.startsWith('pp-') &&
                key !== STATIC_CACHE &&
                key !== DYNAMIC_CACHE
              );
            })
            .map(function (key) {
              return caches.delete(key);
            })
        );
      })
      .then(function () {
        return self.clients.claim();
      })
      .then(function () {
        return self.clients.matchAll({ type: 'window' });
      })
      .then(function (clientList) {
        clientList.forEach(function (client) {
          client.postMessage({ type: 'SW_UPDATE_READY' });
        });
      })
  );
});

self.addEventListener('fetch', function (event) {
  var url = new URL(event.request.url);

  if (event.request.method !== 'GET') return;

  if (isNavigationRequest(event.request)) {
    event.respondWith(
      fetch(event.request)
        .then(function (response) {
          if (response.ok) {
            var clone = response.clone();
            caches.open(DYNAMIC_CACHE).then(function (cache) {
              cache.put(event.request, clone);
            });
          }
          return response;
        })
        .catch(function () {
          return caches.match(event.request).then(function (cached) {
            return cached || caches.match('/');
          });
        })
    );
    return;
  }

  if (event.request.destination === '') return;
  if (url.origin !== self.location.origin) return;

  if (isStaticAsset(url)) {
    event.respondWith(
      caches.match(event.request).then(function (cached) {
        if (cached) return cached;
        return fetch(event.request).then(function (response) {
          if (response.ok) {
            var clone = response.clone();
            caches.open(STATIC_CACHE).then(function (cache) {
              cache.put(event.request, clone);
            });
          }
          return response;
        });
      })
    );
    return;
  }

  event.respondWith(
    fetch(event.request)
      .then(function (response) {
        if (response.ok) {
          var clone = response.clone();
          caches.open(DYNAMIC_CACHE).then(function (cache) {
            cache.put(event.request, clone);
          });
        }
        return response;
      })
      .catch(function () {
        return caches.match(event.request);
      })
  );
});

self.addEventListener('push', function (event) {
  if (!event.data) return;
  var data = event.data.json();
  var title = data.title || 'Protocol Pawns';
  var options = {
    body: data.body || '',
    icon: '/icons/icon-192.png',
    badge: '/icons/icon-192.png',
    vibrate: [100, 50, 100],
    data: data.url ? { url: data.url } : undefined,
    actions: data.url ? [{ action: 'open', title: 'Open' }] : undefined
  };
  event.waitUntil(self.registration.showNotification(title, options));
});

self.addEventListener('notificationclick', function (event) {
  event.notification.close();
  var url = event.notification.data && event.notification.data.url;
  if (!url) return;
  event.waitUntil(
    self.clients
      .matchAll({ type: 'window', includeUncontrolled: true })
      .then(function (clientList) {
        for (var i = 0; i < clientList.length; i++) {
          if (clientList[i].url === url && 'focus' in clientList[i]) {
            return clientList[i].focus();
          }
        }
        return self.clients.openWindow(url);
      })
  );
});

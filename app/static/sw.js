self.addEventListener('push', function(event) {
	if (!event.data) return;
	var data = event.data.json();
	var title = data.title || 'Protocol Pawns';
	var options = {
		body: data.body || '',
		icon: '/icons/icon-192.png',
		badge: '/icons/icon-192.png',
		data: data.url ? { url: data.url } : undefined
	};
	event.waitUntil(self.registration.showNotification(title, options));
});

self.addEventListener('notificationclick', function(event) {
	event.notification.close();
	if (event.notification.data && event.notification.data.url) {
		event.waitUntil(clients.openWindow(event.notification.data.url));
	}
});

self.addEventListener('install', function() {
	self.skipWaiting();
});

self.addEventListener('activate', function() {
	self.clients.claim();
});

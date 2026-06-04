import { getConnector, contract } from './connector';
import { browser } from '$app/environment';
import { writable, derived } from 'svelte/store';
import { registerPushNotifications, unregisterPushNotifications } from '$lib/push/register';

export const accountStore = writable<string | undefined>(undefined);
export const isLoggedIn = derived(accountStore, ($a) => $a !== undefined);
export const isRegistered = writable(false);
export const isCheckingRegistration = writable(false);

if (browser) {
	const c = getConnector();
	c.on('wallet:signIn', async (payload) => {
		const accountId = payload.accounts?.[0]?.accountId;
		if (accountId) {
			accountStore.set(accountId);
			await checkRegistration(accountId);
			if ('serviceWorker' in navigator) {
				try {
					await navigator.serviceWorker.register('/sw.js');
					registerPushNotifications(accountId);
				} catch (e) {
					console.warn('SW registration failed:', e);
				}
			}
		}
	});
	c.on('wallet:signOut', async () => {
		const accountId = await new Promise<string | undefined>((resolve) => {
			const unsub = accountStore.subscribe((v) => { resolve(v); unsub(); });
		});
		if (accountId) unregisterPushNotifications(accountId);
		accountStore.set(undefined);
		isRegistered.set(false);
	});
}

async function checkRegistration(accountId: string) {
	isCheckingRegistration.set(true);
	try {
		const balance = await contract.storageBalanceOf(accountId);
		isRegistered.set(balance !== null);
	} catch {
		isRegistered.set(false);
	} finally {
		isCheckingRegistration.set(false);
	}
}

export async function connect() {
	const c = getConnector();
	await c.connect();
}

export async function disconnect() {
	const c = getConnector();
	await c.disconnect();
	accountStore.set(undefined);
	isRegistered.set(false);
}

export async function register() {
	isCheckingRegistration.set(true);
	try {
		await contract.storageDeposit();
		const accountId = await new Promise<string>((resolve) => {
			const unsub = accountStore.subscribe((v) => {
				if (v) { resolve(v); unsub(); }
			});
		});
		await checkRegistration(accountId);
	} finally {
		isCheckingRegistration.set(false);
	}
}

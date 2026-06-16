import { writable } from 'svelte/store';

export const bravePushModalOpen = writable(false);

export function openBravePushModal() {
  bravePushModalOpen.set(true);
}

export function closeBravePushModal() {
  bravePushModalOpen.set(false);
}

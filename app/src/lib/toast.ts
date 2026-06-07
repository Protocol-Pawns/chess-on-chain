import { writable } from 'svelte/store';

export interface Toast {
  id: number;
  type: 'success' | 'error' | 'info';
  message: string;
  detail?: string;
  link?: string;
}

let nextId = 0;

export const toasts = writable<Toast[]>([]);

export function showToast(
  type: Toast['type'],
  message: string,
  detail?: string,
  link?: string
) {
  const id = nextId++;
  toasts.update(list => [...list, { id, type, message, detail, link }]);
  setTimeout(() => {
    toasts.update(list => list.filter(t => t.id !== id));
  }, 8000);
}

export function dismissToast(id: number) {
  toasts.update(list => list.filter(t => t.id !== id));
}

export function decodeSuccessValue<T = unknown>(result: unknown): T | null {
  const status = (result as { status?: { SuccessValue?: string } } | undefined)
    ?.status;
  if (status?.SuccessValue) {
    return JSON.parse(atob(status.SuccessValue)) as T;
  }
  return null;
}

export function showTxToast(promise: Promise<unknown>) {
  showToast('info', 'Transaction pending...');
  promise
    .then((result: unknown) => {
      const tx = result as { transaction?: { hash?: string } } | undefined;
      const hash = tx?.transaction?.hash;
      showToast(
        'success',
        'Transaction confirmed!',
        undefined,
        hash ? `https://explorer.near.org/txns/${hash}` : undefined
      );
    })
    .catch((err: unknown) => {
      const msg = err instanceof Error ? err.message : String(err);
      showToast('error', 'Transaction failed', msg);
    });
}

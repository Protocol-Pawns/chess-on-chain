<script lang="ts">
  import { toasts, dismissToast } from '$lib/toast';
</script>

<div class="fixed bottom-2 right-2 z-50 space-y-2 max-w-[calc(100vw-1rem)]">
  {#each $toasts as toast (toast.id)}
    <div
      class="flex items-start gap-3 rounded border px-3 py-2 shadow-lg {toast.type ===
      'success'
        ? 'bg-primary-bgOk border-primary-green text-primary-green'
        : toast.type === 'error'
          ? 'bg-primary-bgErr border-primary-err text-primary-err'
          : 'bg-bg border-primary text-primary'}"
      style="border-width: 2px;"
    >
      <div class="flex-1 min-w-0">
        <div class="font-semibold text-sm">{toast.message}</div>
        {#if toast.detail}
          <div class="text-xs mt-0.5 opacity-75 truncate">{toast.detail}</div>
        {/if}
        {#if toast.link}
          <a
            href={toast.link}
            target="_blank"
            rel="noopener"
            class="text-xs underline opacity-75 hover:opacity-100"
          >
            View on Explorer
          </a>
        {/if}
      </div>
      <button
        onclick={() => dismissToast(toast.id)}
        class="opacity-50 hover:opacity-100 text-lg leading-none"
        >&times;</button
      >
    </div>
  {/each}
</div>

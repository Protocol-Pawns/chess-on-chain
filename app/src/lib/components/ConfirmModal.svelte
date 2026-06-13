<script lang="ts">
  import Modal from './Modal.svelte';

  interface Props {
    open: boolean;
    title: string;
    message: string;
    confirmLabel: string;
    confirmClass?: string;
    confirmDisabled?: boolean;
    onconfirm: () => void;
    onclose: () => void;
  }

  let {
    open,
    title,
    message,
    confirmLabel,
    confirmClass = 'btn text-sm text-white bg-primary-err hover:bg-primary-err/80',
    confirmDisabled = false,
    onconfirm,
    onclose
  }: Props = $props();
</script>

<Modal {open} {onclose}>
  <div class="card max-w-sm w-full bg-surface">
    <h3 class="text-base font-semibold mb-2">{title}</h3>
    <p class="text-sm text-white/70 mb-4">{message}</p>
    <div class="flex gap-2 justify-end">
      <button class="btn-secondary text-sm" onclick={onclose}>Cancel</button>
      <button
        class={confirmClass}
        onclick={onconfirm}
        disabled={confirmDisabled}
      >
        {#if confirmDisabled}
          <span class="inline-block animate-spin mr-1.5">⏳</span>
        {/if}
        {confirmLabel}
      </button>
    </div>
  </div>
</Modal>

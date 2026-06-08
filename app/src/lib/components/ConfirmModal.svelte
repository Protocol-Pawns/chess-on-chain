<script lang="ts">
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

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
    onclick={onclose}
  >
    <div
      class="card max-w-sm w-full mx-4 bg-[#1a1a2e]"
      onclick={e => e.stopPropagation()}
    >
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
  </div>
{/if}
